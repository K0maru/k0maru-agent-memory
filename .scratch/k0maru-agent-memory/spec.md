# Spec 42: Open-Source Orthogonal Sidecar Evolution & Legacy Protocol Cleanup

## 1. Background & Motivation

`k0maru-agent-memory` serves both the general open-source AI coding agent ecosystem (`Claude Code`, `Cursor`, `Gemini CLI`, `Windsurf`, `Cline`, `Hermes`, `OpenClaw`) and `Tern + omp` (`Oh My Pi`).

Following the open-source boundary principle:
1. **Delete dead private protocols (`crystal` & `kdsl`)**:
   - `src/crystal/mod.rs` (`k0maru crystal` / `tuple-crystal`) consumes private `~/.k0maru/tuples/` Linda tuple files from a retired internal router. No open-source agent generates or consumes this format.
   - `crates/k0maru-core/src/guard/kdsl.rs` (`k0maru guard check --kdsl` and `guard_check(kdsl)`) parses private `!δ:0.1|...` K-DSL packets. Open-source agents audit standard `git diff` or file paths.
2. **Retain Context Tools Behind `--sidecar` Flag for `omp`**:
   - `omp` natively provides `bash` auto-spilling (`artifact://<id>:N-M`), persistent `eval` kernels, `xd://eval/archive`, and `seance` subagents, making `offload_context`, `inspect_log_node`, and `gc_session_context` redundant on `omp`.
   - Other open-source hosts (`Cursor`, `Windsurf`, `Cline`, `Claude Code`) still rely on these 3 tools.
   - Therefore, `k0maru mcp` keeps all **8 tools** by default for open-source compatibility, and adds `--sidecar` (`McpServer::with_sidecar(true)`) to expose only the **5 core orthogonal memory tools** (`get_project_loadout`, `recall_memory`, `flush_session`, `distill_session_skill`, `guard_check`).
3. **Add First-Class `omp` Client Ecosystem & Installer Support (`McpClient::Omp`)**:
   - Resolve `~/.omp/agent/mcp.json` (or `$OMP_HOME/mcp.json`).
   - Support `k0maru install --target omp`, automatically injecting `["mcp", "--vault", "<path>", "--sidecar"]`.

---

## 2. Tracer-Bullet Tickets (Parallel Slices)

- **Ticket 42-1 (`42-1-remove-legacy-crystal-and-kdsl.md`)**: Remove `src/crystal/`, `crates/k0maru-core/src/guard/kdsl.rs`, `guard --kdsl`, and `crystal` CLI subcommand; wire `--sidecar` into `McpArgs` in `src/main.rs`.
- **Ticket 42-2 (`42-2-mcp-sidecar-mode-flag.md`)**: Add `sidecar: bool` and `with_sidecar(bool)` to `McpServer` (`src/mcp/server.rs`), filter `tools/list` (8 tools default vs 5 tools in sidecar mode), and remove `kdsl` from `guard_check`.
- **Ticket 42-3 (`42-3-omp-client-ecosystem-and-installer.md`)**: Add `McpClient::Omp` and `InstallTarget::Omp` across `src/ecosystem/mod.rs`, `src/install/mod.rs`, `src/doctor/mod.rs`, and update `tests/test_install.rs` & `tests/test_doctor.rs`.
