//! SQLite schema v3 migration, FTS5 stale-index repair, and lazy section repair.
//!
//! Supports a versioned migration ladder (`v0/v1 -> v2 -> v3`) without disturbing
//! existing `document_vectors` embeddings and without parsing/embedding inside `open_rw`.

use std::collections::{HashMap, HashSet, VecDeque};

use rusqlite::{params, Connection};

use crate::parser::sections::{
    section_text_hash, sections_hash_for, split_sections, SECTION_PARSER_VERSION,
};
use crate::text::{fts_hash_for, segment_for_index, FTS_SEGMENTER_VERSION};

/// Current SQLite cache schema version stored in `PRAGMA user_version`.
pub const CURRENT_SCHEMA_VERSION: i32 = 3;

/// Metadata key storing the active FTS segmenter version in `k0maru_meta`.
pub const META_FTS_SEGMENTER_VERSION: &str = "fts_segmenter_version";
/// Metadata key storing the active section splitter version in `k0maru_meta`.
pub const META_SECTION_PARSER_VERSION: &str = "section_parser_version";
/// Metadata key storing the embedding model name of the last vector sync.
pub const META_EMBED_MODEL: &str = "embed_model";
/// Metadata key storing the embedding dimension of the last vector sync.
pub const META_EMBED_DIM: &str = "embed_dim";
/// Metadata key indicating whether all sections are embedded for `embed_model` (`'1'` or `'0'`).
pub const META_SECTION_VECTORS_COMPLETE: &str = "section_vectors_complete";
/// Metadata key indicating whether `link_edges` needs rebuilding (`'1'` or `'0'`).
pub const META_LINK_EDGES_DIRTY: &str = "link_edges_dirty";
/// Metadata key storing the Unix timestamp (seconds) of the last completed sync.
pub const META_LAST_SYNC_AT: &str = "last_sync_at";
/// Metadata key storing the git HEAD SHA used for the last `doc_git_time` backfill.
pub const META_GIT_TIME_HEAD: &str = "git_time_head";

struct TxGuard<'a> {
    conn: &'a Connection,
    committed: bool,
}

impl<'a> TxGuard<'a> {
    fn begin_immediate(conn: &'a Connection) -> Result<Self, rusqlite::Error> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        let mut delay_ms = 20u64;
        loop {
            match conn.execute_batch("BEGIN IMMEDIATE;") {
                Ok(()) => {
                    return Ok(Self {
                        conn,
                        committed: false,
                    });
                }
                Err(rusqlite::Error::SqliteFailure(e, _))
                    if e.code == rusqlite::ErrorCode::DatabaseBusy
                        && std::time::Instant::now() < deadline =>
                {
                    std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                    delay_ms = (delay_ms * 2).min(250);
                }
                Err(e) => return Err(e),
            }
        }
    }

    fn commit(mut self) -> Result<(), rusqlite::Error> {
        self.conn.execute_batch("COMMIT;")?;
        self.committed = true;
        Ok(())
    }
}

impl Drop for TxGuard<'_> {
    fn drop(&mut self) {
        if !self.committed {
            let _ = self.conn.execute_batch("ROLLBACK;");
        }
    }
}

/// Returns `true` if a table or virtual table with `name` exists in `sqlite_master`.
pub(crate) fn table_exists(conn: &Connection, name: &str) -> Result<bool, rusqlite::Error> {
    let count: i64 = conn.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
        params![name],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

pub(crate) fn upsert_meta_in_tx(
    conn: &Connection,
    key: &str,
    value: &str,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        r#"
        INSERT INTO k0maru_meta (key, value)
        VALUES (?1, ?2)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value
        "#,
        params![key, value],
    )?;
    Ok(())
}

/// Validates `PRAGMA user_version <= CURRENT_SCHEMA_VERSION` without performing writes.
pub fn validate_schema_version_read_only(
    conn: &Connection,
) -> Result<i32, Box<dyn std::error::Error>> {
    let user_version: i32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if user_version > CURRENT_SCHEMA_VERSION {
        return Err(format!(
            "Unsupported SQLite cache schema version {} (supported up to {}). Upgrade k0maru before reading this cache.",
            user_version, CURRENT_SCHEMA_VERSION
        )
        .into());
    }
    Ok(user_version)
}

