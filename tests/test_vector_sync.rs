use std::fs;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

use assert_cmd::Command;
use k0maru::adapters::ObsidianAdapter;
use k0maru::scanner::{IncrementalScanner, VectorSyncEngine, VectorSyncStats};
use k0maru::storage::SqliteStorage;
use k0maru::vector::MockEmbeddingEngine;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn test_vector_sync_initial_embeds_all() {
    let dir = tempdir().expect("tempdir");
    let vault_root = dir.path();

    fs::write(
        vault_root.join("NoteA.md"),
        "# Note A\nThis note is about artificial intelligence and neural networks.",
    )
    .unwrap();
    fs::write(
        vault_root.join("NoteB.md"),
        "# Note B\nDistributed systems and raft consensus protocols.",
    )
    .unwrap();
    fs::write(
        vault_root.join("NoteC.md"),
        "# Note C\nCompiler design and abstract syntax trees.",
    )
    .unwrap();

    let adapter = ObsidianAdapter::new(vault_root);
    let mut storage = SqliteStorage::in_memory().expect("in-memory db");
    let embedder = Arc::new(MockEmbeddingEngine::new(384));

    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    let (sync_stats, vec_stats) = scanner
        .sync_vault_with_vector(vault_root, Some(embedder.clone()))
        .expect("sync_vault_with_vector should succeed");

    assert_eq!(sync_stats.added, 3);
    assert_eq!(vec_stats.embedded_count, 3);
    assert_eq!(vec_stats.skipped_count, 0);
    assert_eq!(vec_stats.deleted_count, 0);

    // Verify vectors and hashes are recorded in storage
    assert!(scanner
        .storage()
        .get_vector_content_hash("NoteA.md")
        .unwrap()
        .is_some());
    assert!(scanner
        .storage()
        .get_vector_content_hash("NoteB.md")
        .unwrap()
        .is_some());
    assert!(scanner
        .storage()
        .get_vector_content_hash("NoteC.md")
        .unwrap()
        .is_some());

    // Search nearest vector for NoteA
    let query_emb = embedder.embed_single("neural networks artificial intelligence");
    let hits = scanner
        .storage()
        .search_vectors(&query_emb, 3)
        .expect("search vectors");
    assert!(!hits.is_empty());
    assert_eq!(hits[0].0, "NoteA.md");
}

#[test]
fn test_vector_sync_zero_dirty_recomputation() {
    let dir = tempdir().expect("tempdir");
    let vault_root = dir.path();

    fs::write(vault_root.join("Alpha.md"), "# Alpha\nAlpha content.").unwrap();
    fs::write(vault_root.join("Beta.md"), "# Beta\nBeta content.").unwrap();

    let adapter = ObsidianAdapter::new(vault_root);
    let mut storage = SqliteStorage::in_memory().expect("in-memory db");
    let embedder = Arc::new(MockEmbeddingEngine::new(384));

    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);

    // Initial sync
    let (_s1, v1) = scanner
        .sync_vault_with_vector(vault_root, Some(embedder.clone()))
        .unwrap();
    assert_eq!(v1.embedded_count, 2);
    assert_eq!(v1.skipped_count, 0);

    // Second sync without modifications -> 0-dirty skip
    let (s2, v2) = scanner
        .sync_vault_with_vector(vault_root, Some(embedder.clone()))
        .unwrap();
    assert_eq!(s2.unchanged, 2);
    assert_eq!(s2.added, 0);
    assert_eq!(s2.modified, 0);
    assert_eq!(v2.embedded_count, 0);
    assert_eq!(v2.skipped_count, 2);
    assert_eq!(v2.deleted_count, 0);
}

