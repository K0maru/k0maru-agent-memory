# 28 — Install Command & MCP Client Configurator

**Type:** task  
**Status:** resolved  
**Blocked by:** 27  

## Context
Provide developers with a frictionless one-click command (`k0maru install`) that automatically injects the `k0maru-memory` FastMCP stdio server definition into installed agent and IDE configurations (Claude Code, Cursor, Antigravity/Gemini CLI, Windsurf, Cline), eliminating manual JSON editing.

## Objectives & Deliverables
1. **Configurator Engine (`src/install/mod.rs`)**:
   - Resolve canonical vault path (defaults to detected/current vault or explicit `--vault`).
   - Support selective targeting via `--target <all|claude|cursor|gemini|windsurf|cline>`.
   - Safe atomic JSON merging:
     - Safely parses existing JSON without clobbering other MCP servers or custom keys.
     - Creates parent directories and default `{ "mcpServers": {} }` if file does not exist.
     - Injects or updates `"k0maru-memory"` server entry with executable and resolved `--vault` argument.
   - Dry-run support (`--dry-run`): previews planned changes and diffs without writing to disk.
   - Atomic write via temporary file replacement or direct safe write.
2. **CLI Integration (`src/main.rs`)**:
   - Add `Install` subcommand to `Commands`:
     ```bash
     k0maru install [--vault <path>] [--target <all|claude|cursor|gemini|windsurf|cline>] [--dry-run]
     ```
   - Terminal output: Clear step-by-step reporting with `✓` confirmation for each configured client, and next-steps guidance.
3. **Tests**:
   - Unit tests for JSON merging (ensuring existing servers like `fetch`, `postgres` are preserved untouched).
   - Integration test for `k0maru install` with `--dry-run` and mock temporary home/config directories.
4. **Acceptance Criteria**:
   - Existing configs retain all keys and other servers.
   - New configs are properly created with valid JSON.
   - `--dry-run` performs zero disk modifications.
   - 100% test pass rate, zero warnings on `cargo clippy`, zero formatting issues on `cargo fmt`.

## Implementation Summary & Verification
- Created `src/install/mod.rs` with `InstallTarget`, `InstallOptions`, `ClientInstallOutcome`, `InstallReport`, `inject_k0maru_mcp`, `run_install`, and `format_install_report`.
- Integrated `k0maru install` subcommand in `src/main.rs` with `--vault`, `--target`, `--dry-run`, and `--json` support.
- Added 15 unit and CLI integration tests in `tests/test_install.rs`.
- All tests pass (179 passing tests across test suite).
- `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` pass with zero errors.

