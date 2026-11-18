use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Barrier};
use std::thread;

use assert_cmd::Command;
use k0maru::adapters::ObsidianAdapter;
use k0maru::core::traits::{CacheStorage, VaultAdapter};
use k0maru::mcp::McpServer;
use k0maru::scanner::IncrementalScanner;
use k0maru::storage::{
    ensure_sqlite_vec_registered, HybridSearchEngine, SearchMode, SqliteStorage,
    CURRENT_SCHEMA_VERSION,
};
use k0maru::text::{fts_hash_for, FTS_SEGMENTER_VERSION};
use k0maru::vector::MockEmbeddingEngine;
use k0maru::{run_diagnostics, run_diagnostics_with_compact, EmbeddingEngine, StatusLevel};
use rusqlite::{params, Connection};
use tempfile::tempdir;

fn f32_slice_to_bytes(slice: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(std::mem::size_of_val(slice));
    for &val in slice {
        bytes.extend_from_slice(&val.to_le_bytes());
    }
    bytes
}

fn create_legacy_v1_database(db_path: &std::path::Path) -> (Vec<(String, i64, String)>, Vec<f32>) {
    ensure_sqlite_vec_registered();
    let conn = Connection::open(db_path).expect("create legacy v1 db");
    conn.execute_batch(
        r#"
        CREATE TABLE documents (
            path TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            hierarchy TEXT NOT NULL,
            frontmatter TEXT NOT NULL,
            content_hash TEXT NOT NULL,
            mtime INTEGER NOT NULL,
            body TEXT NOT NULL
        );

        CREATE TABLE links (
            source_path TEXT NOT NULL,
            target TEXT NOT NULL,
            alias TEXT,
            raw_text TEXT NOT NULL
        );

        CREATE TABLE tags (
            document_path TEXT NOT NULL,
            tag TEXT NOT NULL
        );

        CREATE VIRTUAL TABLE documents_fts USING fts5(
            path UNINDEXED,
            title,
            body,
            tags,
            tokenize = 'unicode61'
        );

        CREATE TABLE vector_schema_info (
            dimension INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );

        CREATE VIRTUAL TABLE document_vectors USING vec0(
            id TEXT PRIMARY KEY,
            embedding float[384] distance_metric=cosine
        );

        CREATE TABLE vector_metadata (
            document_id TEXT PRIMARY KEY,
            content_hash INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );

        PRAGMA user_version = 0;
        "#,
    )
    .expect("create legacy schema");

    conn.execute(
        "INSERT INTO vector_schema_info (dimension, updated_at) VALUES (384, 1700000000)",
        [],
    )
    .unwrap();

    // Insert 4 docs then delete the 2nd to leave an intentional rowid gap (1, 3, 4).
    let docs = vec![
        (
            "10_Projects/payment-gateway.md",
            "Payment Gateway",
            "L1Resource",
            "{\"title\":\"Payment Gateway\",\"tags\":[\"gateway\",\"settlement\"]}",
            "hash_gateway_001",
            1700000001i64,
            "Distributed payment-gateway pipeline using ADR-042 and acquire_fenced_lease.",
            vec!["gateway", "settlement"],
        ),
        (
            "00_Daily/ephemeral.md",
            "Ephemeral Scratch",
            "L0Ephemeral",
            "{}",
            "hash_ephemeral_002",
            1700000002i64,
            "Temporary note to be deleted before migration.",
            vec!["temp"],
        ),
        (
            "20_Cards/vector-search.md",
            "向量检索引擎",
            "L3Evergreen",
            "{\"title\":\"向量检索引擎\"}",
            "hash_vector_003",
            1700000003i64,
            "使用Rust构建分布式结算与向量检索引擎，支持并发一致性。",
            vec!["vector", "rust"],
        ),
        (
            "20_Cards/legacy-adr.md",
            "ADR-024 Notes",
            "L3Evergreen",
            "{\"title\":\"ADR-024 Notes\"}",
            "hash_adr_004",
            1700000004i64,
            "ADR-024 describes issue 042 in legacy retry logic.",
            vec!["adr"],
        ),
    ];

    for (path, title, hierarchy, fm, hash, mtime, body, tags) in &docs {
        conn.execute(
            "INSERT INTO documents (path, title, hierarchy, frontmatter, content_hash, mtime, body) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![path, title, hierarchy, fm, hash, mtime, body],
        )
        .unwrap();
        let tags_str = tags.join(" ");
        conn.execute(
            "INSERT INTO documents_fts (path, title, body, tags) VALUES (?1, ?2, ?3, ?4)",
            params![path, title, body, tags_str],
        )
        .unwrap();
        for tag in tags {
            conn.execute(
                "INSERT INTO tags (document_path, tag) VALUES (?1, ?2)",
                params![path, tag],
            )
            .unwrap();
        }
    }

    conn.execute(
        "INSERT INTO links (source_path, target, alias, raw_text) VALUES (?1, ?2, NULL, ?3)",
        params![
            "10_Projects/payment-gateway.md",
            "vector-search",
            "[[vector-search]]"
        ],
    )
    .unwrap();

    // Delete doc 2 to create rowid gap
    conn.execute(
        "DELETE FROM documents WHERE path = '00_Daily/ephemeral.md'",
        [],
    )
    .unwrap();
    conn.execute(
        "DELETE FROM documents_fts WHERE path = '00_Daily/ephemeral.md'",
        [],
    )
    .unwrap();
    conn.execute(
        "DELETE FROM tags WHERE document_path = '00_Daily/ephemeral.md'",
        [],
    )
    .unwrap();

    let embedder = MockEmbeddingEngine::new(384);
    let mut expected_rows = Vec::new();
    {
        let mut stmt = conn
            .prepare(
                "SELECT path, rowid, content_hash, title, body FROM documents ORDER BY rowid ASC",
            )
            .unwrap();
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                ))
            })
            .unwrap();
        for r in rows {
            let (path, rowid, content_hash, title, body) = r.unwrap();
            let emb = embedder.embed(&format!("{} {}", title, body)).unwrap();
            conn.execute(
                "INSERT INTO document_vectors (id, embedding) VALUES (?1, ?2)",
                params![path, f32_slice_to_bytes(&emb)],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO vector_metadata (document_id, content_hash, updated_at) VALUES (?1, ?2, 1700000005)",
                params![path, rowid * 100],
            )
            .unwrap();
            expected_rows.push((path, rowid, content_hash));
        }
    }

    let query_vec = embedder.embed("向量检索引擎 分布式结算").unwrap();
    (expected_rows, query_vec)
}