/// Ensures the SQLite database is initialized or migrated to [`CURRENT_SCHEMA_VERSION`] (v3).
///
/// Returns `Ok(true)` if an existing database was migrated (`v0/v1 -> v3` or `v2 -> v3`),
/// or `Ok(false)` if the database was freshly initialized or already on v3.
pub fn ensure_schema(conn: &Connection) -> Result<bool, Box<dyn std::error::Error>> {
    let user_version: i32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if user_version > CURRENT_SCHEMA_VERSION {
        return Err(format!(
            "Unsupported SQLite cache schema version {} (supported up to {}). Refusing downgrade write.",
            user_version, CURRENT_SCHEMA_VERSION
        )
        .into());
    }

    if user_version == CURRENT_SCHEMA_VERSION
        && table_exists(conn, "documents")?
        && table_exists(conn, "fts_documents")?
        && table_exists(conn, "k0maru_meta")?
        && table_exists(conn, "document_sections")?
    {
        return Ok(false);
    }

    let (migrated, ran_v2) = {
        let guard = TxGuard::begin_immediate(conn)?;

        let locked_version: i32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if locked_version > CURRENT_SCHEMA_VERSION {
            return Err(format!(
                "Unsupported SQLite cache schema version {} (supported up to {}). Refusing downgrade write.",
                locked_version, CURRENT_SCHEMA_VERSION
            )
            .into());
        }

        if locked_version == CURRENT_SCHEMA_VERSION
            && table_exists(conn, "documents")?
            && table_exists(conn, "fts_documents")?
            && table_exists(conn, "k0maru_meta")?
            && table_exists(conn, "document_sections")?
        {
            guard.commit()?;
            return Ok(false);
        }

        let has_documents = table_exists(conn, "documents")?;
        if !has_documents {
            conn.execute_batch(super::sqlite::SCHEMA_SQL)?;
            seed_vector_schema_info_if_empty(conn)?;
            upsert_segmenter_meta(conn)?;
            upsert_meta_in_tx(
                conn,
                META_SECTION_PARSER_VERSION,
                &SECTION_PARSER_VERSION.to_string(),
            )?;
            upsert_meta_in_tx(conn, META_SECTION_VECTORS_COMPLETE, "0")?;
            upsert_meta_in_tx(conn, META_LINK_EDGES_DIRTY, "0")?;
            conn.execute_batch(&format!(
                "PRAGMA user_version = {};",
                CURRENT_SCHEMA_VERSION
            ))?;
            guard.commit()?;
            (false, false)
        } else {
            let mut ran_v2 = false;
            if locked_version < 2 {
                migrate_legacy_to_v2_in_tx(conn)?;
                ran_v2 = true;
            }
            migrate_v2_to_v3_in_tx(conn)?;
            guard.commit()?;
            (true, ran_v2)
        }
    };

    if ran_v2 {
        conn.execute(
            "INSERT INTO fts_documents(fts_documents) VALUES('optimize')",
            [],
        )?;
    }

    Ok(migrated)
}

/// Legacy alias for [`ensure_schema`].
#[deprecated(note = "Use ensure_schema instead")]
pub fn ensure_schema_v2(conn: &Connection) -> Result<bool, Box<dyn std::error::Error>> {
    ensure_schema(conn)
}

fn seed_vector_schema_info_if_empty(conn: &Connection) -> Result<(), rusqlite::Error> {
    let count: i64 = conn
        .query_row("SELECT count(*) FROM vector_schema_info", [], |r| r.get(0))
        .unwrap_or(0);
    if count == 0 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        conn.execute(
            "INSERT INTO vector_schema_info (dimension, updated_at) VALUES (?1, ?2)",
            params![super::sqlite::VECTOR_DIMENSIONS as i64, now],
        )?;
    }
    Ok(())
}

fn upsert_segmenter_meta(conn: &Connection) -> Result<(), rusqlite::Error> {
    upsert_meta_in_tx(
        conn,
        META_FTS_SEGMENTER_VERSION,
        &FTS_SEGMENTER_VERSION.to_string(),
    )
}

fn document_columns(conn: &Connection) -> Result<HashSet<String>, rusqlite::Error> {
    let mut stmt = conn.prepare("PRAGMA table_info(documents)")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
    let mut cols = HashSet::new();
    for col in rows {
        cols.insert(col?);
    }
    Ok(cols)
}

