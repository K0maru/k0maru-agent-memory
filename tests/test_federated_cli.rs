mod common;

use std::fs;
use std::path::Path;
use std::process::{Command as StdCommand, Stdio};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, SystemTime};

use assert_cmd::Command;
use k0maru::handoff::{HandoffRecord, HandoffState, HandoffStore};
use k0maru::{
    loadout::estimate_tokens, EmbeddingEngine, IncrementalScanner, MockEmbeddingEngine,
    ObsidianAdapter, SqliteStorage, VectorSyncEngine,
};
use serde_json::Value;
use tempfile::tempdir;

#[derive(Debug, Clone, PartialEq, Eq)]
struct DbFileSnapshot {
    sqlite_len: u64,
    sqlite_mtime: Option<SystemTime>,
    wal_len: u64,
}

fn snapshot_sqlite_files(vault_path: &Path) -> DbFileSnapshot {
    let k0maru_dir = vault_path.join(".k0maru");
    let sqlite_path = k0maru_dir.join("cache.sqlite");
    let wal_path = k0maru_dir.join("cache.sqlite-wal");

    let sqlite_meta = fs::metadata(&sqlite_path).expect("cache.sqlite must exist");
    let wal_len = fs::metadata(&wal_path).map(|m| m.len()).unwrap_or(0);

    DbFileSnapshot {
        sqlite_len: sqlite_meta.len(),
        sqlite_mtime: sqlite_meta.modified().ok(),
        wal_len,
    }
}

fn init_synced_vault(root: &Path, notes: &[(&str, &str)], embedder: Option<&MockEmbeddingEngine>) {
    for (rel, content) in notes {
        let full = root.join(rel);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(full, content).unwrap();
    }

    let cache_path = root.join(".k0maru").join("cache.sqlite");
    let mut storage = SqliteStorage::open_rw(&cache_path).unwrap();
    if let Some(emb) = embedder {
        storage.ensure_vector_dimension(emb.dimension()).unwrap();
    }
    let adapter = ObsidianAdapter::new(root);
    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    scanner.sync(false).unwrap();
    if let Some(emb) = embedder {
        VectorSyncEngine::sync(&mut storage, emb).unwrap();
        VectorSyncEngine::new()
            .sync_sections(&mut storage, emb, None)
            .unwrap();
    }
}

