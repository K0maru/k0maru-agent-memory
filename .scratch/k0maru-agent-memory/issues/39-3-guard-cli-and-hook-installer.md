# Issue 39-3: Guard CLI Commands and Pre-Commit Hook Installer

**Status**: Completed
**Blocked by**: 39-2

## Goal
Implement CLI subcommands under `k0maru guard` in `src/guard/` and `src/main.rs`:
- `k0maru guard check [--vault <path>] [--diff] [--project <name>] [--json] [--strict]`
- `k0maru guard install [--hook pre-commit] [--path <repo_dir>]`

## Acceptance Criteria
1. Add `Guard` subcommand to CLI clap arguments in `src/main.rs`.
2. `k0maru guard check`:
   - Inspects git diff in current repository using `GitInspector`.
   - Resolves active project invariants using `LoadoutBuilder` or vault scanner.
   - Executes `GuardEngine::check_diff`.
   - Exits code 0 on pass, code 1 on deny violations, code 2 on error.
   - Formats human-friendly terminal report with colored warning/error boxes and remediation hints, or `--json` output.
3. `k0maru guard install`:
   - Safely creates or chains into `.git/hooks/pre-commit`.
   - Makes hook file executable (`0o755`).
4. Unit and CLI integration tests using `assert_cmd` and `tempfile`.
