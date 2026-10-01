# 21 — Search Debugger and Explainability Cockpit

**What to build:**
Implement Cockpit Panel 1: Interactive Search Debugger with visual explainability waterfall.
Allows developers to query their memory hub in real-time, inspect how BM25 and Vector ranks fuse via RRF, and verify the +0.05 1-hop WikiLinks Graph Boost.

**Blocked by:** 20 (Dashboard Frontend Asset Pipeline and Static Embedding)

**Status:** resolved

- [x] Implement search input bar with shortcut focus (`/` or `Cmd+K`) and debounce.
- [x] Implement mode switcher toggle buttons: `Hybrid (RRF)` (default), `BM25 Lexical`, `Vector Dense`.
- [x] Implement limit slider (1–20 results, default 5).
- [x] Render search result cards:
  - Note Title, relative path, and hierarchy tier badge.
  - Scoring breakdown waterfall:
    - RRF Fusion score (e.g. `0.0164`)
    - BM25 Rank & BM25 score
    - Vector Rank & Cosine distance
    - Prominent Emerald "+0.05 Graph Boost" badge when graph boost is applied
  - Markdown note snippet preview with search query highlights.
  - Direct links to connected backlinks and forward links.
- [x] Empty state: informative prompt when query is empty; clear "No matching notes found" message with suggestions when search yields 0 hits.
- [x] Integration smoke test for search API and client view state.
