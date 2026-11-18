use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use k0maru_core::core::models::{Document, HierarchyLevel};
use k0maru_core::core::traits::CacheStorage;
use k0maru_core::scanner::VectorSyncEngine;
use k0maru_core::storage::federated::{
    resolve_global_vault, FederatedSearch, ReadOnlyStorage, GLOBAL_PREFIX,
};
use k0maru_core::storage::{
    ensure_sqlite_vec_registered, HybridSearchEngine, SearchMode, SqliteStorage,
};
use k0maru_core::text::fts_hash_for;
use k0maru_core::vector::{EmbeddingEngine, VectorError};
use rusqlite::{params, Connection};
use serde_json::json;
use tempfile::TempDir;

static ENV_MUTEX: Mutex<()> = Mutex::new(());

struct NamedTopicEmbedder {
    name: String,
    dim: usize,
}

impl NamedTopicEmbedder {
    fn new(name: &str, dim: usize) -> Self {
        Self {
            name: name.to_string(),
            dim,
        }
    }
}

impl EmbeddingEngine for NamedTopicEmbedder {
    fn embed(&self, text: &str) -> Result<Vec<f32>, VectorError> {
        let mut vec = vec![0.0f32; self.dim];
        let lower = text.to_lowercase();
        if lower.contains("settlement") || lower.contains("quorum") || lower.contains("consensus") {
            vec[0] = 1.0;
        } else if lower.contains("gateway") || lower.contains("idempotency") {
            vec[1] = 1.0;
        } else {
            vec[2] = 1.0;
        }
        Ok(vec)
    }

    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, VectorError> {
        texts.iter().map(|t| self.embed(t)).collect()
    }

    fn dimension(&self) -> usize {
        self.dim
    }

    fn model_name(&self) -> &str {
        &self.name
    }
}

fn make_doc(
    path: &str,
    title: &str,
    hierarchy: HierarchyLevel,
    content_hash: &str,
    body: &str,
) -> Document {
    Document {
        path: PathBuf::from(path),
        title: title.to_string(),
        hierarchy,
        frontmatter: json!({}),
        links: vec![],
        tags: vec![],
        content_hash: content_hash.to_string(),
        mtime: 1_794_960_000,
        body: body.to_string(),
        body_line_offset: 0,
    }
}

fn f32_slice_to_bytes(floats: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(floats.len() * 4);
    for &f in floats {
        bytes.extend_from_slice(&f.to_le_bytes());
    }
    bytes
}

fn create_v2_global_db(db_path: &Path, embedder: &dyn EmbeddingEngine) {
    ensure_sqlite_vec_registered();
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
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
        INSERT INTO k0maru_meta (key, value) VALUES ('embed_model', 'model-shared-384');
        PRAGMA user_version = 2;
        "#,
    )
    .expect("v2 ddl");

    let docs = [
        (
            1i64,
            "20_Cards/global-v2-settlement.md",
            "Global V2 Settlement Runbook",
            "hash_v2_g1",
            "## Quorum\nDistributed settlement consensus quorum validation on legacy v2.",
        ),
        (
            2i64,
            "20_Cards/global-v2-ledger.md",
            "Global V2 Ledger Audit",
            "hash_v2_g2",
            "## Ledger\nPayment gateway reconciliation ledger checkpoint.",
        ),
    ];

    for (id, path, title, hash, body) in docs {
        let fts_hash = fts_hash_for(hash);
        conn.execute(
            "INSERT INTO documents (id, path, title, hierarchy, frontmatter, content_hash, mtime, body, body_line_offset, fts_hash) VALUES (?1, ?2, ?3, 'L3Evergreen', '{}', ?4, 1700000000, ?5, 0, ?6)",
            params![id, path, title, hash, body, fts_hash],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO fts_documents (rowid, title, body, tags) VALUES (?1, ?2, ?3, '')",
            params![id, title, body],
        )
        .unwrap();
        let emb = embedder.embed(&format!("{title} {body}")).unwrap();
        conn.execute(
            "INSERT INTO document_vectors (id, embedding) VALUES (?1, ?2)",
            params![path, f32_slice_to_bytes(&emb)],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO vector_metadata (document_id, content_hash, updated_at) VALUES (?1, ?2, 1700000001)",
            params![path, id],
        )
        .unwrap();
    }
}

