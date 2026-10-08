# Ticket 32-2: Hermes & OpenClaw One-Click Installation & CLI Integration

## Context & Goal
Enable users to configure `k0maru-memory` into Hermes Agent and OpenClaw via `k0maru install --target hermes` or `k0maru install --target openclaw` (or `k0maru install --target all`). Ensure atomic, non-destructive JSON merges, directory creation, dry-run previews, and comprehensive CLI tests.

## Blocked by
32-1

## Scope of Work
1. **CLI `InstallArgs` update (`src/main.rs`)**:
   - Update docstring for `--target` to list `all, claude, cursor, gemini, windsurf, cline, hermes, openclaw`;
2. **Installation Pipeline Validation**:
   - Ensure `run_install()` correctly creates `.hermes/` and `.openclaw/` directories and config files when targeted;
   - Verify `--dry-run` returns previews without touching disk;
   - Ensure existing custom keys in `~/.hermes/mcp.json` or `~/.openclaw/config.json` are retained.
3. **Integration Tests (`tests/test_install.rs`)**:
   - Add test case: installing to sandboxed Hermes home directory (`--target hermes`);
   - Add test case: installing to sandboxed OpenClaw home directory (`--target openclaw`);
   - Add test case: `run_install` with `InstallTarget::All` configures all 7 clients;
   - Add CLI test via `assert_cmd` executing `k0maru install --target hermes --dry-run --json`.

## Acceptance Criteria
- [x] `k0maru install --target hermes` injects `k0maru-memory` into `~/.hermes/mcp.json`;
- [x] `k0maru install --target openclaw` injects `k0maru-memory` into `~/.openclaw/config.json`;
- [x] Dry-run preview displays correctly;
- [x] All tests pass with zero warnings (`cargo clippy --all-targets -- -D warnings`).
