use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use k0maru::{
    parse_locator, EmbeddingEngine, HybridSearchEngine, IncrementalScanner, MockEmbeddingEngine,
    ObsidianAdapter, SearchMode, SearchResult, SectionLocator, SqliteStorage, VectorError,
    VectorSyncEngine,
};
use tempfile::tempdir;

/// Deterministic test embedder with configurable model name and keyword-aligned vectors.
struct NamedMockEmbedder {
    model: String,
    dim: usize,
}

impl NamedMockEmbedder {
    fn new(model: &str, dim: usize) -> Self {
        Self {
            model: model.to_string(),
            dim,
        }
    }
}

impl EmbeddingEngine for NamedMockEmbedder {
    fn embed(&self, text: &str) -> Result<Vec<f32>, VectorError> {
        let mut vec = vec![0.0f32; self.dim];
        let lower = text.to_lowercase();
        if lower.contains("settlement") || lower.contains("quorum") {
            vec[0] = 1.0;
        }
        if lower.contains("sharding") || lower.contains("partition") {
            vec[1] = 1.0;
        }
        if lower.contains("idempotency") || lower.contains("replay") {
            vec[2] = 1.0;
        }
        if vec.iter().all(|&v| v == 0.0) {
            // Fall back to deterministic hash direction
            let fallback = MockEmbeddingEngine::new(self.dim);
            return fallback.embed(text);
        }
        let norm: f32 = vec.iter().map(|v| v * v).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in &mut vec {
                *v /= norm;
            }
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
        &self.model
    }
}

fn write_long_multi_section_doc(vault: &Path, rel: &str) {
    let full = vault.join(rel);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let content = "\
---
title: Payment Gateway Architecture
hierarchy: L3Evergreen
tags:
  - architecture
---
# Payment Gateway Architecture

Overview preamble covering gateway lifecycle and general routing.

## 1. Ingress Validation
Request signature verification and rate limiting policies.

## 2. Currency Formatting
Decimal rounding and currency ISO code normalization.

## 3. Retry Policies
Exponential backoff and jitter bounds for downstream calls.

## 4. Telemetry
OpenTelemetry span export and Prometheus histograms.

## 5. Circuit Breakers
Half-open probe thresholds for degraded upstream banks.

## 6. Webhook Dispatch
Signed asynchronous webhook delivery and dead-letter queues.

## 7. Idempotency & Replay Protection
Every mutation carries an idempotency key stored in the ledger-service cache to block duplicate replay transfers during network partitions.

## 8. Archival
Cold storage compaction schedules.
";
    fs::write(full, content).unwrap();
}

#[test]
fn test_section_locator_rendering_sanitization_and_json_compat() {
    let plain = SearchResult {
        path: "20_Cards/Gateway.md".to_string(),
        title: "Gateway".to_string(),
        score: 0.5,
        bm25_rank: Some(1),
        vector_rank: None,
        graph_boost: 0.0,
        snippet: "snippet".to_string(),
        section: None,
        expanded_from: None,
        scope: None,
    };
    assert_eq!(plain.locator(), "20_Cards/Gateway.md");
    let json = serde_json::to_value(&plain).unwrap();
    let obj = json.as_object().unwrap();
    assert!(!obj.contains_key("section"));
    assert!(!obj.contains_key("expanded_from"));
    assert!(!obj.contains_key("scope"));

    // Level-0 section omits #heading
    let lvl0 = SearchResult {
        section: Some(SectionLocator {
            section_id: 1,
            start_line: 1,
            end_line: 4,
            heading: String::new(),
            breadcrumb: "Gateway".to_string(),
        }),
        ..plain.clone()
    };
    assert_eq!(lvl0.locator(), "20_Cards/Gateway.md:1-4");

    // Heading sanitization: '|' -> '/', drop '§' and newlines, truncate to 48 chars, @global: prefix
    let long_heading =
        "Part A | Part B § Hidden\nLine2 with extra suffix exceeding forty eight characters";
    let sanitized = SearchResult {
        scope: Some("global".to_string()),
        section: Some(SectionLocator {
            section_id: 2,
            start_line: 10,
            end_line: 25,
            heading: long_heading.to_string(),
            breadcrumb: "Gateway > Part A".to_string(),
        }),
        ..plain
    };
    let loc = sanitized.locator();
    assert!(loc.starts_with("@global:20_Cards/Gateway.md:10-25#"));
    let frag = loc.split('#').nth(1).unwrap();
    assert!(!frag.contains('|'));
    assert!(!frag.contains('§'));
    assert!(!frag.contains('\n'));
    assert!(frag.contains('/'));
    assert_eq!(frag.chars().count(), 48);
}