#[test]
fn test_federated_none_global_exact_score_parity() {
    let mut primary = SqliteStorage::in_memory().unwrap();
    let embedder: Arc<dyn EmbeddingEngine> =
        Arc::new(NamedTopicEmbedder::new("model-shared-384", 384));

    primary
        .upsert_document(&make_doc(
            "20_Cards/settlement-alpha.md",
            "Settlement Alpha",
            HierarchyLevel::L3Evergreen,
            "hash_p1",
            "## Consensus\nDistributed settlement quorum finality protocol.\n\n## Recovery\nReplay window.",
        ))
        .unwrap();
    primary
        .upsert_document(&make_doc(
            "20_Cards/gateway-beta.md",
            "Gateway Beta",
            HierarchyLevel::L3Evergreen,
            "hash_p2",
            "## Idempotency\nPayment gateway retry key and settlement fallback.",
        ))
        .unwrap();
    primary.rebuild_link_edges().unwrap();

    let vec_sync = VectorSyncEngine::new();
    vec_sync.sync_storage(&mut primary, &*embedder).unwrap();
    vec_sync
        .sync_sections(&mut primary, &*embedder, None)
        .unwrap();

    for mode in [SearchMode::Hybrid, SearchMode::Bm25, SearchMode::Vector] {
        let direct = HybridSearchEngine::new(&primary, Some(embedder.clone()))
            .with_rrf_k(45)
            .search("settlement quorum", mode, 5)
            .unwrap();
        let federated = FederatedSearch::new(&primary, None, Some(embedder.clone()))
            .with_rrf_k(45)
            .search("settlement quorum", mode, 5)
            .unwrap();

        assert_eq!(
            federated, direct,
            "global=None must return score-for-score identical results for mode {:?}",
            mode
        );
        for res in &federated {
            assert!(res.scope.is_none());
            assert!(!res.locator().starts_with(GLOBAL_PREFIX));
        }
    }
}

