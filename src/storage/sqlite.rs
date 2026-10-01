//! SQLite and FTS5 cache storage engine.
//!
//! Provides disposable, high-performance local SQLite caching implementing
//! [`CacheStorage`], with BM25-ranked full-text search and link graph index.

use std::path::{Path, PathBuf};
use std::sync::Once;

use rusqlite::params;

use crate::core::models::{CachedDocMeta, Document, HierarchyLevel, WikiLink};
use crate::core::traits::CacheStorage;

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

/// Ephemeral SQLite-backed storage implementing [`CacheStorage`].
pub struct SqliteStorage {
    conn: rusqlite::Connection,
}

impl SqliteStorage {
    /// Default embedding dimensions for document vectors.
    pub const VECTOR_DIMENSIONS: usize = VECTOR_DIMENSIONS;

    /// Opens or creates a SQLite cache database at the specified file path.
    ///
    /// Automatically ensures parent directories exist and runs schema initialization.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        ensure_sqlite_vec_registered();
        let path_ref = path.as_ref();
        if let Some(parent) = path_ref.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let conn = rusqlite::Connection::open(path_ref)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        let mut storage = Self { conn };
        storage.initialize()?;
        Ok(storage)
    }

    /// Creates an in-memory SQLite cache database (ideal for tests and ephemeral runs).
    pub fn in_memory() -> Result<Self, Box<dyn std::error::Error>> {
        ensure_sqlite_vec_registered();
        let conn = rusqlite::Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        let mut storage = Self { conn };
        storage.initialize()?;
        Ok(storage)
    }

    /// Returns an immutable reference to the underlying [`rusqlite::Connection`].
    pub fn connection(&self) -> &rusqlite::Connection {
        &self.conn
    }

    /// Returns a mutable reference to the underlying [`rusqlite::Connection`].
    pub fn connection_mut(&mut self) -> &mut rusqlite::Connection {
        &mut self.conn
    }

    /// Inserts or replaces a document vector and its corresponding metadata content hash.
    pub fn insert_vector(
        &self,
        doc_id: &str,
        embedding: &[f32],
        hash: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if embedding.len() != Self::VECTOR_DIMENSIONS {
            return Err(format!(
                "Embedding dimensions mismatch: expected {}, got {}",
                Self::VECTOR_DIMENSIONS,
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
        if query_embedding.len() != Self::VECTOR_DIMENSIONS {
            return Err(format!(
                "Embedding dimensions mismatch: expected {}, got {}",
                Self::VECTOR_DIMENSIONS,
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

    /// Retrieves a document by its relative path if present in cache.
    pub fn get_document(
        &self,
        path: &Path,
    ) -> Result<Option<Document>, Box<dyn std::error::Error>> {
        <Self as CacheStorage>::get_document(self, path)
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
            })
        })?;
        let mut docs = Vec::new();
        for row in mapped {
            docs.push(row?);
        }
        Ok(docs)
    }
}

const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS documents (
    path TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    hierarchy TEXT NOT NULL,
    frontmatter TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    mtime INTEGER NOT NULL,
    body TEXT NOT NULL
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

CREATE VIRTUAL TABLE IF NOT EXISTS documents_fts USING fts5(
    path UNINDEXED,
    title,
    body,
    tags,
    tokenize = 'unicode61'
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
"#;

impl CacheStorage for SqliteStorage {
    fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.conn.execute_batch(SCHEMA_SQL)?;
        Ok(())
    }

    fn wipe_and_rebuild(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let drop_sql = r#"
        DROP TABLE IF EXISTS vector_metadata;
        DROP TABLE IF EXISTS document_vectors;
        DROP TABLE IF EXISTS documents_fts;
        DROP TABLE IF EXISTS tags;
        DROP TABLE IF EXISTS links;
        DROP TABLE IF EXISTS documents;
        "#;
        self.conn.execute_batch(drop_sql)?;
        self.initialize()?;
        Ok(())
    }

    fn upsert_document(&mut self, doc: &Document) -> Result<(), Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;
        let path_str = doc.path.to_string_lossy();
        let frontmatter_str = serde_json::to_string(&doc.frontmatter)?;
        let tags_str = doc.tags.join(" ");

        // 1. Upsert document record
        tx.execute(
            r#"
            INSERT INTO documents (path, title, hierarchy, frontmatter, content_hash, mtime, body)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(path) DO UPDATE SET
                title = excluded.title,
                hierarchy = excluded.hierarchy,
                frontmatter = excluded.frontmatter,
                content_hash = excluded.content_hash,
                mtime = excluded.mtime,
                body = excluded.body
            "#,
            params![
                path_str,
                doc.title,
                doc.hierarchy.as_str(),
                frontmatter_str,
                doc.content_hash,
                doc.mtime as i64,
                doc.body,
            ],
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
                insert_link.execute(params![path_str, link.target, link.alias, link.raw_text,])?;
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

        // 4. Synchronize FTS entry
        tx.execute(
            "DELETE FROM documents_fts WHERE path = ?1",
            params![path_str],
        )?;
        tx.execute(
            "INSERT INTO documents_fts (path, title, body, tags) VALUES (?1, ?2, ?3, ?4)",
            params![path_str, doc.title, doc.body, tags_str],
        )?;

        tx.commit()?;
        Ok(())
    }

    fn delete_document(&mut self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;
        let path_str = path.to_string_lossy();

        tx.execute("DELETE FROM documents WHERE path = ?1", params![path_str])?;
        tx.execute(
            "DELETE FROM links WHERE source_path = ?1",
            params![path_str],
        )?;
        tx.execute(
            "DELETE FROM tags WHERE document_path = ?1",
            params![path_str],
        )?;
        tx.execute(
            "DELETE FROM documents_fts WHERE path = ?1",
            params![path_str],
        )?;
        let _ = tx.execute(
            "DELETE FROM document_vectors WHERE id = ?1",
            params![path_str],
        );
        let _ = tx.execute(
            "DELETE FROM vector_metadata WHERE document_id = ?1",
            params![path_str],
        );

        tx.commit()?;
        Ok(())
    }

    fn search_fts(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<Document>, Box<dyn std::error::Error>> {
        let trimmed = query.trim();
        if trimmed.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }

        // Execute query, falling back to quoted tokens if user query hits FTS syntax error
        let search_sql = r#"
        SELECT d.path, d.title, d.hierarchy, d.frontmatter, d.content_hash, d.mtime, d.body
        FROM documents_fts
        JOIN documents d ON d.path = documents_fts.path
        WHERE documents_fts MATCH ?1
        ORDER BY rank
        LIMIT ?2
        "#;

        let raw_docs = match self.query_raw_docs(search_sql, trimmed, limit) {
            Ok(docs) => docs,
            Err(_) => {
                let sanitized = sanitize_fts5_query(trimmed);
                if sanitized.is_empty() {
                    return Ok(Vec::new());
                }
                self.query_raw_docs(search_sql, &sanitized, limit)?
            }
        };

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
            });
        }

        Ok(results)
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
            "SELECT path, title, hierarchy, frontmatter, content_hash, mtime, body FROM documents WHERE path = ?1",
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
            }))
        } else {
            Ok(None)
        }
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

fn sanitize_fts5_query(query: &str) -> String {
    let mut terms = Vec::new();
    for token in query.split_whitespace() {
        let cleaned = token.replace('"', "\"\"");
        let trimmed = cleaned.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
        if !trimmed.is_empty() {
            terms.push(format!("\"{}\"", trimmed));
        }
    }
    terms.join(" ")
}

fn f32_slice_to_bytes(slice: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(std::mem::size_of_val(slice));
    for &val in slice {
        bytes.extend_from_slice(&val.to_le_bytes());
    }
    bytes
}