#[test]
fn test_long_doc_recall_returns_section_locator_and_inspect_note_matches_lines() {
    let dir = tempdir().unwrap();
    let vault = dir.path();
    write_long_multi_section_doc(vault, "20_Cards/PaymentGateway.md");

    let mut storage = SqliteStorage::in_memory().unwrap();
    let adapter = ObsidianAdapter::new(vault);
    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    scanner.sync(false).unwrap();

    let engine = HybridSearchEngine::new(&storage, None);
    assert_eq!(engine.section_lexical_weight(), 0.0);

    for mode in [SearchMode::Bm25, SearchMode::Hybrid] {
        let hits = engine
            .search("idempotency key replay", mode, 5)
            .expect("search should succeed");
        assert_eq!(hits.len(), 1);
        let hit = &hits[0];
        assert_eq!(hit.path, "20_Cards/PaymentGateway.md");
        let sec = hit.section.as_ref().expect("section locator attached");
        assert_eq!(sec.heading, "7. Idempotency & Replay Protection");
        let locator = hit.locator();
        assert!(locator.starts_with("20_Cards/PaymentGateway.md:"));
        assert!(locator.ends_with("#7. Idempotency & Replay Protection"));
        assert!(hit.snippet.contains("idempotency key"));

        // inspect_note on the returned locator must return the exact 1-based file lines
        let report = k0maru::inspect_note_detail(vault, &locator).unwrap();
        assert!(report
            .body
            .starts_with("## 7. Idempotency & Replay Protection"));
        assert!(report.body.contains("ledger-service cache"));
        assert!(!report.body.contains("## 8. Archival"));

        // full_body: true returns the whole document body regardless of line range
        let full_report = k0maru::inspect_note_detail_with_options(vault, &locator, true).unwrap();
        assert!(full_report.body.contains("## 1. Ingress Validation"));
        assert!(full_report.body.contains("## 8. Archival"));
    }
}

#[test]
fn test_upsert_diff_reembeds_only_changed_section_and_delete_purges_cleanly() {
    let dir = tempdir().unwrap();
    let vault = dir.path();
    write_long_multi_section_doc(vault, "20_Cards/PaymentGateway.md");

    let mut storage = SqliteStorage::in_memory().unwrap();
    let adapter = ObsidianAdapter::new(vault);
    let embedder = Arc::new(NamedMockEmbedder::new("test-embed-v1", 384));

    {
        let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
        scanner.sync(false).unwrap();
    }
    let vec_sync = VectorSyncEngine::new();
    let stats1 = vec_sync
        .sync_sections(&mut storage, &*embedder, None)
        .unwrap();
    assert!(stats1.embedded >= 8);
    assert_eq!(stats1.pending, 0);
    assert!(storage.section_vectors_ready("test-embed-v1"));

    // Modify only Section 7 (and advance mtime so incremental scan detects the edit within the same second)
    let doc_path = vault.join("20_Cards/PaymentGateway.md");
    let modified = fs::read_to_string(&doc_path).unwrap().replace(
        "block duplicate replay transfers during network partitions.",
        "prevent duplicate replay settlement during failover windows.",
    );
    fs::write(&doc_path, modified).unwrap();
    fs::OpenOptions::new()
        .write(true)
        .open(&doc_path)
        .unwrap()
        .set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(5))
        .unwrap();

    {
        let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
        scanner.sync(false).unwrap();
    }
    let stats2 = vec_sync
        .sync_sections(&mut storage, &*embedder, None)
        .unwrap();
    // upsert_document already purged the replaced section's vector & meta synchronously,
    // so sync_sections only embeds the 1 newly modified section.
    assert_eq!(stats2.embedded, 1);
    assert_eq!(stats2.deleted, 0);
    assert_eq!(stats2.pending, 0);
    let active_secs: i64 = storage
        .connection()
        .query_row("SELECT COUNT(*) FROM document_sections", [], |r| r.get(0))
        .unwrap();
    let active_sec_vecs: i64 = storage
        .connection()
        .query_row("SELECT COUNT(*) FROM section_vectors", [], |r| r.get(0))
        .unwrap();
    assert_eq!(active_sec_vecs, active_secs);

    // Delete the file and sync -> FTS, vectors, meta, and section state must be completely purged
    fs::remove_file(&doc_path).unwrap();
    {
        let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
        scanner.sync(false).unwrap();
    }

    let doc_count: i64 = storage
        .connection()
        .query_row("SELECT COUNT(*) FROM documents", [], |r| r.get(0))
        .unwrap();
    let sec_count: i64 = storage
        .connection()
        .query_row("SELECT COUNT(*) FROM document_sections", [], |r| r.get(0))
        .unwrap();
    let sec_vec_count: i64 = storage
        .connection()
        .query_row("SELECT COUNT(*) FROM section_vectors", [], |r| r.get(0))
        .unwrap();
    let sec_meta_count: i64 = storage
        .connection()
        .query_row("SELECT COUNT(*) FROM section_vector_meta", [], |r| r.get(0))
        .unwrap();
    let sec_state_count: i64 = storage
        .connection()
        .query_row("SELECT COUNT(*) FROM document_section_state", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(doc_count, 0);
    assert_eq!(sec_count, 0);
    assert_eq!(sec_vec_count, 0);
    assert_eq!(sec_meta_count, 0);
    assert_eq!(sec_state_count, 0);

    // FTS5 integrity check on fts_sections must succeed and KNN must return zero ghost hits
    storage
        .connection()
        .execute(
            "INSERT INTO fts_sections(fts_sections) VALUES ('integrity-check')",
            [],
        )
        .unwrap();
    let q = embedder.embed("idempotency replay").unwrap();
    let knn = storage.search_section_vectors(&q, 10).unwrap();
    assert!(knn.is_empty());
}

