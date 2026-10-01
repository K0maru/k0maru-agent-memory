use axum::body::to_bytes;
use axum::http::header::CONTENT_TYPE;
use axum::http::{Request, StatusCode};
use tempfile::tempdir;
use tower::ServiceExt;

use k0maru::storage::SqliteStorage;
use k0maru::ui::routes::{create_router, AppState, DashboardAssets, StatusResponse};

#[test]
fn test_embedded_assets_bundle_integrity() {
    assert!(DashboardAssets::get("index.html").is_some());
    assert!(DashboardAssets::get("style.css").is_some());
    assert!(DashboardAssets::get("app.js").is_some());

    let index_file = DashboardAssets::get("index.html").unwrap();
    let index_html = std::str::from_utf8(&index_file.data).unwrap();
    assert!(index_html.contains("K0maru Agent Memory Hub"));
    assert!(index_html.contains("Search Debugger"));
    assert!(index_html.contains("Graph Explorer"));
    assert!(index_html.contains("Log Inspector"));
    assert!(index_html.contains("Token Scoreboard"));

    let style_file = DashboardAssets::get("style.css").unwrap();
    let style_css = std::str::from_utf8(&style_file.data).unwrap();
    assert!(style_css.contains("#0F172A"));
    assert!(style_css.contains("#1B2336"));
    assert!(style_css.contains("#22C55E"));
}

#[tokio::test]
async fn test_ui_static_embed_root() {
    let temp = tempdir().unwrap();
    let vault_path = temp.path().to_path_buf();
    let storage = SqliteStorage::in_memory().unwrap();
    let state = AppState::from_storage(vault_path, storage, None);
    let app = create_router(state);

    let req = Request::builder()
        .uri("/")
        .body(axum::body::Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let content_type = res
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        content_type.starts_with("text/html"),
        "Content-Type should be text/html, got {}",
        content_type
    );

    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let html = String::from_utf8_lossy(&body);
    assert!(
        html.contains("K0maru"),
        "Root index.html must contain brand name K0maru"
    );
    assert!(
        html.contains("K0maru Agent Memory Hub"),
        "Root index.html must contain header title"
    );
}

#[tokio::test]
async fn test_ui_static_embed_css() {
    let temp = tempdir().unwrap();
    let vault_path = temp.path().to_path_buf();
    let storage = SqliteStorage::in_memory().unwrap();
    let state = AppState::from_storage(vault_path, storage, None);
    let app = create_router(state);

    let req = Request::builder()
        .uri("/style.css")
        .body(axum::body::Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let content_type = res
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        content_type.starts_with("text/css"),
        "Content-Type should be text/css, got {}",
        content_type
    );

    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let css = String::from_utf8_lossy(&body);
    assert!(
        css.contains("#0F172A"),
        "CSS must contain dark cockpit background color #0F172A"
    );
    assert!(
        css.contains("#1B2336"),
        "CSS must contain dark cockpit card surface #1B2336"
    );
    assert!(
        css.contains("#22C55E"),
        "CSS must contain emerald accent color #22C55E"
    );
}

#[tokio::test]
async fn test_ui_static_embed_js() {
    let temp = tempdir().unwrap();
    let vault_path = temp.path().to_path_buf();
    let storage = SqliteStorage::in_memory().unwrap();
    let state = AppState::from_storage(vault_path, storage, None);
    let app = create_router(state);

    let req = Request::builder()
        .uri("/app.js")
        .body(axum::body::Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let content_type = res
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        content_type.contains("javascript"),
        "Content-Type should be javascript MIME, got {}",
        content_type
    );

    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let js = String::from_utf8_lossy(&body);
    assert!(
        js.contains("/api/status"),
        "app.js must contain status endpoint polling"
    );
    assert!(
        js.contains("/api/sync"),
        "app.js must contain sync endpoint call"
    );
}

#[tokio::test]
async fn test_ui_static_embed_spa_fallback() {
    let temp = tempdir().unwrap();
    let vault_path = temp.path().to_path_buf();
    let storage = SqliteStorage::in_memory().unwrap();
    let state = AppState::from_storage(vault_path, storage, None);
    let app = create_router(state);

    // Any client-side SPA route should serve index.html with text/html
    for spa_route in &[
        "/search",
        "/graph",
        "/logs",
        "/scoreboard",
        "/explorer/deep",
    ] {
        let req = Request::builder()
            .uri(*spa_route)
            .body(axum::body::Body::empty())
            .unwrap();

        let res = app.clone().oneshot(req).await.unwrap();
        assert_eq!(
            res.status(),
            StatusCode::OK,
            "Route {} should return 200 OK",
            spa_route
        );

        let content_type = res
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(
            content_type.starts_with("text/html"),
            "Route {} should have text/html Content-Type, got {}",
            spa_route,
            content_type
        );

        let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let html = String::from_utf8_lossy(&body);
        assert!(
            html.contains("K0maru Agent Memory Hub"),
            "Route {} should return index.html shell",
            spa_route
        );
    }
}

#[tokio::test]
async fn test_ui_api_status_not_swallowed_by_static_handler() {
    let temp = tempdir().unwrap();
    let vault_path = temp.path().to_path_buf();
    let storage = SqliteStorage::in_memory().unwrap();
    let state = AppState::from_storage(vault_path, storage, None);
    let app = create_router(state);

    let req = Request::builder()
        .uri("/api/status")
        .body(axum::body::Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let content_type = res
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        content_type.starts_with("application/json"),
        "Content-Type for /api/status should be application/json, got {}",
        content_type
    );

    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let status: StatusResponse = serde_json::from_slice(&body).expect("Valid StatusResponse JSON");
    assert_eq!(status.total_documents, 0);
    assert_eq!(status.total_vectors, 0);
}

#[tokio::test]
async fn test_ui_unknown_api_returns_404_json_not_html() {
    let temp = tempdir().unwrap();
    let vault_path = temp.path().to_path_buf();
    let storage = SqliteStorage::in_memory().unwrap();
    let state = AppState::from_storage(vault_path, storage, None);
    let app = create_router(state);

    let req = Request::builder()
        .uri("/api/unknown_endpoint")
        .body(axum::body::Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    let content_type = res
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        content_type.starts_with("application/json"),
        "Unknown /api route should return JSON 404, not HTML"
    );
}
