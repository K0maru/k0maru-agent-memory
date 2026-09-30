# 16 — Incremental Vector Synchronization Pipeline

**Type:** task  
**Status:** ready-for-agent  
**Blocked by:** 13, 14  

## Context
Embedding large Markdown collections on CPU requires efficient incremental synchronization. If a file has not changed, its vector embedding must not be recomputed. The vector pipeline coordinates with `IncrementalScanner` to embed only new or modified documents.

## Objectives & Deliverables
1. Implement `VectorSyncEngine` in `src/scanner/vector_sync.rs`:
   - Checks `vector_metadata` against current document `content_hash`;
   - Identifies candidate documents:
     - New documents (no vector entry);
     - Modified documents (`content_hash` differs from `vector_metadata.content_hash`);
     - Deleted documents (remove from `document_vectors` and `vector_metadata`).
2. Batch embedding execution:
   - Chunk candidate documents into batches of 32;
   - Pass chunks to `EmbeddingEngine::embed_batch`;
   - Upsert vectors into `SqliteStorage::insert_vector` in a single SQLite transaction.
3. Integrate `--vector` flag into `IncrementalScanner::sync_vault`:
   - `scanner.sync_vault(vault_root, enable_vector: bool) -> Result<SyncStats>`
4. Tests in `tests/test_vector_sync.rs`:
   - Verify initial sync embeds all documents;
   - Verify second sync without edits has zero re-computations;
   - Verify editing one file only re-embeds that single file;
   - Verify deleting a file cleans up its vector.

## Acceptance Criteria
- [ ] Incremental zero-dirty check bypasses embedding computation completely.
- [ ] Edited documents update both vector and metadata hash.
- [ ] Deleted files are cleanly purged from vector index.
