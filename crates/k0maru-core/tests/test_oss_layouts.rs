//! Integration tests for open-source neutral layouts, innermost-segment hierarchy,
//! CommonMark relative link graph extraction/resolution, guard doc exemptions,
//! `.k0maru/refs` GC defaults, and live SQLite health checks.

use std::fs;
use std::path::Path;

use k0maru_core::adapters::{
    detect_layout, determine_llm_wiki_hierarchy, find_index_markdown, has_index_markdown,
    is_structured_vault, open_vault_adapter, ObsidianAdapter, VaultLayout,
};
use k0maru_core::core::models::{Document, HierarchyLevel};
use k0maru_core::core::traits::VaultAdapter;
use k0maru_core::gc::{ContextGcEngine, GcOptions, SessionTrajectory, TrajectoryTurn, TurnRole};
use k0maru_core::graph::{LinkResolver, Resolution};
use k0maru_core::guard::engine::GuardEngine;
use k0maru_core::guard::models::{GuardRule, RuleCondition, RuleSeverity};
use k0maru_core::parser::{extract_elements, extract_wikilinks};
use k0maru_core::scanner::IncrementalScanner;
use k0maru_core::storage::sqlite::LiveDbHealth;
use k0maru_core::storage::{HybridSearchEngine, SearchMode, SqliteStorage};
use serde_json::json;
use tempfile::tempdir;

#[derive(Debug, Clone, Copy)]
enum LayoutKind {
    NeutralWiki,
    DocsAdrRepo,
    FlatCommonmarkDocs,
    JdCompat,
}

fn write_note(root: &Path, rel: &str, content: &str) {
    let full = root.join(rel);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(full, content).unwrap();
}

fn build_fixture(kind: LayoutKind) -> tempfile::TempDir {
    let dir = tempdir().unwrap();
    let root = dir.path();
    match kind {
        LayoutKind::NeutralWiki => {
            write_note(
                root,
                "INDEX.md",
                "# Wiki Index\n\n- [Gateway](projects/payment-gateway.md)\n- [ADR-001](decisions/adr-001-idempotency.md)\n",
            );
            write_note(
                root,
                "projects/payment-gateway.md",
                "---\ntype: project\n---\n# Payment Gateway\nUses [ADR-001](../decisions/adr-001-idempotency.md) for idempotency keys.\n",
            );
            write_note(
                root,
                "decisions/adr-001-idempotency.md",
                "---\ntype: adr\n---\n# ADR-001 Idempotency\nAll payment mutations require an idempotency key.\n",
            );
            write_note(
                root,
                "logs/2026-01-02-session.md",
                "# Session Log\nVerified idempotency key retry handling.\n",
            );
        }
        LayoutKind::DocsAdrRepo => {
            write_note(
                root,
                "README.md",
                "# Service Repo\n\nSee [Architecture](docs/architecture.md) and [ADR-001](docs/adr/0001-idempotency.md).\n",
            );
            write_note(
                root,
                "docs/architecture.md",
                "# Architecture\nService relies on [ADR-001](./adr/0001-idempotency.md).\n",
            );
            write_note(
                root,
                "docs/adr/0001-idempotency.md",
                "# ADR-0001 Idempotency\nEvery settlement request carries an idempotency key.\n",
            );
            write_note(
                root,
                "docs/logs/2026-01-03-rollout.md",
                "# Rollout Log\nRolled out idempotency header validation.\n",
            );
        }
        LayoutKind::FlatCommonmarkDocs => {
            write_note(
                root,
                "index.md",
                "# Documentation\n\n- [Intro](guide/sub/intro.md)\n- [First Steps](tutorial/first-steps.md)\n",
            );
            write_note(
                root,
                "guide/auth.md",
                "# Authentication\nToken rotation and idempotency configuration.\n",
            );
            write_note(
                root,
                "guide/sub/intro.md",
                "# Guide Intro\nRead [Auth](../auth.md#tokens) and [First Steps](../../tutorial/first-steps.md).\n",
            );
            write_note(
                root,
                "tutorial/first-steps.md",
                "# First Steps\nGetting started with idempotency tokens.\n",
            );
        }
        LayoutKind::JdCompat => {
            write_note(
                root,
                "10_Projects/payment-gateway.md",
                "---\ntitle: Payment Gateway\n---\n# Payment Gateway\nLinks to [[20_Cards/adr-001-idempotency]].\n",
            );
            write_note(
                root,
                "20_Cards/adr-001-idempotency.md",
                "---\ntitle: ADR-001 Idempotency\n---\n# ADR-001 Idempotency\nAll payment mutations require an idempotency key.\n",
            );
            write_note(
                root,
                "01_AI_Logs/2026-01-02-session.md",
                "# Session Log\nChecked idempotency key enforcement.\n",
            );
        }
    }
    dir
}