#[test]
fn test_vector_sync_edit_single_file() {
    let dir = tempdir().expect("tempdir");
    let vault_root = dir.path();

    let file_a = vault_root.join("DocA.md");
    let file_b = vault_root.join("DocB.md");
    let file_c = vault_root.join("DocC.md");

    fs::write(&file_a, "# Doc A\nOriginal content A.").unwrap();
    fs::write(&file_b, "# Doc B\nOriginal content B.").unwrap();
    fs::write(&file_c, "# Doc C\nOriginal content C.").unwrap();

    let adapter = ObsidianAdapter::new(vault_root);
    let mut storage = SqliteStorage::in_memory().expect("in-memory db");
    let embedder = Arc::new(MockEmbeddingEngine::new(384));

    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    let (_s1, v1) = scanner
        .sync_vault_with_vector(vault_root, Some(embedder.clone()))
        .unwrap();
    assert_eq!(v1.embedded_count, 3);

    let old_hash_b = scanner
        .storage()
        .get_vector_content_hash("DocB.md")
        .unwrap()
        .unwrap();

    // Modify only DocB.md
    sleep(Duration::from_millis(50));
    fs::write(
        &file_b,
        "# Doc B\nHeavily edited content for B with new semantics.",
    )
    .unwrap();

    let (s2, v2) = scanner
        .sync_vault_with_vector(vault_root, Some(embedder.clone()))
        .unwrap();

    assert_eq!(s2.modified, 1);
    assert_eq!(s2.unchanged, 2);
    assert_eq!(v2.embedded_count, 1);
    assert_eq!(v2.skipped_count, 2);
    assert_eq!(v2.deleted_count, 0);

    let new_hash_b = scanner
        .storage()
        .get_vector_content_hash("DocB.md")
        .unwrap()
        .unwrap();
    assert_ne!(old_hash_b, new_hash_b);
}

#[test]
fn test_vector_sync_delete_file() {
    let dir = tempdir().expect("tempdir");
    let vault_root = dir.path();

    let file_x = vault_root.join("NoteX.md");
    let file_y = vault_root.join("NoteY.md");

    fs::write(&file_x, "# Note X\nPersistent note content.").unwrap();
    fs::write(&file_y, "# Note Y\nTemporary note to be purged soon.").unwrap();

    let adapter = ObsidianAdapter::new(vault_root);
    let mut storage = SqliteStorage::in_memory().expect("in-memory db");
    let embedder = Arc::new(MockEmbeddingEngine::new(384));

    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    let (_s1, v1) = scanner
        .sync_vault_with_vector(vault_root, Some(embedder.clone()))
        .unwrap();
    assert_eq!(v1.embedded_count, 2);
    assert!(scanner
        .storage()
        .get_vector_content_hash("NoteY.md")
        .unwrap()
        .is_some());

    // Remove NoteY.md from disk
    fs::remove_file(&file_y).unwrap();

    let (s2, v2) = scanner
        .sync_vault_with_vector(vault_root, Some(embedder.clone()))
        .unwrap();

    assert_eq!(s2.deleted, 1);
    assert_eq!(s2.unchanged, 1);
    assert_eq!(v2.embedded_count, 0);
    assert_eq!(v2.skipped_count, 1);
    assert_eq!(v2.deleted_count, 1);

    // Vector for NoteY.md should be gone
    assert_eq!(
        scanner
            .storage()
            .get_vector_content_hash("NoteY.md")
            .unwrap(),
        None
    );

    // NoteX.md should still be intact
    assert!(scanner
        .storage()
        .get_vector_content_hash("NoteX.md")
        .unwrap()
        .is_some());
}

#[test]
fn test_vector_sync_graceful_handling_none_embedder() {
    let dir = tempdir().expect("tempdir");
    let vault_root = dir.path();

    fs::write(vault_root.join("Test.md"), "# Test\nContent.").unwrap();

    let adapter = ObsidianAdapter::new(vault_root);
    let mut storage = SqliteStorage::in_memory().expect("in-memory db");

    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);

    // Call sync_vault_with_vector with embedder = None
    let (s, v) = scanner
        .sync_vault_with_vector(vault_root, None)
        .expect("sync_vault_with_vector with None embedder should succeed");

    assert_eq!(s.added, 1);
    assert_eq!(v.embedded_count, 0);
    assert_eq!(v.skipped_count, 0);
    assert_eq!(v.deleted_count, 0);

    // Vector metadata should NOT exist
    assert_eq!(
        scanner
            .storage()
            .get_vector_content_hash("Test.md")
            .unwrap(),
        None
    );

    // Call backwards-compatible sync_vault
    let s2 = scanner
        .sync_vault(vault_root)
        .expect("sync_vault should succeed");
    assert_eq!(s2.unchanged, 1);
}

