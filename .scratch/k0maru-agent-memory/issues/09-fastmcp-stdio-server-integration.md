# 09 — FastMCP stdio server and agent tool endpoints

**What to build:** A zero-daemon MCP server in `src/mcp/server.rs` running over `stdio` using JSON-RPC 2.0, implementing four lean tools: `get_project_loadout`, `recall_memory`, `offload_context`, and `inspect_log_node`, making K0maru-Agent-Memory natively callable by Claude Code, Cursor, and Windsurf.

**Blocked by:** 07 — Sub-300-token context loadout builder and CLI command, 08 — Symbolic log offloader pipe and inspect CLI command

**Status:** resolved

- [x] `McpServer` handles JSON-RPC 2.0 lines received via `std::io::stdin` and writes responses to `std::io::stdout`
- [x] Exposes `get_project_loadout(project_name: String) -> String` invoking `LoadoutBuilder`
- [x] Exposes `recall_memory(query: String, limit: usize) -> String` querying SQLite FTS5 BM25 and returning top formatted matches
- [x] Exposes `offload_context(raw_text: String, task_name: String) -> String` returning truncated Mermaid state diagram
- [x] Exposes `inspect_log_node(node_id: String) -> String` retrieving stored offload logs
- [x] CLI command `k0maru mcp` boots the stdio server on demand with 0 persistent network ports
- [x] `tests/test_mcp_server.rs` tests tool registration, valid JSON-RPC tool invocation, and error handling

---

### Completion Summary

1. **FastMCP Server Architecture (`src/mcp/`)**:
   - Implemented `McpServer` in `src/mcp/server.rs` and exported via `src/mcp/mod.rs` and `src/lib.rs`.
   - Provides `run_stdio<R: BufRead, W: Write>` for robust, zero-daemon stream communication over standard I/O (with broken-pipe tolerance).
   - Provides `handle_line(&mut self, line: &str) -> Option<String>` for line-by-line deterministic black-box testing.
   - Fully compliant with JSON-RPC 2.0 and Model Context Protocol (MCP) specifications:
     - `initialize`: Returns serverInfo (`k0maru v0.1.0`), protocolVersion (`2024-11-05`), and tools capability.
     - `notifications/initialized` and `initialized`: Silently acknowledged (`None` returned).
     - Standard JSON-RPC error handling: MethodNotFound (-32601), ParseError (-32700), InvalidRequest (-32600).
2. **Four Atomic MCP Tools**:
   - `get_project_loadout`: Adapts to Obsidian / Generic Wiki, synchronizes incremental cache, and runs `LoadoutBuilder` to produce sub-300-token task pack.
   - `recall_memory`: Synchronizes incremental cache, executes SQLite FTS5 BM25 query, and returns formatted markdown cards and notes.
   - `offload_context`: Uses `OffloadEngine` to truncate long command logs (>50 lines) to disk reference and returns compact Mermaid state machine diagram.
   - `inspect_log_node`: Uses `inspect_node` to retrieve original offloaded raw logs by node ID.
3. **CLI Subcommand (`src/main.rs`)**:
   - Added `k0maru mcp [--vault <PATH>]` subcommand.
   - Listens on `std::io::stdin()` and outputs to `std::io::stdout()`. 0 persistent network ports, 0 daemons.
4. **Verification & Quality Baseline**:
   - 11 comprehensive tests in `tests/test_mcp_server.rs` covering initialization, notifications, tool listings, schema validation, all four tool executions, error codes, and subprocess CLI stdio communication.
   - Total test suite: 73 tests passing 100% (`cargo test`).
   - `cargo clippy --all-targets -- -D warnings`: 0 warnings.
   - `cargo fmt --check`: 100% formatted.
