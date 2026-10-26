//! Comprehensive core unit and integration tests for k0maru-core.

mod isolation {
    use std::fs;
    use std::sync::Arc;
    use tempfile::tempdir;

    use k0maru_core::{
        extract_elements, parse_frontmatter, HierarchyLevel, HybridSearchEngine,
        IncrementalScanner, MockEmbeddingEngine, ObsidianAdapter, SearchMode, SqliteStorage,
        VaultAdapter, VectorSyncEngine,
    };

    #[test]
    fn test_core_isolation_full_lifecycle() {
        let temp_vault = tempdir().expect("Failed to create temp vault dir");
        let vault_path = temp_vault.path();

        let daily_dir = vault_path.join("00_Daily");
        let cards_dir = vault_path.join("20_Cards");
        fs::create_dir_all(&daily_dir).expect("create daily dir");
        fs::create_dir_all(&cards_dir).expect("create cards dir");

        let card_a = cards_dir.join("Architecture.md");
        let card_b = cards_dir.join("Modularity.md");
        let daily_note = daily_dir.join("2026-10-11.md");

        fs::write(
            &card_a,
            r#"---
title: "Core Architecture"
hierarchy: L3Evergreen
tags:
  - architecture
  - rust
---
# Core Architecture

This note explains the core library decoupling in [[Modularity]].
"#,
        )
        .expect("write card A");

        fs::write(
            &card_b,
            r#"---
title: "Modularity Principles"
hierarchy: L3Evergreen
tags:
  - modularity
---
# Modularity Principles

Deep modules provide simple interfaces for complex operations.
Backlink to [[Architecture]].
"#,
        )
        .expect("write card B");

        fs::write(
            &daily_note,
            r#"# 2026-10-11 Scratchpad
Working on k0maru-core standalone extraction.
"#,
        )
        .expect("write daily note");

        let adapter = ObsidianAdapter::new(vault_path);
        let mut storage = SqliteStorage::in_memory().expect("init in-memory storage");
        let mut scanner = IncrementalScanner::new(&adapter, &mut storage);

        let sync_stats = scanner.sync(false).expect("initial sync");
        assert_eq!(sync_stats.added, 3);
        assert_eq!(sync_stats.modified, 0);

        let card_a_content = fs::read_to_string(&card_a).unwrap();
        let (fm_a, body_a) = parse_frontmatter(&card_a_content);
        assert_eq!(fm_a["title"], "Core Architecture");
        let (links_a, tags_a) = extract_elements(&body_a);
        assert_eq!(links_a.len(), 1);
        assert_eq!(links_a[0].target, "Modularity");
        assert_eq!(tags_a.len(), 0);

        let doc_a = adapter
            .read_document(&std::path::PathBuf::from("20_Cards/Architecture.md"))
            .expect("read card A");
        assert_eq!(doc_a.title, "Core Architecture");
        assert_eq!(doc_a.hierarchy, HierarchyLevel::L3Evergreen);
        assert_eq!(doc_a.tags, vec!["architecture", "rust"]);

        let mock_embedder = Arc::new(MockEmbeddingEngine::new(384));
        let vec_stats = VectorSyncEngine::sync(&mut storage, &*mock_embedder).expect("vector sync");
        assert_eq!(vec_stats.embedded_count, 3);

        let search_engine = HybridSearchEngine::new(&storage, Some(mock_embedder));
        let search_results = search_engine
            .search("Modularity", SearchMode::Hybrid, 5)
            .expect("hybrid search");
        assert!(!search_results.is_empty());
        assert!(search_results
            .iter()
            .any(|r| r.title == "Modularity Principles"));
    }
}

mod dynamic_vector {
    use k0maru_core::core::traits::CacheStorage;
    use k0maru_core::storage::SqliteStorage;
    use tempfile::NamedTempFile;

