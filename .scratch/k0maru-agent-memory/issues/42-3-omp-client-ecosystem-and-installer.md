# Ticket 42-3: Add `McpClient::Omp` Ecosystem Detector and `k0maru install --target omp`

- **Status**: `done`
- **Blocked by**: None (Parallel Slice 3)

## Target Files
- Edit `src/ecosystem/mod.rs`
- Edit `src/install/mod.rs`
- Edit `src/doctor/mod.rs`
- Edit `tests/test_install.rs`
- Edit `tests/test_doctor.rs`

## Acceptance Criteria
- [x] `McpClient::Omp` added to `src/ecosystem/mod.rs` (`name()` = `"omp (Oh My Pi)"`, `McpClient::all().len() == 8`).
- [x] `McpClient::Omp::config_path` resolves `$OMP_HOME/mcp.json` (when `home_override.is_none()` and `OMP_HOME` set) or `<home>/.omp/agent/mcp.json`.
- [x] `check_installed` for `McpClient::Omp` checks `home.join(".omp").exists() || std::env::var("OMP_HOME").is_ok()`.
- [x] `InstallTarget::Omp` added to `src/install/mod.rs` (parses `"omp"` and `"oh-my-pi"`).
- [x] When installing for `McpClient::Omp`, `run_install` injects `"args": ["mcp", "--vault", "<vault>", "--sidecar"]`.
- [x] `target_flag_name` in `src/doctor/mod.rs` maps `McpClient::Omp => "omp"`.
- [x] Unit and integration tests in `tests/test_install.rs` and `tests/test_doctor.rs` updated and passing.