#[test]
fn test_lazy_upgrade_budgeted_section_sync_and_model_guard_fallback() {
    let dir = tempdir().unwrap();
    let vault = dir.path();
    write_long_multi_section_doc(vault, "20_Cards/PaymentGateway.md");
    fs::write(
        vault.join("20_Cards/Settlement.md"),
        "# Settlement Engine\n\n## Quorum\nDistributed quorum consensus for distributed-settlement.\n",
    )
    .unwrap();

    let mut storage = SqliteStorage::in_memory().unwrap();
    let adapter = ObsidianAdapter::new(vault);
    let embedder_a: Arc<dyn EmbeddingEngine> = Arc::new(NamedMockEmbedder::new("model-alpha", 384));

    {
        let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
        scanner.sync(false).unwrap();
    }
    let vec_sync = VectorSyncEngine::new();

    // 1. Only doc vectors synced -> section_vectors_ready is false, uses doc vectors
    vec_sync.sync_storage(&mut storage, &*embedder_a).unwrap();
    assert!(!storage.section_vectors_ready("model-alpha"));

    {
        let engine_a = HybridSearchEngine::new(&storage, Some(embedder_a.clone()));
        // Query shares no lexical FTS tokens with the note, only semantic alignment via "replay".
        let vec_before = engine_a
            .search("token_replay_semantic", SearchMode::Vector, 5)
            .unwrap();
        assert!(!vec_before.is_empty());
        assert!(
            vec_before[0].section.is_none(),
            "without section vectors and lexical match, doc-vector fallback leaves section as None"
        );
    }

    // 2. Budgeted section sync (budget = 3) embeds at most 3, leaves pending > 0, ready == false
    let partial = vec_sync
        .sync_sections(&mut storage, &*embedder_a, Some(3))
        .unwrap();
    assert_eq!(partial.embedded, 3);
    assert!(partial.pending > 0);
    assert!(!storage.section_vectors_ready("model-alpha"));

    // 3. Complete section sync (budget = None) -> section_vectors_complete='1' and section KNN active
    let full = vec_sync
        .sync_sections(&mut storage, &*embedder_a, None)
        .unwrap();
    assert!(full.embedded > 0);
    assert_eq!(full.pending, 0);
    assert!(storage.section_vectors_ready("model-alpha"));
    assert_eq!(
        storage
            .meta_get("section_vectors_complete")
            .unwrap()
            .as_deref(),
        Some("1")
    );

    let engine_a_after = HybridSearchEngine::new(&storage, Some(embedder_a.clone()));
    let vec_after = engine_a_after
        .search("token_replay_semantic", SearchMode::Vector, 5)
        .unwrap();
    assert!(!vec_after.is_empty());
    let sec_after = vec_after[0]
        .section
        .as_ref()
        .expect("section KNN must populate section locator even without lexical FTS hit");
    assert_eq!(sec_after.heading, "7. Idempotency & Replay Protection");

    // 4. Mismatched embedder disables vector channel, while BM25 results still return in Hybrid
    let mismatched: Arc<dyn EmbeddingEngine> = Arc::new(MockEmbeddingEngine::new(384)); // "mock-embedding-384" != "model-alpha"
    assert!(!k0maru::vector_scope_enabled(&storage, &*mismatched));
    let engine_mismatch = HybridSearchEngine::new(&storage, Some(mismatched.clone()));
    let hybrid_fallback = engine_mismatch
        .search("distributed-settlement", SearchMode::Hybrid, 5)
        .unwrap();
    assert!(!hybrid_fallback.is_empty());
    assert!(hybrid_fallback
        .iter()
        .any(|r| r.path == "20_Cards/Settlement.md" && r.vector_rank.is_none()));

    // 5. Model guard wipes old vectors when embed_model changes on next sync
    vec_sync.sync_storage(&mut storage, &*mismatched).unwrap();
    assert_eq!(
        storage.meta_get("embed_model").unwrap().as_deref(),
        Some("mock-embedding-384")
    );
    assert_eq!(
        storage
            .meta_get("section_vectors_complete")
            .unwrap()
            .as_deref(),
        Some("0")
    );
    let old_sec_vecs: i64 = storage
        .connection()
        .query_row("SELECT COUNT(*) FROM section_vectors", [], |r| r.get(0))
        .unwrap();
    assert_eq!(old_sec_vecs, 0);
}

