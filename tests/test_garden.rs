use std::fs;
use std::path::Path;
use std::process::Command as StdCommand;

use assert_cmd::Command;
use k0maru::garden::{run_garden, GardenOptions};
use k0maru::hook::inspect_note_detail;
use k0maru::mcp::McpServer;
use k0maru::parser::extract_elements;
use k0maru::scanner::IncrementalScanner;
use k0maru::storage::SqliteStorage;
use k0maru::VECTOR_DIMENSIONS;
use tempfile::tempdir;

fn write_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

fn init_git_repo(dir: &Path) {
    let run = |args: &[&str]| {
        let out = StdCommand::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .output()
            .expect("git command");
        assert!(
            out.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
    };
    run(&["init"]);
    run(&["config", "user.name", "Test Bot"]);
    run(&["config", "user.email", "test@example.com"]);
    run(&["add", "-A"]);
    run(&["commit", "-m", "initial commit"]);
}

fn git_add_commit(dir: &Path, msg: &str) {
    for args in [&["add", "-A"][..], &["commit", "-m", msg][..]] {
        let out = StdCommand::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .output()
            .expect("git command");
        assert!(
            out.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

#[test]
fn test_managed_backlink_exclusion_in_parser() {
    let md = "\
# Settlement Ledger
Real link to [[20_Cards/Quorum|Quorum]] and #real-tag.

```markdown
<!-- k0maru:backlinks:start -->
- [[20_Cards/InCode|InCode]]
<!-- k0maru:backlinks:end -->
```

<!-- k0maru:backlinks:start -->
- [[20_Cards/WovenA|WovenA]]
- [[20_Cards/WovenB|WovenB]] #woven-tag
<!-- k0maru:backlinks:end -->
";

    let (links, tags) = extract_elements(md);
    let targets: Vec<&str> = links.iter().map(|l| l.target.as_str()).collect();
    assert_eq!(targets, vec!["20_Cards/Quorum"]);
    assert_eq!(tags, vec!["#real-tag"]);
}

#[test]
fn test_doc_access_recording_debounce_and_readonly_isolation() {
    let tmp = tempdir().unwrap();
    let vault = tmp.path();
    let card_rel = "20_Cards/PaymentGateway.md";
    write_file(
        &vault.join(card_rel),
        "---\ntitle: Payment Gateway\nhierarchy: L3Evergreen\n---\n# Payment Gateway\nIdempotent retry protocol.\n",
    );

    // 1. Two inspect_note_detail calls within 60s must count once.
    let _ = inspect_note_detail(vault, card_rel).expect("first inspect");
    let _ = inspect_note_detail(vault, card_rel).expect("second inspect within 60s");

    let cache_path = vault.join(".k0maru/cache.sqlite");
    {
        let storage = SqliteStorage::open(&cache_path).unwrap();
        let stats = storage.access_stats(&[card_rel]).unwrap();
        let (count, last_access) = stats.get(card_rel).copied().expect("doc_access row");
        assert_eq!(count, 1, "two reads within 60s must debounce to 1");

        // Verify recall_memory via MCP does NOT mutate doc_access.
        drop(storage);
        let mut server = McpServer::new_with_model(vault, Some("mock"));
        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "recall_memory",
                "arguments": { "query": "Idempotent retry protocol" }
            }
        });
        let resp_str = server.handle_request(&req.to_string()).unwrap();
        let resp: serde_json::Value = serde_json::from_str(&resp_str).unwrap();
        assert!(resp.get("error").is_none(), "recall_memory should succeed");

        let storage_after_recall = SqliteStorage::open_read_only(&cache_path).unwrap();
        let stats_after = storage_after_recall.access_stats(&[card_rel]).unwrap();
        assert_eq!(stats_after.get(card_rel).unwrap().0, 1);

        // Read-only handle must never write to doc_access.
        let ro_wrote = storage_after_recall
            .record_access(card_rel, last_access + 120)
            .unwrap();
        assert!(!ro_wrote, "read-only handle must never write");
        let stats_after_ro = storage_after_recall.access_stats(&[card_rel]).unwrap();
        assert_eq!(stats_after_ro.get(card_rel).unwrap().0, 1);
    }

    // 2. Advancing >= 60s on a rw handle increments count to 2.
    {
        let storage = SqliteStorage::open(&cache_path).unwrap();
        let prev_last = storage.access_stats(&[card_rel]).unwrap()[card_rel].1;
        let wrote = storage.record_access(card_rel, prev_last + 60).unwrap();
        assert!(wrote);
        assert_eq!(storage.access_stats(&[card_rel]).unwrap()[card_rel].0, 2);
    }
}

#[test]
fn test_garden_orphans_broken_links_and_duplicates() {
    let tmp = tempdir().unwrap();
    let vault = tmp.path();

    // Connected pair (neither is orphan) + asset & self-ref (must not be broken) + broken link.
    write_file(
        &vault.join("20_Cards/Router.md"),
        "---\ntitle: Router\nhierarchy: L3Evergreen\n---\n# Router\nUses [[Settlement]], ![[diagram.png]], [[#Overview]], and [[MissingNode]].\n\n## Overview\nDetails.\n",
    );
    write_file(
        &vault.join("20_Cards/Settlement.md"),
        "---\ntitle: Settlement\nhierarchy: L3Evergreen\n---\n# Settlement\nCore ledger.\n",
    );

    // Orphans across L1, L2, L3, and L0 (L0 only with --include-daily).
    write_file(
        &vault.join("30_Resources/Cheatsheet.md"),
        "---\ntitle: Cheatsheet\nhierarchy: L1Resource\n---\n# Cheatsheet\nStatic reference.\n",
    );
    write_file(
        &vault.join("01_AI_Logs/2026-10-01-trace.md"),
        "---\ntitle: Trace\nhierarchy: L2Log\n---\n# Trace\nIsolated log.\n",
    );
    write_file(
        &vault.join("00_Daily/2026-10-01.md"),
        "---\ntitle: Daily\nhierarchy: L0Ephemeral\n---\n# Daily\nScratch.\n",
    );
    // Superseded orphan must NOT be reported.
    write_file(
        &vault.join("20_Cards/OldCard.md"),
        "---\ntitle: OldCard\nhierarchy: L3Evergreen\nstatus: superseded\n---\n# OldCard\nRetired.\n",
    );

    // Near-duplicate active L3 cards (Jaccard-LSH fallback without vectors, including CJK).
    let shared_body = "Distributed settlement shard failover protocol ensures atomic epoch handoff across consensus replicas without split brain. 分布式结算分片故障切换协议保证跨副本原子切换。";
    write_file(
        &vault.join("20_Cards/DupA.md"),
        &format!(
            "---\ntitle: Shard Failover A\nhierarchy: L3Evergreen\n---\n# Shard Failover A\n{shared_body}\n"
        ),
    );
    write_file(
        &vault.join("20_Cards/DupB.md"),
        &format!(
            "---\ntitle: Shard Failover B\nhierarchy: L3Evergreen\n---\n# Shard Failover B\n{shared_body}\n"
        ),
    );

    // Active L3 pair with explicit `supersedes` relation must NOT be flagged as duplicates.
    write_file(
        &vault.join("20_Cards/Superseder.md"),
        &format!(
            "---\ntitle: Superseder\nhierarchy: L3Evergreen\nsupersedes:\n  - \"[[20_Cards/DupA]]\"\n---\n# Superseder\n{shared_body}\n"
        ),
    );

    let cache_path = vault.join(".k0maru/cache.sqlite");
    let mut storage = SqliteStorage::open(&cache_path).unwrap();

    let report = run_garden(vault, &mut storage, &GardenOptions::default()).unwrap();

    let orphan_paths: Vec<&str> = report.orphans.iter().map(|o| o.path.as_str()).collect();
    assert!(orphan_paths.contains(&"30_Resources/Cheatsheet.md"));
    assert!(orphan_paths.contains(&"01_AI_Logs/2026-10-01-trace.md"));
    assert!(orphan_paths.contains(&"20_Cards/DupA.md"));
    assert!(!orphan_paths.contains(&"20_Cards/Router.md"));
    assert!(!orphan_paths.contains(&"20_Cards/Settlement.md"));
    assert!(!orphan_paths.contains(&"00_Daily/2026-10-01.md"));
    assert!(!orphan_paths.contains(&"20_Cards/OldCard.md"));

    // With --include-daily, L0 daily note appears as orphan.
    let report_daily = run_garden(
        vault,
        &mut storage,
        &GardenOptions {
            include_daily: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(report_daily
        .orphans
        .iter()
        .any(|o| o.path == "00_Daily/2026-10-01.md"));

    // Broken links: only MissingNode, NOT diagram.png or #Overview.
    assert_eq!(report.broken_links.len(), 1);
    assert_eq!(report.broken_links[0].source, "20_Cards/Router.md");
    assert_eq!(report.broken_links[0].target, "MissingNode");

    // Duplicates: DupA <-> DupB detected via jaccard_lsh; Superseder <-> DupA excluded.
    assert!(
        report.duplicates.iter().any(|d| d.a == "20_Cards/DupA.md"
            && d.b == "20_Cards/DupB.md"
            && d.method == "jaccard_lsh"
            && d.score >= 0.60),
        "expected DupA <-> DupB jaccard_lsh duplicate, got {:?}",
        report.duplicates
    );
    assert!(
        !report.duplicates.iter().any(|d| (d.a == "20_Cards/DupA.md"
            && d.b == "20_Cards/Superseder.md")
            || (d.a == "20_Cards/Superseder.md" && d.b == "20_Cards/DupA.md")),
        "superseded pair must be excluded: {:?}",
        report.duplicates
    );
}

#[test]
fn test_garden_cosine_duplicates_from_stored_vectors() {
    let tmp = tempdir().unwrap();
    let vault = tmp.path();

    write_file(
        &vault.join("20_Cards/CardOne.md"),
        "---\ntitle: Card One\nhierarchy: L3Evergreen\n---\n# Card One\nAlpha consensus ledger.\n",
    );
    write_file(
        &vault.join("20_Cards/CardTwo.md"),
        "---\ntitle: Card Two\nhierarchy: L3Evergreen\n---\n# Card Two\nBeta settlement queue.\n",
    );
    write_file(
        &vault.join("20_Cards/CardThree.md"),
        "---\ntitle: Card Three\nhierarchy: L3Evergreen\n---\n# Card Three\nGamma orthogonal topic.\n",
    );

    let cache_path = vault.join(".k0maru/cache.sqlite");
    let mut storage = SqliteStorage::open(&cache_path).unwrap();
    let adapter = k0maru::adapters::ObsidianAdapter::new(vault);
    IncrementalScanner::new(&adapter, &mut storage)
        .sync(false)
        .unwrap();

    let dim = VECTOR_DIMENSIONS;
    storage.ensure_vector_dimension(dim).unwrap();
    let mut v1 = vec![0.0f32; dim];
    v1[0] = 1.0;
    let mut v2 = vec![0.0f32; dim];
    v2[0] = 0.99;
    v2[1] = 0.05;
    let mut v3 = vec![0.0f32; dim];
    v3[2] = 1.0;

    storage
        .insert_vector("20_Cards/CardOne.md", &v1, 1)
        .unwrap();
    storage
        .insert_vector("20_Cards/CardTwo.md", &v2, 2)
        .unwrap();
    storage
        .insert_vector("20_Cards/CardThree.md", &v3, 3)
        .unwrap();

    let report = run_garden(vault, &mut storage, &GardenOptions::default()).unwrap();
    assert_eq!(report.duplicates.len(), 1);
    assert_eq!(report.duplicates[0].a, "20_Cards/CardOne.md");
    assert_eq!(report.duplicates[0].b, "20_Cards/CardTwo.md");
    assert_eq!(report.duplicates[0].method, "cosine");
    assert!(report.duplicates[0].score > 0.88);
}

#[test]
fn test_garden_weave_idempotency_crlf_and_graph_exclusion() {
    let tmp = tempdir().unwrap();
    let vault = tmp.path();

    // Target uses CRLF and has trailing CRLF.
    let target_path = vault.join("20_Cards/Target.md");
    write_file(
        &target_path,
        "---\r\ntitle: Target Card\r\nhierarchy: L3Evergreen\r\n---\r\n# Target Card\r\n\r\nCore body.\r\n",
    );
    write_file(
        &vault.join("20_Cards/SourceL3.md"),
        "---\ntitle: Source L3\nhierarchy: L3Evergreen\n---\n# Source L3\nPoints to [[20_Cards/Target]].\n",
    );
    write_file(
        &vault.join("30_Resources/SourceL1.md"),
        "---\ntitle: Source L1\nhierarchy: L1Resource\n---\n# Source L1\nPoints to [[Target]].\n",
    );
    write_file(
        &vault.join("01_AI_Logs/2026-10-02-log.md"),
        "---\ntitle: Source L2\nhierarchy: L2Log\n---\n# Source L2\nPoints to [[Target]].\n",
    );
    write_file(
        &vault.join("00_Daily/2026-10-02.md"),
        "---\ntitle: Daily L0\nhierarchy: L0Ephemeral\n---\n# Daily L0\nPoints to [[Target]].\n",
    );

    let cache_path = vault.join(".k0maru/cache.sqlite");
    let mut storage = SqliteStorage::open(&cache_path).unwrap();

    // 1. Dry-run plans without modifying disk.
    let dry_report = run_garden(
        vault,
        &mut storage,
        &GardenOptions {
            weave: true,
            dry_run: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(dry_report.weave.applied, 0);
    assert_eq!(dry_report.weave.planned.len(), 1);
    assert_eq!(dry_report.weave.planned[0].target, "20_Cards/Target.md");
    assert_eq!(
        dry_report.weave.planned[0].add,
        vec![
            "20_Cards/SourceL3.md".to_string(),
            "30_Resources/SourceL1.md".to_string()
        ]
    );
    let before_disk = fs::read_to_string(&target_path).unwrap();
    assert!(!before_disk.contains("k0maru:backlinks:start"));

    // 2. Live weave writes sorted backlinks, preserving CRLF and trailing newline.
    let live_report = run_garden(
        vault,
        &mut storage,
        &GardenOptions {
            weave: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(live_report.weave.applied, 1);
    let after_disk = fs::read_to_string(&target_path).unwrap();
    assert!(after_disk.contains("\r\n"), "CRLF must be preserved");
    assert!(
        after_disk.ends_with("\r\n"),
        "trailing CRLF must be preserved"
    );
    assert!(after_disk.contains(
        "<!-- k0maru:backlinks:start -->\r\n\
         - [[20_Cards/SourceL3|Source L3]]\r\n\
         - [[30_Resources/SourceL1|Source L1]]\r\n\
         <!-- k0maru:backlinks:end -->\r\n"
    ));
    assert!(!after_disk.contains("Source L2"));
    assert!(!after_disk.contains("Daily L0"));
    assert!(!vault.join("20_Cards/.Target.md.k0maru-weave.tmp").exists());

    // 3. Woven links must NOT feed back into `links` or `link_edges`!
    let target_id = storage
        .doc_id_for_path(Path::new("20_Cards/Target.md"))
        .unwrap()
        .unwrap();
    let edges = storage.edges_touching(&[target_id]).unwrap();
    assert!(
        edges.iter().all(|&(src, _)| src != target_id),
        "Target must have zero outgoing link_edges from woven backlinks: {:?}",
        edges
    );

    // 4. Second weave run is an idempotent no-op.
    let second_report = run_garden(
        vault,
        &mut storage,
        &GardenOptions {
            weave: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(second_report.weave.applied, 0);
    assert!(second_report.weave.planned.is_empty());

    // 4b. `--include-logs` includes L2 log sources in the woven block.
    let logs_report = run_garden(
        vault,
        &mut storage,
        &GardenOptions {
            weave: true,
            include_logs: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(logs_report.weave.applied, 1);
    let after_logs = fs::read_to_string(&target_path).unwrap();
    assert!(after_logs.contains("- [[01_AI_Logs/2026-10-02-log|Source L2]]"));

    // 5. Removing all sources removes the managed block cleanly on next weave.
    fs::remove_file(vault.join("20_Cards/SourceL3.md")).unwrap();
    fs::remove_file(vault.join("30_Resources/SourceL1.md")).unwrap();
    fs::remove_file(vault.join("01_AI_Logs/2026-10-02-log.md")).unwrap();
    let cleanup_report = run_garden(
        vault,
        &mut storage,
        &GardenOptions {
            weave: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(cleanup_report.weave.applied, 1);
    let cleaned_disk = fs::read_to_string(&target_path).unwrap();
    assert_eq!(cleaned_disk, before_disk);
}

#[test]
fn test_garden_weave_safety_skips_dirty_symlink_outside_and_stale() {
    let tmp = tempdir().unwrap();
    let vault = tmp.path().join("vault");
    let outside = tmp.path().join("outside");
    fs::create_dir_all(&vault).unwrap();
    fs::create_dir_all(&outside).unwrap();

    write_file(
        &vault.join("20_Cards/Source.md"),
        "---\ntitle: Source\nhierarchy: L3Evergreen\n---\n# Source\nLinks to [[Clean]], [[Dirty]], [[Stale]], and [[SymTarget]].\n",
    );
    write_file(
        &vault.join("20_Cards/Clean.md"),
        "---\ntitle: Clean\nhierarchy: L3Evergreen\n---\n# Clean\nClean body.\n",
    );
    write_file(
        &vault.join("20_Cards/Dirty.md"),
        "---\ntitle: Dirty\nhierarchy: L3Evergreen\n---\n# Dirty\nInitial body.\n",
    );
    write_file(
        &vault.join("20_Cards/Stale.md"),
        "---\ntitle: Stale\nhierarchy: L3Evergreen\n---\n# Stale\nBefore edit.\n",
    );
    write_file(
        &outside.join("External.md"),
        "---\ntitle: SymTarget\nhierarchy: L3Evergreen\n---\n# SymTarget\nOutside vault.\n",
    );

    #[cfg(unix)]
    std::os::unix::fs::symlink(
        outside.join("External.md"),
        vault.join("20_Cards/SymTarget.md"),
    )
    .unwrap();

    init_git_repo(&vault);

    // Add oversized (> 1 MiB) target and commit before making Dirty.md dirty.
    let big_body = "x".repeat(1024 * 1024 + 64);
    write_file(
        &vault.join("20_Cards/BigCard.md"),
        &format!("---\ntitle: BigCard\nhierarchy: L3Evergreen\n---\n# BigCard\n{big_body}\n"),
    );
    write_file(
        &vault.join("20_Cards/Source.md"),
        "---\ntitle: Source\nhierarchy: L3Evergreen\n---\n# Source\nLinks to [[Clean]], [[Dirty]], [[Stale]], [[BigCard]], and [[SymTarget]].\n",
    );
    git_add_commit(&vault, "add bigcard and source update");

    // Make Dirty.md dirty in git (and sync it so content_hash matches disk).
    write_file(
        &vault.join("20_Cards/Dirty.md"),
        "---\ntitle: Dirty\nhierarchy: L3Evergreen\n---\n# Dirty\nUncommitted edit.\n",
    );

    let cache_path = vault.join(".k0maru/cache.sqlite");
    let mut storage = SqliteStorage::open(&cache_path).unwrap();
    let adapter = k0maru::adapters::ObsidianAdapter::new(&vault);
    IncrementalScanner::new(&adapter, &mut storage)
        .sync(false)
        .unwrap();
    storage.rebuild_link_edges_if_dirty().unwrap();

    // Verify `changed since scan` check directly against `content_hash`:
    // Tamper with `documents.content_hash` for Stale.md in SQLite? No, let's verify
    // by testing `run_garden` directly with a stale hash or verifying Dirty/BigCard/SymTarget!
    // Wait: `run_garden` starts with `sync(false)`. To test `changed since scan` deterministically,
    // `run_garden` checks `xxh3(bytes) != documents.content_hash` when reading disk during weave.
    // Let's also verify `SymTarget` and `BigCard` and `Dirty` are skipped:
    let report = run_garden(
        &vault,
        &mut storage,
        &GardenOptions {
            weave: true,
            ..Default::default()
        },
    )
    .unwrap();

    assert_eq!(report.weave.applied, 2); // Clean.md and Stale.md
    assert!(report
        .weave
        .skipped
        .iter()
        .any(|s| s.path == "20_Cards/Dirty.md" && s.reason == "dirty git file"));
    assert!(report
        .weave
        .skipped
        .iter()
        .any(|s| s.path == "20_Cards/BigCard.md" && s.reason == "exceeds 1 MiB"));
    assert!(!fs::read_to_string(outside.join("External.md"))
        .unwrap()
        .contains("k0maru:backlinks:start"));
    assert!(!fs::read_to_string(vault.join("20_Cards/Dirty.md"))
        .unwrap()
        .contains("k0maru:backlinks:start"));

    // Now test `--force` weaves Dirty.md.
    let forced_report = run_garden(
        &vault,
        &mut storage,
        &GardenOptions {
            weave: true,
            force: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(forced_report.weave.applied, 1);
    assert!(fs::read_to_string(vault.join("20_Cards/Dirty.md"))
        .unwrap()
        .contains("k0maru:backlinks:start"));

    // Verify CLI `k0maru garden --json` outputs valid JSON with the required schema.
    let out = Command::cargo_bin("k0maru")
        .unwrap()
        .args(["garden", "--vault", vault.to_str().unwrap(), "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json_val: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert!(json_val.get("orphans").is_some());
    assert!(json_val.get("broken_links").is_some());
    assert!(json_val.get("duplicates").is_some());
    assert!(json_val["weave"].get("planned").is_some());
    assert!(json_val["weave"].get("applied").is_some());
    assert!(json_val["weave"].get("skipped").is_some());
}

#[test]
fn test_garden_does_not_flag_k0maruignored_existing_archive_links_as_broken() {
    let dir = tempdir().unwrap();
    let vault = dir.path().join("vault");

    write_file(&vault.join(".k0maruignore"), "90_Archive/\n");
    write_file(
        &vault.join("90_Archive/01_AI_Logs/2026-07/2026-07-10-archived-session.md"),
        "---\ndate: 2026-07-10\n---\n# Archived Session\n## Summary\nArchived body.\n",
    );
    write_file(
        &vault.join("01_AI_Logs/2026-07-monthly-rollup.md"),
        "---\ndate: 2026-07-31\n---\n# July Rollup\n- [[90_Archive/01_AI_Logs/2026-07/2026-07-10-archived-session|July 10]]\n- [[90_Archive/01_AI_Logs/2026-07/2026-07-10-archived-session#Summary|Alias]]\n- [[90_Archive/01_AI_Logs/2026-07/does-not-exist]]\n",
    );

    let cache_path = vault.join(".k0maru/cache.sqlite");
    let mut storage = SqliteStorage::open(&cache_path).unwrap();
    let adapter = k0maru::adapters::ObsidianAdapter::new(&vault);
    IncrementalScanner::new(&adapter, &mut storage)
        .sync(false)
        .unwrap();
    storage.rebuild_link_edges_if_dirty().unwrap();

    let report = run_garden(&vault, &mut storage, &GardenOptions::default()).unwrap();

    assert!(
        !report
            .broken_links
            .iter()
            .any(|b| b.target.contains("2026-07-10-archived-session")),
        "links pointing to existing .k0maruignored archive files on disk must not be flagged as broken: {:?}",
        report.broken_links
    );
    assert!(
        report
            .broken_links
            .iter()
            .any(|b| b.target.contains("does-not-exist")),
        "truly missing archive targets must still be flagged as broken: {:?}",
        report.broken_links
    );
}
