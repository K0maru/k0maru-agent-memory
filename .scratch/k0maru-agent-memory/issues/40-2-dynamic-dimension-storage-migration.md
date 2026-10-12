# Ticket 40-2: Dynamic Dimension Support & Auto-Migration in `SqliteStorage`

## Status: Ready
**Blocked by**: Ticket 40-1

## Goal
Enable `SqliteStorage` to dynamically support variable vector dimensions (e.g. 384, 512, 768) and automatically migrate (drop & recreate vec0 table and reset vector metadata) when switching between models of different dimensions, eliminating dimension mismatch crashes.

## Acceptance Criteria
1. Store vector schema metadata in SQLite:
   - Create table `CREATE TABLE IF NOT EXISTS vector_schema_info (dimension INTEGER NOT NULL, updated_at INTEGER NOT NULL);`.
   - Provide `get_vector_dimension(&self) -> Result<Option<usize>, Box<dyn Error>>`.
2. Add dynamic dimension initialization:
   - `ensure_vector_dimension(&mut self, target_dim: usize) -> Result<bool, Box<dyn Error>>`:
     - If `document_vectors` does not exist, create `embedding float[target_dim] distance_metric=cosine` and record `target_dim` in `vector_schema_info`. Returns `false` (no migration needed).
     - If current dimension == target_dim, do nothing, return `false`.
     - If current dimension != target_dim, drop `document_vectors` and `vector_metadata`, re-create `document_vectors` with `target_dim`, update `vector_schema_info`, and return `true` (migrated).
3. Update `insert_vector` and `search_vectors` to validate against the active table dimension dynamically rather than hardcoded 384.
4. Unit tests in `crates/k0maru-core/tests/test_dynamic_vector_storage.rs` validating:
   - Initial table creation with 384 dimensions.
   - Dimension migration from 384 to 512 (BGE-ZH) drops old vectors cleanly and accepts 512d vectors.
   - Searching vectors with matching dimensions succeeds; searching with mismatched dimensions returns descriptive error.
