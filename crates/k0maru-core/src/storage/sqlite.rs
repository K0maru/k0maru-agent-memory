//! SQLite and FTS5 cache storage engine.
//!
//! Provides disposable, high-performance local SQLite caching implementing
//! [`CacheStorage`], with BM25-ranked full-text search and link graph index.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Once;

use rusqlite::{params, params_from_iter, OptionalExtension};

use crate::core::models::{CachedDocMeta, Document, HierarchyLevel, WikiLink};
use crate::core::traits::CacheStorage;
use crate::graph::{LinkResolver, Resolution};

static INIT_SQLITE_VEC: Once = Once::new();

/// Registers sqlite-vec as an auto-extension for all new SQLite connections in the current process.
pub fn ensure_sqlite_vec_registered() {
    INIT_SQLITE_VEC.call_once(|| unsafe {
        #[allow(clippy::missing_transmute_annotations)]
        rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute(
            sqlite_vec::sqlite3_vec_init as *const (),
        )));
    });
}

/// Default embedding dimensions for document vectors.
pub const VECTOR_DIMENSIONS: usize = 384;

/// A heading-aware Markdown section row loaded from `document_sections`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredSection {
    pub id: i64,
    pub doc_id: i64,
    pub ord: u32,
    pub level: u8,
    pub heading: String,
    pub breadcrumb: String,
    pub byte_start: usize,
    pub byte_end: usize,
    pub start_line: u32,
    pub end_line: u32,
}

/// Tuple of `(section_id, doc_id, bm25_score)` returned by [`SqliteStorage::search_section_fts`].
pub type SectionFtsHit = (i64, i64, f64);

/// A nearest-neighbor section vector hit joined with its parent document path.
#[derive(Debug, Clone, PartialEq)]
pub struct SectionVectorHit {
    pub section_id: i64,
    pub doc_id: i64,
    pub path: String,
    pub distance: f32,
}

/// Summary statistics returned by [`SqliteStorage::rebuild_link_edges`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LinkEdgeStats {
    pub edges: usize,
    pub broken: usize,
    pub assets: usize,
}

pub(crate) fn section_vector_ddl(dim: usize) -> String {
    format!(
        r#"
        CREATE VIRTUAL TABLE IF NOT EXISTS section_vectors USING vec0(
            section_id INTEGER PRIMARY KEY,
            doc_id integer,
            embedding float[{dim}] distance_metric=cosine
        );
        CREATE TABLE IF NOT EXISTS section_vector_meta (
            section_id INTEGER PRIMARY KEY,
            text_hash  INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );
        "#
    )
}

pub(crate) fn get_vector_dimension_conn(
    conn: &rusqlite::Connection,
) -> Result<Option<usize>, Box<dyn std::error::Error>> {
    let table_exists: bool = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type IN ('table', 'shadow') AND name = 'document_vectors'",
            [],
            |row| Ok(row.get::<_, i64>(0)? > 0),
        )
        .unwrap_or(false);

    if !table_exists {
        return Ok(None);
    }

    let schema_info_exists: bool = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'vector_schema_info'",
            [],
            |row| Ok(row.get::<_, i64>(0)? > 0),
        )
        .unwrap_or(false);

    if schema_info_exists {
        let dim: Option<i64> = conn
            .query_row(
                "SELECT dimension FROM vector_schema_info ORDER BY updated_at DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(d) = dim {
            return Ok(Some(d as usize));
        }
    }

    // If document_vectors exists but no schema info is recorded, legacy schema used 384
    Ok(Some(VECTOR_DIMENSIONS))
}

/// Health state of a live [`SqliteStorage`] handle relative to the database file on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveDbHealth {
    /// File identity, schema version, and core cache tables are intact.
    Healthy,
    /// Database file on disk was replaced or removed since this connection was opened.
    ReplacedOnDisk,
    /// `PRAGMA user_version` exceeds [`super::migrate::CURRENT_SCHEMA_VERSION`].
    SchemaTooNew(i32),
    /// Required core tables (`documents` or `fts_documents`) are missing from `sqlite_master`.
    CorruptedSchema,
}

fn file_identity(path: &Path) -> Option<(u64, u64)> {
    let meta = std::fs::metadata(path).ok()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Some((meta.dev(), meta.ino()))
    }
    #[cfg(not(unix))]
    {
        let modified_ns = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        Some((meta.len(), modified_ns))
    }
}

/// Ephemeral SQLite-backed storage implementing [`CacheStorage`].
pub struct SqliteStorage {
    conn: rusqlite::Connection,
    active_dimension: AtomicUsize,
    opened_file_id: Option<(u64, u64)>,
}

impl SqliteStorage {
    /// Default embedding dimensions for document vectors.
    pub const VECTOR_DIMENSIONS: usize = VECTOR_DIMENSIONS;

