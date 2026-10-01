# 27 — Doctor Command & Health Diagnostics

**Type:** task  
**Status:** resolved  
**Blocked by:** none  

## Context
Provide developers and agents with a single diagnostic command (`k0maru doctor`) to verify binary integrity, vault health, SQLite FTS5 / vector index status, and MCP client configurations across the local development ecosystem.

## Objectives & Deliverables
1. **Ecosystem & Diagnostics Domain (`src/ecosystem/mod.rs` & `src/doctor/mod.rs`)**:
   - Model `McpClient` enum (`Claude`, `Cursor`, `Gemini`, `Windsurf`, `Cline`).
   - Detect client config file paths across macOS, Linux, and Windows.
   - Parse client configs to check if `k0maru-memory` is mounted, and extract the configured vault path.
   - Inspect binary health: `current_exe()`, version, `$PATH` availability.
   - Inspect target vault health: directory existence, markdown file count, hierarchy distribution.
   - Inspect cache & search engine health: SQLite cache file size, FTS5 operational status, `sqlite-vec` virtual table count and coverage %, embedding model backend status.
   - Produce a structured `DoctorReport` domain model.
2. **CLI Integration (`src/main.rs`)**:
   - Add `Doctor` subcommand to `Commands`:
     ```bash
     k0maru doctor [--vault <path>] [--json]
     ```
   - Terminal output: Beautiful colorized diagnostic sections with `✓ PASS`, `⚠ WARN`, `✗ FAIL`, plus summary counts.
   - `--json` mode: Outputs serialized `DoctorReport` for machine consumption.
3. **Tests**:
   - Unit tests for ecosystem path detection and status checking.
   - Integration test for `k0maru doctor` and `k0maru doctor --json` against a mock vault fixture.
4. **Acceptance Criteria**:
   - `k0maru doctor` executes cleanly without panics.
   - `--json` outputs valid JSON matching `DoctorReport`.
   - All tests pass, zero warnings on `cargo clippy`, zero formatting issues on `cargo fmt`.

## Implementation & Verification Summary
- **Ecosystem Domain (`src/ecosystem/mod.rs`)**:
  - Implemented `McpClient` (`Claude`, `Cursor`, `Gemini`, `Windsurf`, `Cline`) with cross-platform config path resolution and custom home support.
  - Implemented `ClientConfigStatus` and `inspect_client` / `inspect_all_clients` to discover installed clients, detect `k0maru-memory` or `k0maru` configs, and parse `--vault` args.
- **Doctor Diagnostic Domain (`src/doctor/mod.rs`)**:
  - Implemented `StatusLevel`, `DiagnosticItem`, `DoctorSummary`, and `DoctorReport`.
  - Implemented probes: `probe_binary()`, `probe_vault()`, `probe_storage()`, and `probe_ecosystem()`.
  - Implemented human-readable colorized terminal formatter `format_report()` grouped by category with summary counts.
- **CLI Subcommand (`src/main.rs`)**:
  - Added `Doctor(DoctorArgs)` with `--vault` and `--json` support.
  - Returns exit code 1 if failures > 0.
- **Test Suite (`tests/test_doctor.rs`)**:
  - 19 new tests covering ecosystem detection, all 4 probes, report formatting, CLI smoke, and JSON schema validation.
  - 100% passing across full test suite (`cargo test`), zero clippy warnings (`cargo clippy --all-targets -- -D warnings`), zero formatting issues (`cargo fmt --check`).