#[test]
fn test_legacy_v1_to_v2_migration_preserves_vectors_rowids_and_enables_cjk_search() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("cache.sqlite");
    let (expected_rows, query_vec) = create_legacy_v1_database(&db_path);

    let pre_knn: Vec<(String, f32)> = {
        let conn = Connection::open(&db_path).unwrap();
        let mut stmt = conn
            .prepare("SELECT id, distance FROM document_vectors WHERE embedding MATCH ?1 AND k = 3 ORDER BY distance")
            .unwrap();
        stmt.query_map(params![f32_slice_to_bytes(&query_vec)], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
    };
    assert_eq!(pre_knn.len(), 3);

    // Open with v2 SqliteStorage -> triggers automatic v1->v2 migration.
    let storage = SqliteStorage::open(&db_path).expect("open_rw migrates v1 to v2");
    let conn = storage.connection();

    let user_version: i32 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(user_version, CURRENT_SCHEMA_VERSION);

    // Legacy documents_fts must be completely dropped.
    let legacy_exists: i64 = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'documents_fts'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(legacy_exists, 0);

    // k0maru_meta must record segmenter version.
    let seg_ver: String = conn
        .query_row(
            "SELECT value FROM k0maru_meta WHERE key = 'fts_segmenter_version'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(seg_ver, FTS_SEGMENTER_VERSION.to_string());

    // documents.id must equal old rowid, content_hash unchanged, fts_hash populated.
    for (path, old_rowid, old_hash) in &expected_rows {
        let (id, hash, fts_hash): (i64, String, Option<String>) = conn
            .query_row(
                "SELECT id, content_hash, fts_hash FROM documents WHERE path = ?1",
                params![path],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(id, *old_rowid, "documents.id must preserve legacy rowid");
        assert_eq!(&hash, old_hash);
        assert_eq!(fts_hash, Some(fts_hash_for(old_hash)));
    }

    // Vector counts and KNN search results must be identical.
    assert_eq!(storage.vector_count().unwrap(), 3);
    let vec_table_count: i64 = conn
        .query_row("SELECT count(*) FROM document_vectors", [], |r| r.get(0))
        .unwrap();
    assert_eq!(vec_table_count, 3);
    let post_knn = storage.search_vectors(&query_vec, 3).unwrap();
    assert_eq!(post_knn, pre_knn);

    // Gate checklist (e): CJK & identifier search on migrated v2 FTS5 index.
    let cjk_hits = storage.search_fts("向量检索", 5).unwrap();
    assert_eq!(cjk_hits.len(), 1);
    assert_eq!(cjk_hits[0].path, PathBuf::from("20_Cards/vector-search.md"));

    let mixed_hits = storage.search_fts("使用Rust", 5).unwrap();
    assert_eq!(mixed_hits.len(), 1);
    assert_eq!(
        mixed_hits[0].path,
        PathBuf::from("20_Cards/vector-search.md")
    );

    // ADR-042 must match payment-gateway.md, NOT legacy-adr.md (which has ADR-024 + 042).
    let adr_hits = storage.search_fts("ADR-042", 5).unwrap();
    assert_eq!(adr_hits.len(), 1);
    assert_eq!(
        adr_hits[0].path,
        PathBuf::from("10_Projects/payment-gateway.md")
    );

    let ident_hits = storage.search_fts("acquire_fenced_lease", 5).unwrap();
    assert_eq!(ident_hits.len(), 1);

    let sub_ident_hits = storage.search_fts("acquire", 5).unwrap();
    assert_eq!(sub_ident_hits.len(), 1);
}

#[test]
fn test_downgrade_guard_and_read_only_open() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("cache.sqlite");

    {
        let mut storage = SqliteStorage::open_rw(&db_path).unwrap();
        let doc = k0maru::Document {
            path: PathBuf::from("20_Cards/ro.md"),
            title: "Read Only Card".to_string(),
            hierarchy: k0maru::HierarchyLevel::L3Evergreen,
            frontmatter: serde_json::json!({}),
            links: vec![],
            tags: vec!["readonly".to_string()],
            content_hash: "ro_hash".to_string(),
            mtime: 100,
            body: "Body line after frontmatter.".to_string(),
            body_line_offset: 5,
        };
        storage.upsert_document(&doc).unwrap();
    }

    // open_read_only succeeds and preserves body_line_offset while rejecting writes.
    {
        let mut ro = SqliteStorage::open_read_only(&db_path).expect("open_read_only");
        let retrieved = ro
            .get_document(std::path::Path::new("20_Cards/ro.md"))
            .unwrap()
            .expect("doc exists");
        assert_eq!(retrieved.body_line_offset, 5);
        assert_eq!(ro.search_fts("frontmatter", 5).unwrap().len(), 1);

        let write_attempt = ro.upsert_document(&retrieved);
        assert!(
            write_attempt.is_err(),
            "open_read_only must reject write operations"
        );
    }

    // Bump user_version > 3 and verify downgrade guard rejects both open_rw and open_read_only.
    {
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch("PRAGMA user_version = 99;").unwrap();
    }

    assert!(
        SqliteStorage::open_rw(&db_path).is_err(),
        "open_rw must refuse user_version > 3"
    );
    assert!(
        SqliteStorage::open_read_only(&db_path).is_err(),
        "open_read_only must refuse user_version > 3"
    );
}

#[test]
fn test_concurrent_open_during_v1_migration_runs_once_without_fallback() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("cache.sqlite");
    create_legacy_v1_database(&db_path);

    let barrier = Arc::new(Barrier::new(2));
    let mut handles = Vec::new();
    for _ in 0..2 {
        let path_clone = db_path.clone();
        let b = barrier.clone();
        handles.push(thread::spawn(move || {
            b.wait();
            let storage = SqliteStorage::open_rw(&path_clone).expect("concurrent open_rw");
            let ver: i32 = storage
                .connection()
                .query_row("PRAGMA user_version", [], |r| r.get(0))
                .unwrap();
            assert_eq!(ver, CURRENT_SCHEMA_VERSION);
            assert_eq!(storage.vector_count().unwrap(), 3);
            storage.search_fts("向量检索", 5).unwrap().len()
        }));
    }

    for handle in handles {
        assert_eq!(handle.join().unwrap(), 1);
    }
}

