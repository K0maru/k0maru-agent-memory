# Ticket 31-3: CLI `k0maru distill` Command and FastMCP `distill_session_skill` Tool

## Context & Goal
Expose the Trace-to-Skill capability via the public interfaces: the `k0maru distill` CLI command (supporting stdin pipe, `--node`, and `--file`) and the FastMCP `distill_session_skill` tool for seamless invocation from AI coding agents (Claude Code, Cursor, Windsurf).

## Blocked by
31-2

## Scope of Work
1. **CLI `k0maru distill` (`src/main.rs`)**:
   - Add `Distill(DistillArgs)` subcommand to `Cli` enum;
   - Fields:
     - `vault: Option<PathBuf>`
     - `node: Option<String>`
     - `file: Option<PathBuf>`
     - `title: Option<String>`
     - `context: Option<String>`
     - `category: Option<String>` (default "skill")
     - `tags: Option<String>` (comma separated)
     - `related: Option<String>` (comma separated)
     - `dry_run: bool`
     - `json: bool`
   - Handle stdin when neither `--node` nor `--file` is specified (with terminal check);
   - Human-readable output formatting with color / icons, plus `--json` output.
2. **FastMCP Server Registration (`src/mcp/server.rs`)**:
   - Register `distill_session_skill` tool in `handle_tools_list`:
     - Parameters: `raw_trace`, `node_id`, `title`, `context_hint`, `category`, `tags`, `related_notes`, `dry_run`;
   - Implement `call_distill_session_skill`:
     - Dispatch to `DistillEngine`;
     - Return markdown summary with file path and status.
3. **Integration & End-to-End Tests**:
   - CLI command execution test via `assert_cmd` piping raw log into `k0maru distill`;
   - FastMCP JSON-RPC integration test verifying `tools/call` for `distill_session_skill`;
   - End-to-end test verifying distilled skill can be immediately recalled via `recall_memory`.
4. **Code Quality & Hygiene**:
   - `cargo test --all-targets` all green;
   - `cargo clippy --all-targets -- -D warnings` zero warnings;
   - `cargo fmt --check` zero diffs.

## Acceptance Criteria
- [ ] `k0maru distill` CLI functions with stdin, `--node`, and `--file`;
- [ ] FastMCP `distill_session_skill` callable over JSON-RPC 2.0;
- [ ] Immediate recallability verified (write -> auto-sync -> recall);
- [ ] 100% test pass rate and clean lints.
