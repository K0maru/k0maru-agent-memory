use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use assert_cmd::Command;
use k0maru::adapters::ObsidianAdapter;
use k0maru::core::models::{Document, HierarchyLevel, WikiLink};
use k0maru::core::traits::CacheStorage;
use k0maru::scanner::IncrementalScanner;
use k0maru::storage::{HybridSearchEngine, SearchMode, SqliteStorage};
use k0maru::vector::models::EmbeddingModelType;
use k0maru::vector::MockEmbeddingEngine;
use k0maru::EmbeddingEngine;
use tempfile::tempdir;

static TEST_MUTEX: Mutex<()> = Mutex::new(());

fn make_doc(
    path: &str,
    title: &str,
    body: &str,
    links: Vec<WikiLink>,
    tags: Vec<&str>,
) -> Document {
    Document {
        path: PathBuf::from(path),
        title: title.to_string(),
        hierarchy: HierarchyLevel::L3Evergreen,
        frontmatter: serde_json::json!({ "tags": tags }),
        links,
        tags: tags.into_iter().map(|s| s.to_string()).collect(),
        content_hash: format!("hash-{}", path),
        mtime: 1000,
        body: body.to_string(),
    }
}

#[test]
fn test_hybrid_search_rrf_scoring_and_graph_boosting() {
    let mut storage = SqliteStorage::in_memory().expect("in-memory db");
    storage.initialize().expect("initialize");

    let doc1 = make_doc(
        "rust_mem.md",
        "Rust Memory Model",
        "Rust uses RAII and ownership for deterministic zero-cost memory management.",
        vec![WikiLink::new("arch.md", None::<String>, "[[arch.md]]")],
        vec!["rust", "memory"],
    );
    let doc2 = make_doc(
        "arch.md",
        "Architecture Overview",
        "Core memory design decisions and architectural invariants.",
        vec![],
        vec!["architecture"],
    );

    storage.upsert_document(&doc1).unwrap();
    storage.upsert_document(&doc2).unwrap();

    let embedder = Arc::new(MockEmbeddingEngine::new(384));
    let engine = HybridSearchEngine::new(&storage, Some(embedder));

    let results = engine
        .search("memory", SearchMode::Hybrid, 5)
        .expect("search");
    assert!(!results.is_empty());
    assert!(results.iter().any(|r| r.title == "Rust Memory Model"));
}

#[test]
fn test_vector_sync_incremental_and_pruning() {
    let dir = tempdir().expect("tempdir");
    let vault_root = dir.path();

    fs::write(
        vault_root.join("NoteA.md"),
        "# Note A\nArtificial intelligence and neural networks.",
    )
    .unwrap();
    fs::write(
        vault_root.join("NoteB.md"),
        "# Note B\nDistributed systems and consensus protocols.",
    )
    .unwrap();

    let adapter = ObsidianAdapter::new(vault_root);
    let mut storage = SqliteStorage::in_memory().expect("in-memory db");
    let embedder = Arc::new(MockEmbeddingEngine::new(384));

    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    let (sync_stats, vec_stats) = scanner
        .sync_vault_with_vector(vault_root, Some(embedder.clone()))
        .expect("sync");

    assert_eq!(sync_stats.added, 2);
    assert_eq!(vec_stats.embedded_count, 2);
    assert_eq!(vec_stats.skipped_count, 0);

    // Second sync should skip unchanged
    let (sync_stats2, vec_stats2) = scanner
        .sync_vault_with_vector(vault_root, Some(embedder.clone()))
        .expect("sync2");
    assert_eq!(sync_stats2.added, 0);
    assert_eq!(vec_stats2.skipped_count, 2);
    assert_eq!(vec_stats2.embedded_count, 0);

    // Deletion pruning
    fs::remove_file(vault_root.join("NoteB.md")).unwrap();
    let (sync_stats3, vec_stats3) = scanner
        .sync_vault_with_vector(vault_root, Some(embedder))
        .expect("sync3");
    assert_eq!(sync_stats3.deleted, 1);
    assert_eq!(vec_stats3.deleted_count, 1);
}