#[test]
fn test_mcp_server_never_caches_in_memory_fallback_on_open_failure() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path();
    fs::create_dir_all(vault_path.join("20_Cards")).unwrap();
    fs::write(
        vault_path.join("20_Cards/Note.md"),
        "# Note\nDistributed settlement note.\n",
    )
    .unwrap();

    // Poison cache.sqlite with unsupported future user_version so open() fails cleanly.
    let cache_dir = vault_path.join(".k0maru");
    fs::create_dir_all(&cache_dir).unwrap();
    let cache_path = cache_dir.join("cache.sqlite");
    {
        let _ = SqliteStorage::open_rw(&cache_path).unwrap();
        let conn = Connection::open(&cache_path).unwrap();
        conn.execute_batch("PRAGMA user_version = 99;").unwrap();
    }

    let mut server = McpServer::new_with_model(vault_path, Some("mock"));
    let res1 = server.handle_tool_call(
        serde_json::json!(1),
        &serde_json::json!({
            "name": "recall_memory",
            "arguments": { "query": "Distributed settlement" }
        }),
    );
    assert_eq!(res1["result"]["isError"], serde_json::Value::Bool(true));
    assert!(res1["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("schema mismatch"));
    assert!(
        !server.has_cached_storage(),
        "McpServer must NOT cache storage when opening disk DB failed"
    );

    // Restore valid schema version on disk; next call must open and cache the real DB.
    {
        let conn = Connection::open(&cache_path).unwrap();
        conn.execute_batch("PRAGMA user_version = 2;").unwrap();
    }
    let res2 = server.handle_tool_call(
        serde_json::json!(2),
        &serde_json::json!({
            "name": "recall_memory",
            "arguments": { "query": "Distributed settlement" }
        }),
    );
    assert!(res2["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("Note.md"));
    assert!(
        server.has_cached_storage(),
        "McpServer should cache persistent storage once disk open succeeds"
    );
}

#[test]
fn test_old_binary_coexistence_fts_repair_and_doctor_compact() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path();
    fs::create_dir_all(vault_path.join("20_Cards")).unwrap();
    fs::write(
        vault_path.join("20_Cards/Alpha.md"),
        "# Alpha\nPayment gateway architecture.\n",
    )
    .unwrap();
    fs::write(
        vault_path.join("20_Cards/Beta.md"),
        "# Beta\nConsensus protocol notes.\n",
    )
    .unwrap();

    let cache_path = vault_path.join(".k0maru").join("cache.sqlite");
    let embedder = Arc::new(MockEmbeddingEngine::new(384));
    {
        let mut storage = SqliteStorage::open_rw(&cache_path).unwrap();
        storage.ensure_vector_dimension(384).unwrap();
        let adapter = ObsidianAdapter::new(vault_path);
        let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
        scanner
            .sync_vault_with_vector(vault_path, Some(embedder.clone()))
            .unwrap();
    }

    // Simulate an older v1.1.0 writer: recreate documents_fts, clear fts_hash on Alpha,
    // and insert a stale row Gamma with fts_hash = NULL.
    {
        let conn = Connection::open(&cache_path).unwrap();
        conn.execute_batch(
            r#"
            CREATE VIRTUAL TABLE IF NOT EXISTS documents_fts USING fts5(
                path UNINDEXED, title, body, tags, tokenize = 'unicode61'
            );
            UPDATE documents SET body = '分布式结算引擎', content_hash = 'new_hash_alpha', fts_hash = 'old_hash:0'
            WHERE path = '20_Cards/Alpha.md';
            INSERT INTO documents (path, title, hierarchy, frontmatter, content_hash, mtime, body)
            VALUES ('20_Cards/Gamma.md', 'Gamma', 'L3Evergreen', '{}', 'hash_gamma', 1700000000, '旧版本写入文档');
            "#,
        )
        .unwrap();
    }

    // Doctor must flag both stale fts_hash and stray legacy documents_fts.
    let diag = run_diagnostics(vault_path, Some(dir.path()));
    let fts_item = diag
        .items
        .iter()
        .find(|i| i.name == "FTS5 Search Index")
        .unwrap();
    assert_eq!(fts_item.status, StatusLevel::Warn);
    assert!(fts_item.detail.contains("2 of 3"));

    let legacy_item = diag
        .items
        .iter()
        .find(|i| i.name == "Legacy FTS5 Table")
        .unwrap();
    assert_eq!(legacy_item.status, StatusLevel::Warn);

    // Remove Gamma from disk? Keep Gamma in DB only via repair_stale_fts directly first,
    // then run IncrementalScanner::sync to test both paths.
    {
        let mut storage = SqliteStorage::open_rw(&cache_path).unwrap();
        let repaired = storage.repair_stale_fts().unwrap();
        assert_eq!(repaired, 2);
        assert_eq!(storage.search_fts("旧版本", 5).unwrap().len(), 1);
        assert_eq!(storage.search_fts("分布式结算", 5).unwrap().len(), 1);
    }

    // Delete Beta from disk and sync to create freelist pages, then run doctor --compact.
    fs::remove_file(vault_path.join("20_Cards/Beta.md")).unwrap();
    {
        let mut storage = SqliteStorage::open_rw(&cache_path).unwrap();
        let adapter = ObsidianAdapter::new(vault_path);
        let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
        scanner.sync(false).unwrap();
    }

    let compact_report = run_diagnostics_with_compact(vault_path, Some(dir.path()), true);
    let compact_item = compact_report
        .items
        .iter()
        .find(|i| i.name == "Cache Compaction")
        .unwrap();
    assert_eq!(compact_item.status, StatusLevel::Pass);
    assert!(compact_item.detail.contains("KNN self-check passed"));

    let frag_item = compact_report
        .items
        .iter()
        .find(|i| i.name == "Cache Fragmentation")
        .unwrap();
    assert_eq!(frag_item.status, StatusLevel::Pass);
    assert!(frag_item.detail.contains("0.00 KB reclaimable"));

    // Also verify CLI `k0maru doctor --compact --json` succeeds.
    let mut cmd = Command::cargo_bin("k0maru").unwrap();
    cmd.args([
        "doctor",
        "--vault",
        vault_path.to_str().unwrap(),
        "--home",
        dir.path().to_str().unwrap(),
        "--compact",
        "--json",
    ])
    .assert()
    .success();
}

#[test]
fn test_ignore_purges_vectors_and_unignore_restores_end_to_end() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path();
    fs::create_dir_all(vault_path.join("20_Cards")).unwrap();
    fs::write(
        vault_path.join("20_Cards/Keep.md"),
        "# Keep Card\nDistributed settlement consensus.\n",
    )
    .unwrap();
    fs::write(
        vault_path.join("20_Cards/Ignored.md"),
        "# Ignored Card\nSecret ignored topic xyzzy.\n",
    )
    .unwrap();

    let cache_path = vault_path.join(".k0maru").join("cache.sqlite");
    let embedder = Arc::new(MockEmbeddingEngine::new(384));
    let mut storage = SqliteStorage::open_rw(&cache_path).unwrap();
    storage.ensure_vector_dimension(384).unwrap();

    let adapter = ObsidianAdapter::new(vault_path);
    {
        let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
        let (_, vec_stats) = scanner
            .sync_vault_with_vector(vault_path, Some(embedder.clone()))
            .unwrap();
        assert_eq!(vec_stats.embedded_count, 2);
    }
    assert_eq!(storage.vector_count().unwrap(), 2);

    // Ignore Ignored.md and run plain sync (without vector sync) -> must purge FTS + vectors!
    fs::write(vault_path.join(".k0maruignore"), "20_Cards/Ignored.md\n").unwrap();
    {
        let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
        let stats = scanner.sync(false).unwrap();
        assert_eq!(stats.deleted, 1);
    }
    assert_eq!(storage.document_count().unwrap(), 1);
    assert_eq!(storage.vector_count().unwrap(), 1);
    let vec_rows: i64 = storage
        .connection()
        .query_row("SELECT count(*) FROM document_vectors", [], |r| r.get(0))
        .unwrap();
    assert_eq!(vec_rows, 1);
    assert!(storage.search_fts("xyzzy", 5).unwrap().is_empty());

    let hybrid = HybridSearchEngine::new(&storage, Some(embedder.clone()));
    let hits = hybrid
        .search("Secret ignored topic xyzzy", SearchMode::Hybrid, 5)
        .unwrap();
    assert!(
        hits.iter().all(|h| h.path != "20_Cards/Ignored.md"),
        "Ignored document must not resurface via ghost vectors"
    );

    // Un-ignore and re-sync -> Ignored.md is restored cleanly.
    fs::remove_file(vault_path.join(".k0maruignore")).unwrap();
    {
        let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
        let (sync_stats, vec_stats) = scanner
            .sync_vault_with_vector(vault_path, Some(embedder))
            .unwrap();
        assert_eq!(sync_stats.added, 1);
        assert_eq!(vec_stats.embedded_count, 1);
    }
    assert_eq!(storage.document_count().unwrap(), 2);
    assert_eq!(storage.vector_count().unwrap(), 2);
    assert_eq!(storage.search_fts("xyzzy", 5).unwrap().len(), 1);
}

fn create_v2_database_with_vectors(
    db_path: &std::path::Path,
    doc_count: usize,
) -> (Vec<(i64, String)>, Vec<f32>) {
    ensure_sqlite_vec_registered();
    let conn = Connection::open(db_path).expect("create v2 db");
    conn.execute_batch(
        r#"
        CREATE TABLE documents (
            id INTEGER PRIMARY KEY,
            path TEXT NOT NULL UNIQUE,
            title TEXT NOT NULL,
            hierarchy TEXT NOT NULL,
            frontmatter TEXT NOT NULL,
            content_hash TEXT NOT NULL,
            mtime INTEGER NOT NULL,
            body TEXT NOT NULL,
            body_line_offset INTEGER NOT NULL DEFAULT 0,
            fts_hash TEXT
        );
        CREATE TABLE links (
            source_path TEXT NOT NULL,
            target TEXT NOT NULL,
            alias TEXT,
            raw_text TEXT NOT NULL
        );
        CREATE TABLE tags (
            document_path TEXT NOT NULL,
            tag TEXT NOT NULL
        );
        CREATE VIRTUAL TABLE fts_documents USING fts5(
            title, body, tags,
            content='', contentless_delete=1,
            tokenize='unicode61 remove_diacritics 2'
        );
        CREATE TABLE k0maru_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
        CREATE TABLE vector_schema_info (dimension INTEGER NOT NULL, updated_at INTEGER NOT NULL);
        CREATE VIRTUAL TABLE document_vectors USING vec0(
            id TEXT PRIMARY KEY,
            embedding float[384] distance_metric=cosine
        );
        CREATE TABLE vector_metadata (
            document_id TEXT PRIMARY KEY,
            content_hash INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );
        INSERT INTO vector_schema_info (dimension, updated_at) VALUES (384, 1700000000);
        INSERT INTO k0maru_meta (key, value) VALUES ('fts_segmenter_version', '1');
        PRAGMA user_version = 2;
        "#,
    )
    .expect("v2 ddl");

    let embedder = MockEmbeddingEngine::new(384);
    let mut ids = Vec::with_capacity(doc_count);
    let tx = conn.unchecked_transaction().unwrap();
    for i in 0..doc_count {
        let path = format!("20_Cards/settlement-{:04}.md", i);
        let title = format!("Settlement Node {}", i);
        let body = format!(
            "## Overview\nDistributed settlement consensus node {}.\n\n## Details\nLedger shard {}.",
            i, i
        );
        let hash = format!("hash_{:04}", i);
        let fts_hash = fts_hash_for(&hash);
        tx.execute(
            "INSERT INTO documents (id, path, title, hierarchy, frontmatter, content_hash, mtime, body, body_line_offset, fts_hash) VALUES (?1, ?2, ?3, 'L3Evergreen', '{}', ?4, 1700000000, ?5, 0, ?6)",
            params![(i + 1) as i64, path, title, hash, body, fts_hash],
        )
        .unwrap();
        tx.execute(
            "INSERT INTO fts_documents (rowid, title, body, tags) VALUES (?1, ?2, ?3, '')",
            params![(i + 1) as i64, title, body],
        )
        .unwrap();
        if i < 5 {
            let emb = embedder.embed(&format!("{} {}", title, body)).unwrap();
            tx.execute(
                "INSERT INTO document_vectors (id, embedding) VALUES (?1, ?2)",
                params![path, f32_slice_to_bytes(&emb)],
            )
            .unwrap();
            tx.execute(
                "INSERT INTO vector_metadata (document_id, content_hash, updated_at) VALUES (?1, ?2, 1700000001)",
                params![path, (i + 1) as i64],
            )
            .unwrap();
        }
        ids.push(((i + 1) as i64, path));
    }
    tx.commit().unwrap();

    let q_vec = embedder
        .embed("Distributed settlement consensus node 2")
        .unwrap();
    (ids, q_vec)
}

#[test]
fn test_v2_to_v3_migration_preserves_vectors_is_fast_and_idempotent() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("cache.sqlite");
    let (expected_ids, query_vec) = create_v2_database_with_vectors(&db_path, 1000);

    // Read-only open on a v2 database must succeed and guard all v3 tables gracefully.
    {
        let ro_v2 = SqliteStorage::open_read_only(&db_path).expect("ro open on v2");
        assert!(ro_v2.get_sections(1).unwrap().is_empty());
        assert!(ro_v2
            .search_section_fts("settlement", &[1, 2], 10)
            .unwrap()
            .is_empty());
        assert!(ro_v2
            .search_section_vectors(&query_vec, 5)
            .unwrap()
            .is_empty());
        assert!(!ro_v2.section_vectors_ready("mock-embedding-384"));
        assert!(!ro_v2.link_edges_clean());
        assert!(ro_v2.edges_touching(&[1, 2]).unwrap().is_empty());
        assert!(!ro_v2
            .record_access("20_Cards/settlement-0000.md", 1700000100)
            .unwrap());
        assert!(ro_v2
            .access_stats(&["20_Cards/settlement-0000.md"])
            .unwrap()
            .is_empty());
        assert!(ro_v2
            .git_times(&["20_Cards/settlement-0000.md"])
            .unwrap()
            .is_empty());
    }

    let pre_knn: Vec<(String, f32)> = {
        let conn = Connection::open(&db_path).unwrap();
        let mut stmt = conn
            .prepare("SELECT id, distance FROM document_vectors WHERE embedding MATCH ?1 AND k = 5 ORDER BY distance")
            .unwrap();
        stmt.query_map(params![f32_slice_to_bytes(&query_vec)], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
    };

    // Open with v3 binary: must migrate 1k docs in < 1s (no parsing in open_rw).
    let t0 = std::time::Instant::now();
    let storage = SqliteStorage::open_rw(&db_path).expect("v2->v3 open_rw");
    let elapsed = t0.elapsed();
    assert!(
        elapsed < std::time::Duration::from_secs(1),
        "v2->v3 open_rw took {:?} on 1k docs (expected < 1s)",
        elapsed
    );

    let conn = storage.connection();
    let ver: i32 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(ver, CURRENT_SCHEMA_VERSION);
    assert_eq!(storage.vector_count().unwrap(), 5);
    assert_eq!(storage.search_vectors(&query_vec, 5).unwrap(), pre_knn);

    // No synchronous section parsing during open_rw.
    let sec_count: i64 = conn
        .query_row("SELECT count(*) FROM document_sections", [], |r| r.get(0))
        .unwrap();
    assert_eq!(sec_count, 0);

    // section_vectors must have been created at dim 384 alongside document_vectors.
    let has_sec_vecs: i64 = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'section_vectors'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(has_sec_vecs, 1);

    // Document IDs remain unchanged.
    for (id, path) in expected_ids.iter().take(10) {
        assert_eq!(
            storage.doc_id_for_path(std::path::Path::new(path)).unwrap(),
            Some(*id)
        );
    }
    drop(storage);

    // F20 idempotency check: reset user_version = 2 on a DB that already has v3 tables and re-open.
    {
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch("PRAGMA user_version = 2;").unwrap();
    }
    let storage2 = SqliteStorage::open_rw(&db_path).expect("idempotent re-open after reset to v2");
    let ver2: i32 = storage2
        .connection()
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(ver2, CURRENT_SCHEMA_VERSION);
    assert_eq!(storage2.vector_count().unwrap(), 5);
    assert_eq!(storage2.search_vectors(&query_vec, 5).unwrap(), pre_knn);
}

#[test]
fn test_f15_legacy_frontmatter_offset_self_heals_on_sync() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path();
    fs::create_dir_all(vault_path.join("10_Projects")).unwrap();
    let note_rel = "10_Projects/payment-gateway.md";
    let note_abs = vault_path.join(note_rel);
    fs::write(
        &note_abs,
        "---\ntitle: Payment Gateway\ntags:\n  - gateway\n---\n\n# Payment Gateway\n\n## Architecture\nLedger settlement flow.\n",
    )
    .unwrap();

    let adapter = ObsidianAdapter::new(vault_path);
    let parsed = adapter
        .read_document(std::path::Path::new(note_rel))
        .unwrap();
    assert!(
        parsed.body_line_offset > 0,
        "freshly parsed file must have non-zero body_line_offset"
    );

    // Seed a v2 DB where the row has matching disk mtime & content_hash, non-empty frontmatter,
    // and legacy body_line_offset = 0 (simulating F15 from v1->v2 migration).
    let cache_path = vault_path.join(".k0maru").join("cache.sqlite");
    fs::create_dir_all(cache_path.parent().unwrap()).unwrap();
    {
        let conn = Connection::open(&cache_path).unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE documents (
                id INTEGER PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                title TEXT NOT NULL,
                hierarchy TEXT NOT NULL,
                frontmatter TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                mtime INTEGER NOT NULL,
                body TEXT NOT NULL,
                body_line_offset INTEGER NOT NULL DEFAULT 0,
                fts_hash TEXT
            );
            CREATE TABLE links (source_path TEXT NOT NULL, target TEXT NOT NULL, alias TEXT, raw_text TEXT NOT NULL);
            CREATE TABLE tags (document_path TEXT NOT NULL, tag TEXT NOT NULL);
            CREATE VIRTUAL TABLE fts_documents USING fts5(title, body, tags, content='', contentless_delete=1, tokenize='unicode61 remove_diacritics 2');
            CREATE TABLE k0maru_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
            PRAGMA user_version = 2;
            "#,
        )
        .unwrap();
        let fm_str = serde_json::to_string(&parsed.frontmatter).unwrap();
        conn.execute(
            "INSERT INTO documents (id, path, title, hierarchy, frontmatter, content_hash, mtime, body, body_line_offset, fts_hash) VALUES (1, ?1, ?2, 'L1Resource', ?3, ?4, ?5, ?6, 0, ?7)",
            params![
                note_rel,
                parsed.title,
                fm_str,
                parsed.content_hash,
                parsed.mtime as i64,
                parsed.body,
                fts_hash_for(&parsed.content_hash),
            ],
        )
        .unwrap();
    }

    // Opening with v3 sets mtime = -1 sentinel so the next sync re-reads it once.
    let mut storage = SqliteStorage::open_rw(&cache_path).unwrap();
    let sentinel_mtime: i64 = storage
        .connection()
        .query_row(
            "SELECT mtime FROM documents WHERE path = ?1",
            params![note_rel],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(sentinel_mtime, -1);

    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    let stats = scanner.sync(false).unwrap();
    assert_eq!(stats.modified, 1);

    let healed = storage
        .get_document(std::path::Path::new(note_rel))
        .unwrap()
        .unwrap();
    assert_eq!(healed.body_line_offset, parsed.body_line_offset);

    let doc_id = storage
        .doc_id_for_path(std::path::Path::new(note_rel))
        .unwrap()
        .unwrap();
    let sections = storage.get_sections(doc_id).unwrap();
    let arch = sections
        .iter()
        .find(|s| s.heading == "Architecture")
        .expect("Architecture section present");
    // Line 1..=5 is frontmatter, 6 blank, 7 H1, 8 blank, 9 is "## Architecture"
    assert_eq!(arch.start_line, 9);
}

