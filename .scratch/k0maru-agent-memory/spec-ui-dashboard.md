# Specification: K0maru On-Demand Developer Dashboard (`k0maru ui`)

## Problem Statement

While K0maru-Agent-Memory delivers high-performance hybrid retrieval, symbolic log offloading, and standard FastMCP memory management in a single static binary, developers and agent operators currently interact with it exclusively via CLI commands or headless MCP calls. This creates three major friction points:

1. **Retriever Black-Box Ambiguity**: When an agent recalls a memory or performs a search, the human developer cannot easily see *why* note A scored higher than note B, how BM25 ranking fused with cosine vector distance, or whether a note received a +0.05 1-hop WikiLinks Graph Boost.
2. **Offload Trace Fragmentation**: When long terminal logs are truncated into `node_xxx`, viewing the Mermaid state chart and debugging the collapsible stack trace requires manual CLI inspection (`k0maru inspect <node_id>`), lacking visual ergonomics.
3. **Knowledge Graph Blindness**: Developers lack a live visual representation of their note network, semantic hierarchy tiers (L0 Daily, L1 Resource, L2 Log, L3 Evergreen Card, Project), and disconnected orphan pages.
4. **Daemon Creep Risk**: Existing toolchains often require heavy Docker containers, external PostgreSQL instances, or persistent daemon background processes that consume 100MB+ RAM and monopolize network ports permanently.

## Solution

Implement `k0maru ui` — an **on-demand, zero-daemon, embedded local developer dashboard** compiled directly into the single `k0maru` binary:

- **On-Demand Lifecycle**: Launched explicitly via `k0maru ui [--vault <path>] [--port 3721] [--open]`. Exits completely on `Ctrl+C`, instantly releasing all memory and ports. No background daemons.
- **Single-Static-Binary Packaging**: The web dashboard frontend (HTML/CSS/JS/assets) is compiled into the Rust binary at compile time using `rust-embed`. End users require zero Node.js, Python, or npm dependencies.
- **Developer Dark Cockpit Aesthetic**: Adheres strictly to `ui-ux-pro-max` and `design-taste-frontend` design systems (OLED Slate `#0F172A`, High-contrast `#F8FAFC`, Emerald Run Accent `#22C55E`, JetBrains Mono for metrics, Lucide SVG icons, zero generic AI slop).
- **Four Atomic Cockpit Panels**:
  1. **Hybrid Search Debugger & Explainability**: Interactive query playground with real-time scoring waterfall (BM25 rank, vector distance, RRF fusion, WikiLinks graph boost badge).
  2. **WikiLinks Interactive Graph Explorer**: 2D force-directed knowledge graph with L0–L3 hierarchy color coding, neighbor focusing, and orphan discovery.
  3. **Log & Trace Inspector**: Visual Mermaid execution chart renderer + collapsible syntax-highlighted error stack viewer for offloaded nodes.
  4. **Cache Health & Token Scoreboard**: Real-time stats on documents, vector indexing coverage, cache size, incremental sync latency, and cumulative Token Reduction Ratio (TRR).

## User Stories

1. As a developer, I want to run `k0maru ui` from my terminal, so that I can inspect my agent's memory hub in a modern web dashboard without configuring any web servers or background services.
2. As a developer, I want `k0maru ui --open` to automatically open my default browser to `http://localhost:3721`, so that I have zero-click startup ergonomics.
3. As a developer, I want `k0maru ui` to cleanly shut down when I press `Ctrl+C`, so that no orphaned background daemon or occupied port is left behind.
4. As a developer, I want to type queries into the Search Debugger and view side-by-side results for Hybrid, BM25, and Vector modes, so that I can calibrate retrieval behavior.
5. As a developer, I want to see the detailed scoring breakdown for each search result (BM25 rank, Vector cosine distance, RRF score, and Graph Boost), so that retrieval ranking is 100% explainable.
6. As a developer, I want to see a "+0.05 Graph Boost" visual badge whenever a search result is boosted by 1-hop WikiLinks connections, so that I can verify graph routing effectiveness.
7. As a developer, I want to explore an interactive 2D force-directed graph of all notes and backlinks in my vault, so that I can explore knowledge clusters and topology.
8. As a developer, I want graph nodes to be distinctly colored according to their semantic hierarchy (L0 Daily, L1 Resource, L2 Log, L3 Evergreen Card, Project), so that architecture is visually legible.
9. As a developer, I want to click on any node in the graph to view its metadata, tags, out-links, and backlinks in an inspector side-sheet.
10. As a developer, I want to view a list of all offloaded log nodes (`node_xxx`) stored in `.scratch/refs/`, so that I can review recent command execution failures.
11. As a developer, I want the dashboard to natively render the Mermaid diagram of any offloaded log node, so that I can quickly assess the pipeline state machine.
12. As a developer, I want to expand and inspect the full raw truncated log slice inside a collapsible viewer with monospace font, so that I can diagnose compiler and runtime errors without leaving the dashboard.
13. As a developer, I want to see a live Token Scoreboard displaying the estimated tokens saved by log offloading and the cumulative Token Reduction Ratio (TRR %), so that I can quantify context savings.
14. As a developer, I want to click a "Sync Vault" button in the dashboard, so that I can trigger an incremental file scan and vector reindex on demand without restarting the UI.
15. As a security-conscious user, I want the embedded web server to strictly bind to `127.0.0.1` (loopback only) by default, so that my local notes are never exposed over the local network.

