# Ticket 32-1: Hermes & OpenClaw Ecosystem Domain Models, Detectors & Doctor Checks

## Context & Goal
Integrate Nous Research **Hermes Agent** and **OpenClaw** into the `k0maru::ecosystem` module. This provides native client detection, configuration path discovery (including `$HERMES_HOME` and `$OPENCLAW_HOME` environment overrides), installation status inspection, and automatic inclusion in `k0maru doctor` diagnostics.

## Blocked by
None

## Scope of Work
1. **Extend `McpClient` in `src/ecosystem/mod.rs`**:
   - Add `McpClient::Hermes` and `McpClient::OpenClaw`;
   - Implement `name()`:
     - `Hermes => "Hermes Agent"`
     - `OpenClaw => "OpenClaw"`
   - Update `McpClient::all()` to include both (total 7 clients);
   - Implement `config_path(&self, home_override: Option<&Path>)`:
     - `Hermes`: Check `$HERMES_HOME` if set, else `$HOME/.hermes/mcp.json`. If `$HOME/.hermes/config.json` exists and `mcp.json` doesn't, use `config.json`.
     - `OpenClaw`: Check `$OPENCLAW_HOME` if set, else `$HOME/.openclaw/config.json`. If `$HOME/.openclaw/mcp.json` exists, use `mcp.json`.
   - Update `check_installed`:
     - `Hermes`: `$HOME/.hermes` exists or `std::env::var("HERMES_HOME").is_ok()`;
     - `OpenClaw`: `$HOME/.openclaw` exists or `std::env::var("OPENCLAW_HOME").is_ok()`.
2. **Extend `InstallTarget` in `src/install/mod.rs`**:
   - Add `InstallTarget::Hermes` and `InstallTarget::OpenClaw`;
   - Support parsing `"hermes"`, `"hermes-agent"`, `"openclaw"`, `"claw"`;
   - Update `clients()` mapping:
     - `InstallTarget::All => McpClient::all().to_vec()` (now 7 clients);
     - `InstallTarget::Hermes => vec![McpClient::Hermes]`;
     - `InstallTarget::OpenClaw => vec![McpClient::OpenClaw]`.
3. **Unit Tests (`tests/test_install.rs` & `tests/test_doctor.rs`)**:
   - Update client count assertions from 5 to 7;
   - Test `McpClient::Hermes` and `McpClient::OpenClaw` path resolution with mock home directories;
   - Test `InstallTarget::from_str` for `"hermes"` and `"openclaw"`.

## Acceptance Criteria
- [x] `McpClient::all().len() == 7`;
- [x] `InstallTarget::from_str` parses `hermes` and `openclaw` case-insensitively;
- [x] Config path resolution honors `$HERMES_HOME` and `$OPENCLAW_HOME`;
- [x] `k0maru doctor` inspects Hermes and OpenClaw alongside other clients;
- [x] All unit and integration tests pass cleanly with `cargo test`.