    /// Opens or creates a SQLite cache database at the specified file path in read-write mode.
    ///
    /// Delegates to [`SqliteStorage::open_rw`].
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        Self::open_rw(path)
    }

    /// Opens or creates a SQLite cache database in read-write mode with schema v3 migration.
    ///
    /// Rejects databases whose `PRAGMA user_version` exceeds the supported schema version.
    pub fn open_rw<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        ensure_sqlite_vec_registered();
        let path_ref = path.as_ref();
        if let Some(parent) = path_ref.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let conn = rusqlite::Connection::open(path_ref)?;
        conn.busy_timeout(std::time::Duration::from_secs(30))?;

        // Switching from DELETE to WAL requires an exclusive lock and can transiently
        // return SQLITE_BUSY under concurrent opens; retry with backoff before proceeding.
        let current_mode: String = conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap_or_default();
        if !current_mode.eq_ignore_ascii_case("wal") {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
            let mut delay_ms = 20u64;
            loop {
                match conn.pragma_update(None, "journal_mode", "WAL") {
                    Ok(()) => break,
                    Err(rusqlite::Error::SqliteFailure(e, _))
                        if e.code == rusqlite::ErrorCode::DatabaseBusy
                            && std::time::Instant::now() < deadline =>
                    {
                        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                        delay_ms = (delay_ms * 2).min(250);
                    }
                    Err(e) => return Err(e.into()),
                }
            }
        }
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        let mut storage = Self {
            conn,
            active_dimension: AtomicUsize::new(0),
            opened_file_id: None,
        };
        storage.initialize()?;
        storage.opened_file_id = file_identity(path_ref);
        Ok(storage)
    }

    /// Opens an existing SQLite cache database in strict read-only mode (`SQLITE_OPEN_READ_ONLY`).
    ///
    /// Executes zero DDL or write pragmas and validates `PRAGMA user_version <= CURRENT_SCHEMA_VERSION`.
    pub fn open_read_only<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        ensure_sqlite_vec_registered();
        let path_ref = path.as_ref();
        let conn = rusqlite::Connection::open_with_flags(
            path_ref,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;
        conn.busy_timeout(std::time::Duration::from_millis(5000))?;
        super::migrate::validate_schema_version_read_only(&conn)?;

        let storage = Self {
            conn,
            active_dimension: AtomicUsize::new(0),
            opened_file_id: file_identity(path_ref),
        };
        if let Ok(Some(dim)) = storage.get_vector_dimension() {
            storage.active_dimension.store(dim, Ordering::Relaxed);
        }
        Ok(storage)
    }

    /// Repairs stale or missing `fts_documents` entries and drops any stray legacy `documents_fts` table.
    pub fn repair_stale_fts(&mut self) -> Result<usize, Box<dyn std::error::Error>> {
        super::migrate::repair_stale_fts(&self.conn)
    }

    /// Creates an in-memory SQLite cache database (ideal for tests and ephemeral runs).
    pub fn in_memory() -> Result<Self, Box<dyn std::error::Error>> {
        ensure_sqlite_vec_registered();
        let conn = rusqlite::Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        let mut storage = Self {
            conn,
            active_dimension: AtomicUsize::new(0),
            opened_file_id: None,
        };
        storage.initialize()?;
        Ok(storage)
    }

    /// Checks whether a cached live connection is still backed by the same on-disk database file,
    /// has a compatible `PRAGMA user_version`, and retains its required core tables.
    pub fn check_live_health(
        &self,
        db_path: &Path,
    ) -> Result<LiveDbHealth, Box<dyn std::error::Error>> {
        if let Some(opened_id) = self.opened_file_id {
            if file_identity(db_path) != Some(opened_id) {
                return Ok(LiveDbHealth::ReplacedOnDisk);
            }
        }

        let user_version: i32 = self
            .conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if user_version > super::migrate::CURRENT_SCHEMA_VERSION {
            return Ok(LiveDbHealth::SchemaTooNew(user_version));
        }

        let has_documents = super::migrate::table_exists(&self.conn, "documents")?;
        let has_fts = super::migrate::table_exists(&self.conn, "fts_documents")?;
        let has_sections = super::migrate::table_exists(&self.conn, "document_sections")?;
        if !has_documents || !has_fts || !has_sections {
            return Ok(LiveDbHealth::CorruptedSchema);
        }

        Ok(LiveDbHealth::Healthy)
    }

    /// Returns an immutable reference to the underlying [`rusqlite::Connection`].
    pub fn connection(&self) -> &rusqlite::Connection {
        &self.conn
    }

    /// Returns a mutable reference to the underlying [`rusqlite::Connection`].
    pub fn connection_mut(&mut self) -> &mut rusqlite::Connection {
        &mut self.conn
    }

    /// Returns the currently active vector dimension, querying `vector_schema_info` or active tables.
    pub fn get_vector_dimension(&self) -> Result<Option<usize>, Box<dyn std::error::Error>> {
        get_vector_dimension_conn(&self.conn)
    }

    /// Alias for [`get_vector_dimension`].
    pub fn current_vector_dimension(&self) -> Result<Option<usize>, Box<dyn std::error::Error>> {
        self.get_vector_dimension()
    }

    /// Ensures that the underlying document and section vector tables have the specified dimension.
    ///
    /// If `document_vectors` does not exist, creates document and section vector tables with `target_dim`
    /// and records it in `vector_schema_info`. Returns `false`.
    /// If current dimension equals `target_dim`, ensures section vector tables exist and returns `false`.
    /// If current dimension differs from `target_dim`, drops old vector tables, recreates them with `target_dim`,
    /// resets `section_vectors_complete = '0'`, deletes `embed_model`, updates `vector_schema_info`, and returns `true`.
    pub fn ensure_vector_dimension(
        &mut self,
        target_dim: usize,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        if target_dim == 0 {
            return Err("Target vector dimension must be greater than 0".into());
        }

        let current_dim = self.get_vector_dimension()?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS vector_schema_info (dimension INTEGER NOT NULL, updated_at INTEGER NOT NULL);",
        )?;

        match current_dim {
            None => {
                let sec_ddl = section_vector_ddl(target_dim);
                let sql = format!(
                    r#"
                    CREATE VIRTUAL TABLE IF NOT EXISTS document_vectors USING vec0(
                        id TEXT PRIMARY KEY,
                        embedding float[{}] distance_metric=cosine
                    );
                    CREATE TABLE IF NOT EXISTS vector_metadata (
                        document_id TEXT PRIMARY KEY,
                        content_hash INTEGER NOT NULL,
                        updated_at INTEGER NOT NULL
                    );
                    {}
                    DELETE FROM vector_schema_info;
                    "#,
                    target_dim, sec_ddl
                );
                self.conn.execute_batch(&sql)?;
                self.conn.execute(
                    "INSERT INTO vector_schema_info (dimension, updated_at) VALUES (?1, ?2)",
                    params![target_dim as i64, now],
                )?;
                if super::migrate::table_exists(&self.conn, "k0maru_meta")? {
                    super::migrate::upsert_meta_in_tx(
                        &self.conn,
                        super::migrate::META_SECTION_VECTORS_COMPLETE,
                        "0",
                    )?;
                }
                self.active_dimension.store(target_dim, Ordering::Relaxed);
                Ok(false)
            }
            Some(curr) if curr == target_dim => {
                let sec_ddl = section_vector_ddl(target_dim);
                self.conn.execute_batch(&sec_ddl)?;
                self.active_dimension.store(target_dim, Ordering::Relaxed);
                Ok(false)
            }
            Some(_curr) => {
                let sec_ddl = section_vector_ddl(target_dim);
                let drop_and_recreate_sql = format!(
                    r#"
                    DROP TABLE IF EXISTS section_vector_meta;
                    DROP TABLE IF EXISTS section_vectors;
                    DROP TABLE IF EXISTS document_vectors;
                    DROP TABLE IF EXISTS vector_metadata;
                    CREATE VIRTUAL TABLE document_vectors USING vec0(
                        id TEXT PRIMARY KEY,
                        embedding float[{}] distance_metric=cosine
                    );
                    CREATE TABLE vector_metadata (
                        document_id TEXT PRIMARY KEY,
                        content_hash INTEGER NOT NULL,
                        updated_at INTEGER NOT NULL
                    );
                    {}
                    DELETE FROM vector_schema_info;
                    "#,
                    target_dim, sec_ddl
                );
                self.conn.execute_batch(&drop_and_recreate_sql)?;
                self.conn.execute(
                    "INSERT INTO vector_schema_info (dimension, updated_at) VALUES (?1, ?2)",
                    params![target_dim as i64, now],
                )?;
                if super::migrate::table_exists(&self.conn, "k0maru_meta")? {
                    super::migrate::upsert_meta_in_tx(
                        &self.conn,
                        super::migrate::META_SECTION_VECTORS_COMPLETE,
                        "0",
                    )?;
                    self.conn.execute(
                        "DELETE FROM k0maru_meta WHERE key IN (?1, ?2)",
                        params![
                            super::migrate::META_EMBED_MODEL,
                            super::migrate::META_EMBED_DIM
                        ],
                    )?;
                }
                self.active_dimension.store(target_dim, Ordering::Relaxed);
                Ok(true)
            }
        }
    }

    fn active_dimension(&self) -> Result<usize, Box<dyn std::error::Error>> {
        let cached = self.active_dimension.load(Ordering::Relaxed);
        if cached > 0 {
            return Ok(cached);
        }
        let dim = self
            .get_vector_dimension()?
            .unwrap_or(Self::VECTOR_DIMENSIONS);
        self.active_dimension.store(dim, Ordering::Relaxed);
        Ok(dim)
    }

    /// Inserts or replaces a document vector and its corresponding metadata content hash.
    pub fn insert_vector(
        &self,
        doc_id: &str,
        embedding: &[f32],
        hash: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let expected_dim = self.active_dimension()?;
        if embedding.len() != expected_dim {
            return Err(format!(
                "Embedding dimensions mismatch: expected {}, got {}",
                expected_dim,
                embedding.len()
            )
            .into());
        }

        // Delete existing vector if present to support upsert behavior
        let _ = self.conn.execute(
            "DELETE FROM document_vectors WHERE id = ?1",
            params![doc_id],
        );

        let bytes = f32_slice_to_bytes(embedding);
        self.conn.execute(
            "INSERT INTO document_vectors (id, embedding) VALUES (?1, ?2)",
            params![doc_id, bytes],
        )?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        self.conn.execute(
            r#"
            INSERT INTO vector_metadata (document_id, content_hash, updated_at)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(document_id) DO UPDATE SET
                content_hash = excluded.content_hash,
                updated_at = excluded.updated_at
            "#,
            params![doc_id, hash as i64, now],
        )?;

        Ok(())
    }

    /// Deletes a document vector and its metadata by document ID.
    pub fn delete_vector(&self, doc_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.conn.execute(
            "DELETE FROM document_vectors WHERE id = ?1",
            params![doc_id],
        )?;
        self.conn.execute(
            "DELETE FROM vector_metadata WHERE document_id = ?1",
            params![doc_id],
        )?;
        Ok(())
    }

    /// Searches for the nearest vector neighbors by cosine distance.
    ///
    /// Returns pairs of `(document_id, distance)` ordered by ascending distance.
    pub fn search_vectors(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<(String, f32)>, Box<dyn std::error::Error>> {
        if limit == 0 || query_embedding.is_empty() {
            return Ok(Vec::new());
        }
        let expected_dim = self.active_dimension()?;
        if query_embedding.len() != expected_dim {
            return Err(format!(
                "Embedding dimensions mismatch: expected {}, got {}",
                expected_dim,
                query_embedding.len()
            )
            .into());
        }

        let bytes = f32_slice_to_bytes(query_embedding);
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, distance FROM document_vectors WHERE embedding MATCH ?1 AND k = ?2 ORDER BY distance",
        )?;

        let rows = stmt.query_map(params![bytes, limit as i64], |row| {
            let id: String = row.get(0)?;
            let distance: f32 = row.get(1)?;
            Ok((id, distance))
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Retrieves the content hash associated with a document's vector, if present.
    pub fn get_vector_content_hash(
        &self,
        doc_id: &str,
    ) -> Result<Option<u64>, Box<dyn std::error::Error>> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT content_hash FROM vector_metadata WHERE document_id = ?1")?;
        let mut rows = stmt.query(params![doc_id])?;
        if let Some(row) = rows.next()? {
            let hash: i64 = row.get(0)?;
            Ok(Some(hash as u64))
        } else {
            Ok(None)
        }
    }

    /// Returns the total count of stored vector embeddings in `vector_metadata`.
    pub fn vector_count(&self) -> Result<usize, Box<dyn std::error::Error>> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT count(*) FROM vector_metadata")?;
        let count: i64 = stmt.query_row([], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// Returns the total count of stored documents in `documents`.
    pub fn document_count(&self) -> Result<usize, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare_cached("SELECT count(*) FROM documents")?;
        let count: i64 = stmt.query_row([], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// Returns true if there is at least one vector embedding in storage.
    pub fn has_vectors(&self) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(self.vector_count()? > 0)
    }

    /// Retrieves a document by its relative path if present in cache.
    pub fn get_document(
        &self,
        path: &Path,
    ) -> Result<Option<Document>, Box<dyn std::error::Error>> {
        <Self as CacheStorage>::get_document(self, path)
    }

    /// BM25 full-text search across indexed documents, scoped by path prefix and/or hierarchy.
    pub fn search_scoped_fts(
        &self,
        query: &str,
        path_prefix: Option<&str>,
        hierarchy: Option<&str>,
        limit: usize,
    ) -> Result<Vec<Document>, Box<dyn std::error::Error>> {
        <Self as CacheStorage>::search_scoped_fts(self, query, path_prefix, hierarchy, limit)
    }

    /// BM25 full-text search across all indexed documents, returning matched documents
    /// alongside the [`crate::text::FtsQueryTier`] that produced them (`None` when empty).
    pub fn search_fts_with_tier(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<(Vec<Document>, Option<crate::text::FtsQueryTier>), Box<dyn std::error::Error>>
    {
        let trimmed = query.trim();
        if trimmed.is_empty() || limit == 0 {
            return Ok((Vec::new(), None));
        }

        let search_sql = r#"
        SELECT d.path, d.title, d.hierarchy, d.frontmatter, d.content_hash, d.mtime, d.body, d.body_line_offset
        FROM fts_documents f
        JOIN documents d ON d.id = f.rowid
        WHERE fts_documents MATCH ?1
        ORDER BY f.rank
        LIMIT ?2
        "#;

        let tiers = [
            crate::text::FtsQueryTier::Strict,
            crate::text::FtsQueryTier::AllTokens,
            crate::text::FtsQueryTier::AnyTokens,
        ];
        let mut last_query: Option<String> = None;
        let mut raw_docs = Vec::new();
        let mut matched_tier = None;

        for tier in tiers {
            let Some(fts_query) = crate::text::build_fts_query(trimmed, tier) else {
                continue;
            };
            if last_query.as_deref() == Some(fts_query.as_str()) {
                continue;
            }
            last_query = Some(fts_query.clone());
            let docs = self.query_raw_docs(search_sql, &fts_query, limit)?;
            if !docs.is_empty() {
                raw_docs = docs;
                matched_tier = Some(tier);
                break;
            }
        }

        let docs = self.hydrate_raw_docs(raw_docs)?;
        Ok((docs, matched_tier))
    }

    /// Looks up the stable integer `documents.id` for a given relative vault path.
    pub fn doc_id_for_path(&self, path: &Path) -> Result<Option<i64>, Box<dyn std::error::Error>> {
        let path_str = path.to_string_lossy();
        let id = self
            .conn
            .query_row(
                "SELECT id FROM documents WHERE path = ?1",
                params![path_str],
                |row| row.get(0),
            )
            .optional()?;
        Ok(id)
    }

    /// Returns all heading-aware sections for `doc_id` ordered by `ord`, or `[]` if `document_sections` is absent.
    pub fn get_sections(
        &self,
        doc_id: i64,
    ) -> Result<Vec<StoredSection>, Box<dyn std::error::Error>> {
        if !super::migrate::table_exists(&self.conn, "document_sections")? {
            return Ok(Vec::new());
        }
        let mut stmt = self.conn.prepare_cached(
            r#"
            SELECT id, doc_id, ord, level, heading, breadcrumb, byte_start, byte_end, start_line, end_line
            FROM document_sections
            WHERE doc_id = ?1
            ORDER BY ord ASC
            "#,
        )?;
        let rows = stmt.query_map(params![doc_id], |row| {
            Ok(StoredSection {
                id: row.get(0)?,
                doc_id: row.get(1)?,
                ord: row.get::<_, i64>(2)?.max(0) as u32,
                level: row.get::<_, i64>(3)?.max(0) as u8,
                heading: row.get(4)?,
                breadcrumb: row.get(5)?,
                byte_start: row.get::<_, i64>(6)?.max(0) as usize,
                byte_end: row.get::<_, i64>(7)?.max(0) as usize,
                start_line: row.get::<_, i64>(8)?.max(0) as u32,
                end_line: row.get::<_, i64>(9)?.max(0) as u32,
            })
        })?;
        let mut sections = Vec::new();
        for row in rows {
            sections.push(row?);
        }
        Ok(sections)
    }

    /// Searches `fts_sections` restricted to `doc_ids` using the Strict -> AllTokens -> AnyTokens tier ladder.
    ///
    /// Returns `(section_id, doc_id, bm25_score)` tuples ordered by ascending BM25 score (best first).
    pub fn search_section_fts(
        &self,
        query: &str,
        doc_ids: &[i64],
        limit: usize,
    ) -> Result<Vec<SectionFtsHit>, Box<dyn std::error::Error>> {
        let trimmed = query.trim();
        if trimmed.is_empty() || doc_ids.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        if !super::migrate::table_exists(&self.conn, "fts_sections")?
            || !super::migrate::table_exists(&self.conn, "document_sections")?
        {
            return Ok(Vec::new());
        }

        let mut unique_ids = Vec::with_capacity(doc_ids.len());
        let mut seen = HashSet::with_capacity(doc_ids.len());
        for &id in doc_ids {
            if seen.insert(id) {
                unique_ids.push(id);
                if unique_ids.len() >= 1024 {
                    break;
                }
            }
        }

        let placeholders: Vec<String> = (0..unique_ids.len())
            .map(|i| format!("?{}", i + 2))
            .collect();
        let limit_idx = unique_ids.len() + 2;
        let sql = format!(
            r#"
            SELECT ds.id, ds.doc_id, bm25(fts_sections, 2.0, 1.0) AS r
            FROM fts_sections
            JOIN document_sections ds ON ds.id = fts_sections.rowid
            WHERE fts_sections MATCH ?1 AND ds.doc_id IN ({})
            ORDER BY r ASC
            LIMIT ?{}
            "#,
            placeholders.join(", "),
            limit_idx
        );

        let tiers = [
            crate::text::FtsQueryTier::Strict,
            crate::text::FtsQueryTier::AllTokens,
            crate::text::FtsQueryTier::AnyTokens,
        ];
        let mut last_query: Option<String> = None;
        let mut stmt = self.conn.prepare(&sql)?;

        for tier in tiers {
            let Some(fts_query) = crate::text::build_fts_query(trimmed, tier) else {
                continue;
            };
            if last_query.as_deref() == Some(fts_query.as_str()) {
                continue;
            }
            last_query = Some(fts_query.clone());

            let mut bound: Vec<rusqlite::types::Value> = Vec::with_capacity(unique_ids.len() + 2);
            bound.push(rusqlite::types::Value::from(fts_query));
            for &id in &unique_ids {
                bound.push(rusqlite::types::Value::from(id));
            }
            bound.push(rusqlite::types::Value::from(limit as i64));

            let rows = stmt.query_map(params_from_iter(bound.iter()), |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, f64>(2)?,
                ))
            })?;
            let hits: Vec<(i64, i64, f64)> = rows.collect::<Result<Vec<_>, _>>()?;
            if !hits.is_empty() {
                return Ok(hits);
            }
        }

        Ok(Vec::new())
    }

    /// Nearest-neighbor KNN search over `section_vectors`, joined against `document_sections` and `documents`.
    pub fn search_section_vectors(
        &self,
        q: &[f32],
        k: usize,
    ) -> Result<Vec<SectionVectorHit>, Box<dyn std::error::Error>> {
        if k == 0 || q.is_empty() {
            return Ok(Vec::new());
        }
        if !super::migrate::table_exists(&self.conn, "section_vectors")?
            || !super::migrate::table_exists(&self.conn, "document_sections")?
        {
            return Ok(Vec::new());
        }

        let expected_dim = self.active_dimension()?;
        if q.len() != expected_dim {
            return Err(format!(
                "Embedding dimensions mismatch: expected {}, got {}",
                expected_dim,
                q.len()
            )
            .into());
        }

        let clamped_k = k.min(4096) as i64;
        let bytes = f32_slice_to_bytes(q);
        let mut stmt = self.conn.prepare_cached(
            r#"
            WITH knn AS (
                SELECT section_id, distance
                FROM section_vectors
                WHERE embedding MATCH ?1 AND k = ?2
            )
            SELECT knn.section_id, ds.doc_id, d.path, knn.distance
            FROM knn
            JOIN document_sections ds ON ds.id = knn.section_id
            JOIN documents d          ON d.id  = ds.doc_id
            ORDER BY knn.distance ASC
            "#,
        )?;

        let rows = stmt.query_map(params![bytes, clamped_k], |row| {
            Ok(SectionVectorHit {
                section_id: row.get(0)?,
                doc_id: row.get(1)?,
                path: row.get(2)?,
                distance: row.get(3)?,
            })
        })?;
        let mut hits = Vec::new();
        for row in rows {
            hits.push(row?);
        }
        Ok(hits)
    }

    /// Returns `true` only when `section_vectors` exists, `section_vectors_complete == '1'`,
    /// and `embed_model` matches `model`.
    pub fn section_vectors_ready(&self, model: &str) -> bool {
        if !super::migrate::table_exists(&self.conn, "section_vectors").unwrap_or(false) {
            return false;
        }
        let complete = self
            .meta_get(super::migrate::META_SECTION_VECTORS_COMPLETE)
            .ok()
            .flatten();
        if complete.as_deref() != Some("1") {
            return false;
        }
        let recorded_model = self
            .meta_get(super::migrate::META_EMBED_MODEL)
            .ok()
            .flatten();
        recorded_model.as_deref() == Some(model)
    }

    /// Reads a key from `k0maru_meta`, returning `Ok(None)` if the table or key does not exist.
    pub fn meta_get(&self, key: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
        if !super::migrate::table_exists(&self.conn, "k0maru_meta")? {
            return Ok(None);
        }
        let val = self
            .conn
            .query_row(
                "SELECT value FROM k0maru_meta WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()?;
        Ok(val)
    }

    /// Upserts a key-value pair into `k0maru_meta`.
    pub fn meta_set(&self, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error>> {
        super::migrate::upsert_meta_in_tx(&self.conn, key, value)?;
        Ok(())
    }

    /// Lazily repairs stale or missing section rows up to `budget` documents (`None` = unlimited).
    pub fn repair_stale_sections(
        &mut self,
        budget: Option<usize>,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        super::migrate::repair_stale_sections(&self.conn, budget)
    }

    /// Rebuilds the resolved `link_edges` table from `documents` and `links` in a single transaction.
    pub fn rebuild_link_edges(&mut self) -> Result<LinkEdgeStats, Box<dyn std::error::Error>> {
        let tx = self
            .conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;

        let docs: Vec<(i64, String, String)> = {
            let mut stmt = tx.prepare("SELECT id, path, title FROM documents ORDER BY id ASC")?;
            let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
            rows.collect::<Result<Vec<_>, _>>()?
        };

        let mut path_to_id: HashMap<String, i64> = HashMap::with_capacity(docs.len());
        for (id, path, _) in &docs {
            path_to_id.insert(path.clone(), *id);
        }

        let resolver = LinkResolver::from_docs(docs);

        let raw_links: Vec<(String, String)> = {
            let mut stmt = tx.prepare("SELECT source_path, target FROM links")?;
            let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
            rows.collect::<Result<Vec<_>, _>>()?
        };

        tx.execute("DELETE FROM link_edges", [])?;

        let mut seen_edges: HashSet<(i64, i64)> = HashSet::new();
        let mut broken = 0usize;
        let mut assets = 0usize;

        {
            let mut ins_edge = tx.prepare_cached(
                "INSERT INTO link_edges (src_doc_id, dst_doc_id) VALUES (?1, ?2)",
            )?;
            for (source_path, target) in raw_links {
                let Some(&src_id) = path_to_id.get(&source_path) else {
                    continue;
                };
                match resolver.resolve(&source_path, &target) {
                    Resolution::Doc(dst_id) => {
                        if src_id != dst_id && seen_edges.insert((src_id, dst_id)) {
                            ins_edge.execute(params![src_id, dst_id])?;
                        }
                    }
                    Resolution::Asset => assets += 1,
                    Resolution::SelfRef => {}
                    Resolution::Broken => broken += 1,
                }
            }
        }

        super::migrate::upsert_meta_in_tx(&tx, super::migrate::META_LINK_EDGES_DIRTY, "0")?;
        tx.commit()?;

        Ok(LinkEdgeStats {
            edges: seen_edges.len(),
            broken,
            assets,
        })
    }

    /// Rebuilds `link_edges` when `link_edges_clean()` is `false`, returning `true` if rebuilt.
    pub fn rebuild_link_edges_if_dirty(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        if self.link_edges_clean() {
            return Ok(false);
        }
        self.rebuild_link_edges()?;
        Ok(true)
    }

    /// Returns `true` when `link_edges` exists and `k0maru_meta.link_edges_dirty == '0'`.
    pub fn link_edges_clean(&self) -> bool {
        if !super::migrate::table_exists(&self.conn, "link_edges").unwrap_or(false) {
            return false;
        }
        self.meta_get(super::migrate::META_LINK_EDGES_DIRTY)
            .ok()
            .flatten()
            .as_deref()
            == Some("0")
    }

    /// Returns all `(src_doc_id, dst_doc_id)` pairs in `link_edges` where either endpoint is in `doc_ids`.
    pub fn edges_touching(
        &self,
        doc_ids: &[i64],
    ) -> Result<Vec<(i64, i64)>, Box<dyn std::error::Error>> {
        if doc_ids.is_empty() || !super::migrate::table_exists(&self.conn, "link_edges")? {
            return Ok(Vec::new());
        }

        let mut unique_ids = Vec::with_capacity(doc_ids.len());
        let mut seen = HashSet::with_capacity(doc_ids.len());
        for &id in doc_ids {
            if seen.insert(id) {
                unique_ids.push(id);
            }
        }

        let mut edges_set: HashSet<(i64, i64)> = HashSet::new();
        for chunk in unique_ids.chunks(500) {
            let placeholders: Vec<String> = (1..=chunk.len()).map(|i| format!("?{i}")).collect();
            let in_list = placeholders.join(", ");
            let sql = format!(
                "SELECT src_doc_id, dst_doc_id FROM link_edges WHERE src_doc_id IN ({0}) OR dst_doc_id IN ({0})",
                in_list
            );
            let mut stmt = self.conn.prepare(&sql)?;
            let rows = stmt.query_map(params_from_iter(chunk.iter()), |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?))
            })?;
            for row in rows {
                edges_set.insert(row?);
            }
        }

        let mut edges: Vec<(i64, i64)> = edges_set.into_iter().collect();
        edges.sort_unstable();
        Ok(edges)
    }

    /// Records a note access in `doc_access` with a 60-second SQL debounce window.
    ///
    /// Returns `Ok(false)` on read-only handles, when `doc_access` is absent, or when debounced.
    pub fn record_access(&self, path: &str, now: i64) -> Result<bool, Box<dyn std::error::Error>> {
        if self
            .conn
            .is_readonly(rusqlite::DatabaseName::Main)
            .unwrap_or(false)
            || !super::migrate::table_exists(&self.conn, "doc_access")?
        {
            return Ok(false);
        }
        let changed = self.conn.execute(
            r#"
            INSERT INTO doc_access (path, access_count, last_access)
            VALUES (?1, 1, ?2)
            ON CONFLICT(path) DO UPDATE SET
                access_count = access_count + 1,
                last_access = excluded.last_access
            WHERE excluded.last_access - doc_access.last_access >= 60
            "#,
            params![path, now],
        )?;
        Ok(changed > 0)
    }

    /// Fetches `(access_count, last_access)` entries from `doc_access` for the given paths.
    pub fn access_stats(
        &self,
        paths: &[&str],
    ) -> Result<HashMap<String, (u32, i64)>, Box<dyn std::error::Error>> {
        if paths.is_empty() || !super::migrate::table_exists(&self.conn, "doc_access")? {
            return Ok(HashMap::new());
        }
        let mut stats = HashMap::new();
        for chunk in paths.chunks(500) {
            let placeholders: Vec<String> = (1..=chunk.len()).map(|i| format!("?{i}")).collect();
            let sql = format!(
                "SELECT path, access_count, last_access FROM doc_access WHERE path IN ({})",
                placeholders.join(", ")
            );
            let mut stmt = self.conn.prepare(&sql)?;
            let rows = stmt.query_map(params_from_iter(chunk.iter().copied()), |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, i64>(1)?.max(0) as u32,
                    r.get::<_, i64>(2)?,
                ))
            })?;
            for row in rows {
                let (path, count, last_access) = row?;
                stats.insert(path, (count, last_access));
            }
        }
        Ok(stats)
    }

    /// Replaces `doc_git_time` entries and records `git_time_head` in a single transaction.
    pub fn set_git_times(
        &mut self,
        times: &[(String, i64)],
        head: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let tx = self
            .conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute("DELETE FROM doc_git_time", [])?;
        {
            let mut stmt = tx.prepare_cached(
                r#"
                INSERT INTO doc_git_time (path, commit_time)
                VALUES (?1, ?2)
                ON CONFLICT(path) DO UPDATE SET commit_time = excluded.commit_time
                "#,
            )?;
            for (path, commit_time) in times {
                stmt.execute(params![path, commit_time])?;
            }
        }
        super::migrate::upsert_meta_in_tx(&tx, super::migrate::META_GIT_TIME_HEAD, head)?;
        tx.commit()?;
        Ok(())
    }

    /// Fetches last-commit timestamps from `doc_git_time` for the given paths.
    pub fn git_times(
        &self,
        paths: &[&str],
    ) -> Result<HashMap<String, i64>, Box<dyn std::error::Error>> {
        if paths.is_empty() || !super::migrate::table_exists(&self.conn, "doc_git_time")? {
            return Ok(HashMap::new());
        }
        let mut result = HashMap::new();
        for chunk in paths.chunks(500) {
            let placeholders: Vec<String> = (1..=chunk.len()).map(|i| format!("?{i}")).collect();
            let sql = format!(
                "SELECT path, commit_time FROM doc_git_time WHERE path IN ({})",
                placeholders.join(", ")
            );
            let mut stmt = self.conn.prepare(&sql)?;
            let rows = stmt.query_map(params_from_iter(chunk.iter().copied()), |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
            })?;
            for row in rows {
                let (path, commit_time) = row?;
                result.insert(path, commit_time);
            }
        }
        Ok(result)
    }

    fn query_raw_docs(
        &self,
        sql: &str,
        query_arg: &str,
        limit: usize,
    ) -> Result<Vec<RawDocRow>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(sql)?;
        let mapped = stmt.query_map(params![query_arg, limit as i64], |row| {
            Ok(RawDocRow {
                path: row.get(0)?,
                title: row.get(1)?,
                hierarchy: row.get(2)?,
                frontmatter: row.get(3)?,
                content_hash: row.get(4)?,
                mtime: row.get(5)?,
                body: row.get(6)?,
                body_line_offset: row.get(7)?,
            })
        })?;
        let mut docs = Vec::new();
        for row in mapped {
            docs.push(row?);
        }
        Ok(docs)
    }

    fn query_raw_docs_scoped(
        &self,
        sql: &str,
        query_arg: &str,
        path_prefix: Option<&str>,
        hierarchy: Option<&str>,
        limit: usize,
    ) -> Result<Vec<RawDocRow>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(sql)?;
        let mapped = stmt.query_map(
            params![query_arg, path_prefix, hierarchy, limit as i64],
            |row| {
                Ok(RawDocRow {
                    path: row.get(0)?,
                    title: row.get(1)?,
                    hierarchy: row.get(2)?,
                    frontmatter: row.get(3)?,
                    content_hash: row.get(4)?,
                    mtime: row.get(5)?,
                    body: row.get(6)?,
                    body_line_offset: row.get(7)?,
                })
            },
        )?;
        let mut docs = Vec::new();
        for row in mapped {
            docs.push(row?);
        }
        Ok(docs)
    }

    fn hydrate_raw_docs(
        &self,
        raw_docs: Vec<RawDocRow>,
    ) -> Result<Vec<Document>, Box<dyn std::error::Error>> {
        let mut results = Vec::with_capacity(raw_docs.len());
        for raw in raw_docs {
            let path = PathBuf::from(&raw.path);
            let links = self.get_outgoing_links(&path)?;
            let mut tag_stmt = self
                .conn
                .prepare_cached("SELECT tag FROM tags WHERE document_path = ?1")?;
            let tags: Vec<String> = tag_stmt
                .query_map(params![raw.path], |row| row.get(0))?
                .collect::<Result<Vec<String>, _>>()?;

            let hierarchy = parse_hierarchy(&raw.hierarchy);
            let frontmatter =
                serde_json::from_str(&raw.frontmatter).unwrap_or(serde_json::json!({}));

            results.push(Document {
                path,
                title: raw.title,
                hierarchy,
                frontmatter,
                links,
                tags,
                content_hash: raw.content_hash,
                mtime: raw.mtime as u64,
                body: raw.body,
                body_line_offset: raw.body_line_offset.max(0) as usize,
            });
        }
        Ok(results)
    }
}