#[test]
fn test_vector_sync_engine_custom_batch_size() {
    let mut storage = SqliteStorage::in_memory().expect("in-memory db");
    let embedder = MockEmbeddingEngine::new(384);

    // Insert 5 documents directly into storage
    for i in 1..=5 {
        let doc = k0maru::core::models::Document {
            path: std::path::PathBuf::from(format!("Doc{}.md", i)),
            title: format!("Doc Title {}", i),
            hierarchy: k0maru::core::models::HierarchyLevel::L3Evergreen,
            frontmatter: serde_json::json!({}),
            links: vec![],
            tags: vec![],
            content_hash: format!("000000000000000{:x}", i),
            mtime: 1000,
            body: format!("Body of document {}", i),
        };
        use k0maru::core::traits::CacheStorage;
        storage.upsert_document(&doc).unwrap();
    }

    let engine = VectorSyncEngine::with_batch_size(2);
    assert_eq!(engine.batch_size(), 2);

    let stats = engine
        .sync_storage(&mut storage, &embedder)
        .expect("sync_storage should succeed");

    assert_eq!(stats.embedded_count, 5);
    assert_eq!(stats.skipped_count, 0);
    assert_eq!(stats.deleted_count, 0);
    assert_eq!(stats.total_processed(), 5);

    // Verify all 5 have vector hashes
    for i in 1..=5 {
        let doc_id = format!("Doc{}.md", i);
        assert_eq!(
            storage.get_vector_content_hash(&doc_id).unwrap(),
            Some(i as u64)
        );
    }

    // Re-run -> all 5 skipped
    let stats2 = engine
        .sync_storage(&mut storage, &embedder)
        .expect("sync_storage second run");
    assert_eq!(stats2.embedded_count, 0);
    assert_eq!(stats2.skipped_count, 5);
}

#[test]
fn test_vector_sync_stats_models() {
    let stats = VectorSyncStats {
        embedded_count: 10,
        deleted_count: 2,
        skipped_count: 5,
    };
    assert_eq!(stats.total_processed(), 17);

    let default_stats = VectorSyncStats::new();
    assert_eq!(default_stats.total_processed(), 0);

    let serialized = serde_json::to_string(&stats).unwrap();
    let deserialized: VectorSyncStats = serde_json::from_str(&serialized).unwrap();
    assert_eq!(stats, deserialized);
}

#[test]
fn test_vector_sync_parse_content_hash() {
    use k0maru::scanner::parse_content_hash;

    let hex_hash = "0000000012345678";
    assert_eq!(parse_content_hash(hex_hash), 0x12345678);

    let hex_with_prefix = "0x0000000012345678";
    assert_eq!(parse_content_hash(hex_with_prefix), 0x12345678);

    let arbitrary_str = "random_content_non_hex_string";
    let parsed = parse_content_hash(arbitrary_str);
    assert_ne!(parsed, 0);
    // Deterministic fallback
    assert_eq!(parsed, parse_content_hash(arbitrary_str));
}

#[test]
fn test_cli_sync_vector_flag() {
    let temp_dir = tempdir().expect("tempdir");
    let test_card = temp_dir.path().join("VectorCard.md");
    fs::write(
        &test_card,
        "---\ntitle: Vector Card\n---\n# Vector Card\nSemantic embedding content\n",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("k0maru").expect("binary k0maru should exist");
    cmd.args([
        "sync",
        "--vault",
        temp_dir.path().to_str().unwrap(),
        "--vector",
        "--json",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("\"embedded_count\": 1"));
}