fn snapshot_vector_counts(
    conn: &Connection,
) -> Result<(Option<i64>, Option<i64>), Box<dyn std::error::Error>> {
    let has_vec_meta = table_exists(conn, "vector_metadata")?;
    let pre_vec_meta_count: Option<i64> = if has_vec_meta {
        Some(conn.query_row("SELECT count(*) FROM vector_metadata", [], |r| r.get(0))?)
    } else {
        None
    };

    let has_doc_vecs = table_exists(conn, "document_vectors")?;
    let pre_doc_vecs_count: Option<i64> = if has_doc_vecs {
        Some(conn.query_row("SELECT count(*) FROM document_vectors", [], |r| r.get(0))?)
    } else {
        None
    };

    Ok((pre_vec_meta_count, pre_doc_vecs_count))
}

fn verify_vector_counts(
    conn: &Connection,
    pre_vec_meta_count: Option<i64>,
    pre_doc_vecs_count: Option<i64>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(expected) = pre_vec_meta_count {
        let actual: i64 =
            conn.query_row("SELECT count(*) FROM vector_metadata", [], |r| r.get(0))?;
        if actual != expected {
            return Err(format!(
                "Migration invariant failed: vector_metadata count changed from {} to {}",
                expected, actual
            )
            .into());
        }
    }
    if let Some(expected) = pre_doc_vecs_count {
        let actual: i64 =
            conn.query_row("SELECT count(*) FROM document_vectors", [], |r| r.get(0))?;
        if actual != expected {
            return Err(format!(
                "Migration invariant failed: document_vectors count changed from {} to {}",
                expected, actual
            )
            .into());
        }
    }
    Ok(())
}

fn migrate_legacy_to_v2_in_tx(conn: &Connection) -> Result<(), Box<dyn std::error::Error>> {
    let (pre_vec_meta_count, pre_doc_vecs_count) = snapshot_vector_counts(conn)?;

    let cols = document_columns(conn)?;
    let offset_expr = if cols.contains("body_line_offset") {
        "COALESCE(body_line_offset, 0)"
    } else {
        "0"
    };

    conn.execute_batch(
        r#"
        CREATE TABLE documents_v2 (
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
        "#,
    )?;

    let copy_sql = format!(
        r#"
        INSERT INTO documents_v2 (
            id, path, title, hierarchy, frontmatter, content_hash, mtime, body, body_line_offset, fts_hash
        )
        SELECT rowid AS id, path, title, hierarchy, frontmatter, content_hash, mtime, body, {}, NULL
        FROM documents;
        "#,
        offset_expr
    );
    conn.execute_batch(&copy_sql)?;

    conn.execute_batch(
        r#"
        DROP TABLE documents;
        ALTER TABLE documents_v2 RENAME TO documents;

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

        DROP TABLE IF EXISTS documents_fts;

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
        "#,
    )?;

    populate_fts_for_documents(conn, false)?;
    upsert_segmenter_meta(conn)?;

    verify_vector_counts(conn, pre_vec_meta_count, pre_doc_vecs_count)?;

    conn.execute_batch("PRAGMA user_version = 2;")?;
    Ok(())
}

fn migrate_v2_to_v3_in_tx(conn: &Connection) -> Result<(), Box<dyn std::error::Error>> {
    let (pre_vec_meta_count, pre_doc_vecs_count) = snapshot_vector_counts(conn)?;

    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS k0maru_meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

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
        "#,
    )?;

    if table_exists(conn, "document_vectors")? {
        let dim = super::sqlite::get_vector_dimension_conn(conn)?
            .unwrap_or(super::sqlite::VECTOR_DIMENSIONS);
        let vec_sql = super::sqlite::section_vector_ddl(dim);
        conn.execute_batch(&vec_sql)?;
    }

    // F15 fix: legacy rows with frontmatter kept body_line_offset = 0 across v1->v2 migration.
    // Force a one-time re-read on the next incremental sync so line ranges reflect frontmatter.
    conn.execute_batch(
        "UPDATE documents SET mtime = -1 WHERE body_line_offset = 0 AND frontmatter != '{}';",
    )?;

    upsert_meta_in_tx(
        conn,
        META_SECTION_PARSER_VERSION,
        &SECTION_PARSER_VERSION.to_string(),
    )?;
    upsert_meta_in_tx(conn, META_SECTION_VECTORS_COMPLETE, "0")?;
    upsert_meta_in_tx(conn, META_LINK_EDGES_DIRTY, "1")?;

    verify_vector_counts(conn, pre_vec_meta_count, pre_doc_vecs_count)?;

    conn.execute_batch(&format!(
        "PRAGMA user_version = {};",
        CURRENT_SCHEMA_VERSION
    ))?;
    Ok(())
}