## Implementation Decisions

1. **Embedded Web Server Engine**:
   - Use `axum` (or `tiny_http` / lightweight Tokio-based HTTP server) compiled into the binary.
   - Bind strictly to `127.0.0.1:<port>` (default port `3721`).
   - Graceful shutdown on `tokio::signal::ctrl_c()`.

2. **Single Static Binary Asset Embedding**:
   - Use `rust-embed` crate to pack the built frontend assets into the binary.
   - Fallback route: serve `index.html` for single-page app client-side routing.
   - Correct MIME type deduction for `.html`, `.css`, `.js`, `.svg`, `.json`, `.woff2`.

3. **Backend REST API Endpoints**:
   - `GET /api/status`: Returns vault path, document count, vector count, cache file size bytes, last sync timestamp, and total offload logs count.
   - `GET /api/search?q=<query>&mode=<hybrid|bm25|vector>&limit=<N>`: Executes search via `HybridSearchEngine` and returns results with score breakdowns.
   - `GET /api/graph`: Returns `{ nodes: [...], edges: [...] }` representing documents (with hierarchy tags) and links.
   - `GET /api/logs`: Returns summary list of offloaded log nodes (`node_id`, timestamp, line count, truncated line count, status).
   - `GET /api/logs/:id`: Returns full raw log content and generated Mermaid diagram.
   - `POST /api/sync`: Triggers `sync_vault_with_vector` and returns updated `SyncStats` and `VectorSyncStats`.

4. **Frontend Architecture & Design Standards**:
   - Single-page application built with lightweight web primitives or Vite.
   - Adhere strictly to the **Developer Dark Cockpit** design system from `ui-ux-pro-max`:
     - Background: Deep Slate `#0F172A`
     - Card / Surface: Elevated Slate `#1B2336`
     - Border: Slate `#334155`
     - Accent: Emerald `#22C55E`
     - Monospace: `JetBrains Mono` for hashes, line numbers, scores, and tokens
     - Sans-serif: `IBM Plex Sans` / `Inter` / `Geist`
     - Graph rendering: Canvas-based Force Graph or Cytoscape.js
     - Diagram rendering: Client-side `mermaid.js` SVG renderer
     - SVG Icons: Lucide SVG (zero emojis for functional UI elements).

5. **Resource and Dependency Isolation**:
   - The UI module will reside under `src/ui/`.
   - Feature-gating: provide feature flag `dashboard` (or default on) so headless/embedded builds can exclude web assets if minimal size (<4MB) is strictly required.

## Testing Decisions

- **Backend Route Testing**:
  - Integration tests using `axum::test` or `reqwest` on mock loopback server with mock vault fixtures.
  - Verify `GET /api/status`, `GET /api/search`, `GET /api/graph`, `GET /api/logs`, `POST /api/sync`.
  - Verify loopback binding and static asset serving.
- **CLI Smoke Testing**:
  - Test `k0maru ui --help` output.
  - Test graceful error handling when port is already occupied.
- **Zero-Network Isolation**:
  - All tests must run hermetically on `127.0.0.1` without external internet access.

## Out of Scope

- Multi-user authentication or role-based access control (this is an on-demand personal local developer tool).
- Remote network exposure / cloud hosting (strictly local loopback).
- WYSIWYG note editing inside the dashboard (editing remains in Obsidian / VS Code / editor of choice).
- Long-running background daemon mode.

## Further Notes

- By compiling frontend assets into the binary, we maintain the project's core "Single Static Binary" promise.
- The dashboard is purely read-only and diagnostic, with the sole active operation being on-demand cache re-syncing.