    #[test]
    fn test_initial_vector_dimension_is_384() {
        let mut storage = SqliteStorage::in_memory().expect("in-memory db");
        storage.initialize().expect("initialize");

        let dim = storage.get_vector_dimension().expect("get dim");
        assert_eq!(dim, Some(384));

        let migrated = storage.ensure_vector_dimension(384).expect("ensure dim");
        assert!(!migrated);
        assert_eq!(storage.get_vector_dimension().expect("dim"), Some(384));
    }

    #[test]
    fn test_dimension_migration_from_384_to_512() {
        let mut storage = SqliteStorage::in_memory().expect("in-memory db");
        storage.initialize().expect("initialize");

        let emb_384 = vec![0.1f32; 384];
        storage
            .insert_vector("doc_1", &emb_384, 111)
            .expect("insert 384d");
        assert_eq!(storage.vector_count().expect("count"), 1);

        let migrated = storage.ensure_vector_dimension(512).expect("migrate 512");
        assert!(migrated);
        assert_eq!(storage.get_vector_dimension().expect("dim"), Some(512));
        assert_eq!(storage.vector_count().expect("count after migration"), 0);

        let mut emb_512_a = vec![0.0f32; 512];
        emb_512_a[0] = 1.0;
        let mut emb_512_b = vec![0.0f32; 512];
        emb_512_b[1] = 1.0;

        storage
            .insert_vector("doc_512_a", &emb_512_a, 333)
            .expect("insert 512d a");
        storage
            .insert_vector("doc_512_b", &emb_512_b, 444)
            .expect("insert 512d b");
        assert_eq!(storage.vector_count().expect("count"), 2);

        let results = storage.search_vectors(&emb_512_a, 5).expect("search");
        assert_eq!(results[0].0, "doc_512_a");
        assert!(results[0].1 < 0.001);
    }

    #[test]
    fn test_migration_with_file_backed_database() {
        let tmp = NamedTempFile::new().expect("temp file");
        let path = tmp.path().to_path_buf();

        {
            let mut storage = SqliteStorage::open(&path).expect("open db");
            storage.initialize().expect("init");
            assert_eq!(storage.get_vector_dimension().unwrap(), Some(384));

            let emb_384 = vec![0.5f32; 384];
            storage.insert_vector("doc_disk_1", &emb_384, 999).unwrap();
            assert_eq!(storage.vector_count().unwrap(), 1);

            let migrated = storage.ensure_vector_dimension(768).unwrap();
            assert!(migrated);
            assert_eq!(storage.get_vector_dimension().unwrap(), Some(768));
            assert_eq!(storage.vector_count().unwrap(), 0);

            let emb_768 = vec![0.2f32; 768];
            storage.insert_vector("doc_disk_2", &emb_768, 888).unwrap();
            assert_eq!(storage.vector_count().unwrap(), 1);
        }

        {
            let storage = SqliteStorage::open(&path).expect("reopen db");
            assert_eq!(storage.get_vector_dimension().unwrap(), Some(768));
            assert_eq!(storage.vector_count().unwrap(), 1);
        }
    }
}

mod guard_core {
    use k0maru_core::core::models::{Document, HierarchyLevel};
    use k0maru_core::guard::extractor::extract_rules_from_document;
    use k0maru_core::guard::models::{GuardRule, RuleCondition, RuleSeverity};
    use serde_json::json;
    use std::path::PathBuf;

    #[test]
    fn test_guard_models_serialization() {
        let rule = GuardRule {
            id: "ADR-001".to_string(),
            title: "Test Invariant".to_string(),
            source_note: "20_Cards/ADR-001.md".to_string(),
            file_pattern: Some("*.rs".to_string()),
            condition: RuleCondition::Forbidden("unwrap\\(\\)".to_string()),
            severity: RuleSeverity::Deny,
            remediation: "Use ? instead of unwrap()".to_string(),
        };

        let serialized = serde_json::to_string(&rule).expect("serialize GuardRule");
        let deserialized: GuardRule =
            serde_json::from_str(&serialized).expect("deserialize GuardRule");
        assert_eq!(rule, deserialized);
    }