#[test]
fn test_federated_fusion_order_and_scope_tie_break() {
    let tmp = TempDir::new().unwrap();
    let global_cache = tmp.path().join("global/.k0maru/cache.sqlite");
    fs::create_dir_all(global_cache.parent().unwrap()).unwrap();

    let mut primary = SqliteStorage::in_memory().unwrap();
    primary
        .upsert_document(&make_doc(
            "20_Cards/primary-one.md",
            "Primary One",
            HierarchyLevel::L3Evergreen,
            "p_hash_1",
            "## Settlement\nDistributed settlement settlement settlement core engine.",
        ))
        .unwrap();
    primary
        .upsert_document(&make_doc(
            "20_Cards/primary-two.md",
            "Primary Two",
            HierarchyLevel::L3Evergreen,
            "p_hash_2",
            "## Settlement\nDistributed settlement overview.",
        ))
        .unwrap();

    {
        let mut global_rw = SqliteStorage::open_rw(&global_cache).unwrap();
        global_rw
            .upsert_document(&make_doc(
                "20_Cards/global-one.md",
                "Global One",
                HierarchyLevel::L3Evergreen,
                "g_hash_1",
                "## Settlement\nDistributed settlement settlement settlement reference.",
            ))
            .unwrap();
        global_rw
            .upsert_document(&make_doc(
                "20_Cards/global-two.md",
                "Global Two",
                HierarchyLevel::L3Evergreen,
                "g_hash_2",
                "## Settlement\nDistributed settlement notes.",
            ))
            .unwrap();
    }

    let global_ro = ReadOnlyStorage::open(&global_cache).unwrap();

    // Default global_weight = 0.7, rrf_k = 60:
    // p1 = 1/61 (~0.01639), p2 = 1/62 (~0.01613), g1 = 0.7/61 (~0.01148), g2 = 0.7/62 (~0.01129)
    let results = FederatedSearch::new(&primary, Some(&global_ro), None)
        .search("settlement", SearchMode::Bm25, 10)
        .unwrap();

    assert_eq!(results.len(), 4);
    assert_eq!(results[0].path, "20_Cards/primary-one.md");
    assert_eq!(results[0].scope, None);
    assert!((results[0].score - (1.0 / 61.0)).abs() < 1e-6);

    assert_eq!(results[1].path, "20_Cards/primary-two.md");
    assert_eq!(results[1].scope, None);
    assert!((results[1].score - (1.0 / 62.0)).abs() < 1e-6);

    assert_eq!(results[2].path, "20_Cards/global-one.md");
    assert_eq!(results[2].scope.as_deref(), Some("global"));
    assert!((results[2].score - (0.7 / 61.0)).abs() < 1e-6);
    assert!(results[2]
        .locator()
        .starts_with("@global:20_Cards/global-one.md"));

    assert_eq!(results[3].path, "20_Cards/global-two.md");
    assert_eq!(results[3].scope.as_deref(), Some("global"));
    assert!((results[3].score - (0.7 / 62.0)).abs() < 1e-6);

    // Equal weight = 1.0: p1 (1/61) ties g1 (1/61), primary must come first; g1 beats p2 (1/62).
    let equal_weight_results = FederatedSearch::new(&primary, Some(&global_ro), None)
        .with_global_weight(1.0)
        .search("settlement", SearchMode::Bm25, 10)
        .unwrap();

    let order: Vec<(&str, Option<&str>)> = equal_weight_results
        .iter()
        .map(|r| (r.path.as_str(), r.scope.as_deref()))
        .collect();
    assert_eq!(
        order,
        vec![
            ("20_Cards/primary-one.md", None),
            ("20_Cards/global-one.md", Some("global")),
            ("20_Cards/primary-two.md", None),
            ("20_Cards/global-two.md", Some("global")),
        ]
    );
}

#[test]
fn test_federated_content_hash_dedup_and_distinct_same_path() {
    let tmp = TempDir::new().unwrap();
    let global_cache = tmp.path().join("global/.k0maru/cache.sqlite");
    fs::create_dir_all(global_cache.parent().unwrap()).unwrap();

    let mut primary = SqliteStorage::in_memory().unwrap();
    // Exact duplicate content_hash under different relative path
    primary
        .upsert_document(&make_doc(
            "20_Cards/settlement-local.md",
            "Settlement Local",
            HierarchyLevel::L3Evergreen,
            "shared_content_hash",
            "## Consensus\nDistributed settlement consensus finality.",
        ))
        .unwrap();
    // Same relative path as global, but different content_hash
    primary
        .upsert_document(&make_doc(
            "20_Cards/gateway-policy.md",
            "Gateway Policy Primary",
            HierarchyLevel::L3Evergreen,
            "primary_policy_hash",
            "## Policy\nDistributed settlement primary gateway policy.",
        ))
        .unwrap();

    {
        let mut global_rw = SqliteStorage::open_rw(&global_cache).unwrap();
        global_rw
            .upsert_document(&make_doc(
                "30_Resources/settlement-mirror.md",
                "Settlement Mirror",
                HierarchyLevel::L1Resource,
                "shared_content_hash",
                "## Consensus\nDistributed settlement consensus finality.",
            ))
            .unwrap();
        global_rw
            .upsert_document(&make_doc(
                "20_Cards/gateway-policy.md",
                "Gateway Policy Global",
                HierarchyLevel::L3Evergreen,
                "global_policy_hash",
                "## Policy\nDistributed settlement global baseline gateway policy.",
            ))
            .unwrap();
    }

    let global_ro = ReadOnlyStorage::open(&global_cache).unwrap();
    let results = FederatedSearch::new(&primary, Some(&global_ro), None)
        .search("settlement", SearchMode::Bm25, 10)
        .unwrap();

    // Global mirror with matching content_hash must be dropped (primary wins).
    assert!(
        !results
            .iter()
            .any(|r| r.path == "30_Resources/settlement-mirror.md"),
        "duplicate content_hash hit in global must be deduplicated"
    );

    // Same path with different content_hash must remain distinct via scope / @global: locator.
    let policy_hits: Vec<_> = results
        .iter()
        .filter(|r| r.path == "20_Cards/gateway-policy.md")
        .collect();
    assert_eq!(policy_hits.len(), 2);
    assert_eq!(policy_hits[0].scope, None);
    assert!(policy_hits[0]
        .locator()
        .starts_with("20_Cards/gateway-policy.md"));
    assert_eq!(policy_hits[1].scope.as_deref(), Some("global"));
    assert!(policy_hits[1]
        .locator()
        .starts_with("@global:20_Cards/gateway-policy.md"));
}

