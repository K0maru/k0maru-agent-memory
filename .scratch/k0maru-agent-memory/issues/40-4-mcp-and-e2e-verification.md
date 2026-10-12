# Ticket 40-4: FastMCP Integration & End-to-End Multi-Model Test Suite

**Status**: Completed
**Blocked by**: Ticket 40-1, Ticket 40-2, Ticket 40-3

## Goal
Wire the pluggable embedding engine into the FastMCP server, support model selection in MCP server initialization, and establish an end-to-end regression test suite verifying multi-model switching, migration resilience, and adversarial tests.

## Acceptance Criteria
1. FastMCP Server Update:
   - Update `McpServer::new` and CLI `mcp` runner to respect `--embed-model` or `K0MARU_EMBED_MODEL`.
   - Ensure `recall_memory` and `get_project_loadout` execute hybrid search and vector fallback with the selected model.
2. End-to-End Test Matrix in `tests/test_embed_e2e.rs`:
   - Initialize mock vault with 384d embedder; verify vectors stored.
   - Switch to 512d embedder (`bge-small-zh-v1.5` or 512d mock); verify auto-migration triggers without error and re-indexes all files.
   - Verify `recall_memory` tool via FastMCP returns accurate results after migration.
   - Verify invalid model string gracefully errors with list of valid models or falls back safely.
3. Code Quality:
   - `cargo test --all-targets` passes 100%.
   - `cargo clippy --all-targets -- -D warnings` has 0 warnings.
   - `cargo fmt --check` clean.