/// Incrementally diffs and updates `document_sections`, `fts_sections`, and `document_section_state`
/// for a single document inside an active transaction, reusing matching `text_hash` section IDs.
pub(crate) fn upsert_doc_sections_in_tx(
    conn: &Connection,
    doc_id: i64,
    title: &str,
    body: &str,
    body_line_offset: usize,
    content_hash: &str,
    set_links_dirty: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let spans = split_sections(title, body, body_line_offset);

    let mut old_by_hash: HashMap<i64, VecDeque<i64>> = HashMap::new();
    {
        let mut stmt = conn.prepare_cached(
            "SELECT id, text_hash FROM document_sections WHERE doc_id = ?1 ORDER BY ord ASC",
        )?;
        let rows = stmt.query_map(params![doc_id], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?))
        })?;
        for row in rows {
            let (id, text_hash) = row?;
            old_by_hash.entry(text_hash).or_default().push_back(id);
        }
    }

    let mut structure_changed = false;

    {
        let mut upd_sec = conn.prepare_cached(
            r#"
            UPDATE document_sections
            SET ord = ?1, level = ?2, heading = ?3, breadcrumb = ?4,
                byte_start = ?5, byte_end = ?6, start_line = ?7, end_line = ?8
            WHERE id = ?9
            "#,
        )?;
        let mut ins_sec = conn.prepare_cached(
            r#"
            INSERT INTO document_sections (
                doc_id, ord, level, heading, breadcrumb,
                byte_start, byte_end, start_line, end_line, text_hash
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            RETURNING id
            "#,
        )?;
        let mut ins_fts = conn.prepare_cached(
            "INSERT INTO fts_sections (rowid, heading, body) VALUES (?1, ?2, ?3)",
        )?;

        for span in &spans {
            let section_text = &body[span.byte_start..span.byte_end];
            let text_hash = section_text_hash(&span.breadcrumb, section_text);

            if let Some(old_id) = old_by_hash.get_mut(&text_hash).and_then(|q| q.pop_front()) {
                upd_sec.execute(params![
                    span.ord as i64,
                    span.level as i64,
                    span.heading,
                    span.breadcrumb,
                    span.byte_start as i64,
                    span.byte_end as i64,
                    span.start_line as i64,
                    span.end_line as i64,
                    old_id,
                ])?;
            } else {
                structure_changed = true;
                let sec_id: i64 = ins_sec.query_row(
                    params![
                        doc_id,
                        span.ord as i64,
                        span.level as i64,
                        span.heading,
                        span.breadcrumb,
                        span.byte_start as i64,
                        span.byte_end as i64,
                        span.start_line as i64,
                        span.end_line as i64,
                        text_hash,
                    ],
                    |r| r.get(0),
                )?;
                let seg_heading = segment_for_index(&span.breadcrumb);
                let seg_body = segment_for_index(section_text);
                ins_fts.execute(params![sec_id, seg_heading, seg_body])?;
            }
        }
    }

    let unmatched_old_ids: Vec<i64> = old_by_hash.into_values().flatten().collect();
    if !unmatched_old_ids.is_empty() {
        structure_changed = true;
        let has_sec_vecs = table_exists(conn, "section_vectors")?;
        let has_sec_vec_meta = table_exists(conn, "section_vector_meta")?;

        let mut del_fts = conn.prepare_cached("DELETE FROM fts_sections WHERE rowid = ?1")?;
        let mut del_sec = conn.prepare_cached("DELETE FROM document_sections WHERE id = ?1")?;

        for old_id in unmatched_old_ids {
            del_fts.execute(params![old_id])?;
            if has_sec_vecs {
                conn.execute(
                    "DELETE FROM section_vectors WHERE section_id = ?1",
                    params![old_id],
                )?;
            }
            if has_sec_vec_meta {
                conn.execute(
                    "DELETE FROM section_vector_meta WHERE section_id = ?1",
                    params![old_id],
                )?;
            }
            del_sec.execute(params![old_id])?;
        }
    }

    let sec_hash = sections_hash_for(content_hash);
    conn.execute(
        r#"
        INSERT INTO document_section_state (doc_id, sections_hash)
        VALUES (?1, ?2)
        ON CONFLICT(doc_id) DO UPDATE SET sections_hash = excluded.sections_hash
        "#,
        params![doc_id, sec_hash],
    )?;

    if structure_changed {
        upsert_meta_in_tx(conn, META_SECTION_VECTORS_COMPLETE, "0")?;
    }
    if set_links_dirty {
        upsert_meta_in_tx(conn, META_LINK_EDGES_DIRTY, "1")?;
    }

    Ok(())
}