    #[test]
    fn test_extract_rules_from_frontmatter() {
        let doc = Document {
            path: PathBuf::from("20_Cards/ADR-SETTLE-001.md"),
            title: "Mandatory Distributed Lock".to_string(),
            hierarchy: HierarchyLevel::L3Evergreen,
            frontmatter: json!({
                "invariants": [
                    {
                        "id": "ADR-SETTLE-001",
                        "pattern": "*.go",
                        "forbidden": "execWithoutLock",
                        "severity": "deny",
                        "remediation": "Acquire Redis lock before modifying ledger."
                    }
                ]
            }),
            links: vec![],
            tags: vec!["adr".to_string()],
            content_hash: "hash".to_string(),
            mtime: 1000,
            body: "# Mandatory Distributed Lock\n".to_string(),
        };

        let rules = extract_rules_from_document(&doc);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].id, "ADR-SETTLE-001");
        assert_eq!(rules[0].severity, RuleSeverity::Deny);
    }

    #[test]
    fn test_extract_rules_from_readme_schema() {
        let doc = Document {
            path: PathBuf::from("20_Cards/ADR-SEC-002.md"),
            title: "No Unprepared SQL".to_string(),
            hierarchy: HierarchyLevel::L3Evergreen,
            frontmatter: json!({
                "invariants": [
                    {
                        "id": "SEC-002",
                        "title": "Forbid raw execute_unprepared",
                        "condition": "forbidden",
                        "pattern": "execute_unprepared",
                        "paths": ["src/api/**/*.rs"],
                        "fix_hint": "Use sqlx query! macro instead."
                    }
                ]
            }),
            links: vec![],
            tags: vec!["adr".to_string()],
            content_hash: "hash".to_string(),
            mtime: 1000,
            body: String::new(),
        };

        let rules = extract_rules_from_document(&doc);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].id, "SEC-002");
        assert_eq!(rules[0].title, "Forbid raw execute_unprepared");
        assert_eq!(rules[0].file_pattern, Some("src/api/**/*.rs".to_string()));
        assert_eq!(
            rules[0].condition,
            RuleCondition::Forbidden("execute_unprepared".to_string())
        );
        assert_eq!(rules[0].remediation, "Use sqlx query! macro instead.");
    }
}

mod guard_engine {
    use k0maru_core::guard::engine::GuardEngine;
    use k0maru_core::guard::extractor::parse_markdown_list_rule;
    use k0maru_core::guard::models::{GuardRule, RuleCondition, RuleSeverity};

    #[test]
    fn test_guard_engine_forbidden_violation_in_diff() {
        let diff = r#"
diff --git a/src/payment/settle.go b/src/payment/settle.go
--- a/src/payment/settle.go
+++ b/src/payment/settle.go
@@ -10,3 +10,5 @@ func SettleLedger(ctx context.Context) error {
+    execWithoutLock(ctx)
+    return nil
 }
"#;

        let rules = vec![GuardRule {
            id: "ADR-SETTLE-001".to_string(),
            title: "Mandatory Distributed Lock".to_string(),
            source_note: "20_Cards/ADR-SETTLE-001.md".to_string(),
            file_pattern: Some("*.go".to_string()),
            condition: RuleCondition::Forbidden("execWithoutLock".to_string()),
            severity: RuleSeverity::Deny,
            remediation: "Acquire Redis lock before modifying ledger.".to_string(),
        }];

        let report = GuardEngine::check_diff(diff, &rules);
        assert_eq!(report.checked_files, 1);
        assert!(!report.is_passed);
        assert_eq!(report.violations.len(), 1);
        assert_eq!(report.violations[0].rule_id, "ADR-SETTLE-001");
    }