#[test]
fn test_federated_per_scope_embed_model_gating() {
    let tmp = TempDir::new().unwrap();
    let global_cache = tmp.path().join("global/.k0maru/cache.sqlite");
    fs::create_dir_all(global_cache.parent().unwrap()).unwrap();

    let primary_embedder: Arc<dyn EmbeddingEngine> =
        Arc::new(NamedTopicEmbedder::new("model-primary-384", 384));
    let global_embedder: Arc<dyn EmbeddingEngine> =
        Arc::new(NamedTopicEmbedder::new("model-global-other-384", 384));

    let mut primary = SqliteStorage::in_memory().unwrap();
    primary
        .upsert_document(&make_doc(
            "20_Cards/primary-quorum.md",
            "Primary Quorum",
            HierarchyLevel::L3Evergreen,
            "hash_pq",
            "## Consensus\nQuorum state replication for distributed settlement.",
        ))
        .unwrap();
    let vec_sync = VectorSyncEngine::new();
    vec_sync
        .sync_storage(&mut primary, &*primary_embedder)
        .unwrap();

    {
        let mut global_rw = SqliteStorage::open_rw(&global_cache).unwrap();
        // Doc 1 matches lexically ("ledger")
        global_rw
            .upsert_document(&make_doc(
                "20_Cards/global-lexical.md",
                "Global Lexical Ledger",
                HierarchyLevel::L3Evergreen,
                "hash_gl",
                "## Ledger\nLedger replay verification checkpoint.",
            ))
            .unwrap();
        // Doc 2 matches only semantically via vector[0] ("settlement") when using vector search
        global_rw
            .upsert_document(&make_doc(
                "20_Cards/global-semantic.md",
                "Global Semantic Quorum",
                HierarchyLevel::L3Evergreen,
                "hash_gs",
                "## Consensus\nQuorum finality.",
            ))
            .unwrap();
        vec_sync
            .sync_storage(&mut global_rw, &*global_embedder)
            .unwrap();
    }

    let global_ro = ReadOnlyStorage::open(&global_cache).unwrap();

    // In Hybrid mode with primary_embedder:
    // - Primary scope runs vector + BM25 (vector_rank is Some)
    // - Global scope has mismatched embed_model -> vector disabled, BM25-only (vector_rank is None)
    let hybrid_hits =
        FederatedSearch::new(&primary, Some(&global_ro), Some(primary_embedder.clone()))
            .search("ledger quorum", SearchMode::Hybrid, 10)
            .unwrap();

    let p_hit = hybrid_hits
        .iter()
        .find(|r| r.path == "20_Cards/primary-quorum.md")
        .expect("primary hit expected");
    assert!(
        p_hit.vector_rank.is_some(),
        "matching embed_model on primary must keep vector channel active"
    );

    let g_hit = hybrid_hits
        .iter()
        .find(|r| r.path == "20_Cards/global-lexical.md")
        .expect("global lexical hit expected via BM25 fallback");
    assert_eq!(g_hit.scope.as_deref(), Some("global"));
    assert!(
        g_hit.vector_rank.is_none(),
        "mismatched embed_model on global must disable vector channel and fall back to BM25"
    );

    // In Vector mode, mismatched global scope must return 0 vector hits without failing primary vector hits.
    let vec_hits = FederatedSearch::new(&primary, Some(&global_ro), Some(primary_embedder))
        .search("quorum", SearchMode::Vector, 10)
        .unwrap();
    assert_eq!(vec_hits.len(), 1);
    assert_eq!(vec_hits[0].path, "20_Cards/primary-quorum.md");
    assert_eq!(vec_hits[0].scope, None);
}