#[test]
fn test_section_text_hash_reuse_partial_edit_and_delete_purge() {
    let mut storage = SqliteStorage::in_memory().unwrap();
    storage.ensure_vector_dimension(4).unwrap();

    let make_doc = |body: &str, hash: &str| k0maru::Document {
        path: PathBuf::from("20_Cards/settlement.md"),
        title: "Settlement".to_string(),
        hierarchy: k0maru::HierarchyLevel::L3Evergreen,
        frontmatter: serde_json::json!({}),
        links: vec![],
        tags: vec![],
        content_hash: hash.to_string(),
        mtime: 1700000000,
        body: body.to_string(),
        body_line_offset: 0,
    };

    let body_v1 = "## Alpha\nAlpha consensus body.\n\n## Beta\nBeta routing body.\n\n## Gamma\nGamma finality body.\n\n";
    storage
        .upsert_document(&make_doc(body_v1, "hash_v1"))
        .unwrap();

    let doc_id = storage
        .doc_id_for_path(std::path::Path::new("20_Cards/settlement.md"))
        .unwrap()
        .unwrap();
    let secs_v1 = storage.get_sections(doc_id).unwrap();
    assert_eq!(secs_v1.len(), 3);
    let id_alpha = secs_v1[0].id;
    let id_beta = secs_v1[1].id;
    let id_gamma = secs_v1[2].id;

    // Seed mock section vectors + metadata for all 3 sections.
    let emb_bytes = f32_slice_to_bytes(&[1.0, 0.0, 0.0, 0.0]);
    for s in &secs_v1 {
        storage
            .connection()
            .execute(
                "INSERT INTO section_vectors (section_id, doc_id, embedding) VALUES (?1, ?2, ?3)",
                params![s.id, doc_id, emb_bytes],
            )
            .unwrap();
        storage
            .connection()
            .execute(
                "INSERT INTO section_vector_meta (section_id, text_hash, updated_at) VALUES (?1, 1, 1700000000)",
                params![s.id],
            )
            .unwrap();
    }

    // Edit only Beta and reorder Gamma before Alpha.
    let body_v2 = "## Gamma\nGamma finality body.\n\n## Beta\nBeta routing body UPDATED.\n\n## Alpha\nAlpha consensus body.\n\n";
    storage
        .upsert_document(&make_doc(body_v2, "hash_v2"))
        .unwrap();

    let secs_v2 = storage.get_sections(doc_id).unwrap();
    assert_eq!(secs_v2.len(), 3);
    assert_eq!(secs_v2[0].heading, "Gamma");
    assert_eq!(
        secs_v2[0].id, id_gamma,
        "Gamma ID must be reused across reorder"
    );
    assert_eq!(secs_v2[1].heading, "Beta");
    assert_ne!(
        secs_v2[1].id, id_beta,
        "Edited Beta must receive new section ID"
    );
    assert_eq!(secs_v2[2].heading, "Alpha");
    assert_eq!(secs_v2[2].id, id_alpha, "Alpha ID must be reused");

    // Old Beta vector and FTS row must have been purged, while Alpha & Gamma remain.
    let remaining_vec_ids: Vec<i64> = {
        let mut stmt = storage
            .connection()
            .prepare("SELECT section_id FROM section_vector_meta ORDER BY section_id ASC")
            .unwrap();
        stmt.query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    };
    assert_eq!(remaining_vec_ids, vec![id_alpha, id_gamma]);
    assert!(storage
        .search_section_fts("UPDATED", &[doc_id], 10)
        .unwrap()
        .iter()
        .any(|(sid, _, _)| *sid == secs_v2[1].id));

    // Delete document and verify all section, FTS, vector, meta, and state rows are purged.
    storage
        .delete_document(std::path::Path::new("20_Cards/settlement.md"))
        .unwrap();
    assert!(storage.get_sections(doc_id).unwrap().is_empty());
    let sec_vec_cnt: i64 = storage
        .connection()
        .query_row("SELECT count(*) FROM section_vectors", [], |r| r.get(0))
        .unwrap();
    let sec_meta_cnt: i64 = storage
        .connection()
        .query_row("SELECT count(*) FROM section_vector_meta", [], |r| r.get(0))
        .unwrap();
    let state_cnt: i64 = storage
        .connection()
        .query_row("SELECT count(*) FROM document_section_state", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(sec_vec_cnt, 0);
    assert_eq!(sec_meta_cnt, 0);
    assert_eq!(state_cnt, 0);
    storage
        .connection()
        .execute_batch("INSERT INTO fts_sections(fts_sections, rank) VALUES('integrity-check', 0);")
        .expect("fts_sections integrity-check passes after delete");
}

#[test]
fn test_v3_graph_edges_access_debounce_git_times_and_budgeted_repair() {
    let mut storage = SqliteStorage::in_memory().unwrap();
    storage.ensure_vector_dimension(4).unwrap();
    storage.meta_set("embed_model", "mock-4").unwrap();
    storage.meta_set("section_vectors_complete", "1").unwrap();
    assert!(storage.section_vectors_ready("mock-4"));
    assert!(!storage.section_vectors_ready("other-model"));

    // Dimension change drops section_vectors/meta, resets section_vectors_complete='0', deletes embed_model.
    assert!(storage.ensure_vector_dimension(8).unwrap());
    assert_eq!(
        storage.meta_get("section_vectors_complete").unwrap(),
        Some("0".to_string())
    );
    assert_eq!(storage.meta_get("embed_model").unwrap(), None);
    assert!(!storage.section_vectors_ready("mock-4"));

    let doc_a = k0maru::Document {
        path: PathBuf::from("10_Projects/gateway.md"),
        title: "Gateway".to_string(),
        hierarchy: k0maru::HierarchyLevel::L1Resource,
        frontmatter: serde_json::json!({}),
        links: vec![
            k0maru::WikiLink::new("settlement", None::<String>, "[[settlement]]"),
            k0maru::WikiLink::new("diagram.png", None::<String>, "[[diagram.png]]"),
            k0maru::WikiLink::new("missing-note", None::<String>, "[[missing-note]]"),
        ],
        tags: vec![],
        content_hash: "h_a".to_string(),
        mtime: 1700000000,
        body: "## Gateway\nLinks to settlement.".to_string(),
        body_line_offset: 0,
    };
    let doc_b = k0maru::Document {
        path: PathBuf::from("20_Cards/settlement.md"),
        title: "Settlement".to_string(),
        hierarchy: k0maru::HierarchyLevel::L3Evergreen,
        frontmatter: serde_json::json!({}),
        links: vec![],
        tags: vec![],
        content_hash: "h_b".to_string(),
        mtime: 1700000000,
        body: "## Settlement\nCore ledger.".to_string(),
        body_line_offset: 0,
    };

    storage.upsert_document(&doc_a).unwrap();
    storage.upsert_document(&doc_b).unwrap();
    assert!(!storage.link_edges_clean());

    let edge_stats = storage.rebuild_link_edges().unwrap();
    assert_eq!(edge_stats.edges, 1);
    assert_eq!(edge_stats.assets, 1);
    assert_eq!(edge_stats.broken, 1);
    assert!(storage.link_edges_clean());
    assert!(!storage.rebuild_link_edges_if_dirty().unwrap());

    let id_a = storage
        .doc_id_for_path(std::path::Path::new("10_Projects/gateway.md"))
        .unwrap()
        .unwrap();
    let id_b = storage
        .doc_id_for_path(std::path::Path::new("20_Cards/settlement.md"))
        .unwrap()
        .unwrap();
    assert_eq!(storage.edges_touching(&[id_a]).unwrap(), vec![(id_a, id_b)]);

    // Access debounce: 60s window.
    assert!(storage
        .record_access("20_Cards/settlement.md", 1000)
        .unwrap());
    assert!(!storage
        .record_access("20_Cards/settlement.md", 1059)
        .unwrap());
    assert!(storage
        .record_access("20_Cards/settlement.md", 1060)
        .unwrap());
    let stats = storage.access_stats(&["20_Cards/settlement.md"]).unwrap();
    assert_eq!(stats.get("20_Cards/settlement.md"), Some(&(2, 1060)));

    // Git times.
    storage
        .set_git_times(
            &[("20_Cards/settlement.md".to_string(), 1700000555)],
            "deadbeef",
        )
        .unwrap();
    assert_eq!(
        storage
            .git_times(&["20_Cards/settlement.md"])
            .unwrap()
            .get("20_Cards/settlement.md"),
        Some(&1700000555)
    );
    assert_eq!(
        storage.meta_get("git_time_head").unwrap(),
        Some("deadbeef".to_string())
    );

    // Budgeted section repair: clear state for both docs and repair 1 at a time.
    storage
        .connection()
        .execute("DELETE FROM document_section_state", [])
        .unwrap();
    assert_eq!(storage.repair_stale_sections(Some(1)).unwrap(), 1);
    assert_eq!(storage.repair_stale_sections(None).unwrap(), 1);
    assert_eq!(storage.repair_stale_sections(None).unwrap(), 0);
}
