# Ticket 37: Docs E2E Build, Search Indexing & Link Verification

**Status**: open
**Blocked by**: 35, 36

## Goal
Verify that the complete VitePress documentation builds cleanly with zero errors, validate local Minisearch indexing for both Chinese and English keywords, ensure zero dead links, and confirm no regressions in the Rust core test suite.

## Tasks
1. Create root entry `docs/index.md` redirecting cleanly to `/zh/` or `/en/` depending on browser preference / landing.
2. Ensure all relative paths, image assets (`docs/images/*`), and cross-links resolve with 0 broken links.
3. Run `npm run docs:build` and verify static bundle generation in `docs/.vitepress/dist/`.
4. Test local search functionality across typical search queries in both Chinese (`混合检索`, `安装`, `A100`, `flush`, `Hermes`) and English (`hybrid search`, `install`, `benchmarks`, `offload`).
5. Run workspace regression tests:
   - `cargo test` (195 tests pass 100%)
   - `cargo clippy --all-targets -- -D warnings`
   - `cargo fmt --check`
6. Update issue status and prepare pull request for integration into `dev`.