#[test]
fn parametrized_layouts_detect_sync_search_and_links() {
    let cases = [
        (
            LayoutKind::NeutralWiki,
            VaultLayout::Structured,
            true,
            4usize,
        ),
        (
            LayoutKind::DocsAdrRepo,
            VaultLayout::Structured,
            false,
            4usize,
        ),
        (
            LayoutKind::FlatCommonmarkDocs,
            VaultLayout::Generic,
            true,
            4usize,
        ),
        (LayoutKind::JdCompat, VaultLayout::Structured, false, 3usize),
    ];

    for (kind, expected_layout, expect_index, expected_docs) in cases {
        let dir = build_fixture(kind);
        let root = dir.path();

        assert_eq!(detect_layout(root), expected_layout, "layout for {kind:?}");
        assert_eq!(
            is_structured_vault(root),
            expected_layout == VaultLayout::Structured,
            "is_structured_vault for {kind:?}"
        );
        assert_eq!(
            has_index_markdown(root),
            expect_index,
            "has_index_markdown for {kind:?}"
        );
        if expect_index {
            assert!(find_index_markdown(root).is_some());
        }

        let adapter = open_vault_adapter(root);
        let mut storage = SqliteStorage::in_memory().unwrap();
        let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
        let stats = scanner.sync(false).unwrap();
        assert_eq!(stats.added, expected_docs, "synced count for {kind:?}");
        // `sync()` already clears dirty link edges by rebuilding them.
        assert!(!storage.rebuild_link_edges_if_dirty().unwrap());

        let edge_count: i64 = storage
            .connection()
            .query_row("SELECT count(*) FROM link_edges", [], |r| r.get(0))
            .unwrap();
        assert!(
            edge_count >= 1,
            "expected resolved link_edges for {kind:?}, got {edge_count}"
        );

        let engine = HybridSearchEngine::new(&storage, None);
        let hits = engine.search("idempotency", SearchMode::Bm25, 5).unwrap();
        assert!(!hits.is_empty(), "expected search hits for {kind:?}");
    }

    // Verify CommonMark graph edges in FlatCommonmarkDocs resolve `../auth.md#tokens` and `../../tutorial/first-steps.md`
    let docs_dir = build_fixture(LayoutKind::FlatCommonmarkDocs);
    let docs_adapter = open_vault_adapter(docs_dir.path());
    let intro_doc = docs_adapter
        .read_document(Path::new("guide/sub/intro.md"))
        .unwrap();
    assert_eq!(intro_doc.hierarchy, HierarchyLevel::L3Evergreen);

    let mut docs_storage = SqliteStorage::in_memory().unwrap();
    let mut docs_scanner = IncrementalScanner::new(&docs_adapter, &mut docs_storage);
    docs_scanner.sync(false).unwrap();
    let resolved_from_intro: Vec<String> = {
        let mut stmt = docs_storage
            .connection()
            .prepare(
                "SELECT dst.path FROM link_edges e
                 JOIN documents src ON src.id = e.src_doc_id
                 JOIN documents dst ON dst.id = e.dst_doc_id
                 WHERE src.path = 'guide/sub/intro.md'
                 ORDER BY dst.path",
            )
            .unwrap();
        stmt.query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    };
    assert_eq!(
        resolved_from_intro,
        vec![
            "guide/auth.md".to_string(),
            "tutorial/first-steps.md".to_string(),
        ]
    );
}

