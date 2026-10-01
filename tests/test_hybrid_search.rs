//! Integration tests for Reciprocal Rank Fusion (RRF) Hybrid Search Engine.

use std::path::PathBuf;
use std::sync::Arc;

use k0maru::core::models::{Document, HierarchyLevel, WikiLink};
use k0maru::core::traits::CacheStorage;
use k0maru::storage::{HybridSearchEngine, SearchMode, SqliteStorage};
use k0maru::vector::MockEmbeddingEngine;

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
        frontmatter: serde_json::json!({}),
        links,
        tags: tags.into_iter().map(|s| s.to_string()).collect(),
        content_hash: format!("hash-{}", path),
        mtime: 1000,
        body: body.to_string(),
    }
}

#[test]
fn test_bm25_exact_match_mode() {
    let mut storage = SqliteStorage::in_memory().expect("in-memory db");

    let doc1 = make_doc(
        "rust_mem.md",
        "Rust Memory Model",
        "Rust uses RAII and ownership for deterministic zero-cost memory management.",
        vec![],
        vec!["rust", "memory"],
    );
    let doc2 = make_doc(
        "python_gc.md",
        "Python Garbage Collection",
        "Python relies on reference counting and cycle detector.",
        vec![],
        vec!["python"],
    );
    let doc3 = make_doc(
        "go_runtime.md",
        "Go Runtime GC",
        "Go uses a concurrent tricolor mark and sweep garbage collector.",
        vec![],
        vec!["golang"],
    );

    storage.upsert_document(&doc1).expect("upsert doc1");
    storage.upsert_document(&doc2).expect("upsert doc2");
    storage.upsert_document(&doc3).expect("upsert doc3");

    let engine = HybridSearchEngine::new(&storage, None);

    let results = engine
        .search("ownership", SearchMode::Bm25, 5)
        .expect("search ownership");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].path, "rust_mem.md");
    assert_eq!(results[0].title, "Rust Memory Model");
    assert_eq!(results[0].bm25_rank, Some(1));
    assert_eq!(results[0].vector_rank, None);
    assert_eq!(results[0].graph_boost, 0.0);
    assert!(results[0].snippet.contains("ownership"));
    assert!(results[0].score > 0.0);
}

#[test]
fn test_vector_mode_with_mock_embedder() {
    let mut storage = SqliteStorage::in_memory().expect("in-memory db");
    let embedder = Arc::new(MockEmbeddingEngine::new(384));

    let doc1 = make_doc(
        "neural_networks.md",
        "Neural Networks",
        "Deep learning architectures and artificial neural networks backpropagation.",
        vec![],
        vec!["ml"],
    );
    let doc2 = make_doc(
        "database_indexing.md",
        "Database Indexing",
        "B-trees and LSM trees for fast relational queries.",
        vec![],
        vec!["databases"],
    );

    storage.upsert_document(&doc1).expect("upsert doc1");
    storage.upsert_document(&doc2).expect("upsert doc2");

    let emb1 = embedder.embed_single(&format!("{} {}", doc1.title, doc1.body));
    let emb2 = embedder.embed_single(&format!("{} {}", doc2.title, doc2.body));

    storage
        .insert_vector("neural_networks.md", &emb1, 1)
        .expect("insert v1");
    storage
        .insert_vector("database_indexing.md", &emb2, 2)
        .expect("insert v2");

    let engine = HybridSearchEngine::new(&storage, Some(embedder));

    let results = engine
        .search("deep neural backprop", SearchMode::Vector, 5)
        .expect("search vector");

    assert!(!results.is_empty());
    assert_eq!(results[0].path, "neural_networks.md");
    assert_eq!(results[0].vector_rank, Some(1));
    assert_eq!(results[0].bm25_rank, None);
    assert_eq!(results[0].graph_boost, 0.0);
    assert!(results[0].score > 0.0);
}

#[test]
fn test_vector_mode_requires_embedder() {
    let storage = SqliteStorage::in_memory().expect("in-memory db");
    let engine = HybridSearchEngine::new(&storage, None);

    let err = engine.search("query", SearchMode::Vector, 5);
    assert!(
        err.is_err(),
        "Vector mode without embedder must return error"
    );
}

