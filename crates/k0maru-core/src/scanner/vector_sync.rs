//! Incremental vector synchronization engine.
//!
//! Provides batching, 0-dirty checking, and cache synchronization between
//! SQLite document cache and `sqlite-vec` vector embeddings.

use std::collections::{HashMap, HashSet};

use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::storage::SqliteStorage;
use crate::vector::EmbeddingEngine;

/// Default batch size for chunked embedding generation.
pub const DEFAULT_BATCH_SIZE: usize = 32;

/// Metrics and statistics summarizing a vector synchronization execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct VectorSyncStats {
    pub embedded_count: usize,
    pub deleted_count: usize,
    pub skipped_count: usize,
}

impl VectorSyncStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn total_processed(&self) -> usize {
        self.embedded_count + self.deleted_count + self.skipped_count
    }
}

/// Parses a content hash string (typically 16 hex chars from xxh3_64) into a u64.
/// Falls back to xxHash64 of the string bytes if hexadecimal parsing fails.
pub fn parse_content_hash(hash_str: &str) -> u64 {
    u64::from_str_radix(hash_str.trim_start_matches("0x"), 16)
        .unwrap_or_else(|_| xxhash_rust::xxh3::xxh3_64(hash_str.as_bytes()))
}

/// Incremental vector synchronization engine.
///
/// Compares `vector_metadata` against current cached documents in [`SqliteStorage`],
/// embeds new or modified documents in batches, purges orphaned vectors,
/// and bypasses unchanged documents (0-dirty check).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VectorSyncEngine {
    batch_size: usize,
}

impl Default for VectorSyncEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl VectorSyncEngine {
    /// Creates a new `VectorSyncEngine` with default batch size ([`DEFAULT_BATCH_SIZE`]).
    pub fn new() -> Self {
        Self {
            batch_size: DEFAULT_BATCH_SIZE,
        }
    }

    /// Creates a new `VectorSyncEngine` with a custom embedding batch size.
    pub fn with_batch_size(batch_size: usize) -> Self {
        assert!(
            batch_size > 0,
            "Embedding batch size must be greater than 0"
        );
        Self { batch_size }
    }

    /// Returns the configured embedding batch size.
    pub fn batch_size(&self) -> usize {
        self.batch_size
    }

    /// Convenience static method to synchronize vector embeddings using default configuration.
    pub fn sync(
        storage: &mut SqliteStorage,
        embedder: &dyn EmbeddingEngine,
    ) -> Result<VectorSyncStats, Box<dyn std::error::Error>> {
        Self::new().sync_storage(storage, embedder)
    }

    /// Synchronizes vector embeddings for all documents in `storage`.
    ///
    /// Identifies:
    /// - New/modified documents requiring embedding generation.
    /// - Orphaned vector entries whose source documents have been deleted.
    /// - Unchanged documents whose hashes match (0-dirty bypass).
    pub fn sync_storage(
        &self,
        storage: &mut SqliteStorage,
        embedder: &dyn EmbeddingEngine,
    ) -> Result<VectorSyncStats, Box<dyn std::error::Error>> {
        // 1. Fetch existing vector metadata
        let mut meta_stmt = storage
            .connection()
            .prepare("SELECT document_id, content_hash FROM vector_metadata")?;
        let vector_rows = meta_stmt.query_map([], |row| {
            let doc_id: String = row.get(0)?;
            let hash: i64 = row.get(1)?;
            Ok((doc_id, hash as u64))
        })?;

        let mut existing_vectors: HashMap<String, u64> = HashMap::new();
        for item in vector_rows {
            let (id, h) = item?;
            existing_vectors.insert(id, h);
        }

        // 2. Fetch all current documents from cache
        let mut doc_stmt = storage
            .connection()
            .prepare("SELECT path, content_hash FROM documents ORDER BY path ASC")?;
        let doc_rows = doc_stmt.query_map([], |row| {
            let path: String = row.get(0)?;
            let hash_str: String = row.get(1)?;
            Ok((path, hash_str))
        })?;

        let mut doc_paths: HashSet<String> = HashSet::new();
        let mut candidates_to_embed: Vec<(String, u64)> = Vec::new();
        let mut skipped_count = 0;

        for item in doc_rows {
            let (path, hash_str) = item?;
            let parsed_hash = parse_content_hash(&hash_str);
            doc_paths.insert(path.clone());

            if let Some(&existing_hash) = existing_vectors.get(&path) {
                if existing_hash == parsed_hash {
                    skipped_count += 1;
                } else {
                    candidates_to_embed.push((path, parsed_hash));
                }
            } else {
                candidates_to_embed.push((path, parsed_hash));
            }
        }

        // 3. Identify orphaned vectors whose documents were deleted
        let mut candidates_to_delete: Vec<String> = Vec::new();
        for existing_id in existing_vectors.keys() {
            if !doc_paths.contains(existing_id) {
                candidates_to_delete.push(existing_id.clone());
            }
        }
        candidates_to_delete.sort();

        // 4. Purge orphaned vectors in a single transaction
        let deleted_count = candidates_to_delete.len();
        if !candidates_to_delete.is_empty() {
            storage.connection().execute("BEGIN IMMEDIATE", [])?;
            let res: Result<(), Box<dyn std::error::Error>> = (|| {
                for doc_id in &candidates_to_delete {
                    storage.delete_vector(doc_id)?;
                }
                Ok(())
            })();

            match res {
                Ok(_) => {
                    storage.connection().execute("COMMIT", [])?;
                }
                Err(e) => {
                    let _ = storage.connection().execute("ROLLBACK", []);
                    return Err(e);
                }
            }
        }

        // 5. Generate embeddings for candidates in batches
        let mut embedded_count = 0;
        if !candidates_to_embed.is_empty() {
            for chunk in candidates_to_embed.chunks(self.batch_size) {
                let mut texts_to_embed: Vec<String> = Vec::with_capacity(chunk.len());

                for (path, _) in chunk {
                    let mut stmt = storage
                        .connection()
                        .prepare_cached("SELECT title, body FROM documents WHERE path = ?1")?;
                    let (title, body): (String, String) =
                        stmt.query_row(params![path], |row| Ok((row.get(0)?, row.get(1)?)))?;

                    let text = if title.is_empty() {
                        body
                    } else if body.is_empty() {
                        title
                    } else {
                        format!("{} {}", title, body)
                    };
                    texts_to_embed.push(text);
                }

                let text_slices: Vec<&str> = texts_to_embed.iter().map(|s| s.as_str()).collect();
                let embeddings = embedder.embed_batch(&text_slices)?;

                if embeddings.len() != chunk.len() {
                    return Err(format!(
                        "Embedding engine returned {} embeddings for {} texts",
                        embeddings.len(),
                        chunk.len()
                    )
                    .into());
                }

                // Upsert vectors in a single SQLite transaction
                storage.connection().execute("BEGIN IMMEDIATE", [])?;
                let res: Result<(), Box<dyn std::error::Error>> = (|| {
                    for ((path, hash), emb) in chunk.iter().zip(embeddings.iter()) {
                        storage.insert_vector(path, emb, *hash)?;
                    }
                    Ok(())
                })();

                match res {
                    Ok(_) => {
                        storage.connection().execute("COMMIT", [])?;
                        embedded_count += chunk.len();
                    }
                    Err(e) => {
                        let _ = storage.connection().execute("ROLLBACK", []);
                        return Err(e);
                    }
                }
            }
        }

        Ok(VectorSyncStats {
            embedded_count,
            deleted_count,
            skipped_count,
        })
    }
}
