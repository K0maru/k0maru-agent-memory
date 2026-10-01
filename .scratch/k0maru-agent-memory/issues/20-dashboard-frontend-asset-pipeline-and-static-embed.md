# 20 — Dashboard Frontend Asset Pipeline and Static Embedding

**What to build:**
Implement the compile-time asset embedding pipeline using `rust-embed` and provide the foundational SPA shell conforming to the Developer Dark Cockpit design system (`ui-ux-pro-max` & `design-taste-frontend`).
The binary serves the embedded web application directly from memory without requiring external node_modules or static file directories at runtime.

**Blocked by:** 19 (Dashboard Backend Routes and Embedded HTTP Server)

**Status:** ready-for-agent

- [ ] Add `rust-embed = "8"` (with MIME guessing) to `Cargo.toml`.
- [ ] Create static asset directory `src/ui/assets/` containing embedded HTML, CSS, and JS bundle.
- [ ] Implement embedded file handler in `src/ui/server.rs`:
  - Serves exact files for static assets (`/assets/*`, `.css`, `.js`, `.svg`).
  - Correct `Content-Type` headers (e.g. `text/html`, `application/javascript`, `text/css`, `image/svg+xml`).
  - Fallback route: any non-`/api` path returns `index.html` for single-page routing.
- [ ] Implement base SPA shell:
  - Adhere to Developer Dark Cockpit design system: Deep Slate `#0F172A`, Card `#1B2336`, Accent Emerald `#22C55E`, Border `#334155`.
  - Global navigation bar with logo, vault path badge, live status pills, and tab switcher:
    - 🔍 Search Debugger
    - 🕸️ Graph Explorer
    - 📜 Log Inspector
    - 📊 Token Scoreboard
- [ ] Create `tests/test_ui_static_embed.rs` asserting embedded assets are served with 200 OK and valid MIME types.