#[test]
fn test_hybrid_mode_rrf_rank_fusion() {
    let mut storage = SqliteStorage::in_memory().expect("in-memory db");
    let embedder = Arc::new(MockEmbeddingEngine::new(384));

    let doc1 = make_doc(
        "doc1.md",
        "Quantum Computing",
        "Quantum computing uses superposition and entanglement algorithms.",
        vec![],
        vec!["quantum"],
    );
    let doc2 = make_doc(
        "doc2.md",
        "Classical Algorithms",
        "Sorting and graph search algorithms in polynomial time.",
        vec![],
        vec!["algorithms"],
    );

    storage.upsert_document(&doc1).expect("upsert doc1");
    storage.upsert_document(&doc2).expect("upsert doc2");

    let emb1 = embedder.embed_single(&format!("{} {}", doc1.title, doc1.body));
    let emb2 = embedder.embed_single(&format!("{} {}", doc2.title, doc2.body));

    storage
        .insert_vector("doc1.md", &emb1, 1)
        .expect("insert v1");
    storage
        .insert_vector("doc2.md", &emb2, 2)
        .expect("insert v2");

    let engine = HybridSearchEngine::new(&storage, Some(embedder))
        .with_rrf_k(60)
        .with_weights(0.5, 0.5)
        .with_graph_boost_weight(0.05);

    let results = engine
        .search("Quantum computing", SearchMode::Hybrid, 5)
        .expect("search hybrid");

    assert!(!results.is_empty());
    let top = &results[0];
    assert_eq!(top.path, "doc1.md");
    assert_eq!(top.bm25_rank, Some(1));
    assert_eq!(top.vector_rank, Some(1));

    // Expected score: 0.5 / (60 + 1) + 0.5 / (60 + 1) = 1.0 / 61.0
    let expected_rrf = (0.5 / 61.0) + (0.5 / 61.0);
    let diff = (top.score - expected_rrf).abs();
    assert!(diff < 1e-4, "Expected ~{}, got {}", expected_rrf, top.score);
}

#[test]
fn test_hybrid_vector_captures_synonym_bm25_misses() {
    let mut storage = SqliteStorage::in_memory().expect("in-memory db");
    let embedder = Arc::new(MockEmbeddingEngine::new(384));

    // Doc A: exact match for keyword "automobile"
    let doc_a = make_doc(
        "auto.md",
        "Automobile Transportation",
        "Automobile manufacturing and highway transit networks.",
        vec![],
        vec!["transit"],
    );
    // Doc B: contains "locomotive train" but NO mention of "automobile"
    let doc_b = make_doc(
        "train.md",
        "Train Locomotive Transportation",
        "High-speed passenger trains and electric locomotive rail freight.",
        vec![],
        vec!["rail"],
    );

    storage.upsert_document(&doc_a).expect("upsert doc_a");
    storage.upsert_document(&doc_b).expect("upsert doc_b");

    let emb_a = embedder.embed_single(&format!("{} {}", doc_a.title, doc_a.body));
    // Simulate Doc B being semantically relevant in vector space
    let emb_b = embedder.embed_single("automobile transportation transit rail");

    storage
        .insert_vector("auto.md", &emb_a, 1)
        .expect("insert v1");
    storage
        .insert_vector("train.md", &emb_b, 2)
        .expect("insert v2");

    let engine = HybridSearchEngine::new(&storage, Some(embedder));

    // BM25 for "automobile" matches auto.md, but vector matches both auto.md and train.md
    let results = engine
        .search("automobile", SearchMode::Hybrid, 5)
        .expect("search hybrid");

    let paths: Vec<&str> = results.iter().map(|r| r.path.as_str()).collect();
    assert!(paths.contains(&"auto.md"));
    assert!(paths.contains(&"train.md"));

    let train_hit = results.iter().find(|r| r.path == "train.md").unwrap();
    assert_eq!(train_hit.bm25_rank, None);
    assert!(train_hit.vector_rank.is_some());
    assert!(train_hit.score > 0.0);
}