pub(crate) const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS documents (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    hierarchy TEXT NOT NULL,
    frontmatter TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    mtime INTEGER NOT NULL,
    body TEXT NOT NULL,
    body_line_offset INTEGER NOT NULL DEFAULT 0,
    fts_hash TEXT
);

CREATE TABLE IF NOT EXISTS links (
    source_path TEXT NOT NULL,
    target TEXT NOT NULL,
    alias TEXT,
    raw_text TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS tags (
    document_path TEXT NOT NULL,
    tag TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_links_source ON links(source_path);
CREATE INDEX IF NOT EXISTS idx_links_target ON links(target COLLATE NOCASE);
CREATE INDEX IF NOT EXISTS idx_tags_path ON tags(document_path);
CREATE INDEX IF NOT EXISTS idx_tags_tag ON tags(tag COLLATE NOCASE);

CREATE VIRTUAL TABLE IF NOT EXISTS fts_documents USING fts5(
    title,
    body,
    tags,
    content='',
    contentless_delete=1,
    tokenize='unicode61 remove_diacritics 2'
);

CREATE TABLE IF NOT EXISTS k0maru_meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS vector_schema_info (
    dimension INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE VIRTUAL TABLE IF NOT EXISTS document_vectors USING vec0(
    id TEXT PRIMARY KEY,
    embedding float[384] distance_metric=cosine
);

CREATE TABLE IF NOT EXISTS vector_metadata (
    document_id TEXT PRIMARY KEY,
    content_hash INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- Step 2: heading-aware sections
CREATE TABLE IF NOT EXISTS document_sections (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    doc_id      INTEGER NOT NULL,
    ord         INTEGER NOT NULL,
    level       INTEGER NOT NULL,
    heading     TEXT    NOT NULL,
    breadcrumb  TEXT    NOT NULL,
    byte_start  INTEGER NOT NULL,
    byte_end    INTEGER NOT NULL,
    start_line  INTEGER NOT NULL,
    end_line    INTEGER NOT NULL,
    text_hash   INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_sections_doc ON document_sections(doc_id, ord);

CREATE VIRTUAL TABLE IF NOT EXISTS fts_sections USING fts5(
    heading, body,
    content='', contentless_delete=1,
    tokenize='unicode61 remove_diacritics 2'
);

CREATE TABLE IF NOT EXISTS document_section_state (
    doc_id        INTEGER PRIMARY KEY,
    sections_hash TEXT NOT NULL
);

-- Step 3: graph, access reinforcement, git time
CREATE TABLE IF NOT EXISTS link_edges (
    src_doc_id INTEGER NOT NULL,
    dst_doc_id INTEGER NOT NULL,
    PRIMARY KEY (src_doc_id, dst_doc_id)
) WITHOUT ROWID;
CREATE INDEX IF NOT EXISTS idx_link_edges_dst ON link_edges(dst_doc_id, src_doc_id);

CREATE TABLE IF NOT EXISTS doc_access (
    path         TEXT PRIMARY KEY,
    access_count INTEGER NOT NULL DEFAULT 0,
    last_access  INTEGER NOT NULL
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS doc_git_time (
    path        TEXT PRIMARY KEY,
    commit_time INTEGER NOT NULL
) WITHOUT ROWID;
"#;

impl CacheStorage for SqliteStorage {
    fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        super::migrate::ensure_schema(&self.conn)?;
        let dim = self
            .get_vector_dimension()?
            .unwrap_or(Self::VECTOR_DIMENSIONS);
        self.active_dimension.store(dim, Ordering::Relaxed);
        Ok(())
    }

    fn wipe_and_rebuild(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if super::migrate::table_exists(&self.conn, "sqlite_sequence")? {
            self.conn.execute(
                "DELETE FROM sqlite_sequence WHERE name = 'document_sections'",
                [],
            )?;
        }
        let drop_sql = r#"
        DROP TABLE IF EXISTS vector_schema_info;
        DROP TABLE IF EXISTS vector_metadata;
        DROP TABLE IF EXISTS document_vectors;
        DROP TABLE IF EXISTS section_vector_meta;
        DROP TABLE IF EXISTS section_vectors;
        DROP TABLE IF EXISTS fts_sections;
        DROP TABLE IF EXISTS document_sections;
        DROP TABLE IF EXISTS document_section_state;
        DROP TABLE IF EXISTS link_edges;
        DROP TABLE IF EXISTS doc_access;
        DROP TABLE IF EXISTS doc_git_time;
        DROP TABLE IF EXISTS fts_documents;
        DROP TABLE IF EXISTS documents_fts;
        DROP TABLE IF EXISTS k0maru_meta;
        DROP TABLE IF EXISTS tags;
        DROP TABLE IF EXISTS links;
        DROP TABLE IF EXISTS documents;
        PRAGMA user_version = 0;
        "#;
        self.conn.execute_batch(drop_sql)?;
        self.active_dimension.store(0, Ordering::Relaxed);
        self.initialize()?;
        Ok(())
    }

    fn upsert_document(&mut self, doc: &Document) -> Result<(), Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;
        let path_str = doc.path.to_string_lossy();
        let frontmatter_str = serde_json::to_string(&doc.frontmatter)?;
        let tags_str = doc.tags.join(" ");
        let fts_hash = crate::text::fts_hash_for(&doc.content_hash);

        // 1. Upsert document record and retrieve its stable INTEGER PRIMARY KEY id
        let doc_id: i64 = tx.query_row(
            r#"
            INSERT INTO documents (
                path, title, hierarchy, frontmatter, content_hash, mtime, body, body_line_offset, fts_hash
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ON CONFLICT(path) DO UPDATE SET
                title = excluded.title,
                hierarchy = excluded.hierarchy,
                frontmatter = excluded.frontmatter,
                content_hash = excluded.content_hash,
                mtime = excluded.mtime,
                body = excluded.body,
                body_line_offset = excluded.body_line_offset,
                fts_hash = excluded.fts_hash
            RETURNING id
            "#,
            params![
                path_str,
                doc.title,
                doc.hierarchy.as_str(),
                frontmatter_str,
                doc.content_hash,
                doc.mtime as i64,
                doc.body,
                doc.body_line_offset as i64,
                fts_hash,
            ],
            |row| row.get(0),
        )?;

        // 2. Synchronize outgoing links
        tx.execute(
            "DELETE FROM links WHERE source_path = ?1",
            params![path_str],
        )?;
        {
            let mut insert_link = tx.prepare_cached(
                "INSERT INTO links (source_path, target, alias, raw_text) VALUES (?1, ?2, ?3, ?4)",
            )?;
            for link in &doc.links {
                insert_link.execute(params![path_str, link.target, link.alias, link.raw_text])?;
            }
        }

        // 3. Synchronize tags
        tx.execute(
            "DELETE FROM tags WHERE document_path = ?1",
            params![path_str],
        )?;
        {
            let mut insert_tag =
                tx.prepare_cached("INSERT INTO tags (document_path, tag) VALUES (?1, ?2)")?;
            for tag in &doc.tags {
                insert_tag.execute(params![path_str, tag])?;
            }
        }

        // 4. Synchronize contentless_delete FTS entry keyed by documents.id
        let seg_title = crate::text::segment_for_index(&doc.title);
        let seg_body = crate::text::segment_for_index(&doc.body);
        let seg_tags = crate::text::segment_for_index(&tags_str);

        tx.execute(
            "DELETE FROM fts_documents WHERE rowid = ?1",
            params![doc_id],
        )?;
        tx.execute(
            "INSERT INTO fts_documents (rowid, title, body, tags) VALUES (?1, ?2, ?3, ?4)",
            params![doc_id, seg_title, seg_body, seg_tags],
        )?;

        // 5. Synchronize heading-aware sections and mark link_edges dirty
        super::migrate::upsert_doc_sections_in_tx(
            &tx,
            doc_id,
            &doc.title,
            &doc.body,
            doc.body_line_offset,
            &doc.content_hash,
            true,
        )?;

        tx.commit()?;
        Ok(())
    }

    fn delete_document(&mut self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;
        let path_str = path.to_string_lossy();

        // Purge FTS, section FTS/vectors/meta, section rows, state, and link edges before deleting documents row.
        if let Some(doc_id) = tx
            .query_row(
                "SELECT id FROM documents WHERE path = ?1",
                params![path_str],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
        {
            tx.execute(
                "DELETE FROM fts_documents WHERE rowid = ?1",
                params![doc_id],
            )?;
            if super::migrate::table_exists(&tx, "fts_sections")? {
                tx.execute(
                    "DELETE FROM fts_sections WHERE rowid IN (SELECT id FROM document_sections WHERE doc_id = ?1)",
                    params![doc_id],
                )?;
            }
            if super::migrate::table_exists(&tx, "section_vectors")? {
                tx.execute(
                    "DELETE FROM section_vectors WHERE section_id IN (SELECT id FROM document_sections WHERE doc_id = ?1)",
                    params![doc_id],
                )?;
            }
            if super::migrate::table_exists(&tx, "section_vector_meta")? {
                tx.execute(
                    "DELETE FROM section_vector_meta WHERE section_id IN (SELECT id FROM document_sections WHERE doc_id = ?1)",
                    params![doc_id],
                )?;
            }
            if super::migrate::table_exists(&tx, "document_sections")? {
                tx.execute(
                    "DELETE FROM document_sections WHERE doc_id = ?1",
                    params![doc_id],
                )?;
            }
            if super::migrate::table_exists(&tx, "document_section_state")? {
                tx.execute(
                    "DELETE FROM document_section_state WHERE doc_id = ?1",
                    params![doc_id],
                )?;
            }
            if super::migrate::table_exists(&tx, "link_edges")? {
                tx.execute(
                    "DELETE FROM link_edges WHERE src_doc_id = ?1 OR dst_doc_id = ?1",
                    params![doc_id],
                )?;
            }
        }

        if super::migrate::table_exists(&tx, "doc_access")? {
            tx.execute("DELETE FROM doc_access WHERE path = ?1", params![path_str])?;
        }

        tx.execute("DELETE FROM documents WHERE path = ?1", params![path_str])?;
        tx.execute(
            "DELETE FROM links WHERE source_path = ?1",
            params![path_str],
        )?;
        tx.execute(
            "DELETE FROM tags WHERE document_path = ?1",
            params![path_str],
        )?;

        if super::migrate::table_exists(&tx, "document_vectors")? {
            tx.execute(
                "DELETE FROM document_vectors WHERE id = ?1",
                params![path_str],
            )?;
        }
        if super::migrate::table_exists(&tx, "vector_metadata")? {
            tx.execute(
                "DELETE FROM vector_metadata WHERE document_id = ?1",
                params![path_str],
            )?;
        }
        if super::migrate::table_exists(&tx, "k0maru_meta")? {
            super::migrate::upsert_meta_in_tx(&tx, super::migrate::META_LINK_EDGES_DIRTY, "1")?;
        }

        tx.commit()?;
        Ok(())
    }

    fn search_fts(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<Document>, Box<dyn std::error::Error>> {
        self.search_fts_with_tier(query, limit)
            .map(|(docs, _)| docs)
    }

    fn search_scoped_fts(
        &self,
        query: &str,
        path_prefix: Option<&str>,
        hierarchy: Option<&str>,
        limit: usize,
    ) -> Result<Vec<Document>, Box<dyn std::error::Error>> {
        let trimmed = query.trim();
        if trimmed.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }

        let search_sql = r#"
        SELECT d.path, d.title, d.hierarchy, d.frontmatter, d.content_hash, d.mtime, d.body, d.body_line_offset
        FROM fts_documents f
        JOIN documents d ON d.id = f.rowid
        WHERE fts_documents MATCH ?1
          AND (?2 IS NULL OR d.path LIKE ?2 || '%' OR replace(d.path, '\', '/') LIKE ?2 || '%')
          AND (?3 IS NULL OR d.hierarchy = ?3)
        ORDER BY f.rank
        LIMIT ?4
        "#;

        let tiers = [
            crate::text::FtsQueryTier::Strict,
            crate::text::FtsQueryTier::AllTokens,
            crate::text::FtsQueryTier::AnyTokens,
        ];
        let mut last_query: Option<String> = None;
        let mut raw_docs = Vec::new();

        for tier in tiers {
            let Some(fts_query) = crate::text::build_fts_query(trimmed, tier) else {
                continue;
            };
            if last_query.as_deref() == Some(fts_query.as_str()) {
                continue;
            }
            last_query = Some(fts_query.clone());
            let docs =
                self.query_raw_docs_scoped(search_sql, &fts_query, path_prefix, hierarchy, limit)?;
            if !docs.is_empty() {
                raw_docs = docs;
                break;
            }
        }

        self.hydrate_raw_docs(raw_docs)
    }

    fn get_outgoing_links(&self, path: &Path) -> Result<Vec<WikiLink>, Box<dyn std::error::Error>> {
        let path_str = path.to_string_lossy();
        let mut stmt = self.conn.prepare_cached(
            "SELECT target, alias, raw_text FROM links WHERE source_path = ?1 ORDER BY rowid ASC",
        )?;

        let links = stmt
            .query_map(params![path_str], |row| {
                let target: String = row.get(0)?;
                let alias: Option<String> = row.get(1)?;
                let raw_text: String = row.get(2)?;
                Ok(WikiLink {
                    target,
                    alias,
                    raw_text,
                })
            })?
            .collect::<Result<Vec<WikiLink>, _>>()?;

        Ok(links)
    }

    fn get_backlinks(
        &self,
        target_title: &str,
    ) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        let clean = target_title.strip_suffix(".md").unwrap_or(target_title);
        let with_md = format!("{}.md", clean);
        let stem = Path::new(clean)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or(clean);
        let stem_with_md = format!("{}.md", stem);

        // Pattern matching any path ending with the card title (e.g. 20_Cards/Arch matches Arch)
        let like_clean = format!("%/{}", clean);
        let like_stem = format!("%/{}", stem);

        let mut stmt = self.conn.prepare_cached(
            r#"
            SELECT DISTINCT source_path FROM links
            WHERE target = ?1 COLLATE NOCASE
               OR target = ?2 COLLATE NOCASE
               OR target = ?3 COLLATE NOCASE
               OR target = ?4 COLLATE NOCASE
               OR target LIKE ?5 COLLATE NOCASE
               OR target LIKE ?6 COLLATE NOCASE
            ORDER BY source_path ASC
            "#,
        )?;

        let paths = stmt
            .query_map(
                params![clean, with_md, stem, stem_with_md, like_clean, like_stem,],
                |row| {
                    let s: String = row.get(0)?;
                    Ok(PathBuf::from(s))
                },
            )?
            .collect::<Result<Vec<PathBuf>, _>>()?;

        Ok(paths)
    }

    fn get_cached_metadata(&self) -> Result<Vec<CachedDocMeta>, Box<dyn std::error::Error>> {
        let mut stmt = self
            .conn
            .prepare("SELECT path, mtime, content_hash FROM documents ORDER BY path ASC")?;

        let rows = stmt.query_map([], |row| {
            let path_str: String = row.get(0)?;
            let mtime: i64 = row.get(1)?;
            let content_hash: String = row.get(2)?;
            Ok(CachedDocMeta {
                path: PathBuf::from(path_str),
                mtime: mtime as u64,
                content_hash,
            })
        })?;

        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }

    fn get_document(&self, path: &Path) -> Result<Option<Document>, Box<dyn std::error::Error>> {
        let path_str = path.to_string_lossy();
        let mut stmt = self.conn.prepare_cached(
            "SELECT path, title, hierarchy, frontmatter, content_hash, mtime, body, body_line_offset FROM documents WHERE path = ?1",
        )?;
        let mut rows = stmt.query(params![path_str])?;
        if let Some(row) = rows.next()? {
            let path_val: String = row.get(0)?;
            let title: String = row.get(1)?;
            let hierarchy_str: String = row.get(2)?;
            let frontmatter_str: String = row.get(3)?;
            let content_hash: String = row.get(4)?;
            let mtime: i64 = row.get(5)?;
            let body: String = row.get(6)?;
            let body_line_offset: i64 = row.get(7)?;

            let path_buf = PathBuf::from(path_val);
            let links = self.get_outgoing_links(&path_buf)?;
            let mut tag_stmt = self
                .conn
                .prepare_cached("SELECT tag FROM tags WHERE document_path = ?1")?;
            let tags: Vec<String> = tag_stmt
                .query_map(params![path_str], |r| r.get(0))?
                .collect::<Result<Vec<String>, _>>()?;

            let hierarchy = parse_hierarchy(&hierarchy_str);
            let frontmatter =
                serde_json::from_str(&frontmatter_str).unwrap_or(serde_json::json!({}));

            Ok(Some(Document {
                path: path_buf,
                title,
                hierarchy,
                frontmatter,
                links,
                tags,
                content_hash,
                mtime: mtime as u64,
                body,
                body_line_offset: body_line_offset.max(0) as usize,
            }))
        } else {
            Ok(None)
        }
    }

    fn search_vectors(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<(String, f32)>, Box<dyn std::error::Error>> {
        SqliteStorage::search_vectors(self, query_embedding, limit)
    }

    fn repair_stale_fts(&mut self) -> Result<usize, Box<dyn std::error::Error>> {
        SqliteStorage::repair_stale_fts(self)
    }

    fn repair_stale_sections(
        &mut self,
        budget: Option<usize>,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        SqliteStorage::repair_stale_sections(self, budget)
    }

    fn rebuild_link_edges_if_dirty(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        SqliteStorage::rebuild_link_edges_if_dirty(self)
    }

    fn meta_set(&mut self, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error>> {
        SqliteStorage::meta_set(self, key, value)
    }
}

struct RawDocRow {
    path: String,
    title: String,
    hierarchy: String,
    frontmatter: String,
    content_hash: String,
    mtime: i64,
    body: String,
    body_line_offset: i64,
}

fn parse_hierarchy(s: &str) -> HierarchyLevel {
    match s {
        "L0Ephemeral" => HierarchyLevel::L0Ephemeral,
        "L1Resource" => HierarchyLevel::L1Resource,
        "L2Log" => HierarchyLevel::L2Log,
        "L3Evergreen" => HierarchyLevel::L3Evergreen,
        _ => serde_json::from_str(&format!("\"{}\"", s)).unwrap_or(HierarchyLevel::L0Ephemeral),
    }
}

/// Legacy FTS5 query sanitizer retained as a deprecated wrapper around [`crate::text::build_fts_query`].
#[deprecated(note = "Use k0maru_core::text::build_fts_query instead")]
pub fn sanitize_fts5_query(query: &str) -> String {
    crate::text::build_fts_query(query, crate::text::FtsQueryTier::Strict).unwrap_or_default()
}

/// Legacy FTS5 OR-query sanitizer retained as a deprecated wrapper around [`crate::text::build_fts_query`].
#[deprecated(note = "Use k0maru_core::text::build_fts_query instead")]
pub fn sanitize_fts5_or_query(query: &str) -> String {
    crate::text::build_fts_query(query, crate::text::FtsQueryTier::AnyTokens).unwrap_or_default()
}

fn f32_slice_to_bytes(slice: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(std::mem::size_of_val(slice));
    for &val in slice {
        bytes.extend_from_slice(&val.to_le_bytes());
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_doc(
        path: &str,
        title: &str,
        hierarchy: HierarchyLevel,
        body: &str,
        tags: Vec<&str>,
    ) -> Document {
        Document {
            path: PathBuf::from(path),
            title: title.to_string(),
            hierarchy,
            frontmatter: serde_json::json!({ "title": title }),
            links: vec![],
            tags: tags.into_iter().map(|s| s.to_string()).collect(),
            content_hash: format!("hash_{}", path),
            mtime: 1774843200,
            body: body.to_string(),
            body_line_offset: 0,
        }
    }

    #[test]
    fn test_search_scoped_fts_unit_test() {
        let mut storage = SqliteStorage::in_memory().expect("in-memory sqlite");
        storage.initialize().expect("init");

        // 10 daily logs
        for i in 1..=10 {
            let doc = make_test_doc(
                &format!("00_Daily/2026-10-{:02}.md", i),
                &format!("Telemetry Log {}", i),
                HierarchyLevel::L0Ephemeral,
                "Telemetry Telemetry Telemetry Telemetry Telemetry distributed tracing pipelines today.",
                vec!["daily", "telemetry"],
            );
            storage.upsert_document(&doc).expect("upsert daily");
        }

        // 1 project document
        let proj = make_test_doc(
            "10_Projects/Telemetry.md",
            "Telemetry Engine",
            HierarchyLevel::L1Resource,
            "Core architecture for Telemetry system.",
            vec!["project", "topic/telemetry-engine"],
        );
        storage.upsert_document(&proj).expect("upsert project");

        let scoped = storage
            .search_scoped_fts("Telemetry", Some("10_Projects/"), None, 5)
            .expect("search_scoped_fts");
        assert_eq!(scoped.len(), 1);
        assert_eq!(scoped[0].path, PathBuf::from("10_Projects/Telemetry.md"));
        assert_eq!(scoped[0].title, "Telemetry Engine");

        let scoped_hierarchy = storage
            .search_scoped_fts("Telemetry", None, Some("L1Resource"), 5)
            .expect("hierarchy scoped");
        assert_eq!(scoped_hierarchy.len(), 1);
        assert_eq!(
            scoped_hierarchy[0].path,
            PathBuf::from("10_Projects/Telemetry.md")
        );

        let scoped_hyphen = storage
            .search_scoped_fts("telemetry-system", Some("10_Projects/"), None, 5)
            .expect("hyphen scoped");
        assert_eq!(scoped_hyphen.len(), 1);
        assert_eq!(
            scoped_hyphen[0].path,
            PathBuf::from("10_Projects/Telemetry.md")
        );
    }

    #[test]
    fn test_search_fts_hyphenated_query() {
        let mut storage = SqliteStorage::in_memory().expect("in-memory sqlite");
        storage.initialize().expect("init");

        let doc = make_test_doc(
            "20_Cards/pre-commit-hooks.md",
            "Git Pre-commit Guardrails",
            HierarchyLevel::L3Evergreen,
            "Installing pre-commit hooks prevents bad commits.",
            vec!["git", "pre-commit"],
        );
        storage.upsert_document(&doc).expect("upsert doc");

        let results = storage
            .search_fts("pre-commit", 5)
            .expect("search_fts hyphen");
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].path,
            PathBuf::from("20_Cards/pre-commit-hooks.md")
        );
    }
}