    #[test]
    fn test_parse_markdown_list_rule_with_pipe_in_quotes() {
        let line = r#"- [NO-UNWRAP] (deny) [*.rs] forbidden: "unwrap\(\)|expect\(" | remediation: "no unwrap""#;
        let rules = parse_markdown_list_rule(line, "20_Cards/ADR-001.md").expect("parsed rule");
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].id, "NO-UNWRAP");
        assert_eq!(rules[0].remediation, "no unwrap");

        // Verify it matches foo.expect("boom")
        let diff = r#"
diff --git a/src/lib.rs b/src/lib.rs
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -10,3 +10,4 @@ fn work() {
+    let val = foo.expect("boom");
 }
"#;
        let report = GuardEngine::check_diff(diff, &rules);
        assert_eq!(report.violations.len(), 1);
        assert_eq!(report.violations[0].rule_id, "NO-UNWRAP");
    }

    #[test]
    fn test_guard_engine_ignores_code_comment() {
        let diff = r#"
diff --git a/src/api.rs b/src/api.rs
--- a/src/api.rs
+++ b/src/api.rs
@@ -1,3 +1,4 @@
+// do not call legacy_api()
 fn safe() {}
"#;
        let rules = vec![GuardRule {
            id: "RULE-LEGACY".to_string(),
            title: "No Legacy API".to_string(),
            source_note: "ADR-001.md".to_string(),
            file_pattern: Some("*.rs".to_string()),
            condition: RuleCondition::Forbidden("legacy_api()".to_string()),
            severity: RuleSeverity::Deny,
            remediation: "use new api".to_string(),
        }];
        let report = GuardEngine::check_diff(diff, &rules);
        assert_eq!(
            report.violations.len(),
            0,
            "Comment line must not trigger violation"
        );
    }

    #[test]
    fn test_guard_engine_accurate_line_numbers_with_blank_context() {
        let diff = "diff --git a/src/api.rs b/src/api.rs\n--- a/src/api.rs\n+++ b/src/api.rs\n@@ -10,6 +10,6 @@\n let a = 1;\n \n let b = 2;\n+legacy_api();\n";
        let rules = vec![GuardRule {
            id: "RULE-LEGACY".to_string(),
            title: "No Legacy API".to_string(),
            source_note: "ADR-001.md".to_string(),
            file_pattern: Some("*.rs".to_string()),
            condition: RuleCondition::Forbidden("legacy_api()".to_string()),
            severity: RuleSeverity::Deny,
            remediation: "use new api".to_string(),
        }];
        let report = GuardEngine::check_diff(diff, &rules);
        assert_eq!(report.violations.len(), 1);
        assert_eq!(report.violations[0].line_number, Some(13));
    }

    #[test]
    fn test_guard_engine_blocks_deletion_of_must_contain() {
        let diff = r#"
diff --git a/src/lib.rs b/src/lib.rs
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,4 +1,3 @@
-#![forbid(unsafe_code)]
 fn main() {}
"#;
        let rules = vec![GuardRule {
            id: "RULE-UNSAFE".to_string(),
            title: "Require Forbid Unsafe".to_string(),
            source_note: "ADR-002.md".to_string(),
            file_pattern: Some("*.rs".to_string()),
            condition: RuleCondition::MustContain("#![forbid(unsafe_code)]".to_string()),
            severity: RuleSeverity::Deny,
            remediation: "Add #![forbid(unsafe_code)]".to_string(),
        }];
        let report = GuardEngine::check_diff(diff, &rules);
        assert_eq!(report.violations.len(), 1);
        assert_eq!(report.violations[0].rule_id, "RULE-UNSAFE");
    }

    #[test]
    fn test_guard_engine_ignores_k0maru_wiki_self_matches() {
        let diff = r#"
diff --git a/.k0maru/wiki/ADR-001.md b/.k0maru/wiki/ADR-001.md
--- a/.k0maru/wiki/ADR-001.md
+++ b/.k0maru/wiki/ADR-001.md
@@ -1,3 +1,4 @@
+legacy_api() must never be called.
"#;
        let rules = vec![GuardRule {
            id: "RULE-LEGACY".to_string(),
            title: "No Legacy API".to_string(),
            source_note: "ADR-001.md".to_string(),
            file_pattern: None,
            condition: RuleCondition::Forbidden("legacy_api()".to_string()),
            severity: RuleSeverity::Deny,
            remediation: "use new api".to_string(),
        }];
        let report = GuardEngine::check_diff(diff, &rules);
        assert_eq!(report.violations.len(), 0);
    }
}

