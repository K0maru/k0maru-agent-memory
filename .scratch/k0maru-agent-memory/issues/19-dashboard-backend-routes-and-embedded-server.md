# 19 — Dashboard Backend Routes and Embedded HTTP Server

**What to build:**
Implement the embedded HTTP server and core REST JSON API in `src/ui/` exposed via the CLI command `k0maru ui [--vault <path>] [--port <port>] [--open]`.
The server strictly binds to `127.0.0.1:<port>` (default port 3721) and gracefully shuts down upon receiving `Ctrl+C`.

**Blocked by:** None — can start immediately.

**Status:** resolved

- [x] Add `axum` (or lightweight Tokio HTTP server) and `tower-http` to `Cargo.toml`.
- [x] Create `src/ui/mod.rs`, `src/ui/server.rs`, `src/ui/routes.rs`.
- [x] Implement `GET /api/status`: returns JSON with vault path, total documents, vector count, cache size bytes, last sync time, and total offload log nodes count.
- [x] Implement `GET /api/search?q=<query>&mode=<hybrid|bm25|vector>&limit=<N>`: invokes `HybridSearchEngine` and returns JSON array of `SearchResult` with score breakdowns.
- [x] Implement `GET /api/graph`: queries `SqliteStorage` and returns `{ nodes: [...], edges: [...] }` with note paths, titles, hierarchy levels, and directional links.
- [x] Implement `GET /api/logs`: lists summary of all offloaded log nodes in `.scratch/refs/` (or `<vault>/.k0maru/refs/`).
- [x] Implement `GET /api/logs/:node_id`: returns JSON with node ID, Mermaid flowchart code, and full truncated log slice.
- [x] Implement `POST /api/sync`: triggers `sync_vault_with_vector` and returns updated `SyncStats` and `VectorSyncStats`.
- [x] Add `Ui` variant to `Commands` in `src/main.rs`: `k0maru ui [--vault <path>] [--port 3721] [--open]`.
- [x] Create `tests/test_ui_server.rs` testing all API routes and graceful error handling on loopback.

## Implementation Notes & Verification
- Embedded HTTP engine using `axum 0.7`, `tokio 1.37`, `tower`, and `tower-http` (CORS support).
- Implemented `src/ui/routes.rs`:
  - `GET /api/status`: returns vault path, document count, vector count, cache size bytes, last sync time, and total offload log count.
  - `GET /api/search`: supports hybrid, bm25, and vector retrieval modes with limits.
  - `GET /api/graph`: returns full nodes (path, title, hierarchy, tags) and edges (source, target links).
  - `GET /api/logs`: scans vault and scratch ref directories, returning log summaries (id, line_count, task_id, ISO8601 created_at).
  - `GET /api/logs/:id`: inspects log nodes and formats dynamic Mermaid state machine diagrams + raw content with 404 handling.
  - `POST /api/sync`: triggers incremental vault scan and vector embedding update.
  - Root fallback: `<h1>K0maru Dashboard API Active</h1>`.
- Implemented `src/ui/server.rs`: binds to `127.0.0.1:{port}` with graceful `tokio::signal::ctrl_c()` shutdown and optional browser launch.
- Added `Ui` command variant to CLI `src/main.rs`.
- Comprehensive test coverage in `tests/test_ui_server.rs` (9 integration tests passing, including status, sync, hybrid/bm25/vector search, graph, logs listing/inspection, loopback TCP connection, port collision detection, and CLI help).
- All 129 workspace tests pass, zero clippy warnings (`cargo clippy --all-targets -- -D warnings`), formatted with `cargo fmt`.
