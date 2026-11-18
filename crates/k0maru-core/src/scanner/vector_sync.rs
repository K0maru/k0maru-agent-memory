//! Incremental vector synchronization engine.
//!
//! Provides batching, 0-dirty checking, and cache synchronization between
//! SQLite document cache and `sqlite-vec` vector embeddings.

use std::collections::{HashMap, HashSet};

use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::storage::migrate::{
    table_exists, META_EMBED_DIM, META_EMBED_MODEL, META_SECTION_VECTORS_COMPLETE,
};
use crate::storage::SqliteStorage;
use crate::vector::EmbeddingEngine;

/// Default batch size for chunked embedding generation.
pub const DEFAULT_BATCH_SIZE: usize = 32;

/// Maximum characters per section embedding input (`breadcrumb + "\n\n" + section_text`).
pub const MAX_SECTION_EMBED_CHARS: usize = 2000;

/// Metrics and statistics summarizing a vector synchronization execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct VectorSyncStats {
    pub embedded_count: usize,
    pub deleted_count: usize,
    pub skipped_count: usize,
}

/// Metrics and statistics summarizing a section-level vector synchronization execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SectionVectorStats {
    pub embedded: usize,
    pub deleted: usize,
    pub pending: usize,
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
        apply_embed_model_guard(storage, embedder)?;

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

    /// Synchronizes heading-aware section embeddings in `section_vectors`, optionally capped at `budget` sections.
    ///
    /// Enforces the [`apply_embed_model_guard`], purges orphaned section vectors, embeds pending sections
    /// in batches of `self.batch_size` (`DELETE + INSERT` inside one transaction per batch), and updates
    /// `section_vectors_complete` (`'1'` only when `pending == 0`).
    pub fn sync_sections(
        &self,
        storage: &mut SqliteStorage,
        embedder: &dyn EmbeddingEngine,
        budget: Option<usize>,
    ) -> Result<SectionVectorStats, Box<dyn std::error::Error>> {
        apply_embed_model_guard(storage, embedder)?;

        // 1. Purge orphaned section vectors and metadata
        let deleted = run_in_tx(storage, |conn| {
            let del_vec = if table_exists(conn, "section_vectors")? {
                conn.execute(
                    "DELETE FROM section_vectors WHERE section_id NOT IN (SELECT id FROM document_sections)",
                    [],
                )?
            } else {
                0
            };
            let del_meta = if table_exists(conn, "section_vector_meta")? {
                conn.execute(
                    "DELETE FROM section_vector_meta WHERE section_id NOT IN (SELECT id FROM document_sections)",
                    [],
                )?
            } else {
                0
            };
            Ok(del_vec.max(del_meta))
        })?;

        // 2. Query pending sections up to `budget`
        let mut embedded = 0usize;
        if budget != Some(0) {
            let limit_val: i64 = match budget {
                Some(b) => i64::try_from(b).unwrap_or(i64::MAX),
                None => -1,
            };

            struct PendingSection {
                section_id: i64,
                doc_id: i64,
                breadcrumb: String,
                byte_start: usize,
                byte_end: usize,
                text_hash: i64,
            }

            let pending_rows: Vec<PendingSection> = {
                let mut stmt = storage.connection().prepare(
                    r#"
                    SELECT ds.id, ds.doc_id, ds.breadcrumb, ds.byte_start, ds.byte_end, ds.text_hash
                    FROM document_sections ds
                    LEFT JOIN section_vector_meta m ON m.section_id = ds.id
                    WHERE m.section_id IS NULL OR m.text_hash != ds.text_hash
                    ORDER BY ds.doc_id ASC, ds.ord ASC
                    LIMIT ?1
                    "#,
                )?;
                let mapped = stmt.query_map(params![limit_val], |row| {
                    Ok(PendingSection {
                        section_id: row.get(0)?,
                        doc_id: row.get(1)?,
                        breadcrumb: row.get(2)?,
                        byte_start: row.get::<_, i64>(3)?.max(0) as usize,
                        byte_end: row.get::<_, i64>(4)?.max(0) as usize,
                        text_hash: row.get(5)?,
                    })
                })?;
                mapped.collect::<Result<Vec<_>, _>>()?
            };

            if !pending_rows.is_empty() {
                let mut doc_bodies: HashMap<i64, String> = HashMap::new();
                for item in &pending_rows {
                    if let std::collections::hash_map::Entry::Vacant(entry) =
                        doc_bodies.entry(item.doc_id)
                    {
                        let body: Option<String> = storage
                            .connection()
                            .query_row(
                                "SELECT body FROM documents WHERE id = ?1",
                                params![item.doc_id],
                                |row| row.get(0),
                            )
                            .ok();
                        if let Some(b) = body {
                            entry.insert(b);
                        }
                    }
                }

                for chunk in pending_rows.chunks(self.batch_size) {
                    let mut texts_to_embed: Vec<String> = Vec::with_capacity(chunk.len());
                    for item in chunk {
                        let body = doc_bodies
                            .get(&item.doc_id)
                            .map(String::as_str)
                            .unwrap_or("");
                        let section_text = if item.byte_start <= item.byte_end
                            && item.byte_end <= body.len()
                            && body.is_char_boundary(item.byte_start)
                            && body.is_char_boundary(item.byte_end)
                        {
                            &body[item.byte_start..item.byte_end]
                        } else {
                            ""
                        };
                        let mut embed_input = format!("{}\n\n{}", item.breadcrumb, section_text);
                        truncate_in_place_chars(&mut embed_input, MAX_SECTION_EMBED_CHARS);
                        texts_to_embed.push(embed_input);
                    }

                    let text_slices: Vec<&str> =
                        texts_to_embed.iter().map(String::as_str).collect();
                    let embeddings = embedder.embed_batch(&text_slices)?;
                    if embeddings.len() != chunk.len() {
                        return Err(format!(
                            "Embedding engine returned {} embeddings for {} sections",
                            embeddings.len(),
                            chunk.len()
                        )
                        .into());
                    }

                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i64;

                    run_in_tx(storage, |conn| {
                        let mut del_stmt = conn
                            .prepare_cached("DELETE FROM section_vectors WHERE section_id = ?1")?;
                        let mut ins_stmt = conn.prepare_cached(
                            "INSERT INTO section_vectors (section_id, doc_id, embedding) VALUES (?1, ?2, ?3)",
                        )?;
                        let mut meta_stmt = conn.prepare_cached(
                            r#"
                            INSERT INTO section_vector_meta (section_id, text_hash, updated_at)
                            VALUES (?1, ?2, ?3)
                            ON CONFLICT(section_id) DO UPDATE SET
                                text_hash = excluded.text_hash,
                                updated_at = excluded.updated_at
                            "#,
                        )?;

                        for (item, emb) in chunk.iter().zip(embeddings.iter()) {
                            let bytes = f32_slice_to_bytes(emb);
                            del_stmt.execute(params![item.section_id])?;
                            ins_stmt.execute(params![item.section_id, item.doc_id, bytes])?;
                            meta_stmt.execute(params![item.section_id, item.text_hash, now])?;
                        }
                        Ok(())
                    })?;

                    embedded += chunk.len();
                }
            }
        }

        // 3. Recount remaining pending sections and update readiness meta flag
        let pending_count: i64 = storage.connection().query_row(
            r#"
            SELECT COUNT(*)
            FROM document_sections ds
            LEFT JOIN section_vector_meta m ON m.section_id = ds.id
            WHERE m.section_id IS NULL OR m.text_hash != ds.text_hash
            "#,
            [],
            |row| row.get(0),
        )?;
        let pending = pending_count.max(0) as usize;
        storage.meta_set(
            META_SECTION_VECTORS_COMPLETE,
            if pending == 0 { "1" } else { "0" },
        )?;

        Ok(SectionVectorStats {
            embedded,
            deleted,
            pending,
        })
    }
}

