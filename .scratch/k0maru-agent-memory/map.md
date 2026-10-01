# Wayfinding Map: K0maru-Agent-Memory (Rust)

## Notes & Overview
Standalone, cleanroom, single-static-binary, zero-daemon agent memory hub mounting Obsidian SecondBrain and Karpathy LLM-Wiki. Built with **Rust (2021 edition)**, `cargo`, `clap` (derive), `rusqlite` (bundled SQLite with FTS5 BM25 + WikiLinks Graph Router), `pulldown-cmark`, `arboard`, and stdio JSON-RPC MCP.

## Decisions-so-far
- **Language & Binary**: Adopted **Route 1 (Rust)**. Single standalone executable `k0maru` with <5ms cold start, <15MB RAM, and zero runtime dependencies.
- **Architecture**: Four decoupled layers (Adapters, Incremental Cache, Hybrid Retrieval & Graph Router, Interfaces/MCP).
- **Storage Philosophy**: "Bring Your Own Wiki" + "Truth in Files, Performance in Cache". Markdown is the single source of truth; SQLite `cache.sqlite` is transient and disposable.
- **Vector Strategy**: Plan A — Native bundled SQLite FTS5 (BM25) and WikiLinks graph routing in Phase 1; vector interface seam reserved for subsequent phase to guarantee zero-build hurdles.
- **Log Offloading**: Persisted locally to `.scratch/refs/<task_id>_<node_id>.log` (or `<vault_root>/.k0maru/refs/`).
- **Hierarchy Mapping**: Prioritize Karpathy LLM-Wiki and Obsidian SecondBrain folder conventions natively; Frontmatter overrides supported; no bloated config engine needed.
- **MCP Scope**: Strict Occam's Razor — exactly 4 atomic tools (`get_project_loadout`, `recall_memory`, `offload_context`, `inspect_log_node`).
- **Agent Ergonomics**: Full `--json` support across all maintenance commands (`sync`, `doctor`, `search`), deterministic exit codes, non-interactive defaults.
- **Test Isolation**: All automated tests strictly use `tempfile::tempdir` mock vault fixtures; touching host SecondBrain vault in tests is strictly forbidden.

## Ticket DAG & Frontier
- [x] [01-scaffolding-and-toolchain.md](issues/01-scaffolding-and-toolchain.md) (Blocked by: None)
- [x] [02-core-domain-models-and-fixtures.md](issues/02-core-domain-models-and-fixtures.md) (Blocked by: 01)
- [x] [03-markdown-ast-and-wikilinks-parser.md](issues/03-markdown-ast-and-wikilinks-parser.md) (Blocked by: 02)
- [x] [04-obsidian-and-generic-wiki-adapters.md](issues/04-obsidian-and-generic-wiki-adapters.md) (Blocked by: 03)
- [x] [05-disposable-sqlite-cache-and-fts5.md](issues/05-disposable-sqlite-cache-and-fts5.md) (Blocked by: 02)
- [x] [06-incremental-scanner-and-sync-engine.md](issues/06-incremental-scanner-and-sync-engine.md) (Blocked by: 04, 05)
- [x] [07-loadout-builder-and-cli-command.md](issues/07-loadout-builder-and-cli-command.md) (Blocked by: 06)
- [x] [08-symbolic-log-offloader-and-inspect-cli.md](issues/08-symbolic-log-offloader-and-inspect-cli.md) (Blocked by: 01)
- [x] [09-fastmcp-stdio-server-integration.md](issues/09-fastmcp-stdio-server-integration.md) (Blocked by: 07, 08) ✅ **COMPLETED (v0.1.0)**
- [x] [10-benchmark-fixtures-and-token-evaluator.md](issues/10-benchmark-fixtures-and-token-evaluator.md) (Blocked by: None) ✅ **COMPLETED**
- [x] [11-benchmark-systems-and-latency-runner.md](issues/11-benchmark-systems-and-latency-runner.md) (Blocked by: 10) ✅ **COMPLETED**
- [x] [12-benchmark-chart-generator-and-ci-pipeline.md](issues/12-benchmark-chart-generator-and-ci-pipeline.md) (Blocked by: 10, 11) ✅ **COMPLETED (v0.2.0)**
- [x] [13-sqlite-vec-virtual-table-and-storage-layer.md](issues/13-sqlite-vec-virtual-table-and-storage-layer.md) (Blocked by: None) ✅ **COMPLETED**
- [x] [14-embedding-engine-trait-and-mock-provider.md](issues/14-embedding-engine-trait-and-mock-provider.md) (Blocked by: None) ✅ **COMPLETED**
- [x] [15-fastembed-local-onnx-backend.md](issues/15-fastembed-local-onnx-backend.md) (Blocked by: 14) ✅ **COMPLETED**
- [x] [16-incremental-vector-sync-pipeline.md](issues/16-incremental-vector-sync-pipeline.md) (Blocked by: 13, 14) ✅ **COMPLETED**
- [x] [17-reciprocal-rank-fusion-hybrid-search.md](issues/17-reciprocal-rank-fusion-hybrid-search.md) (Blocked by: 13, 14) ✅ **COMPLETED**
- [x] [18-cli-search-command-and-mcp-hybrid-recall.md](issues/18-cli-search-command-and-mcp-hybrid-recall.md) (Blocked by: 16, 17) ✅ **COMPLETED (v0.3.0)**
- [x] [19-dashboard-backend-routes-and-embedded-server.md](issues/19-dashboard-backend-routes-and-embedded-server.md) (Blocked by: None) ✅ **COMPLETED**
- [x] [20-dashboard-frontend-asset-pipeline-and-static-embed.md](issues/20-dashboard-frontend-asset-pipeline-and-static-embed.md) (Blocked by: 19) ✅ **COMPLETED**
- [x] [21-search-debugger-and-explainability-cockpit.md](issues/21-search-debugger-and-explainability-cockpit.md) (Blocked by: 20) ✅ **COMPLETED**
- [ ] [22-wikilinks-graph-explorer-and-hierarchy-visualizer.md](issues/22-wikilinks-graph-explorer-and-hierarchy-visualizer.md) (Blocked by: 20) 🎯 **CURRENT FRONTIER**
- [ ] [23-log-trace-inspector-and-token-scoreboard.md](issues/23-log-trace-inspector-and-token-scoreboard.md) (Blocked by: 21, 22)