/// Repairs stale or missing `fts_documents` entries where `fts_hash != fts_hash_for(content_hash)`
/// and drops any stray legacy `documents_fts` table recreated by older binaries.
///
/// Returns the number of documents re-indexed.
pub fn repair_stale_fts(conn: &Connection) -> Result<usize, Box<dyn std::error::Error>> {
    let has_legacy_fts = table_exists(conn, "documents_fts")?;
    let expected_ver = FTS_SEGMENTER_VERSION.to_string();

    let stale_count: i64 = conn.query_row(
        "SELECT count(*) FROM documents WHERE fts_hash IS NULL OR fts_hash != (content_hash || ':' || ?1)",
        params![expected_ver],
        |row| row.get(0),
    )?;

    if !has_legacy_fts && stale_count == 0 {
        return Ok(0);
    }

    let guard = TxGuard::begin_immediate(conn)?;
    if has_legacy_fts {
        conn.execute_batch("DROP TABLE IF EXISTS documents_fts;")?;
    }
    let repaired = populate_fts_for_documents(conn, true)?;
    upsert_segmenter_meta(conn)?;
    guard.commit()?;

    Ok(repaired)
}

/// Lazily repairs stale or missing section indices up to `budget` documents (`None` = unlimited),
/// purging orphaned section, state, and section-vector rows first.
///
/// Returns the number of documents whose sections were repaired.
pub fn repair_stale_sections(
    conn: &Connection,
    budget: Option<usize>,
) -> Result<usize, Box<dyn std::error::Error>> {
    if !table_exists(conn, "document_sections")?
        || !table_exists(conn, "document_section_state")?
        || !table_exists(conn, "fts_sections")?
    {
        return Ok(0);
    }

    let expected_suffix = format!(":s{}:f{}", SECTION_PARSER_VERSION, FTS_SEGMENTER_VERSION);
    let has_sec_vecs = table_exists(conn, "section_vectors")?;
    let has_sec_vec_meta = table_exists(conn, "section_vector_meta")?;

    let has_orphan_sections: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM document_sections WHERE doc_id NOT IN (SELECT id FROM documents))",
        [],
        |r| r.get(0),
    )?;
    let has_orphan_state: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM document_section_state WHERE doc_id NOT IN (SELECT id FROM documents))",
        [],
        |r| r.get(0),
    )?;
    let has_orphan_vec_meta: bool = if has_sec_vec_meta {
        conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM section_vector_meta WHERE section_id NOT IN (SELECT id FROM document_sections))",
            [],
            |r| r.get(0),
        )?
    } else {
        false
    };
    let stale_count: i64 = conn.query_row(
        r#"
        SELECT count(*)
        FROM documents d
        LEFT JOIN document_section_state s ON s.doc_id = d.id
        WHERE s.sections_hash IS NULL OR s.sections_hash != (d.content_hash || ?1)
        "#,
        params![expected_suffix],
        |r| r.get(0),
    )?;

    let has_orphans = has_orphan_sections || has_orphan_state || has_orphan_vec_meta;
    if !has_orphans && (stale_count == 0 || budget == Some(0)) {
        return Ok(0);
    }

    let guard = TxGuard::begin_immediate(conn)?;

    // Purge orphans from legacy writers or doc-id reuse before repairing stale documents.
    if has_orphan_sections {
        conn.execute(
            "DELETE FROM fts_sections WHERE rowid IN (SELECT id FROM document_sections WHERE doc_id NOT IN (SELECT id FROM documents))",
            [],
        )?;
        if has_sec_vecs {
            conn.execute(
                "DELETE FROM section_vectors WHERE section_id IN (SELECT id FROM document_sections WHERE doc_id NOT IN (SELECT id FROM documents))",
                [],
            )?;
        }
        if has_sec_vec_meta {
            conn.execute(
                "DELETE FROM section_vector_meta WHERE section_id IN (SELECT id FROM document_sections WHERE doc_id NOT IN (SELECT id FROM documents))",
                [],
            )?;
        }
        conn.execute(
            "DELETE FROM document_sections WHERE doc_id NOT IN (SELECT id FROM documents)",
            [],
        )?;
    }

    if has_orphan_state {
        conn.execute(
            "DELETE FROM document_section_state WHERE doc_id NOT IN (SELECT id FROM documents)",
            [],
        )?;
    }

    if has_sec_vecs {
        conn.execute(
            "DELETE FROM section_vectors WHERE section_id NOT IN (SELECT id FROM document_sections)",
            [],
        )?;
    }
    if has_sec_vec_meta {
        conn.execute(
            "DELETE FROM section_vector_meta WHERE section_id NOT IN (SELECT id FROM document_sections)",
            [],
        )?;
    }

    let mut repaired = 0usize;
    if budget != Some(0) && stale_count > 0 {
        let limit_val: i64 = match budget {
            Some(b) => i64::try_from(b).unwrap_or(i64::MAX),
            None => -1,
        };
        let rows: Vec<(i64, String, String, i64, String)> = {
            let mut stmt = conn.prepare(
                r#"
                SELECT d.id, d.title, d.body, d.body_line_offset, d.content_hash
                FROM documents d
                LEFT JOIN document_section_state s ON s.doc_id = d.id
                WHERE s.sections_hash IS NULL OR s.sections_hash != (d.content_hash || ?1)
                ORDER BY d.id ASC
                LIMIT ?2
                "#,
            )?;
            let mapped = stmt.query_map(params![expected_suffix, limit_val], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
            })?;
            mapped.collect::<Result<Vec<_>, _>>()?
        };

        for (doc_id, title, body, offset, content_hash) in rows {
            upsert_doc_sections_in_tx(
                conn,
                doc_id,
                &title,
                &body,
                offset.max(0) as usize,
                &content_hash,
                false,
            )?;
            repaired += 1;
        }
    }

    upsert_meta_in_tx(
        conn,
        META_SECTION_PARSER_VERSION,
        &SECTION_PARSER_VERSION.to_string(),
    )?;
    guard.commit()?;

    Ok(repaired)
}