mod vector_catalog {
    use k0maru_core::vector::models::EmbeddingModelType;
    use k0maru_core::vector::resolve_embedding_engine;

    #[test]
    fn test_model_catalog_dimensions() {
        assert_eq!(EmbeddingModelType::AllMiniLML6V2.dimension(), 384);
        assert_eq!(EmbeddingModelType::BgeSmallZhV15.dimension(), 512);
        assert_eq!(EmbeddingModelType::BgeSmallEnV15.dimension(), 384);
        assert_eq!(EmbeddingModelType::BgeBaseEnV15.dimension(), 768);
        assert_eq!(EmbeddingModelType::ParaphraseMlMiniLmL12V2.dimension(), 384);
        assert_eq!(EmbeddingModelType::Mock.dimension(), 384);
    }

    #[test]
    fn test_model_alias_resolution() {
        assert_eq!(
            EmbeddingModelType::from_str("minilm").unwrap(),
            EmbeddingModelType::AllMiniLML6V2
        );
        assert_eq!(
            EmbeddingModelType::from_str("bge-zh").unwrap(),
            EmbeddingModelType::BgeSmallZhV15
        );
        assert_eq!(
            EmbeddingModelType::from_str("bge-base-en").unwrap(),
            EmbeddingModelType::BgeBaseEnV15
        );
        assert_eq!(
            EmbeddingModelType::from_str("mock").unwrap(),
            EmbeddingModelType::Mock
        );
    }

    #[test]
    fn test_resolve_embedding_engine_mock() {
        let engine = resolve_embedding_engine(Some("mock"), None).expect("resolve mock");
        assert_eq!(engine.dimension(), 384);
        assert_eq!(engine.model_name(), "mock-embedding-384");
    }
}

mod round2_adversarial_tests {
    use k0maru_core::core::models::{Document, NoteLifecycleStatus};
    use k0maru_core::guard::engine::GuardEngine;
    use k0maru_core::guard::extractor::parse_markdown_list_rule;
    use k0maru_core::guard::models::{GuardRule, RuleCondition, RuleSeverity};
    use serde_json::json;

    #[test]
    fn test_inj_diff_header_injection() {
        let at = "@@";
        let diff = format!(
            "diff --git a/src/app.js b/src/app.js\n--- a/src/app.js\n+++ b/src/app.js\n{} -1,4 +1,6 {}\n const a = 1;\n+++ b/zzz.md\n+legacy_api();\n const b = 2;\n",
            at, at
        );
        let rules = vec![GuardRule {
            id: "RULE-LEGACY".to_string(),
            title: "No Legacy API".to_string(),
            source_note: "ADR-001.md".to_string(),
            file_pattern: Some("*.js".to_string()),
            condition: RuleCondition::Forbidden("legacy_api()".to_string()),
            severity: RuleSeverity::Deny,
            remediation: "use new api".to_string(),
        }];
        let report = GuardEngine::check_diff(&diff, &rules);
        assert_eq!(report.violations.len(), 1);
        assert_eq!(report.violations[0].file_path, "src/app.js");
        assert_eq!(report.violations[0].rule_id, "RULE-LEGACY");
    }

    #[test]
    fn test_s4_prose_superseded_does_not_deactivate() {
        let mut doc = Document::default();
        doc.title = "ADR-099".to_string();
        doc.frontmatter = json!({"status": "active"});
        doc.body =
            "This document explains why we do NOT use `> [SUPERSEDED]` in prose.\nStill active!"
                .to_string();
        assert_eq!(doc.lifecycle_status(), NoteLifecycleStatus::Active);
        assert!(doc.is_active());
        assert_eq!(doc.superseded_by(), None);
    }

