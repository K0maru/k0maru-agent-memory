# Ticket 37: Docs E2E Build, Search Indexing & Link Verification

**Status**: closed
**Blocked by**: 35, 36

## Goal
Verify that the complete VitePress documentation builds cleanly with zero errors, validate local Minisearch indexing for both Chinese and English keywords, ensure zero dead links, and confirm no regressions in the Rust core test suite.

## Tasks
1. Create root entry `docs/index.md` redirecting cleanly to `/zh/` or `/en/` depending on browser preference / landing. [x]
2. Ensure all relative paths, image assets (`docs/images/*`), and cross-links resolve with 0 broken links. [x]
3. Run `npm run docs:build` and verify static bundle generation in `docs/.vitepress/dist/`. [x]
4. Test local search functionality across typical search queries in both Chinese (`混合检索`, `安装`, `A100`, `flush`, `Hermes`) and English (`hybrid search`, `install`, `benchmarks`, `offload`). [x]
5. Run workspace regression tests: [x]
   - `cargo test` (232 passed, 0 failed across 36 test targets)
   - `cargo clippy --all-targets -- -D warnings` (clean, 0 warnings)
   - `cargo fmt --check` (clean)
6. Update issue status and prepare pull request for integration into `dev`. [x]

## Verification Report
- **Redirection**: `docs/index.md` detects `navigator.language` on client side (`zh` -> `/zh/`, fallback -> `/en/`) with clean `<noscript>` hero fallback cards.
- **Link & Asset Health**: Scanned 31 markdown files and 17 assets under `docs/images/`; 0 broken links and 0 missing assets.
- **VitePress Build**: `npm run docs:build` succeeds in ~1.5s with zero errors or warnings.
- **Minisearch Validation**:
  - Chinese: `混合检索` (5), `安装` (11), `A100` (22), `flush` (20), `Hermes` (14).
  - English: `hybrid search` (24), `install` (19), `benchmarks` (9), `offload` (33).
- **Rust Regression**: 232 tests passing 100%, 0 failures, clippy & fmt clean.