fn populate_fts_for_documents(
    conn: &Connection,
    only_stale: bool,
) -> Result<usize, Box<dyn std::error::Error>> {
    let expected_ver = FTS_SEGMENTER_VERSION.to_string();
    let rows: Vec<(i64, String, String, String, String)> = if only_stale {
        let mut stmt = conn.prepare(
            "SELECT id, path, title, body, content_hash FROM documents WHERE fts_hash IS NULL OR fts_hash != (content_hash || ':' || ?1) ORDER BY id ASC",
        )?;
        let mapped = stmt.query_map(params![expected_ver], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })?;
        mapped.collect::<Result<Vec<_>, _>>()?
    } else {
        let mut stmt = conn
            .prepare("SELECT id, path, title, body, content_hash FROM documents ORDER BY id ASC")?;
        let mapped = stmt.query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })?;
        mapped.collect::<Result<Vec<_>, _>>()?
    };

    let mut tag_stmt =
        conn.prepare_cached("SELECT tag FROM tags WHERE document_path = ?1 ORDER BY rowid ASC")?;
    let mut del_fts_stmt = conn.prepare_cached("DELETE FROM fts_documents WHERE rowid = ?1")?;
    let mut ins_fts_stmt = conn.prepare_cached(
        "INSERT INTO fts_documents (rowid, title, body, tags) VALUES (?1, ?2, ?3, ?4)",
    )?;
    let mut upd_hash_stmt =
        conn.prepare_cached("UPDATE documents SET fts_hash = ?1 WHERE id = ?2")?;

    let mut count = 0usize;
    for (id, path, title, body, content_hash) in rows {
        let tags: Vec<String> = tag_stmt
            .query_map(params![path], |r| r.get(0))?
            .collect::<Result<Vec<String>, _>>()?;
        let tags_str = tags.join(" ");

        let seg_title = segment_for_index(&title);
        let seg_body = segment_for_index(&body);
        let seg_tags = segment_for_index(&tags_str);
        let hash = fts_hash_for(&content_hash);

        if only_stale {
            del_fts_stmt.execute(params![id])?;
        }
        ins_fts_stmt.execute(params![id, seg_title, seg_body, seg_tags])?;
        upd_hash_stmt.execute(params![hash, id])?;
        count += 1;
    }

    Ok(count)
}
