# Issue 39-4: Guard MCP Tool and End-to-End Test Suite

**Status**: Completed
**Blocked by**: 39-3

## Goal
Expose `guard_check` as a native MCP tool in `src/mcp/server.rs` and build end-to-end integration tests validating AI agent self-auditing workflows.

## Acceptance Criteria
1. Add `guard_check` to FastMCP tool list in `src/mcp/server.rs`:
   - Arguments: `diff: Option<String>`, `project_name: Option<String>`.
   - Audits provided diff or automatically calls `GitInspector::inspect_diff`.
   - Returns structured `GuardReport`.
2. End-to-end integration tests:
   - MCP `guard_check` tool call correctly flags missing invariant in diff.
   - MCP `guard_check` passes clean diff with 0 violations.
   - CLI pre-commit hook blocks violation commit in temporary git repo.
3. `cargo test --all-targets` passes 100%.
4. `cargo clippy --all-targets -- -D warnings` clean.