#[test]
fn test_federated_v2_global_db_without_v3_tables_works() {
    let tmp = TempDir::new().unwrap();
    let global_cache = tmp.path().join("global-v2/.k0maru/cache.sqlite");
    let embedder: Arc<dyn EmbeddingEngine> =
        Arc::new(NamedTopicEmbedder::new("model-shared-384", 384));
    create_v2_global_db(&global_cache, &*embedder);

    let mut primary = SqliteStorage::in_memory().unwrap();
    primary
        .upsert_document(&make_doc(
            "20_Cards/primary-settlement.md",
            "Primary Settlement",
            HierarchyLevel::L3Evergreen,
            "hash_p_v3",
            "## Overview\nDistributed settlement consensus design.",
        ))
        .unwrap();
    VectorSyncEngine::new()
        .sync_storage(&mut primary, &*embedder)
        .unwrap();

    let global_ro = ReadOnlyStorage::open(&global_cache).unwrap();

    for mode in [SearchMode::Bm25, SearchMode::Vector, SearchMode::Hybrid] {
        let hits = FederatedSearch::new(&primary, Some(&global_ro), Some(embedder.clone()))
            .search("settlement consensus", mode, 10)
            .unwrap();
        assert!(
            hits.iter().any(|r| r.scope.as_deref() == Some("global")
                && r.path == "20_Cards/global-v2-settlement.md"),
            "v2 global DB without v3 tables must serve hits in {:?} mode",
            mode
        );
    }
}

