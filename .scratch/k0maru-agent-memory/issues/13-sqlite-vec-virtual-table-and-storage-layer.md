# 13 — sqlite-vec Virtual Table & Vector Storage Seam

**Type:** task  
**Status:** resolved  
**Blocked by:** None  

## Context
Phase 4 introduces dense vector storage into the transient `cache.sqlite` database using the `sqlite-vec` extension (`vec0` virtual table). The vector store must live cleanly alongside existing `documents`, `documents_fts`, `links`, and `tags` tables, while preserving the disposable cache contract (instant recreation from Markdown files).

## Objectives & Deliverables
1. Integrate `sqlite-vec` into `Cargo.toml` (`sqlite-vec = "0.1"` or static C FFI extension auto-registration for `rusqlite`).
2. Add `document_vectors` virtual table and `vector_metadata` table to `SqliteStorage::initialize_schema`:
   ```sql
   CREATE VIRTUAL TABLE IF NOT EXISTS document_vectors USING vec0(
       id TEXT PRIMARY KEY,
       embedding float[384] distance_metric=cosine
   );
   CREATE TABLE IF NOT EXISTS vector_metadata (
       document_id TEXT PRIMARY KEY,
       content_hash INTEGER NOT NULL,
       updated_at INTEGER NOT NULL
   );
   ```
3. Implement `SqliteStorage` vector operations:
   - `insert_vector(document_id: &str, embedding: &[f32], content_hash: u64) -> Result<()>`
   - `delete_vector(document_id: &str) -> Result<()>`
   - `search_vectors(query_embedding: &[f32], limit: usize) -> Result<Vec<(String, f32)>>` (returns `(document_id, distance)` sorted by closest cosine distance)
   - `get_vector_content_hash(document_id: &str) -> Result<Option<u64>>`
4. Automated tests in `tests/test_sqlite_vec_storage.rs`:
   - Initialize schema with vector support;
   - Insert multiple embeddings (unit vectors);
   - Query nearest neighbors and assert cosine distance ordering;
   - Delete vectors and verify clean removal.

## Acceptance Criteria
- [x] Schema initializes with `document_vectors` and `vector_metadata` without SQLite errors.
- [x] Inserting, querying, and deleting vectors succeeds with proper cosine distance calculations.
- [x] Rebuilding `cache.sqlite` clears and recreates vector tables safely.
- [x] All new tests pass with `cargo test`.

## Comments

### Implementation Summary
- Integrated `sqlite-vec = "0.1"` into `Cargo.toml`.
- Implemented process-wide automatic extension registration `ensure_sqlite_vec_registered()` via `rusqlite::ffi::sqlite3_auto_extension` and `sqlite_vec::sqlite3_vec_init`.
- Added `document_vectors` virtual table (`vec0`, 384 dimensions, cosine distance metric) and `vector_metadata` table to `SCHEMA_SQL`.
- Implemented `insert_vector`, `delete_vector`, `search_vectors`, and `get_vector_content_hash` on `SqliteStorage`.
- Updated `wipe_and_rebuild` to drop vector tables and recreate schema cleanly.
- Exported `SqliteStorage`, `ensure_sqlite_vec_registered`, and `VECTOR_DIMENSIONS` from `crate::storage`.
- Added comprehensive integration tests in `tests/test_sqlite_vec_storage.rs` covering schema initialization, vector upsert, cosine distance ranking, deletion, wipe-and-rebuild, and disk persistence.
- Verified 100% test pass rate, `cargo clippy --all-targets -- -D warnings` zero warnings, and `cargo fmt --check` clean.