#[test]
fn innermost_segment_hierarchy_ranks_leaf_above_container() {
    let fm = json!({});

    // Logs nested inside `wiki/` or `.k0maru/wiki/` must be L2Log, never L3Evergreen
    assert_eq!(
        determine_llm_wiki_hierarchy(
            &fm,
            Path::new("wiki/logs/2026-01-02-session.md"),
            HierarchyLevel::L1Resource
        ),
        HierarchyLevel::L2Log
    );
    assert_eq!(
        determine_llm_wiki_hierarchy(
            &fm,
            Path::new(".k0maru/wiki/logs/x.md"),
            HierarchyLevel::L1Resource
        ),
        HierarchyLevel::L2Log
    );
    assert_eq!(
        determine_llm_wiki_hierarchy(
            &fm,
            Path::new("projects/acme/daily/2026-01-02.md"),
            HierarchyLevel::L3Evergreen
        ),
        HierarchyLevel::L0Ephemeral
    );

    // Decisions / ADRs inside `docs/` classify as L3Evergreen; general `docs/` classify as L1Resource
    assert_eq!(
        determine_llm_wiki_hierarchy(
            &fm,
            Path::new("docs/adr/0001-x.md"),
            HierarchyLevel::L1Resource
        ),
        HierarchyLevel::L3Evergreen
    );
    assert_eq!(
        determine_llm_wiki_hierarchy(&fm, Path::new("decisions/x.md"), HierarchyLevel::L1Resource),
        HierarchyLevel::L3Evergreen
    );
    assert_eq!(
        determine_llm_wiki_hierarchy(
            &fm,
            Path::new("docs/architecture.md"),
            HierarchyLevel::L3Evergreen
        ),
        HierarchyLevel::L1Resource
    );
    assert_eq!(
        determine_llm_wiki_hierarchy(
            &fm,
            Path::new("docs/guide/intro.md"),
            HierarchyLevel::L3Evergreen
        ),
        HierarchyLevel::L1Resource
    );
}

#[test]
fn obsidian_write_card_uses_neutral_defaults_and_preserves_existing_jd_dirs() {
    let neutral_dir = tempdir().unwrap();
    let neutral_adapter = ObsidianAdapter::new(neutral_dir.path());
    assert_eq!(
        neutral_adapter
            .write_card("adr-001", "# ADR-001\n", HierarchyLevel::L3Evergreen)
            .unwrap(),
        Path::new("decisions/adr-001.md")
    );
    assert_eq!(
        neutral_adapter
            .write_card("session-1", "# Session\n", HierarchyLevel::L2Log)
            .unwrap(),
        Path::new("logs/session-1.md")
    );
    assert_eq!(
        neutral_adapter
            .write_card("today", "# Today\n", HierarchyLevel::L0Ephemeral)
            .unwrap(),
        Path::new("daily/today.md")
    );
    assert_eq!(
        neutral_adapter
            .write_card("spec", "# Spec\n", HierarchyLevel::L1Resource)
            .unwrap(),
        Path::new("resources/spec.md")
    );

    // Existing JD folders on disk are respected only when they already exist
    let jd_dir = tempdir().unwrap();
    fs::create_dir_all(jd_dir.path().join("20_Cards")).unwrap();
    fs::create_dir_all(jd_dir.path().join("01_AI_Logs")).unwrap();
    fs::create_dir_all(jd_dir.path().join("00_Daily")).unwrap();
    fs::create_dir_all(jd_dir.path().join("30_Resources")).unwrap();
    let jd_adapter = ObsidianAdapter::new(jd_dir.path());
    assert_eq!(
        jd_adapter
            .write_card("card-1", "# Card\n", HierarchyLevel::L3Evergreen)
            .unwrap(),
        Path::new("20_Cards/card-1.md")
    );
    assert_eq!(
        jd_adapter
            .write_card("log-1", "# Log\n", HierarchyLevel::L2Log)
            .unwrap(),
        Path::new("01_AI_Logs/log-1.md")
    );
    assert_eq!(
        jd_adapter
            .write_card("daily-1", "# Daily\n", HierarchyLevel::L0Ephemeral)
            .unwrap(),
        Path::new("00_Daily/daily-1.md")
    );
    assert_eq!(
        jd_adapter
            .write_card("res-1", "# Res\n", HierarchyLevel::L1Resource)
            .unwrap(),
        Path::new("30_Resources/res-1.md")
    );
}

