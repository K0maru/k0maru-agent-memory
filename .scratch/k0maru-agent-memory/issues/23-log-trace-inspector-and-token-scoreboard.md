# 23 — Log Trace Inspector, Token Scoreboard, and End-to-End Verification

**What to build:**
Implement Cockpit Panels 3 & 4 (Log Trace Inspector & Token Scoreboard) and perform end-to-end integration verification for `k0maru ui`.

**Blocked by:** 21 (Search Debugger), 22 (WikiLinks Graph Explorer)

**Status:** ready-for-agent

- [ ] Implement Panel 3: Log & Trace Inspector:
  - Sidebar: list of offloaded log nodes (`node_xxx`) with timestamp, line count, and truncated percentage.
  - Detail view:
    - Render Mermaid pipeline state diagram natively (using `mermaid.js`).
    - Collapsible raw log viewer with syntax highlighting and line numbers.
    - One-click "Copy Node ID" and "Copy Log Slice" buttons.
- [ ] Implement Panel 4: Cache Health & Token Scoreboard:
  - Metric cards: Total Markdown Documents, Indexed Vectors, SQLite Cache File Size, Last Sync Latency.
  - Token Reduction Ratio (TRR %) meter and estimated cumulative token savings counter.
  - "Sync Vault" button with real-time loading spinner and toast notification displaying sync stats (`embedded_count`, `skipped_count`).
- [ ] Browser auto-open logic in CLI (`--open` via `webbrowser` or OS-specific command `open`/`xdg-open`).
- [ ] Full end-to-end smoke test in `tests/test_ui_e2e.rs`:
  - Launch embedded server against a mock vault fixture.
  - Query all endpoints, verify JSON payload shapes.
  - Verify embedded HTML/CSS/JS delivery.
- [ ] Quality assurance: 100% test pass rate, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`.