#[test]
fn test_resolve_global_vault_precedence_and_guards() {
    let _guard = ENV_MUTEX.lock().unwrap();
    let tmp = TempDir::new().unwrap();
    let primary = tmp.path().join("primary");
    let global = tmp.path().join("global");
    let env_global = tmp.path().join("env_global");
    let uninitialized = tmp.path().join("uninitialized");
    let nested_global = primary.join("shared_global");

    fs::create_dir_all(&primary).unwrap();
    fs::create_dir_all(global.join(".k0maru")).unwrap();
    fs::create_dir_all(env_global.join(".k0maru")).unwrap();
    fs::create_dir_all(&uninitialized).unwrap();
    fs::create_dir_all(nested_global.join(".k0maru")).unwrap();

    fs::write(global.join(".k0maru/cache.sqlite"), b"stub").unwrap();
    fs::write(env_global.join(".k0maru/cache.sqlite"), b"stub").unwrap();
    fs::write(nested_global.join(".k0maru/cache.sqlite"), b"stub").unwrap();

    let prev_global = std::env::var_os("K0MARU_GLOBAL_VAULT");
    let prev_no = std::env::var_os("K0MARU_NO_GLOBAL");

    std::env::remove_var("K0MARU_NO_GLOBAL");
    std::env::remove_var("K0MARU_GLOBAL_VAULT");

    // 1. Missing global cache -> None, and must never create .k0maru/cache.sqlite
    assert_eq!(resolve_global_vault(Some(&uninitialized), &primary), None);
    assert!(!uninitialized.join(".k0maru").exists());

    // 2. global == primary -> None
    assert_eq!(resolve_global_vault(Some(&primary), &primary), None);

    // 3. Nested global inside primary -> allowed (warns, does not drop)
    assert_eq!(
        resolve_global_vault(Some(&nested_global), &primary),
        Some(fs::canonicalize(&nested_global).unwrap())
    );

    // 4. Empty K0MARU_GLOBAL_VAULT -> None
    std::env::set_var("K0MARU_GLOBAL_VAULT", "   ");
    assert_eq!(resolve_global_vault(None, &primary), None);

    // 5. Env fallback works, and explicit flag overrides env
    std::env::set_var("K0MARU_GLOBAL_VAULT", &env_global);
    assert_eq!(
        resolve_global_vault(None, &primary),
        Some(fs::canonicalize(&env_global).unwrap())
    );
    assert_eq!(
        resolve_global_vault(Some(&global), &primary),
        Some(fs::canonicalize(&global).unwrap())
    );

    // 6. K0MARU_NO_GLOBAL=1 kill switch overrides both flag and env
    std::env::set_var("K0MARU_NO_GLOBAL", "1");
    assert_eq!(resolve_global_vault(Some(&global), &primary), None);
    assert_eq!(resolve_global_vault(None, &primary), None);

    match prev_global {
        Some(v) => std::env::set_var("K0MARU_GLOBAL_VAULT", v),
        None => std::env::remove_var("K0MARU_GLOBAL_VAULT"),
    }
    match prev_no {
        Some(v) => std::env::set_var("K0MARU_NO_GLOBAL", v),
        None => std::env::remove_var("K0MARU_NO_GLOBAL"),
    }
}

#[test]
fn test_federated_read_only_invariants_file_mtime_size_wal_unchanged() {
    let tmp = TempDir::new().unwrap();
    let global_cache = tmp.path().join("global/.k0maru/cache.sqlite");
    let wal_path = tmp.path().join("global/.k0maru/cache.sqlite-wal");
    fs::create_dir_all(global_cache.parent().unwrap()).unwrap();

    let embedder: Arc<dyn EmbeddingEngine> =
        Arc::new(NamedTopicEmbedder::new("model-shared-384", 384));

    let mut primary = SqliteStorage::in_memory().unwrap();
    primary
        .upsert_document(&make_doc(
            "20_Cards/primary-doc.md",
            "Primary Doc",
            HierarchyLevel::L3Evergreen,
            "hash_p_ro",
            "## Settlement\nDistributed settlement primary engine.",
        ))
        .unwrap();
    VectorSyncEngine::new()
        .sync_storage(&mut primary, &*embedder)
        .unwrap();

    {
        let mut global_rw = SqliteStorage::open_rw(&global_cache).unwrap();
        global_rw
            .upsert_document(&make_doc(
                "20_Cards/global-doc.md",
                "Global Doc",
                HierarchyLevel::L3Evergreen,
                "hash_g_ro",
                "## Settlement\nDistributed settlement global invariant.",
            ))
            .unwrap();
        let vec_sync = VectorSyncEngine::new();
        vec_sync.sync_storage(&mut global_rw, &*embedder).unwrap();
        vec_sync
            .sync_sections(&mut global_rw, &*embedder, None)
            .unwrap();
        global_rw.rebuild_link_edges().unwrap();
    }

    let meta_before = fs::metadata(&global_cache).unwrap();
    let size_before = meta_before.len();
    let mtime_before = meta_before.modified().unwrap();
    let wal_size_before = fs::metadata(&wal_path).map(|m| m.len()).unwrap_or(0);

    let global_ro = ReadOnlyStorage::open(&global_cache).unwrap();

    // Direct write attempt against the read-only handle must fail at SQLite level.
    let write_err = global_ro
        .storage()
        .connection()
        .execute("DELETE FROM documents", []);
    assert!(
        write_err.is_err(),
        "read-only connection must reject writes"
    );
    assert!(!global_ro
        .storage()
        .record_access("20_Cards/global-doc.md", 1_800_000_000)
        .unwrap());

    for mode in [SearchMode::Hybrid, SearchMode::Bm25, SearchMode::Vector] {
        let hits = FederatedSearch::new(&primary, Some(&global_ro), Some(embedder.clone()))
            .search("settlement", mode, 10)
            .unwrap();
        assert!(hits.iter().any(|r| r.scope.as_deref() == Some("global")));
    }

    drop(global_ro);

    let meta_after = fs::metadata(&global_cache).unwrap();
    assert_eq!(meta_after.len(), size_before, "global cache size changed");
    assert_eq!(
        meta_after.modified().unwrap(),
        mtime_before,
        "global cache mtime changed"
    );
    let wal_size_after = fs::metadata(&wal_path).map(|m| m.len()).unwrap_or(0);
    assert_eq!(
        wal_size_after, wal_size_before,
        "global cache -wal size changed"
    );
}