#[test]
fn commonmark_link_extraction_filters_external_images_code_and_backlinks() {
    let md = r#"
# Architecture

Internal relative links:
- [Guide](./guide.md)
- [ADR 0002](../adr/0002.md#context)
- [First Steps](tutorial/first-steps.md)
- [Section](#overview)
- [Topology PDF](./assets/topology.pdf)
- [[WikiTarget|Wiki Alias]]

Images and external URLs must be excluded:
- ![Diagram](./assets/arch.png)
- [Website](https://example.com/docs)
- [Insecure](http://example.com)
- [Mail](mailto:dev@example.com)
- [Phone](tel:+123456)
- [Data](data:text/plain;base64,SGVsbG8=)
- [JS](javascript:void(0))
- [File](file:///tmp/secret.md)
- [ProtoRel](//cdn.example.com/lib.js)

Inline `[InCode](./ignored-inline.md)` and fenced blocks must be ignored:
```markdown
[InFence](./ignored-fence.md)
[[IgnoredWiki]]
```

<!-- k0maru:backlinks:start -->
- [Backlink](./woven-backlink.md)
- [[WovenWiki]]
<!-- k0maru:backlinks:end -->
"#;

    let via_wikilinks = extract_wikilinks(md);
    let (via_elements, _) = extract_elements(md);
    assert_eq!(via_wikilinks, via_elements);

    let targets: Vec<&str> = via_elements.iter().map(|l| l.target.as_str()).collect();
    assert_eq!(
        targets,
        vec![
            "./guide",
            "../adr/0002.md#context",
            "tutorial/first-steps",
            "#overview",
            "./assets/topology.pdf",
            "WikiTarget",
        ]
    );
    assert_eq!(via_elements[0].alias.as_deref(), Some("Guide"));
    assert_eq!(via_elements[1].alias.as_deref(), Some("ADR 0002"));
    assert_eq!(via_elements[5].alias.as_deref(), Some("Wiki Alias"));
}

#[test]
fn resolver_handles_relative_dot_and_parent_segments_and_broken_links() {
    let resolver = LinkResolver::from_docs(vec![
        (1, "guide/auth.md".to_string(), "Authentication".to_string()),
        (2, "guide/sub/intro.md".to_string(), "Sub Intro".to_string()),
        (
            3,
            "guide/sub/sibling.md".to_string(),
            "Sub Sibling".to_string(),
        ),
        (4, "adr/0002.md".to_string(), "ADR 0002".to_string()),
        (
            5,
            "other/unique-elsewhere.md".to_string(),
            "Unique Elsewhere".to_string(),
        ),
    ]);

    let src = "guide/sub/intro.md";
    assert_eq!(
        resolver.resolve(src, "../auth.md#tokens"),
        Resolution::Doc(1)
    );
    assert_eq!(resolver.resolve(src, "./sibling.md"), Resolution::Doc(3));
    assert_eq!(
        resolver.resolve(src, "../../adr/0002.md#context"),
        Resolution::Doc(4)
    );
    assert_eq!(
        resolver.resolve(src, "../sub/intro.md"),
        Resolution::SelfRef
    );
    assert_eq!(resolver.resolve(src, "#local"), Resolution::SelfRef);
    assert_eq!(
        resolver.resolve(src, "../assets/flow.png"),
        Resolution::Asset
    );

    // Relative links to missing targets must report Broken rather than matching another folder by stem
    assert_eq!(
        resolver.resolve(src, "../unique-elsewhere.md"),
        Resolution::Broken
    );
    assert_eq!(
        resolver.resolve(src, "./unique-elsewhere.md"),
        Resolution::Broken
    );
    assert_eq!(
        resolver.resolve(src, "../../../outside.md"),
        Resolution::Broken
    );
}

#[test]
fn guard_exempts_adr_and_wiki_markdown_on_wildcard_rules() {
    let rules = vec![GuardRule {
        id: "SEC-001".to_string(),
        title: "No hardcoded key literal".to_string(),
        source_note: "docs/adr/0001-secrets.md".to_string(),
        file_pattern: Some("**/*".to_string()),
        condition: RuleCondition::Forbidden("SECRET_PATTERN".to_string()),
        severity: RuleSeverity::Deny,
        remediation: "Use environment secrets.".to_string(),
    }];

    let diff = r#"
diff --git a/docs/adr/0002-rotation.md b/docs/adr/0002-rotation.md
--- a/docs/adr/0002-rotation.md
+++ b/docs/adr/0002-rotation.md
@@ -1,0 +1,2 @@
+# Rotation Policy
+Forbids `SECRET_PATTERN` in application source.
diff --git a/adr/0003-note.md b/adr/0003-note.md
--- a/adr/0003-note.md
+++ b/adr/0003-note.md
@@ -1,0 +1,1 @@
+Documents `SECRET_PATTERN` handling.
diff --git a/docs/decisions/0004-note.md b/docs/decisions/0004-note.md
--- a/docs/decisions/0004-note.md
+++ b/docs/decisions/0004-note.md
@@ -1,0 +1,1 @@
+Mentions `SECRET_PATTERN` in ADR prose.
diff --git a/wiki/overview.md b/wiki/overview.md
--- a/wiki/overview.md
+++ b/wiki/overview.md
@@ -1,0 +1,1 @@
+Wiki note referencing `SECRET_PATTERN`.
diff --git a/src/auth.rs b/src/auth.rs
--- a/src/auth.rs
+++ b/src/auth.rs
@@ -1,0 +1,1 @@
+let k = "SECRET_PATTERN";
"#;

    let report = GuardEngine::check_diff(diff, &rules);
    assert_eq!(report.violations.len(), 1);
    assert_eq!(report.violations[0].file_path, "src/auth.rs");
}

#[test]
fn gc_options_default_refs_doc_and_k0maru_refs_dir() {
    let opts = GcOptions::default();
    assert_eq!(opts.refs_dir, None);

    let tmp = tempdir().unwrap();
    let k0maru_refs = tmp.path().join(".k0maru/refs");
    fs::create_dir_all(&k0maru_refs).unwrap();

    // Ensure explicit refs_dir works and default GcOptions compiles and runs cleanly
    let mut big_lines = String::new();
    for i in 0..30 {
        big_lines.push_str(&format!("line {i}: ok\n"));
    }
    let traj = SessionTrajectory {
        session_id: "sess-1".to_string(),
        turns: vec![
            TrajectoryTurn {
                turn_index: 0,
                role: TurnRole::Tool,
                content: big_lines,
                tool_name: Some("cargo".to_string()),
                tool_call_id: None,
            },
            TrajectoryTurn {
                turn_index: 1,
                role: TurnRole::User,
                content: "Recent turn".to_string(),
                tool_name: None,
                tool_call_id: None,
            },
        ],
    };
    let explicit_opts = GcOptions {
        hot_window_turns: 1,
        line_threshold: 5,
        refs_dir: Some(k0maru_refs.clone()),
        ..GcOptions::default()
    };
    let res = ContextGcEngine::new()
        .compact(&traj, &explicit_opts)
        .unwrap();
    assert_eq!(res.folded_nodes_count, 1);
    assert!(fs::read_dir(&k0maru_refs).unwrap().next().is_some());
}

#[test]
fn sqlite_live_health_detects_replacement_schema_upgrade_and_corruption() {
    // 1. Healthy on fresh open_rw
    let dir1 = tempdir().unwrap();
    let db1 = dir1.path().join("cache.sqlite");
    let storage1 = SqliteStorage::open_rw(&db1).unwrap();
    assert_eq!(
        storage1.check_live_health(&db1).unwrap(),
        LiveDbHealth::Healthy
    );

    // 2. SchemaTooNew(4) when external connection bumps PRAGMA user_version
    {
        let ext = rusqlite::Connection::open(&db1).unwrap();
        ext.pragma_update(None, "user_version", 4).unwrap();
    }
    assert_eq!(
        storage1.check_live_health(&db1).unwrap(),
        LiveDbHealth::SchemaTooNew(4)
    );
    drop(storage1);

    // 3. CorruptedSchema when core tables are dropped externally
    let dir2 = tempdir().unwrap();
    let db2 = dir2.path().join("cache.sqlite");
    let storage2 = SqliteStorage::open_rw(&db2).unwrap();
    assert_eq!(
        storage2.check_live_health(&db2).unwrap(),
        LiveDbHealth::Healthy
    );
    {
        let ext = rusqlite::Connection::open(&db2).unwrap();
        ext.execute_batch("DROP TABLE fts_documents;").unwrap();
    }
    assert_eq!(
        storage2.check_live_health(&db2).unwrap(),
        LiveDbHealth::CorruptedSchema
    );
    drop(storage2);

    // 4. ReplacedOnDisk when the database file is replaced with a new inode
    let dir3 = tempdir().unwrap();
    let db3 = dir3.path().join("cache.sqlite");
    let storage3 = SqliteStorage::open_rw(&db3).unwrap();
    assert_eq!(
        storage3.check_live_health(&db3).unwrap(),
        LiveDbHealth::Healthy
    );
    let replacement = dir3.path().join("cache.sqlite.new");
    {
        let _new_storage = SqliteStorage::open_rw(&replacement).unwrap();
    }
    fs::rename(&replacement, &db3).unwrap();
    assert_eq!(
        storage3.check_live_health(&db3).unwrap(),
        LiveDbHealth::ReplacedOnDisk
    );

    // 5. Healthy check on Document sanity (unused import guard)
    let _doc_type_check: Option<Document> = None;
}