#[test]
fn test_cli_federated_search_global_and_readonly_proof() {
    let primary_dir = tempdir().unwrap();
    let global_dir = tempdir().unwrap();

    init_synced_vault(
        primary_dir.path(),
        &[
            (
                "10_Projects/payment-gateway.md",
                "---\ntitle: Payment Gateway\nhierarchy: L1Resource\n---\n# Payment Gateway\n\n> Core settlement service.\n\n- [[20_Cards/LocalIdempotency]]\n",
            ),
            (
                "20_Cards/LocalIdempotency.md",
                "---\ntitle: Local Idempotency\nhierarchy: L3Evergreen\n---\n# Local Idempotency\n\nIdempotency keys expire after 24 hours in payment-gateway.\n",
            ),
            (
                "20_Cards/SharedPolicy.md",
                "---\ntitle: Shared Policy\nhierarchy: L3Evergreen\n---\n# Shared Policy\n\nIdentical content-hash deduplication test note.\n",
            ),
        ],
        None,
    );

    init_synced_vault(
        global_dir.path(),
        &[
            (
                "20_Cards/GlobalQuorum.md",
                "---\ntitle: Global Quorum\nhierarchy: L3Evergreen\n---\n# Global Quorum\n\n## Quorum Rules\nDistributed settlement quorum requires three independent replicas and idempotency keys.\n",
            ),
            (
                "20_Cards/SharedPolicy.md",
                "---\ntitle: Shared Policy\nhierarchy: L3Evergreen\n---\n# Shared Policy\n\nIdentical content-hash deduplication test note.\n",
            ),
            (
                "20_Cards/GlobalBacklinker.md",
                "---\ntitle: Global Backlinker\nhierarchy: L3Evergreen\n---\n# Global Backlinker\n\nLinks to [[GlobalQuorum]].\n",
            ),
        ],
        None,
    );

    let before = snapshot_sqlite_files(global_dir.path());
    // Brief sleep so any accidental write would observably shift mtime.
    thread::sleep(Duration::from_millis(20));

    // 1. Federated search with --global-vault returns @global: locators and dedups SharedPolicy.
    let out = Command::cargo_bin("k0maru")
        .unwrap()
        .args([
            "search",
            "quorum replicas idempotency",
            "--vault",
            primary_dir.path().to_str().unwrap(),
            "--global-vault",
            global_dir.path().to_str().unwrap(),
            "--mode",
            "bm25",
            "--limit",
            "10",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "search failed: {:?}", out);
    let hits: Vec<Value> = serde_json::from_slice(&out.stdout).unwrap();
    assert!(
        hits.iter()
            .any(|h| h["path"] == "20_Cards/GlobalQuorum.md" && h["scope"] == "global"),
        "expected global hit in JSON output: {:?}",
        hits
    );

    // Human output renders @global: locator.
    let human_out = Command::cargo_bin("k0maru")
        .unwrap()
        .args([
            "search",
            "quorum replicas",
            "--vault",
            primary_dir.path().to_str().unwrap(),
            "--global-vault",
            global_dir.path().to_str().unwrap(),
            "--mode",
            "bm25",
        ])
        .output()
        .unwrap();
    assert!(human_out.status.success());
    let human_str = String::from_utf8_lossy(&human_out.stdout);
    assert!(
        human_str.contains("@global:20_Cards/GlobalQuorum.md"),
        "expected @global: locator in human output, got:\n{}",
        human_str
    );

    // 2. Content-hash dedup: SharedPolicy exists in both with identical hash -> primary wins.
    let dedup_out = Command::cargo_bin("k0maru")
        .unwrap()
        .args([
            "search",
            "deduplication test note",
            "--vault",
            primary_dir.path().to_str().unwrap(),
            "--global-vault",
            global_dir.path().to_str().unwrap(),
            "--mode",
            "bm25",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(dedup_out.status.success());
    let dedup_hits: Vec<Value> = serde_json::from_slice(&dedup_out.stdout).unwrap();
    let shared_hits: Vec<&Value> = dedup_hits
        .iter()
        .filter(|h| h["path"] == "20_Cards/SharedPolicy.md")
        .collect();
    assert_eq!(
        shared_hits.len(),
        1,
        "duplicate hash must collapse to 1 hit"
    );
    assert!(
        shared_hits[0]["scope"].is_null(),
        "primary scope must win dedup"
    );

    // 3. K0MARU_NO_GLOBAL=1 suppresses K0MARU_GLOBAL_VAULT and --global-vault, preserving primary-only scores.
    let no_global_out = Command::cargo_bin("k0maru")
        .unwrap()
        .env("K0MARU_NO_GLOBAL", "1")
        .env("K0MARU_GLOBAL_VAULT", global_dir.path())
        .args([
            "search",
            "quorum replicas idempotency",
            "--vault",
            primary_dir.path().to_str().unwrap(),
            "--global-vault",
            global_dir.path().to_str().unwrap(),
            "--mode",
            "bm25",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(no_global_out.status.success());
    let plain_out = Command::cargo_bin("k0maru")
        .unwrap()
        .env_remove("K0MARU_GLOBAL_VAULT")
        .env_remove("K0MARU_NO_GLOBAL")
        .args([
            "search",
            "quorum replicas idempotency",
            "--vault",
            primary_dir.path().to_str().unwrap(),
            "--mode",
            "bm25",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(plain_out.status.success());
    assert_eq!(
        serde_json::from_slice::<Value>(&no_global_out.stdout).unwrap(),
        serde_json::from_slice::<Value>(&plain_out.stdout).unwrap(),
        "with no global, scores and hits must match Gate 3 identically"
    );

    // 4. Missing global cache warns on stderr and continues without failing.
    let missing_global = tempdir().unwrap();
    let missing_out = Command::cargo_bin("k0maru")
        .unwrap()
        .args([
            "search",
            "idempotency",
            "--vault",
            primary_dir.path().to_str().unwrap(),
            "--global-vault",
            missing_global.path().to_str().unwrap(),
            "--mode",
            "bm25",
        ])
        .output()
        .unwrap();
    assert!(missing_out.status.success());
    let stderr_str = String::from_utf8_lossy(&missing_out.stderr);
    assert!(
        stderr_str.contains("warning: global vault cache not found"),
        "expected missing global cache warning on stderr, got: {}",
        stderr_str
    );

    // 5. Global == Primary is ignored cleanly.
    let self_global_out = Command::cargo_bin("k0maru")
        .unwrap()
        .args([
            "search",
            "idempotency",
            "--vault",
            primary_dir.path().to_str().unwrap(),
            "--global-vault",
            primary_dir.path().to_str().unwrap(),
            "--mode",
            "bm25",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(self_global_out.status.success());
    let self_hits: Vec<Value> = serde_json::from_slice(&self_global_out.stdout).unwrap();
    assert!(self_hits.iter().all(|h| h["scope"].is_null()));

    // 6. Flush writes ONLY to primary vault, never touching global vault.
    let flush_out = Command::cargo_bin("k0maru")
        .unwrap()
        .env("K0MARU_GLOBAL_VAULT", global_dir.path())
        .args([
            "flush",
            "--vault",
            primary_dir.path().to_str().unwrap(),
            "--category",
            "log",
            "--title",
            "Primary Flush Only",
            "--content",
            "Written strictly to primary vault.",
        ])
        .output()
        .unwrap();
    assert!(flush_out.status.success());
    assert!(!global_dir.path().join("01_AI_Logs").exists());

    // 7. Read-only proof: global cache.sqlite mtime/size and -wal size are unchanged.
    let after = snapshot_sqlite_files(global_dir.path());
    assert_eq!(before.sqlite_len, after.sqlite_len);
    assert_eq!(before.sqlite_mtime, after.sqlite_mtime);
    assert_eq!(before.wal_len, after.wal_len);
}

#[test]
fn test_cli_federated_loadout_guard_and_doctor() {
    let repo_dir = tempdir().unwrap();
    let primary_vault = repo_dir.path().join(".k0maru").join("wiki");
    let global_dir = tempdir().unwrap();

    // Initialize git repo for guard diff test.
    StdCommand::new("git")
        .args(["init"])
        .current_dir(repo_dir.path())
        .output()
        .unwrap();
    StdCommand::new("git")
        .args([
            "config",
            "user.email",
            "93422639+K0maru@users.noreply.github.com",
        ])
        .current_dir(repo_dir.path())
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["config", "user.name", "Test Bot"])
        .current_dir(repo_dir.path())
        .output()
        .unwrap();
    fs::create_dir_all(repo_dir.path().join("src")).unwrap();
    fs::write(repo_dir.path().join("src/lib.rs"), "pub fn ok() {}\n").unwrap();
    StdCommand::new("git")
        .args(["add", "src/lib.rs"])
        .current_dir(repo_dir.path())
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(repo_dir.path())
        .output()
        .unwrap();

    let emb_primary = MockEmbeddingEngine::new(384);
    init_synced_vault(
        &primary_vault,
        &[
            (
                "10_Projects/payment-gateway.md",
                "---\ntitle: Payment Gateway\nhierarchy: L1Resource\n---\n# Payment Gateway\n\n> Distributed payment settlement engine.\n",
            ),
            (
                "20_Cards/adr-primary.md",
                "---\ntitle: Primary Override\nhierarchy: L3Evergreen\ninvariants:\n  - id: ADR-SHARED-001\n    title: Primary overrides global\n    file_pattern: \"*.rs\"\n    forbidden: \"primary_forbidden_call\"\n    remediation: \"Use primary_safe_call instead.\"\n---\n# Primary Override\n",
            ),
        ],
        Some(&emb_primary),
    );

    let emb_global = MockEmbeddingEngine::new(384);
    init_synced_vault(
        global_dir.path(),
        &[
            (
                "10_Projects/global-only-proj.md",
                "---\ntitle: Global Only Project\nhierarchy: L1Resource\n---\n# Global Only Project\n\n> Must never resolve as primary project.\n",
            ),
            (
                "20_Cards/GlobalSettlementInvariant.md",
                "---\ntitle: Global Settlement Invariant\nhierarchy: L3Evergreen\ninvariants:\n  - id: ADR-GLOBAL-002\n    title: Global lock required\n    file_pattern: \"*.rs\"\n    forbidden: \"global_unsafe_call\"\n    remediation: \"Use fenced_settlement_lock.\"\n  - id: ADR-SHARED-001\n    title: Shadowed global rule\n    file_pattern: \"*.rs\"\n    forbidden: \"primary_forbidden_call\"\n    remediation: \"Shadowed global remediation.\"\n---\n# Global Settlement Invariant\n\n## Core Concept\n> Every payment-gateway settlement mutation must hold a fenced lease.\n",
            ),
        ],
        Some(&emb_global),
    );

    // Set global embed_model to a different model and backdate last_sync_at > 7d to test doctor warnings.
    {
        let cache_path = global_dir.path().join(".k0maru").join("cache.sqlite");
        let storage = SqliteStorage::open_rw(&cache_path).unwrap();
        storage
            .meta_set("embed_model", "different-embedding-model")
            .unwrap();
        storage.meta_set("last_sync_at", "1000").unwrap();
    }

    let before = snapshot_sqlite_files(global_dir.path());
    thread::sleep(Duration::from_millis(20));

    // 1. Loadout: primary project resolves + Global invariants appended <= 300 tokens.
    let loadout_out = Command::cargo_bin("k0maru")
        .unwrap()
        .args([
            "loadout",
            "payment-gateway",
            "--vault",
            primary_vault.to_str().unwrap(),
            "--global-vault",
            global_dir.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(loadout_out.status.success(), "{:?}", loadout_out);
    let loadout_md = String::from_utf8_lossy(&loadout_out.stdout);
    assert!(
        loadout_md.contains("## Global invariants"),
        "loadout should include Global invariants section:\n{}",
        loadout_md
    );
    assert!(loadout_md.contains("@global:20_Cards/GlobalSettlementInvariant.md"));
    assert!(
        estimate_tokens(&loadout_md) <= 300,
        "loadout exceeded 300-token budget: {}",
        estimate_tokens(&loadout_md)
    );

    // Global-only project must NOT resolve in primary loadout.
    let global_only_out = Command::cargo_bin("k0maru")
        .unwrap()
        .args([
            "loadout",
            "global-only-proj",
            "--vault",
            primary_vault.to_str().unwrap(),
            "--global-vault",
            global_dir.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!global_only_out.status.success());

    // 2. Guard check: primary overrides ADR-SHARED-001 (no [global] tag), ADR-GLOBAL-002 carries [global].
    fs::write(
        repo_dir.path().join("src/lib.rs"),
        "pub fn bad() { primary_forbidden_call(); global_unsafe_call(); }\n",
    )
    .unwrap();
    let guard_out = Command::cargo_bin("k0maru")
        .unwrap()
        .current_dir(repo_dir.path())
        .args([
            "guard",
            "check",
            "--diff",
            "--vault",
            primary_vault.to_str().unwrap(),
            "--global-vault",
            global_dir.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(guard_out.status.code(), Some(1));
    let guard_stdout = String::from_utf8_lossy(&guard_out.stdout);
    assert!(
        guard_stdout.contains("[DENY] ADR-GLOBAL-002 [global]:"),
        "expected [global] tag on global-origin rule:\n{}",
        guard_stdout
    );
    assert!(
        guard_stdout.contains("[DENY] ADR-SHARED-001:"),
        "expected overridden primary rule without [global] tag:\n{}",
        guard_stdout
    );
    assert!(
        guard_stdout.contains("Use primary_safe_call instead."),
        "primary remediation must win over shadowed global rule:\n{}",
        guard_stdout
    );

    // 3. Doctor: reports global vault path, user_version, embed_model mismatch, and stale sync (>7d).
    let doctor_out = Command::cargo_bin("k0maru")
        .unwrap()
        .args([
            "doctor",
            "--vault",
            primary_vault.to_str().unwrap(),
            "--global-vault",
            global_dir.path().to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();
    assert!(doctor_out.status.success());
    let doctor_json: Value = serde_json::from_slice(&doctor_out.stdout).unwrap();
    let items = doctor_json["items"].as_array().unwrap();
    assert!(items.iter().any(|i| i["name"] == "Global Vault Path"));
    assert!(items.iter().any(|i| i["name"] == "Global Schema Version"));
    assert!(items.iter().any(|i| {
        i["name"] == "Global Embed Model"
            && i["status"] == "warn"
            && i["detail"].as_str().unwrap().contains("Mismatch")
    }));
    assert!(items.iter().any(|i| {
        i["name"] == "Global Sync Age"
            && i["status"] == "warn"
            && i["detail"].as_str().unwrap().contains("> 7d")
    }));

    // 4. Read-only proof across loadout, guard, and doctor.
    let after = snapshot_sqlite_files(global_dir.path());
    assert_eq!(before.sqlite_len, after.sqlite_len);
    assert_eq!(before.sqlite_mtime, after.sqlite_mtime);
    assert_eq!(before.wal_len, after.wal_len);
}

#[test]
fn test_cli_handoff_lifecycle_race_exit3_and_hook_session_start() {
    let vault_dir = tempdir().unwrap();
    init_synced_vault(
        vault_dir.path(),
        &[(
            "10_Projects/payment-gateway.md",
            "---\ntitle: Payment Gateway\nhierarchy: L1Resource\n---\n# Payment Gateway\n\n> Distributed settlement gateway.\n",
        )],
        None,
    );

    // 1. CLI create
    let create_out = Command::cargo_bin("k0maru")
        .unwrap()
        .args([
            "handoff",
            "create",
            "--vault",
            vault_dir.path().to_str().unwrap(),
            "--from",
            "agent-alpha",
            "--to",
            "agent-beta",
            "--project",
            "payment-gateway",
            "--title",
            "Wire settlement retry queue",
            "--goal",
            "Finish retry backoff integration",
            "--state",
            "in_progress",
            "--done",
            "Added schema",
            "--next",
            "Run integration suite",
            "--file",
            "src/retry.rs:12-45",
            "--ref",
            "10_Projects/payment-gateway.md:1-10#Payment Gateway",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(create_out.status.success(), "{:?}", create_out);
    let created: HandoffRecord = serde_json::from_slice(&create_out.stdout).unwrap();
    assert_eq!(created.from_agent, "agent-alpha");
    assert_eq!(created.to_agent.as_deref(), Some("agent-beta"));

    // 2. hook session-start appends ## Pending Handoff WITHOUT claiming.
    let hook_out = Command::cargo_bin("k0maru")
        .unwrap()
        .args([
            "hook",
            "session-start",
            "--vault",
            vault_dir.path().to_str().unwrap(),
            "--repo",
            vault_dir.path().to_str().unwrap(),
            "--project",
            "payment-gateway",
        ])
        .output()
        .unwrap();
    assert!(hook_out.status.success(), "{:?}", hook_out);
    let hook_md = String::from_utf8_lossy(&hook_out.stdout);
    assert!(
        hook_md.contains("## Pending Handoff"),
        "session-start must surface pending handoff hint:\n{}",
        hook_md
    );
    assert!(hook_md.contains(&created.id));
    assert!(
        estimate_tokens(&hook_md) <= 300,
        "session-start output exceeded 300 tokens: {}",
        estimate_tokens(&hook_md)
    );
    assert!(
        vault_dir
            .path()
            .join(".k0maru/handoffs/pending")
            .join(format!("{}.json", created.id))
            .is_file(),
        "session-start must never claim the handoff"
    );

    // 3. Target-agent mismatch returns exit code 3 without claiming.
    let mismatch_out = Command::cargo_bin("k0maru")
        .unwrap()
        .args([
            "handoff",
            "claim",
            "--vault",
            vault_dir.path().to_str().unwrap(),
            "--agent",
            "agent-gamma",
            "--id",
            &created.id,
        ])
        .output()
        .unwrap();
    assert_eq!(mismatch_out.status.code(), Some(3));

    // 4. Multi-subprocess concurrent claim race: 4 processes race on --id -> exactly 1 wins (exit 0), 3 exit 3!
    let bin = assert_cmd::cargo::cargo_bin("k0maru");
    let barrier = Arc::new(Barrier::new(4));
    let mut handles = Vec::new();
    for idx in 0..4 {
        let bin_path = bin.clone();
        let vault_str = vault_dir.path().to_str().unwrap().to_string();
        let handoff_id = created.id.clone();
        let b = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            b.wait();
            StdCommand::new(bin_path)
                .args([
                    "handoff",
                    "claim",
                    "--vault",
                    &vault_str,
                    "--agent",
                    "agent-beta",
                    "--id",
                    &handoff_id,
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .map(|o| (idx, o.status.code()))
                .unwrap()
        }));
    }
    let outcomes: Vec<(usize, Option<i32>)> =
        handles.into_iter().map(|h| h.join().unwrap()).collect();
    let winners = outcomes.iter().filter(|(_, c)| *c == Some(0)).count();
    let exit3_losers = outcomes.iter().filter(|(_, c)| *c == Some(3)).count();
    assert_eq!(winners, 1, "exactly 1 racer must win: {:?}", outcomes);
    assert_eq!(
        exit3_losers, 3,
        "all losing racers must exit with code 3: {:?}",
        outcomes
    );

    // 5. Expired handoff cannot be claimed via CLI (exit 3).
    let store = HandoffStore::new(vault_dir.path());
    let expired = store
        .create(k0maru::handoff::CreateHandoffRequest {
            from_agent: "agent-alpha".to_string(),
            title: "Already expired".to_string(),
            state: HandoffState::Blocked,
            expires_at: Some("2020-01-01T00:00:00Z".to_string()),
            ..Default::default()
        })
        .unwrap();
    let expired_out = Command::cargo_bin("k0maru")
        .unwrap()
        .args([
            "handoff",
            "claim",
            "--vault",
            vault_dir.path().to_str().unwrap(),
            "--agent",
            "agent-beta",
            "--id",
            &expired.id,
        ])
        .output()
        .unwrap();
    assert_eq!(expired_out.status.code(), Some(3));

    // 6. Crash simulation: claimed file with claimed_by=None renders "claimed (claimer unknown)".
    let crash_id = "20261118T000000Z-deadbeef";
    let crash_record = HandoffRecord {
        schema: k0maru::handoff::HANDOFF_SCHEMA_VERSION.to_string(),
        id: crash_id.to_string(),
        created_at: "2026-11-18T00:00:00Z".to_string(),
        from_agent: "agent-alpha".to_string(),
        to_agent: None,
        project: Some("payment-gateway".to_string()),
        title: "Crash recovery test".to_string(),
        goal: "Verify unknown claimer rendering".to_string(),
        state: HandoffState::InProgress,
        done: vec![],
        next_steps: vec![],
        open_questions: vec![],
        refs: vec![],
        files: vec![],
        branch: None,
        commit: None,
        expires_at: None,
        claimed_by: None,
        claimed_at: None,
    };
    fs::write(
        vault_dir
            .path()
            .join(".k0maru/handoffs/claimed")
            .join(format!("{}.json", crash_id)),
        serde_json::to_vec_pretty(&crash_record).unwrap(),
    )
    .unwrap();

    let show_crash = Command::cargo_bin("k0maru")
        .unwrap()
        .args([
            "handoff",
            "show",
            crash_id,
            "--vault",
            vault_dir.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(show_crash.status.success());
    assert!(String::from_utf8_lossy(&show_crash.stdout).contains("claimed (claimer unknown)"));

    let list_all = Command::cargo_bin("k0maru")
        .unwrap()
        .args([
            "handoff",
            "list",
            "--vault",
            vault_dir.path().to_str().unwrap(),
            "--all",
        ])
        .output()
        .unwrap();
    assert!(list_all.status.success());
    let list_str = String::from_utf8_lossy(&list_all.stdout);
    assert!(list_str.contains(crash_id));
    assert!(list_str.contains("claimed (claimer unknown)"));

    // 7. Prune removes old claimed/expired records and preserves any unexpired pending records.
    let kept = store
        .create(k0maru::handoff::CreateHandoffRequest {
            from_agent: "agent-alpha".to_string(),
            title: "Fresh unexpired pending".to_string(),
            ..Default::default()
        })
        .unwrap();
    let prune_out = Command::cargo_bin("k0maru")
        .unwrap()
        .args([
            "handoff",
            "prune",
            "--vault",
            vault_dir.path().to_str().unwrap(),
            "--older-than",
            "0s",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(prune_out.status.success());
    let prune_json: Value = serde_json::from_slice(&prune_out.stdout).unwrap();
    assert!(prune_json["deleted_expired"].as_u64().unwrap() >= 1);
    assert!(prune_json["deleted_claimed"].as_u64().unwrap() >= 1);
    assert_eq!(prune_json["kept_pending"], 1);
    assert!(vault_dir
        .path()
        .join(".k0maru/handoffs/pending")
        .join(format!("{}.json", kept.id))
        .is_file());
}
