# Ticket 32-3: CLI `--home <PATH>` Parameter & Complete MCP Pipe Conformance

## Context & Goal
Solidify local sandbox protection by exposing `--home <PATH>` directly on `k0maru install` and `k0maru doctor` CLI commands, allowing users to sandbox testing without modifying `$HOME`. In addition, create a comprehensive protocol conformance test in `tests/test_mcp_conformance.rs` testing full `initialize`, `tools/list`, and `tools/call` for `recall_memory`, `flush_session`, and `distill_session_skill`.

## Blocked by
32-2

## Scope of Work
1. **CLI Arguments (`src/main.rs`)**:
   - Add `#[arg(long, value_name = "PATH")] pub home: Option<PathBuf>` to `InstallArgs` and `DoctorArgs`;
   - Pass `home` to `InstallOptions.home_override`;
   - Pass `home` to `run_diagnostics(vault_path, home.as_deref())`.
2. **MCP Conformance Test Suite (`tests/test_mcp_conformance.rs`)**:
   - Pipe simulation via stdin/stdout:
     - `initialize`: handshake and protocol version agreement;
     - `tools/list`: verify all 6 tools are registered with valid schemas;
     - `tools/call` (`flush_session`): write ADR/log note into mock vault;
     - `tools/call` (`recall_memory`): retrieve newly written note via hybrid search;
     - `tools/call` (`distill_session_skill`): distill trace into reusable skill;
     - Verify zero extra unformatted bytes on stdout (100% JSON-RPC 2.0 lines).

## Acceptance Criteria
- [x] `k0maru install --home /tmp/custom --target hermes` writes strictly to `/tmp/custom/.hermes/mcp.json`;
- [x] `k0maru doctor --home /tmp/custom` inspects client configurations relative to `/tmp/custom`;
- [x] `tests/test_mcp_conformance.rs` executes full roundtrip `tools/call` without error;
- [x] `cargo test` and `cargo clippy` 100% pass.