/// Ensures vector tables match `embedder.dimension()`, wipes stale vectors if `embed_model`
/// changed, and records `embed_model` + `embed_dim` in `k0maru_meta`.
fn apply_embed_model_guard(
    storage: &mut SqliteStorage,
    embedder: &dyn EmbeddingEngine,
) -> Result<(), Box<dyn std::error::Error>> {
    storage.ensure_vector_dimension(embedder.dimension())?;

    let recorded_model = storage.meta_get(META_EMBED_MODEL)?;
    if let Some(prev) = recorded_model {
        if prev != embedder.model_name() {
            run_in_tx(storage, |conn| {
                if table_exists(conn, "document_vectors")? {
                    conn.execute("DELETE FROM document_vectors", [])?;
                }
                if table_exists(conn, "vector_metadata")? {
                    conn.execute("DELETE FROM vector_metadata", [])?;
                }
                if table_exists(conn, "section_vectors")? {
                    conn.execute("DELETE FROM section_vectors", [])?;
                }
                if table_exists(conn, "section_vector_meta")? {
                    conn.execute("DELETE FROM section_vector_meta", [])?;
                }
                Ok(())
            })?;
            storage.meta_set(META_SECTION_VECTORS_COMPLETE, "0")?;
        }
    }

    storage.meta_set(META_EMBED_MODEL, embedder.model_name())?;
    storage.meta_set(META_EMBED_DIM, &embedder.dimension().to_string())?;
    Ok(())
}

fn run_in_tx<T, F>(storage: &mut SqliteStorage, op: F) -> Result<T, Box<dyn std::error::Error>>
where
    F: FnOnce(&rusqlite::Connection) -> Result<T, Box<dyn std::error::Error>>,
{
    storage.connection().execute("BEGIN IMMEDIATE", [])?;
    match op(storage.connection()) {
        Ok(val) => {
            storage.connection().execute("COMMIT", [])?;
            Ok(val)
        }
        Err(e) => {
            let _ = storage.connection().execute("ROLLBACK", []);
            Err(e)
        }
    }
}

fn truncate_in_place_chars(s: &mut String, max_chars: usize) {
    if let Some((byte_idx, _)) = s.char_indices().nth(max_chars) {
        s.truncate(byte_idx);
    }
}

fn f32_slice_to_bytes(slice: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(std::mem::size_of_val(slice));
    for &val in slice {
        bytes.extend_from_slice(&val.to_le_bytes());
    }
    bytes
}