    #[test]
    fn test_bold_and_brk_markdown_rule_parsing() {
        let bold_line = r#"- **BOLD-1**: forbidden: "bold_one_banned", files: "*.rs""#;
        let bold_rules =
            parse_markdown_list_rule(bold_line, "20_Cards/adr-bold.md").expect("parse bold");
        assert_eq!(bold_rules.len(), 1);
        assert_eq!(bold_rules[0].id, "BOLD-1");
        assert_eq!(bold_rules[0].file_pattern, Some("*.rs".to_string()));
        assert_eq!(
            bold_rules[0].condition,
            RuleCondition::Forbidden("bold_one_banned".to_string())
        );

        let brk_line = r#"- [BRK-1] [src/[ab]*.rs] forbidden: "brk_call""#;
        let brk_rules =
            parse_markdown_list_rule(brk_line, "20_Cards/adr-brk.md").expect("parse brk");
        assert_eq!(brk_rules.len(), 1);
        assert_eq!(brk_rules[0].id, "BRK-1");
        assert_eq!(brk_rules[0].file_pattern, Some("src/[ab]*.rs".to_string()));
        assert_eq!(
            brk_rules[0].condition,
            RuleCondition::Forbidden("brk_call".to_string())
        );
    }

    #[test]
    fn test_rx_word_boundaries_and_plain_literals() {
        let at = "@@";
        let rules = vec![
            GuardRule {
                id: "RX-DEBUG".to_string(),
                title: "No Debug Class".to_string(),
                source_note: "ADR-001.md".to_string(),
                file_pattern: None,
                condition: RuleCondition::Forbidden("[DEBUG]".to_string()),
                severity: RuleSeverity::Deny,
                remediation: "remove debug".to_string(),
            },
            GuardRule {
                id: "RX-UNSAFE".to_string(),
                title: "No Unsafe Mode".to_string(),
                source_note: "ADR-001.md".to_string(),
                file_pattern: None,
                condition: RuleCondition::Forbidden("config.unsafe_mode".to_string()),
                severity: RuleSeverity::Deny,
                remediation: "fix config".to_string(),
            },
            GuardRule {
                id: "RX-OLD-FN".to_string(),
                title: "No Old Fn".to_string(),
                source_note: "ADR-001.md".to_string(),
                file_pattern: None,
                condition: RuleCondition::Forbidden("old_fn".to_string()),
                severity: RuleSeverity::Deny,
                remediation: "use new_fn".to_string(),
            },
        ];

        // 1. [DEBUG] literal should not match single letter 'D'
        let diff_non_debug = format!(
            "diff --git a/test.rs b/test.rs\n--- a/test.rs\n+++ b/test.rs\n{} -1,2 +1,3 {}\n+let letter_d = 'D';\n",
            at, at
        );
        let report = GuardEngine::check_diff(&diff_non_debug, &rules);
        assert_eq!(report.violations.len(), 0);

        let diff_debug = format!(
            "diff --git a/test.rs b/test.rs\n--- a/test.rs\n+++ b/test.rs\n{} -1,2 +1,3 {}\n+println!(\"[DEBUG] tracing\");\n",
            at, at
        );
        let report = GuardEngine::check_diff(&diff_debug, &rules);
        assert_eq!(report.violations.len(), 1);
        assert_eq!(report.violations[0].rule_id, "RX-DEBUG");

        // 2. config.unsafe_mode literal should not match configXunsafe_mode
        let diff_wildcard = format!(
            "diff --git a/test.rs b/test.rs\n--- a/test.rs\n+++ b/test.rs\n{} -1,2 +1,3 {}\n+let val = configXunsafe_mode;\n",
            at, at
        );
        let report = GuardEngine::check_diff(&diff_wildcard, &rules);
        assert_eq!(report.violations.len(), 0);

        let diff_dot = format!(
            "diff --git a/test.rs b/test.rs\n--- a/test.rs\n+++ b/test.rs\n{} -1,2 +1,3 {}\n+let val = config.unsafe_mode;\n",
            at, at
        );
        let report = GuardEngine::check_diff(&diff_dot, &rules);
        assert_eq!(report.violations.len(), 1);
        assert_eq!(report.violations[0].rule_id, "RX-UNSAFE");

        // 3. old_fn should not false-positive on old_fn_safe()
        let diff_safe = format!(
            "diff --git a/test.rs b/test.rs\n--- a/test.rs\n+++ b/test.rs\n{} -1,2 +1,3 {}\n+old_fn_safe();\n",
            at, at
        );
        let report = GuardEngine::check_diff(&diff_safe, &rules);
        assert_eq!(report.violations.len(), 0);

        let diff_unsafe = format!(
            "diff --git a/test.rs b/test.rs\n--- a/test.rs\n+++ b/test.rs\n{} -1,2 +1,3 {}\n+old_fn();\n",
            at, at
        );
        let report = GuardEngine::check_diff(&diff_unsafe, &rules);
        assert_eq!(report.violations.len(), 1);
        assert_eq!(report.violations[0].rule_id, "RX-OLD-FN");
    }

