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
}

mod guard_engine {
    use k0maru_core::guard::engine::GuardEngine;
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
