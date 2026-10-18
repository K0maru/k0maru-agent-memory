# Ticket 42-2: Add `--sidecar` Mode to `McpServer` and Clean Up `guard_check` Schema

- **Status**: `done`
- **Blocked by**: None (Parallel Slice 2)

## Target Files
- Edit `src/mcp/server.rs`

## Acceptance Criteria
- [x] `McpServer` gains `sidecar: bool` field (default `false`), `pub fn with_sidecar(mut self, sidecar: bool) -> Self`, and `pub fn is_sidecar(&self) -> bool`.
- [x] When `sidecar == false` (default), `handle_tools_list` returns all 8 tools (`get_project_loadout`, `recall_memory`, `offload_context`, `inspect_log_node`, `flush_session`, `distill_session_skill`, `gc_session_context`, `guard_check`).
- [x] When `sidecar == true`, `handle_tools_list` omits `offload_context`, `inspect_log_node`, and `gc_session_context`, returning only the 5 orthogonal sidecar tools (`get_project_loadout`, `recall_memory`, `flush_session`, `distill_session_skill`, `guard_check`).
- [x] `kdsl` parameter is removed from `guard_check` schema and `call_guard_check` in `src/mcp/server.rs`.
- [x] Unit tests in `src/mcp/server.rs` verify both default (8 tools) and sidecar (5 tools) modes.
