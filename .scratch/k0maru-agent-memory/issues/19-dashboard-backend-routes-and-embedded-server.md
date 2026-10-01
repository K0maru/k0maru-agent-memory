# 19 — Dashboard Backend Routes and Embedded HTTP Server

**What to build:**
Implement the embedded HTTP server and core REST JSON API in `src/ui/` exposed via the CLI command `k0maru ui [--vault <path>] [--port <port>] [--open]`.
The server strictly binds to `127.0.0.1:<port>` (default port 3721) and gracefully shuts down upon receiving `Ctrl+C`.

**Blocked by:** None — can start immediately.

**Status:** ready-for-agent

- [ ] Add `axum` (or lightweight Tokio HTTP server) and `tower-http` to `Cargo.toml`.
- [ ] Create `src/ui/mod.rs`, `src/ui/server.rs`, `src/ui/routes.rs`.
- [ ] Implement `GET /api/status`: returns JSON with vault path, total documents, vector count, cache size bytes, last sync time, and total offload log nodes count.
- [ ] Implement `GET /api/search?q=<query>&mode=<hybrid|bm25|vector>&limit=<N>`: invokes `HybridSearchEngine` and returns JSON array of `SearchResult` with score breakdowns.
- [ ] Implement `GET /api/graph`: queries `SqliteStorage` and returns `{ nodes: [...], edges: [...] }` with note paths, titles, hierarchy levels, and directional links.
- [ ] Implement `GET /api/logs`: lists summary of all offloaded log nodes in `.scratch/refs/` (or `<vault>/.k0maru/refs/`).
- [ ] Implement `GET /api/logs/:node_id`: returns JSON with node ID, Mermaid flowchart code, and full truncated log slice.
- [ ] Implement `POST /api/sync`: triggers `sync_vault_with_vector` and returns updated `SyncStats` and `VectorSyncStats`.
- [ ] Add `Ui` variant to `Commands` in `src/main.rs`: `k0maru ui [--vault <path>] [--port 3721] [--open]`.
- [ ] Create `tests/test_ui_server.rs` testing all API routes and graceful error handling on loopback.