    #[test]
    fn test_js_private_field_not_stripped_as_comment() {
        let at = "@@";
        let diff = format!(
            "diff --git a/src/service.js b/src/service.js\n--- a/src/service.js\n+++ b/src/service.js\n{} -1,2 +1,3 {}\n+class Evaluator {{ #eval_fn = eval(code); }}\n",
            at, at
        );
        let rules = vec![GuardRule {
            id: "NO-EVAL".to_string(),
            title: "No Eval".to_string(),
            source_note: "ADR-001.md".to_string(),
            file_pattern: Some("*.js".to_string()),
            condition: RuleCondition::Forbidden("eval(code)".to_string()),
            severity: RuleSeverity::Deny,
            remediation: "do not use eval".to_string(),
        }];
        let report = GuardEngine::check_diff(&diff, &rules);
        assert_eq!(report.violations.len(), 1);
        assert_eq!(report.violations[0].rule_id, "NO-EVAL");
    }

    #[test]
    fn test_transitive_supersession_target_chain() {
        use k0maru_core::core::traits::CacheStorage;
        use k0maru_core::storage::{HybridSearchEngine, SqliteStorage};
        use std::path::PathBuf;

        let mut storage = SqliteStorage::in_memory().expect("in-memory storage");
        let mut doc1 = Document::default();
        doc1.path = PathBuf::from("20_Cards/adr-010.md");
        doc1.title = "adr-010".to_string();
        doc1.frontmatter = json!({
            "status": "superseded",
            "superseded_by": "adr-020.md"
        });
        storage.upsert_document(&doc1).expect("upsert 1");

        let mut doc2 = Document::default();
        doc2.path = PathBuf::from("20_Cards/adr-020.md");
        doc2.title = "adr-020".to_string();
        doc2.frontmatter = json!({
            "status": "superseded",
            "superseded_by": "adr-030.md"
        });
        storage.upsert_document(&doc2).expect("upsert 2");

        let mut doc3 = Document::default();
        doc3.path = PathBuf::from("20_Cards/adr-030.md");
        doc3.title = "adr-030".to_string();
        doc3.frontmatter = json!({
            "status": "active"
        });
        storage.upsert_document(&doc3).expect("upsert 3");

        let engine = HybridSearchEngine::new(&storage, None);
        let mut score = 1.0;
        let mut boost = 0.5;
        let mut snippet = "Initial snippet".to_string();
        engine.apply_lifecycle_penalty_by_path(
            "20_Cards/adr-010.md",
            &mut score,
            &mut boost,
            &mut snippet,
        );

        assert_eq!(boost, 0.0);
        assert!((score - 0.2).abs() < 1e-4);
        assert!(snippet.starts_with("[SUPERSEDED by adr-030.md]"));
    }
}
