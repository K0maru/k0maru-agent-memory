# 25 — Temporarily Hide WikiLinks Graph Explorer from UI Navigation

**What to build:**
Temporarily hide the WikiLinks Graph Explorer (`#tab-btn-graph` and `#panel-graph`) from the UI navigation tabs and viewports so only the 3 primary cockpits (Search Debugger, Log Inspector, Token Scoreboard) are visible to users.

**Blocked by:** 24 (Bilingual Internationalization & Language Switcher)

**Status:** resolved

- [x] Hide Graph Tab from Cockpit Navigation:
  - Add CSS rule in `src/ui/assets/style.css` to hide `#tab-btn-graph` and `#panel-graph` (`display: none !important;`).
  - Keep underlying DOM elements and `/api/graph` backend API intact for zero breaking regression.
- [x] Update Tab Routing in `src/ui/assets/app.js`:
  - When hash is `#graph` or invalid tab, ensure default route redirects safely to `#search`.
  - Prevent unnecessary background simulation initialization when graph panel is hidden.
- [x] Test Verification:
  - Add test in `tests/test_ui_graph_explorer.rs` verifying `#tab-btn-graph` and `#panel-graph` are styled hidden and fallback routing is present.
  - Verify all 145 unit and integration tests pass with 100% success rate.
  - Run `cargo clippy --all-targets -- -D warnings`.
  - Run `cargo fmt --check`.

## Resolution Summary
1. **CSS Visibility**:
   - Added `#tab-btn-graph, #panel-graph { display: none !important; }` in `src/ui/assets/style.css`.
   - The DOM elements and Axum `/api/graph` backend endpoint remain intact and backwards-compatible.
2. **Routing & Simulation Fallback**:
   - In `src/ui/assets/app.js`, `switchTab` redirects requests for `'graph'` directly to `'search'`, and synchronizes browser history state via `history.replaceState`.
   - Added `isGraphPanelVisible()` guard to prevent ResizeObserver, `reheatSimulation()`, `startAnimationLoop()`, and `tickPhysics()` from running while the panel is hidden.
3. **Verification**:
   - Added `test_graph_tab_and_panel_temporarily_hidden_in_ui` in `tests/test_ui_graph_explorer.rs`.
   - All 145 tests pass cleanly. `cargo clippy` and `cargo fmt --check` pass with 0 warnings.
