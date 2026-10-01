use std::fs;
use std::path::Path;
use std::sync::Arc;

use axum::body::to_bytes;
use axum::http::header::CONTENT_TYPE;
use axum::http::{Request, StatusCode};
use tempfile::tempdir;
use tower::ServiceExt;

use k0maru::storage::{SearchResult, SqliteStorage};
use k0maru::ui::routes::{
    create_router, AppState, GraphResponse, LogDetailResponse, LogItemResponse, StatusResponse,
    SyncResponse,
};
use k0maru::vector::MockEmbeddingEngine;

fn setup_mock_vault(vault_dir: &Path) {
    let proj_dir = vault_dir.join("10_Projects");
    let cards_dir = vault_dir.join("20_Cards");
    let refs_dir = vault_dir.join(".k0maru").join("refs");

    fs::create_dir_all(&proj_dir).unwrap();
    fs::create_dir_all(&cards_dir).unwrap();
    fs::create_dir_all(&refs_dir).unwrap();

    let alpha_content = r#"---
title: Project Alpha
tags: [project, core]
---
# Project Alpha
High priority project description linking to [[Architecture]] and [[Database]].
"#;
    fs::write(proj_dir.join("Alpha.md"), alpha_content).unwrap();

    let arch_content = r#"---
title: Architecture Spec
tags: [architecture, design]
---
# Architecture Spec
Foundational architectural decisions for [[Alpha]].
"#;
    fs::write(cards_dir.join("Architecture.md"), arch_content).unwrap();

    let log1_content = (1..=25)
        .map(|i| {
            if i == 12 {
                format!("Line {}: Error: failed to compile dependency foo-bar", i)
            } else if i == 18 {
                format!("Line {}: Warning: unused variable `ret`", i)
            } else {
                format!("Line {}: cargo build compiling submodules", i)
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(refs_dir.join("node_11223344.log"), log1_content).unwrap();

    let log2_content = (1..=40)
        .map(|i| format!("Line {}: test suite execution step", i))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(refs_dir.join("task_verify_node_55667788.log"), log2_content).unwrap();
}

#[tokio::test]
async fn test_ui_e2e_full_lifecycle() {
    let temp = tempdir().unwrap();
    let vault_path = temp.path().to_path_buf();
    setup_mock_vault(&vault_path);

    let storage = SqliteStorage::in_memory().unwrap();
    let embedder = Arc::new(MockEmbeddingEngine::new(384));
    let state = AppState::from_storage(vault_path.clone(), storage, Some(embedder));
    let app = create_router(state);

    // =========================================================================
    // 1. Static Asset Delivery and HTML Element Checks
    // =========================================================================
    let req_root = Request::builder()
        .uri("/")
        .body(axum::body::Body::empty())
        .unwrap();
    let res_root = app.clone().oneshot(req_root).await.unwrap();
    assert_eq!(res_root.status(), StatusCode::OK);
    let root_content_type = res_root
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(root_content_type.starts_with("text/html"));

    let body_bytes = to_bytes(res_root.into_body(), usize::MAX).await.unwrap();
    let index_html = String::from_utf8_lossy(&body_bytes);

    // Verify required HTML IDs and elements exist in bundle
    assert!(
        index_html.contains("id=\"panel-logs\""),
        "HTML must contain #panel-logs"
    );
    assert!(
        index_html.contains("id=\"panel-scoreboard\""),
        "HTML must contain #panel-scoreboard"
    );
    assert!(
        index_html.contains("id=\"log-mermaid-viewer\""),
        "HTML must contain #log-mermaid-viewer"
    );
    assert!(
        index_html.contains("id=\"logs-list\""),
        "HTML must contain #logs-list"
    );
    assert!(
        index_html.contains("id=\"token-saved-val\""),
        "HTML must contain #token-saved-val"
    );
    assert!(
        index_html.contains("id=\"log-detail-container\""),
        "HTML must contain #log-detail-container"
    );
    assert!(
        index_html.contains("id=\"log-raw-viewer\""),
        "HTML must contain #log-raw-viewer"
    );
    assert!(
        index_html.contains("id=\"token-trr-val\""),
        "HTML must contain #token-trr-val"
    );
    assert!(
        index_html.contains("id=\"btn-copy-node-id\""),
        "HTML must contain #btn-copy-node-id"
    );
    assert!(
        index_html.contains("id=\"btn-copy-raw-log\""),
        "HTML must contain #btn-copy-raw-log"
    );
    assert!(
        index_html.contains("id=\"btn-scoreboard-sync\""),
        "HTML must contain #btn-scoreboard-sync"
    );

    // CSS Delivery
    let req_css = Request::builder()
        .uri("/style.css")
        .body(axum::body::Body::empty())
        .unwrap();
    let res_css = app.clone().oneshot(req_css).await.unwrap();
    assert_eq!(res_css.status(), StatusCode::OK);
    let css_bytes = to_bytes(res_css.into_body(), usize::MAX).await.unwrap();
    let css_text = String::from_utf8_lossy(&css_bytes);
    assert!(css_text.contains(".logs-layout"));
    assert!(css_text.contains(".mermaid-viewer"));
    assert!(css_text.contains(".raw-log-viewer"));
    assert!(css_text.contains(".line-error"));
    assert!(css_text.contains(".scoreboard-grid"));
    assert!(css_text.contains(".token-hero-value"));

    // JS Delivery
    let req_js = Request::builder()
        .uri("/app.js")
        .body(axum::body::Body::empty())
        .unwrap();
    let res_js = app.clone().oneshot(req_js).await.unwrap();
    assert_eq!(res_js.status(), StatusCode::OK);
    let js_bytes = to_bytes(res_js.into_body(), usize::MAX).await.unwrap();
    let js_text = String::from_utf8_lossy(&js_bytes);
    assert!(js_text.contains("loadLogsList"));
    assert!(js_text.contains("selectLogNode"));
    assert!(js_text.contains("updateScoreboard"));
    assert!(js_text.contains("renderMermaidToSvg"));
    assert!(js_text.contains("copyToClipboard"));

    // =========================================================================
    // 2. Initial Status
    // =========================================================================
    let req_status = Request::builder()
        .uri("/api/status")
        .body(axum::body::Body::empty())
        .unwrap();
    let res_status = app.clone().oneshot(req_status).await.unwrap();
    assert_eq!(res_status.status(), StatusCode::OK);
    let body = to_bytes(res_status.into_body(), usize::MAX).await.unwrap();
    let status_init: StatusResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(status_init.total_documents, 0);
    assert_eq!(status_init.total_logs, 2);

    // =========================================================================
    // 3. Vault Sync
    // =========================================================================
    let req_sync = Request::builder()
        .method("POST")
        .uri("/api/sync")
        .body(axum::body::Body::empty())
        .unwrap();
    let res_sync = app.clone().oneshot(req_sync).await.unwrap();
    assert_eq!(res_sync.status(), StatusCode::OK);
    let body = to_bytes(res_sync.into_body(), usize::MAX).await.unwrap();
    let sync_res: SyncResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(sync_res.sync_stats.added, 2);
    assert_eq!(sync_res.vector_stats.embedded_count, 2);

    // =========================================================================
    // 4. Status After Sync
    // =========================================================================
    let req_status2 = Request::builder()
        .uri("/api/status")
        .body(axum::body::Body::empty())
        .unwrap();
    let res_status2 = app.clone().oneshot(req_status2).await.unwrap();
    assert_eq!(res_status2.status(), StatusCode::OK);
    let body = to_bytes(res_status2.into_body(), usize::MAX).await.unwrap();
    let status_after: StatusResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(status_after.total_documents, 2);
    assert_eq!(status_after.total_vectors, 2);
    assert_eq!(status_after.total_logs, 2);
    assert!(status_after.last_sync_time.is_some());

    // =========================================================================
    // 5. Hybrid, BM25, and Vector Search Endpoints
    // =========================================================================
    // Hybrid mode
    let req_search_hybrid = Request::builder()
        .uri("/api/search?q=Alpha&mode=hybrid&limit=5")
        .body(axum::body::Body::empty())
        .unwrap();
    let res_hybrid = app.clone().oneshot(req_search_hybrid).await.unwrap();
    assert_eq!(res_hybrid.status(), StatusCode::OK);
    let body = to_bytes(res_hybrid.into_body(), usize::MAX).await.unwrap();
    let search_results: Vec<SearchResult> = serde_json::from_slice(&body).unwrap();
    assert!(!search_results.is_empty());
    assert!(search_results[0].title.contains("Alpha"));

    // BM25 mode
    let req_search_bm25 = Request::builder()
        .uri("/api/search?q=Architecture&mode=bm25&limit=5")
        .body(axum::body::Body::empty())
        .unwrap();
    let res_bm25 = app.clone().oneshot(req_search_bm25).await.unwrap();
    assert_eq!(res_bm25.status(), StatusCode::OK);

    // Vector mode
    let req_search_vec = Request::builder()
        .uri("/api/search?q=project&mode=vector&limit=5")
        .body(axum::body::Body::empty())
        .unwrap();
    let res_vec = app.clone().oneshot(req_search_vec).await.unwrap();
    assert_eq!(res_vec.status(), StatusCode::OK);

    // Empty query returns empty array
    let req_empty_q = Request::builder()
        .uri("/api/search?q=")
        .body(axum::body::Body::empty())
        .unwrap();
    let res_empty_q = app.clone().oneshot(req_empty_q).await.unwrap();
    assert_eq!(res_empty_q.status(), StatusCode::OK);
    let body = to_bytes(res_empty_q.into_body(), usize::MAX).await.unwrap();
    let empty_results: Vec<SearchResult> = serde_json::from_slice(&body).unwrap();
    assert!(empty_results.is_empty());

    // Invalid mode returns 400 Bad Request
    let req_invalid_mode = Request::builder()
        .uri("/api/search?q=test&mode=unknown_engine")
        .body(axum::body::Body::empty())
        .unwrap();
    let res_invalid_mode = app.clone().oneshot(req_invalid_mode).await.unwrap();
    assert_eq!(res_invalid_mode.status(), StatusCode::BAD_REQUEST);

    // =========================================================================
    // 6. Knowledge Graph Endpoint
    // =========================================================================
    let req_graph = Request::builder()
        .uri("/api/graph")
        .body(axum::body::Body::empty())
        .unwrap();
    let res_graph = app.clone().oneshot(req_graph).await.unwrap();
    assert_eq!(res_graph.status(), StatusCode::OK);
    let body = to_bytes(res_graph.into_body(), usize::MAX).await.unwrap();
    let graph: GraphResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(graph.nodes.len(), 2);
    assert!(!graph.edges.is_empty());

    // =========================================================================
    // 7. Log Listing and Node Inspection Endpoints
    // =========================================================================
    // List logs
    let req_logs = Request::builder()
        .uri("/api/logs")
        .body(axum::body::Body::empty())
        .unwrap();
    let res_logs = app.clone().oneshot(req_logs).await.unwrap();
    assert_eq!(res_logs.status(), StatusCode::OK);
    let body = to_bytes(res_logs.into_body(), usize::MAX).await.unwrap();
    let logs: Vec<LogItemResponse> = serde_json::from_slice(&body).unwrap();
    assert_eq!(logs.len(), 2);

    let node1 = logs.iter().find(|l| l.id == "node_11223344").unwrap();
    assert_eq!(node1.line_count, 25);
    assert!(node1.task_id.is_none());

    let node2 = logs.iter().find(|l| l.id == "node_55667788").unwrap();
    assert_eq!(node2.line_count, 40);
    assert_eq!(node2.task_id.as_deref(), Some("task_verify"));

    // Inspect valid log node
    let req_inspect = Request::builder()
        .uri("/api/logs/node_11223344")
        .body(axum::body::Body::empty())
        .unwrap();
    let res_inspect = app.clone().oneshot(req_inspect).await.unwrap();
    assert_eq!(res_inspect.status(), StatusCode::OK);
    let body = to_bytes(res_inspect.into_body(), usize::MAX).await.unwrap();
    let detail: LogDetailResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(detail.id, "node_11223344");
    assert!(detail.mermaid.contains("stateDiagram-v2"));
    assert!(detail.mermaid.contains("Offloaded"));
    assert!(detail.mermaid.contains("Captured"));
    assert!(detail.content.contains("failed to compile dependency"));

    // Inspect nonexistent log node returns 404
    let req_inspect_404 = Request::builder()
        .uri("/api/logs/node_missing_999999")
        .body(axum::body::Body::empty())
        .unwrap();
    let res_inspect_404 = app.clone().oneshot(req_inspect_404).await.unwrap();
    assert_eq!(res_inspect_404.status(), StatusCode::NOT_FOUND);

    // =========================================================================
    // 8. Unknown API Endpoint returns 404 JSON, not SPA HTML fallback
    // =========================================================================
    let req_unknown_api = Request::builder()
        .uri("/api/unknown_endpoint")
        .body(axum::body::Body::empty())
        .unwrap();
    let res_unknown = app.clone().oneshot(req_unknown_api).await.unwrap();
    assert_eq!(res_unknown.status(), StatusCode::NOT_FOUND);
    let ct = res_unknown
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(ct.contains("application/json"));
}