#[test]
fn test_federated_global_all_tokens_outranks_primary_any_tokens_or_fallback() {
    use k0maru_core::text::FtsQueryTier;

    let tmp = TempDir::new().unwrap();
    let global_cache = tmp.path().join("global/.k0maru/cache.sqlite");
    fs::create_dir_all(global_cache.parent().unwrap()).unwrap();

    // Primary vault (`checkout-api-service` with 4 small files):
    // ADR-001 only contains "JWT" and "HS256" (missing "oauth2" and "PyJWT").
    // checkout-api-service.md only contains "JWT".
    let mut primary = SqliteStorage::in_memory().unwrap();
    primary
        .upsert_document(&make_doc(
            "10_Projects/checkout-api-service/ADR-001-token-signing.md",
            "ADR 001 Token Signing",
            HierarchyLevel::L3Evergreen,
            "hash_p_adr001",
            "## Decision\nInternal service tokens use JWT signed with HS256.",
        ))
        .unwrap();
    primary
        .upsert_document(&make_doc(
            "10_Projects/checkout-api-service/checkout-api-service.md",
            "Checkout API Service",
            HierarchyLevel::L1Resource,
            "hash_p_svc",
            "## Auth\nValidates caller JWT bearer header at ingress.",
        ))
        .unwrap();
    primary
        .upsert_document(&make_doc(
            "10_Projects/checkout-api-service/retry-policy.md",
            "Retry Policy",
            HierarchyLevel::L1Resource,
            "hash_p_retry",
            "## Retries\nExponential backoff with jitter for idempotent endpoints.",
        ))
        .unwrap();
    primary
        .upsert_document(&make_doc(
            "10_Projects/checkout-api-service/database-pool.md",
            "Database Pool",
            HierarchyLevel::L1Resource,
            "hash_p_db",
            "## Pool\nConnection pool sizing and timeout thresholds.",
        ))
        .unwrap();

    // Global vault: contains exact 4-term document matching all query tokens.
    {
        let mut global_rw = SqliteStorage::open_rw(&global_cache).unwrap();
        global_rw
            .upsert_document(&make_doc(
                "fastapi/tutorial/security/oauth2-jwt.md",
                "OAuth2 with Password (and hashing), Bearer with JWT tokens",
                HierarchyLevel::L1Resource,
                "hash_g_oauth2_jwt",
                "## OAuth2 JWT\nUse OAuth2 password flow with jwt tokens via PyJWT and HS256 signing.",
            ))
            .unwrap();
        global_rw
            .upsert_document(&make_doc(
                "fastapi/tutorial/security/simple-oauth2.md",
                "Simple OAuth2",
                HierarchyLevel::L1Resource,
                "hash_g_simple",
                "## OAuth2\nBasic oauth2 password bearer dependency.",
            ))
            .unwrap();
    }

    let global_ro = ReadOnlyStorage::open(&global_cache).unwrap();
    let query = "oauth2 jwt PyJWT HS256";

    let (p_docs, p_tier) = primary.search_fts_with_tier(query, 10).unwrap();
    assert!(!p_docs.is_empty());
    assert_eq!(p_tier, Some(FtsQueryTier::AnyTokens));

    let (g_docs, g_tier) = global_ro.storage().search_fts_with_tier(query, 10).unwrap();
    assert_eq!(
        g_docs[0].path.to_str(),
        Some("fastapi/tutorial/security/oauth2-jwt.md")
    );
    assert!(matches!(
        g_tier,
        Some(FtsQueryTier::Strict) | Some(FtsQueryTier::AllTokens)
    ));

    for mode in [SearchMode::Bm25, SearchMode::Hybrid] {
        let results = FederatedSearch::new(&primary, Some(&global_ro), None)
            .search(query, mode, 10)
            .unwrap();

        assert!(
            results.len() >= 3,
            "expected >=3 hits, got {}",
            results.len()
        );
        assert_eq!(
            results[0].path, "fastapi/tutorial/security/oauth2-jwt.md",
            "mode {:?}: exact 4-term @global match must rank #1 above primary AnyTokens OR-fallback",
            mode
        );
        assert_eq!(results[0].scope.as_deref(), Some("global"));
        assert!((results[0].score - (0.7 / 61.0)).abs() < 1e-6);

        let adr_hit = results
            .iter()
            .find(|r| r.path == "10_Projects/checkout-api-service/ADR-001-token-signing.md")
            .expect("primary ADR-001 hit expected");
        assert_eq!(adr_hit.scope, None);
        assert!((adr_hit.score - (0.55 / 61.0)).abs() < 1e-6);
        assert!(results[0].score > adr_hit.score);
    }
}

