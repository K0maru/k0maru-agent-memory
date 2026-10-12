# Issue 38-1: Scoped FTS5 SQL Push-Down in SqliteStorage

**Status**: Ready
**Blocked by**: None

## Goal
Implement a scoped full-text search method on `SqliteStorage` in `crates/k0maru-core/src/storage/sqlite.rs` that pushes down path prefix and hierarchy level filtering directly into the SQLite FTS5 query rather than post-filtering in memory.

## Acceptance Criteria
1. Add `search_scoped_fts` to `SqliteStorage`:
   ```rust
   pub fn search_scoped_fts(
       &self,
       query: &str,
       path_prefix: Option<&str>,
       hierarchy: Option<&str>,
       limit: usize,
   ) -> Result<Vec<Document>, Box<dyn std::error::Error>>
   ```
2. The query must properly sanitize tokens (handling hyphens and special syntax), and construct SQL that matches `documents_fts` while filtering `d.path LIKE ? || '%'` and/or `d.hierarchy = ?`.
3. Unit test verifying that when given 10 daily logs containing the query and 1 project document containing the query, `search_scoped_fts` with `path_prefix = Some("10_Projects/")` and `limit = 5` returns the project document and 0 daily logs.
4. `cargo test -p k0maru-core` passes with 0 warnings.
