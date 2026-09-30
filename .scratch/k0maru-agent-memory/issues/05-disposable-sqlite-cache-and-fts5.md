# 05 — Disposable SQLite cache and FTS5 BM25 search engine

**What to build:** Single-file transient SQLite database (`cache.sqlite`) implementing the `CacheStorage` trait via `rusqlite` (bundled), handling document metadata, links directed adjacency table, tags table, and FTS5 BM25 full-text indexing, featuring an instant wipe-and-rebuild mechanism.

**Blocked by:** 02 — Core domain models, traits, and test vault fixtures

**Status:** resolved

- [x] Schema initialization creates tables: `documents`, `links` (source_path, target, alias), `tags`, and virtual table `documents_fts` (FTS5 using BM25 ranking)
- [x] `upsert_document` writes document record and synchronizes its outgoing links, tags, and FTS content in a single SQLite transaction
- [x] `delete_document` cleans up document record, outgoing links, and FTS entries
- [x] `search_fts(query, limit)` executes full-text keyword search and returns ranked matches
- [x] `get_outgoing_links(path)` and `get_backlinks(target_name)` perform 1-hop and 2-hop graph neighbor lookups
- [x] `wipe_and_rebuild()` completely drops tables and reinitializes clean schema in <10ms
- [x] `tests/test_sqlite_storage.rs` passes all CRUD, graph query, and FTS search tests

## Resolution Summary
- Implemented `SqliteStorage` in `src/storage/sqlite.rs` with `open()` and `in_memory()` constructors, WAL mode, foreign keys, and indices.
- Designed complete schema with `documents`, `links`, `tags`, and `documents_fts` virtual table using SQLite FTS5 `unicode61` tokenizer.
- Implemented transactional `upsert_document` and cascade `delete_document`.
- Implemented BM25 full-text keyword ranking in `search_fts` with FTS5 token sanitization and fallback.
- Implemented graph adjacency traversal: `get_outgoing_links` and `get_backlinks` with case-insensitivity and `.md` extension normalization.
- Verified instant self-healing: `wipe_and_rebuild()` drops and recreates schema in ~1-2ms.
- 8 comprehensive integration tests in `tests/test_sqlite_storage.rs` pass with 100% success; total test suite 46/46 passed with zero clippy warnings.