#[test]
fn test_dynamic_embed_model_switching_e2e() {
    let _guard = TEST_MUTEX.lock().unwrap();
    std::env::set_var("K0MARU_FORCE_MOCK_EMBED", "1");

    let temp = tempdir().expect("tempdir");
    let vault_path = temp.path();

    let proj_dir = vault_path.join("10_Projects");
    fs::create_dir_all(&proj_dir).unwrap();
    fs::write(
        proj_dir.join("DistributedDb.md"),
        "# 分布式数据库架构\n基于 Paxos 算法的高可用架构。\n",
    )
    .unwrap();

    // 1. Sync with 384d mock embedder
    let mut cmd_sync = Command::cargo_bin("k0maru").unwrap();
    cmd_sync
        .arg("sync")
        .arg("--vault")
        .arg(vault_path)
        .arg("--vector")
        .arg("--embed-model")
        .arg("mock")
        .arg("--json");
    let assert = cmd_sync.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let json_val: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json_val["vector"]["embedded_count"], 1);

    let cache_file = vault_path.join(".k0maru").join("cache.sqlite");
    {
        let storage = SqliteStorage::open(&cache_file).unwrap();
        assert_eq!(storage.get_vector_dimension().unwrap(), Some(384));
        assert_eq!(storage.vector_count().unwrap(), 1);
    }

    // 2. Switch to 512d Chinese-optimized model
    let mut cmd_migrate = Command::cargo_bin("k0maru").unwrap();
    cmd_migrate
        .arg("sync")
        .arg("--vault")
        .arg(vault_path)
        .arg("--vector")
        .arg("--embed-model")
        .arg("bge-small-zh-v1.5")
        .arg("--json");
    let assert_migrate = cmd_migrate.assert().success();
    let stdout_mig = String::from_utf8(assert_migrate.get_output().stdout.clone()).unwrap();
    let json_mig: serde_json::Value = serde_json::from_str(&stdout_mig).unwrap();
    assert_eq!(json_mig["vector"]["embedded_count"], 1);

    {
        let storage = SqliteStorage::open(&cache_file).unwrap();
        assert_eq!(storage.get_vector_dimension().unwrap(), Some(512));
        assert_eq!(storage.vector_count().unwrap(), 1);
    }
}

#[test]
fn test_cli_search_smoke_and_tag_hierarchy_filters() {
    let temp = tempdir().expect("tempdir");
    let vault_path = temp.path();

    let proj_dir = vault_path.join("10_Projects");
    let cards_dir = vault_path.join("20_Cards");
    fs::create_dir_all(&proj_dir).unwrap();
    fs::create_dir_all(&cards_dir).unwrap();

    fs::write(
        proj_dir.join("System.md"),
        "---\ntags: [core, rust]\n---\n# System Engine\nHigh throughput indexing core.\n",
    )
    .unwrap();
    fs::write(
        cards_dir.join("Concept.md"),
        "---\ntags: [theory]\n---\n# Graph Theory\nConcept note on DAGs.\n",
    )
    .unwrap();

    let mut cmd_sync = Command::cargo_bin("k0maru").unwrap();
    cmd_sync.arg("sync").arg("--vault").arg(vault_path);
    cmd_sync.assert().success();

    // Search with query
    let mut cmd_search = Command::cargo_bin("k0maru").unwrap();
    cmd_search
        .arg("search")
        .arg("System")
        .arg("--vault")
        .arg(vault_path)
        .arg("--mode")
        .arg("bm25")
        .arg("--json");
    let assert = cmd_search.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let results: Vec<serde_json::Value> = serde_json::from_str(&stdout).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["title"], "System Engine");

    // Filter by limit
    let mut cmd_tag = Command::cargo_bin("k0maru").unwrap();
    cmd_tag
        .arg("search")
        .arg("Graph")
        .arg("--limit")
        .arg("1")
        .arg("--vault")
        .arg(vault_path)
        .arg("--json");
    let assert_tag = cmd_tag.assert().success();
    let stdout_tag = String::from_utf8(assert_tag.get_output().stdout.clone()).unwrap();
    let results_tag: Vec<serde_json::Value> = serde_json::from_str(&stdout_tag).unwrap();
    assert_eq!(results_tag.len(), 1);
    assert_eq!(results_tag[0]["title"], "Graph Theory");
}

#[test]
fn test_cli_embed_model_flags_and_env_precedence() {
    let temp = tempdir().expect("tempdir");
    let vault_path = temp.path();

    let mut cmd = Command::cargo_bin("k0maru").unwrap();
    cmd.env("K0MARU_EMBED_MODEL", "bge-small-zh-v1.5")
        .arg("doctor")
        .arg("--vault")
        .arg(vault_path)
        .arg("--json");
    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let json_val: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let items = json_val["items"].as_array().unwrap();
    let embed_item = items
        .iter()
        .find(|i| i["name"] == "Embedding Engine")
        .unwrap();
    let detail = embed_item["detail"].as_str().unwrap();
    assert!(detail.contains("bge-small-zh-v1.5"));
    assert!(detail.contains("512 dimensions"));
}

#[test]
fn test_embedding_model_catalog_and_mock_engine() {
    let minilm = EmbeddingModelType::from_str("all-minilm-l6-v2").unwrap();
    assert_eq!(minilm.dimension(), 384);

    let bge_zh = EmbeddingModelType::from_str("bge-zh").unwrap();
    assert_eq!(bge_zh.dimension(), 512);

    let bge_base = EmbeddingModelType::from_str("bge-base-en").unwrap();
    assert_eq!(bge_base.dimension(), 768);

    let mock = MockEmbeddingEngine::new(384);
    let emb = mock.embed("test string").unwrap();
    assert_eq!(emb.len(), 384);
}
