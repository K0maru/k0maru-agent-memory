//! Integration tests for Search Debugger and Explainability Cockpit (Ticket 21).

use std::fs;
use std::path::Path;
use std::sync::Arc;

use axum::body::to_bytes;
use axum::http::{Request, StatusCode};
use tempfile::tempdir;
use tower::ServiceExt;

use k0maru::storage::{SearchResult, SqliteStorage};
use k0maru::ui::routes::{create_router, AppState, DashboardAssets};
use k0maru::vector::MockEmbeddingEngine;

fn setup_connected_mock_vault(vault_dir: &Path) {
    let proj_dir = vault_dir.join("10_Projects");
    let cards_dir = vault_dir.join("20_Cards");

    fs::create_dir_all(&proj_dir).unwrap();
    fs::create_dir_all(&cards_dir).unwrap();

    // Two connected documents with shared keyword "Memory"
    let alpha_content = r#"---
title: Memory Hub Architecture
tags: [core, memory]
---
# Memory Hub Architecture
Core memory architecture description with forward link to [[Vector Storage]].
"#;
    fs::write(proj_dir.join("MemoryHub.md"), alpha_content).unwrap();

    let beta_content = r#"---
title: Vector Storage
tags: [vector, storage]
---
# Vector Storage
Vector embeddings and sqlite-vec memory index linked back to [[Memory Hub Architecture]].
"#;
    fs::write(cards_dir.join("VectorStorage.md"), beta_content).unwrap();

    // Isolated document
    let gamma_content = r#"---
title: Unrelated Notes
tags: [misc]
---
# Unrelated Notes
Completely standalone note without any WikiLinks.
"#;
    fs::write(cards_dir.join("Isolated.md"), gamma_content).unwrap();
}

#[test]
fn test_embedded_bundle_search_debugger_elements() {
    let index_file = DashboardAssets::get("index.html").expect("index.html must exist");
    let index_html = std::str::from_utf8(&index_file.data).expect("valid utf-8");

    // Search console DOM elements
    assert!(
        index_html.contains("id=\"search-input\""),
        "index.html must contain #search-input"
    );
    assert!(
        index_html.contains("data-mode=\"hybrid\""),
        "index.html must contain data-mode=\"hybrid\""
    );
    assert!(
        index_html.contains("data-mode=\"bm25\""),
        "index.html must contain data-mode=\"bm25\""
    );
    assert!(
        index_html.contains("data-mode=\"vector\""),
        "index.html must contain data-mode=\"vector\""
    );
    assert!(
        index_html.contains("id=\"search-results\""),
        "index.html must contain #search-results"
    );
    assert!(
        index_html.contains("id=\"search-limit\""),
        "index.html must contain #search-limit slider"
    );
    assert!(
        index_html.contains("id=\"search-latency\""),
        "index.html must contain #search-latency indicator"
    );

    // Style bundle checks
    let style_file = DashboardAssets::get("style.css").expect("style.css must exist");
    let style_css = std::str::from_utf8(&style_file.data).expect("valid utf-8");
    assert!(
        style_css.contains("graph-boost"),
        "style.css must contain graph-boost badge styling"
    );
    assert!(
        style_css.contains("search-result-card") || style_css.contains("result-card"),
        "style.css must contain search result card styling"
    );

    // App JS bundle checks
    let js_file = DashboardAssets::get("app.js").expect("app.js must exist");
    let js_code = std::str::from_utf8(&js_file.data).expect("valid utf-8");
    assert!(
        js_code.contains("/api/search"),
        "app.js must contain /api/search fetcher"
    );
    assert!(
        js_code.contains("graph-boost"),
        "app.js must render graph-boost badge"
    );
    assert!(
        js_code.contains("score"),
        "app.js must format retrieval score"
    );
    assert!(
        js_code.contains("bm25_rank"),
        "app.js must handle bm25_rank"
    );
    assert!(
        js_code.contains("vector_rank"),
        "app.js must handle vector_rank"
    );
}

#[tokio::test]
async fn test_api_search_explainability_metrics() {
    let temp = tempdir().unwrap();
    let vault_path = temp.path().to_path_buf();
    setup_connected_mock_vault(&vault_path);

    let storage = SqliteStorage::in_memory().unwrap();
    let embedder = Arc::new(MockEmbeddingEngine::new(384));
    let state = AppState::from_storage(vault_path.clone(), storage, Some(embedder));
    let app = create_router(state);

    // 1. Sync vault documents into storage
    let sync_req = Request::builder()
        .method("POST")
        .uri("/api/sync")
        .body(axum::body::Body::empty())
        .unwrap();
    let sync_res = app.clone().oneshot(sync_req).await.unwrap();
    assert_eq!(sync_res.status(), StatusCode::OK);

    // 2. Query with Hybrid mode: query "Memory" matches MemoryHub and VectorStorage
    let req = Request::builder()
        .uri("/api/search?q=Memory&mode=hybrid&limit=5")
        .body(axum::body::Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let results: Vec<SearchResult> = serde_json::from_slice(&body).unwrap();
    assert!(!results.is_empty(), "Search should return results");

    let first = &results[0];
    assert!(first.score > 0.0, "Score must be positive");
    assert!(
        !first.title.is_empty(),
        "Title must be present and non-empty"
    );
    assert!(!first.path.is_empty(), "Path must be present and non-empty");
    assert!(
        !first.snippet.is_empty(),
        "Snippet must be present and non-empty"
    );

    // Check that graph boost was evaluated and at least one connected note received +0.05
    let boosted = results.iter().find(|r| r.graph_boost > 0.0);
    assert!(
        boosted.is_some(),
        "Connected notes in pool must receive graph_boost > 0"
    );
    let boosted_item = boosted.unwrap();
    assert!(
        (boosted_item.graph_boost - 0.05).abs() < 1e-4,
        "Graph boost should equal default 0.05, got {}",
        boosted_item.graph_boost
    );

    // Verify BM25-only search explainability fields
    let bm25_req = Request::builder()
        .uri("/api/search?q=Architecture&mode=bm25&limit=5")
        .body(axum::body::Body::empty())
        .unwrap();
    let bm25_res = app.clone().oneshot(bm25_req).await.unwrap();
    assert_eq!(bm25_res.status(), StatusCode::OK);
    let bm25_body = to_bytes(bm25_res.into_body(), usize::MAX).await.unwrap();
    let bm25_results: Vec<SearchResult> = serde_json::from_slice(&bm25_body).unwrap();
    assert!(!bm25_results.is_empty());
    assert_eq!(bm25_results[0].bm25_rank, Some(1));
    assert_eq!(bm25_results[0].vector_rank, None);
    assert_eq!(bm25_results[0].graph_boost, 0.0);

    // Verify Vector-only search explainability fields
    let vec_req = Request::builder()
        .uri("/api/search?q=Architecture&mode=vector&limit=5")
        .body(axum::body::Body::empty())
        .unwrap();
    let vec_res = app.clone().oneshot(vec_req).await.unwrap();
    assert_eq!(vec_res.status(), StatusCode::OK);
    let vec_body = to_bytes(vec_res.into_body(), usize::MAX).await.unwrap();
    let vec_results: Vec<SearchResult> = serde_json::from_slice(&vec_body).unwrap();
    assert!(!vec_results.is_empty());
    assert_eq!(vec_results[0].vector_rank, Some(1));
    assert_eq!(vec_results[0].bm25_rank, None);
    assert_eq!(vec_results[0].graph_boost, 0.0);
}