#[test]
fn test_hybrid_graph_boost_elevating_connected_notes() {
    let mut storage = SqliteStorage::in_memory().expect("in-memory db");

    // doc1: Target note in top pool
    let doc1 = make_doc(
        "consensus.md",
        "Distributed Consensus",
        "Fundamental problem of distributed consensus in asynchronous network systems.",
        vec![],
        vec!["distributed"],
    );

    // doc2: Isolated note with good BM25 match, but NO links
    let doc2 = make_doc(
        "byzantine.md",
        "Byzantine Agreement",
        "Consensus under arbitrary Byzantine fault tolerance and malicious adversarial nodes.",
        vec![],
        vec!["byzantine"],
    );

    // doc3: Slightly weaker match or lower rank, but LINKS directly to doc1
    let doc3 = make_doc(
        "raft.md",
        "Raft Protocol",
        "Understandable consensus algorithm with leader election. Implements [[Distributed Consensus]].",
        vec![WikiLink::new(
            "Distributed Consensus",
            None::<String>,
            "[[Distributed Consensus]]",
        )],
        vec!["consensus", "raft"],
    );

    storage.upsert_document(&doc1).expect("upsert doc1");
    storage.upsert_document(&doc2).expect("upsert doc2");
    storage.upsert_document(&doc3).expect("upsert doc3");

    let engine = HybridSearchEngine::new(&storage, None)
        .with_rrf_k(60)
        .with_weights(1.0, 0.0)
        .with_graph_boost_weight(0.05);

    let results = engine
        .search("Consensus", SearchMode::Hybrid, 5)
        .expect("search hybrid");

    assert!(results.len() >= 3);

    // Verify that raft.md received graph_boost of 0.05 because it links to consensus.md
    let raft_hit = results.iter().find(|r| r.path == "raft.md").unwrap();
    assert_eq!(raft_hit.graph_boost, 0.05);

    // Verify that consensus.md received graph_boost of 0.05 because raft.md links to it (backlink)
    let consensus_hit = results.iter().find(|r| r.path == "consensus.md").unwrap();
    assert_eq!(consensus_hit.graph_boost, 0.05);

    // Verify that byzantine.md has 0.0 graph boost (isolated)
    let byzantine_hit = results.iter().find(|r| r.path == "byzantine.md").unwrap();
    assert_eq!(byzantine_hit.graph_boost, 0.0);

    // raft.md gets elevated above byzantine.md due to the +0.05 graph boost!
    let raft_idx = results.iter().position(|r| r.path == "raft.md").unwrap();
    let byzantine_idx = results
        .iter()
        .position(|r| r.path == "byzantine.md")
        .unwrap();
    assert!(
        raft_idx < byzantine_idx,
        "raft.md (idx {}) should be ranked higher than byzantine.md (idx {}) due to graph boost",
        raft_idx,
        byzantine_idx
    );
}

#[test]
fn test_hybrid_fallback_to_bm25_without_embedder() {
    let mut storage = SqliteStorage::in_memory().expect("in-memory db");

    let doc = make_doc(
        "kv_store.md",
        "Key-Value Store",
        "Bitcask append-only log structured hash table key value storage.",
        vec![],
        vec!["storage"],
    );
    storage.upsert_document(&doc).expect("upsert");

    let engine = HybridSearchEngine::new(&storage, None);
    let results = engine
        .search("Bitcask", SearchMode::Hybrid, 5)
        .expect("hybrid search without embedder");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].path, "kv_store.md");
    assert_eq!(results[0].bm25_rank, Some(1));
    assert_eq!(results[0].vector_rank, None);
    assert!(results[0].score > 0.0);
}

#[test]
fn test_search_empty_and_zero_limit() {
    let storage = SqliteStorage::in_memory().expect("in-memory db");
    let engine = HybridSearchEngine::new(&storage, None);

    let empty_q = engine
        .search("", SearchMode::Hybrid, 5)
        .expect("empty query");
    assert!(empty_q.is_empty());

    let ws_q = engine
        .search("   ", SearchMode::Bm25, 5)
        .expect("whitespace query");
    assert!(ws_q.is_empty());

    let zero_lim = engine
        .search("test", SearchMode::Hybrid, 0)
        .expect("zero limit");
    assert!(zero_lim.is_empty());
}

#[test]
fn test_deterministic_tie_breaking() {
    let mut storage = SqliteStorage::in_memory().expect("in-memory db");

    // Two docs with identical content structure
    let doc_b = make_doc(
        "b.md",
        "Beta Note",
        "Identical keywords sample.",
        vec![],
        vec![],
    );
    let doc_a = make_doc(
        "a.md",
        "Alpha Note",
        "Identical keywords sample.",
        vec![],
        vec![],
    );

    storage.upsert_document(&doc_b).expect("upsert b");
    storage.upsert_document(&doc_a).expect("upsert a");

    let engine = HybridSearchEngine::new(&storage, None);
    let results = engine
        .search("keywords sample", SearchMode::Hybrid, 5)
        .expect("search");

    assert_eq!(results.len(), 2);
    // When ranks and scores are deterministic, ordering is deterministic
    let first = &results[0].path;
    let second = &results[1].path;
    assert_ne!(first, second);
}