#[test]
fn test_federated_global_active_outranks_primary_superseded_strict_hit() {
    let tmp = TempDir::new().unwrap();
    let global_cache = tmp.path().join("global/.k0maru/cache.sqlite");
    fs::create_dir_all(global_cache.parent().unwrap()).unwrap();

    let mut primary = SqliteStorage::in_memory().unwrap();
    let mut superseded_doc = make_doc(
        "10_Projects/ADR-000-legacy-oauth2.md",
        "Legacy OAuth2",
        HierarchyLevel::L3Evergreen,
        "hash_p_sup",
        "## Deprecated\nLegacy oauth2 jwt PyJWT HS256 configuration.",
    );
    superseded_doc.frontmatter = json!({
        "status": "superseded",
        "superseded_by": "ADR-002-new-oauth2.md"
    });
    primary.upsert_document(&superseded_doc).unwrap();

    {
        let mut global_rw = SqliteStorage::open_rw(&global_cache).unwrap();
        global_rw
            .upsert_document(&make_doc(
                "fastapi/tutorial/security/oauth2-jwt.md",
                "OAuth2 JWT",
                HierarchyLevel::L1Resource,
                "hash_g_active",
                "## OAuth2\nModern oauth2 jwt PyJWT HS256 guide.",
            ))
            .unwrap();
    }

    let global_ro = ReadOnlyStorage::open(&global_cache).unwrap();
    let results = FederatedSearch::new(&primary, Some(&global_ro), None)
        .search("oauth2 jwt PyJWT HS256", SearchMode::Hybrid, 10)
        .unwrap();

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].path, "fastapi/tutorial/security/oauth2-jwt.md");
    assert_eq!(results[0].scope.as_deref(), Some("global"));
    assert!((results[0].score - (0.7 / 61.0)).abs() < 1e-6);

    assert_eq!(results[1].path, "10_Projects/ADR-000-legacy-oauth2.md");
    assert_eq!(results[1].scope, None);
    assert!((results[1].score - (0.2 / 61.0)).abs() < 1e-6);
}