#[test]
fn test_parse_locator_edge_cases() {
    let p1 = parse_locator("@global:20_Cards/a.md.notes/b.md:12-30#My Heading").unwrap();
    assert!(p1.is_global);
    assert_eq!(p1.path, "20_Cards/a.md.notes/b.md");
    assert_eq!(p1.line_range, Some((12, 30)));
    assert_eq!(p1.heading.as_deref(), Some("My Heading"));

    let p2 = parse_locator("10_Projects/Spec.MD#Intro").unwrap();
    assert!(!p2.is_global);
    assert_eq!(p2.path, "10_Projects/Spec.MD");
    assert_eq!(p2.line_range, None);
    assert_eq!(p2.heading.as_deref(), Some("Intro"));

    let p3 = parse_locator("notes/v1.md.bak").unwrap();
    assert_eq!(PathBuf::from(&p3.path).extension().unwrap(), "bak");

    assert!(parse_locator("20_Cards/x.md:0-10").is_err());
    assert!(parse_locator("20_Cards/x.md:20-10").is_err());
    assert!(parse_locator("20_Cards/x.md:bad").is_err());
}

#[test]
fn test_reorder_sections_preserves_section_ids_and_vectors() {
    let dir = tempdir().unwrap();
    let vault = dir.path();
    fs::create_dir_all(vault.join("20_Cards")).unwrap();
    let card_path = vault.join("20_Cards/Reorder.md");
    fs::write(
        &card_path,
        "# Reorder\n\n## Alpha\nAlpha payload.\n\n## Beta\nBeta payload.\n\n## Omega\nOmega payload.\n",
    )
    .unwrap();

    let mut storage = SqliteStorage::in_memory().unwrap();
    let adapter = ObsidianAdapter::new(vault);
    let embedder = Arc::new(NamedMockEmbedder::new("test-embed-v1", 384));
    {
        let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
        scanner.sync(false).unwrap();
    }
    let vec_sync = VectorSyncEngine::new();
    let stats1 = vec_sync
        .sync_sections(&mut storage, &*embedder, None)
        .unwrap();
    assert!(stats1.embedded >= 3);

    let doc_id = storage
        .doc_id_for_path(Path::new("20_Cards/Reorder.md"))
        .unwrap()
        .unwrap();
    let before = storage.get_sections(doc_id).unwrap();
    let alpha_id_before = before.iter().find(|s| s.heading == "Alpha").unwrap().id;
    let beta_id_before = before.iter().find(|s| s.heading == "Beta").unwrap().id;

    // Swap Alpha and Beta order while keeping each section's exact byte slice identical
    fs::write(
        &card_path,
        "# Reorder\n\n## Beta\nBeta payload.\n\n## Alpha\nAlpha payload.\n\n## Omega\nOmega payload.\n",
    )
    .unwrap();
    fs::OpenOptions::new()
        .write(true)
        .open(&card_path)
        .unwrap()
        .set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(5))
        .unwrap();
    {
        let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
        scanner.sync(false).unwrap();
    }
    let stats2 = vec_sync
        .sync_sections(&mut storage, &*embedder, None)
        .unwrap();
    assert_eq!(stats2.embedded, 0);
    assert_eq!(stats2.deleted, 0);
    assert_eq!(stats2.pending, 0);

    let after = storage.get_sections(doc_id).unwrap();
    let alpha_after = after.iter().find(|s| s.heading == "Alpha").unwrap();
    let beta_after = after.iter().find(|s| s.heading == "Beta").unwrap();
    assert_eq!(alpha_after.id, alpha_id_before);
    assert_eq!(beta_after.id, beta_id_before);
    assert!(beta_after.ord < alpha_after.ord);
}
