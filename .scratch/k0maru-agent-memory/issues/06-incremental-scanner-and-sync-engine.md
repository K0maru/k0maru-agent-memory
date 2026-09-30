# 06 — Incremental file scanner and state synchronization engine

**What to build:** An incremental scanner engine in `src/scanner/engine.rs` comparing filesystem `mtime` and content hashes (`xxhash-rust`) against `cache.sqlite` to compute dirty sets (`added`, `modified`, `deleted`, `unchanged`) and synchronize changes in milliseconds.

**Blocked by:** 04 — Obsidian and Generic Wiki storage adapters, 05 — Disposable SQLite cache and FTS5 BM25 search engine

**Status:** resolved

- [x] `IncrementalScanner` performs dirty checks comparing file `mtime` and xxhash content hash against stored cache
- [x] Correctly categorizes files into `added`, `modified`, `deleted`, and `unchanged` sets
- [x] Synchronizes changed documents via `adapter.read_document` and `storage.upsert_document` in batch transactions
- [x] Cleans up deleted documents via `storage.delete_document`
- [x] Re-running sync on an unmodified vault finishes in <5ms with 0 modifications
- [x] Returns `SyncStats` with precise counts and execution duration
- [x] `tests/test_scanner.rs` validates initial sync, no-op sync, update sync, and deletion sync

## Completion Summary
- Created `src/scanner/mod.rs` and `src/scanner/engine.rs` exposing `IncrementalScanner` and `DirtySets`.
- Added `CachedDocMeta` in `src/core/models.rs`, and extended `CacheStorage` with `get_cached_metadata()`.
- Extended `VaultAdapter` with `root_path()`, `file_mtime()`, and `compute_content_hash()`, implemented across both `ObsidianAdapter` and `GenericWikiAdapter`.
- Implemented high-efficiency dirty checking comparing filesystem `mtime` and `xxh3_64` hashes against cached metadata. Unchanged files skip AST parsing and disk IO completely.
- Added comprehensive TDD integration tests in `tests/test_scanner.rs` covering:
  - Initial synchronization from empty cache.
  - Sub-5ms no-op synchronization on unmodified vaults.
  - Incremental single-file update and immediate FTS5 searchability.
  - Single-file deletion and link/tag cascade pruning.
  - `force_full: true` whole-vault cache re-indexing.
  - Dynamic file additions and inspection of `DirtySets`.
  - Generic Wiki adapter compatibility.
- Verified 100% test pass rate (49 tests passing, 0 warnings in `cargo clippy`, zero formatting diffs in `cargo fmt --check`).

