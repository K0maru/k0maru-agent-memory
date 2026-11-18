//! System, vault, storage, and agent ecosystem health diagnostics.
//!
//! Provides the engine and terminal formatter for `k0maru doctor`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

use crate::ecosystem::{inspect_all_clients, McpClient};

/// Severity level of a diagnostic health check result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StatusLevel {
    Pass,
    Warn,
    Fail,
}

impl StatusLevel {
    /// Visual icon indicator for console display.
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Pass => "✓ PASS",
            Self::Warn => "⚠ WARN",
            Self::Fail => "✗ FAIL",
        }
    }

    /// Colorized terminal label.
    pub fn colored_label(&self) -> String {
        match self {
            Self::Pass => "\x1b[32m✓ PASS\x1b[0m".to_string(),
            Self::Warn => "\x1b[33m⚠ WARN\x1b[0m".to_string(),
            Self::Fail => "\x1b[31m✗ FAIL\x1b[0m".to_string(),
        }
    }
}

/// An individual diagnostic check result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticItem {
    pub category: String,
    pub name: String,
    pub status: StatusLevel,
    pub detail: String,
    pub suggestion: Option<String>,
}

impl DiagnosticItem {
    pub fn pass(
        category: impl Into<String>,
        name: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            category: category.into(),
            name: name.into(),
            status: StatusLevel::Pass,
            detail: detail.into(),
            suggestion: None,
        }
    }

    pub fn warn(
        category: impl Into<String>,
        name: impl Into<String>,
        detail: impl Into<String>,
        suggestion: Option<String>,
    ) -> Self {
        Self {
            category: category.into(),
            name: name.into(),
            status: StatusLevel::Warn,
            detail: detail.into(),
            suggestion,
        }
    }

    pub fn fail(
        category: impl Into<String>,
        name: impl Into<String>,
        detail: impl Into<String>,
        suggestion: Option<String>,
    ) -> Self {
        Self {
            category: category.into(),
            name: name.into(),
            status: StatusLevel::Fail,
            detail: detail.into(),
            suggestion,
        }
    }
}

/// Summary counts across all diagnostic checks in a report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoctorSummary {
    pub total: usize,
    pub passed: usize,
    pub warnings: usize,
    pub failures: usize,
}

/// Comprehensive health diagnostics report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoctorReport {
    pub items: Vec<DiagnosticItem>,
    pub summary: DoctorSummary,
}

impl DoctorReport {
    /// Builds a `DoctorReport` from a collection of diagnostic items, computing summary totals.
    pub fn from_items(items: Vec<DiagnosticItem>) -> Self {
        let total = items.len();
        let mut passed = 0;
        let mut warnings = 0;
        let mut failures = 0;

        for item in &items {
            match item.status {
                StatusLevel::Pass => passed += 1,
                StatusLevel::Warn => warnings += 1,
                StatusLevel::Fail => failures += 1,
            }
        }

        Self {
            items,
            summary: DoctorSummary {
                total,
                passed,
                warnings,
                failures,
            },
        }
    }
}

/// Probe 1: Inspects the running binary, version, PATH availability, and system architecture.
pub fn probe_binary() -> Vec<DiagnosticItem> {
    let cat = "Binary & System";
    let mut items = Vec::new();

    // 1. Current executable path
    match std::env::current_exe() {
        Ok(exe_path) => {
            items.push(DiagnosticItem::pass(
                cat,
                "Executable Path",
                exe_path.display().to_string(),
            ));
        }
        Err(e) => {
            items.push(DiagnosticItem::warn(
                cat,
                "Executable Path",
                format!("Unable to determine current executable path: {}", e),
                None,
            ));
        }
    }

    // 2. Binary version
    items.push(DiagnosticItem::pass(
        cat,
        "k0maru Version",
        format!("v{}", crate::VERSION),
    ));

    // 3. $PATH availability
    let in_path = is_in_path("k0maru");
    if in_path {
        items.push(DiagnosticItem::pass(
            cat,
            "PATH Resolution",
            "k0maru is available in system $PATH",
        ));
    } else {
        items.push(DiagnosticItem::warn(
            cat,
            "PATH Resolution",
            "k0maru not found in system $PATH",
            Some("Add ~/.cargo/bin or your binary installation directory to $PATH".to_string()),
        ));
    }

    // 4. Platform info
    items.push(DiagnosticItem::pass(
        cat,
        "Operating System",
        format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
    ));

    items
}

/// Helper to check whether a binary name exists in any directory listed in $PATH.
fn is_in_path(binary_name: &str) -> bool {
    let exe_name = if cfg!(target_os = "windows") {
        format!("{}.exe", binary_name)
    } else {
        binary_name.to_string()
    };

    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let full = dir.join(&exe_name);
            if full.is_file() {
                return true;
            }
        }
    }

    false
}

/// Probe 2: Inspects target vault path, directory structure, markdown document count, and hierarchy.
pub fn probe_vault(vault_path: &Path) -> Vec<DiagnosticItem> {
    let cat = "Vault & Hierarchy";
    let mut items = Vec::new();

    // 1. Path existence and resolution
    if !vault_path.exists() {
        items.push(DiagnosticItem::fail(
            cat,
            "Vault Path",
            format!("Path does not exist: {}", vault_path.display()),
            Some("Specify an existing directory via --vault <PATH>".to_string()),
        ));
        return items;
    }

    if !vault_path.is_dir() {
        items.push(DiagnosticItem::fail(
            cat,
            "Vault Path",
            format!("Path is not a directory: {}", vault_path.display()),
            Some("Specify a directory root via --vault <PATH>".to_string()),
        ));
        return items;
    }

    let canonical = vault_path
        .canonicalize()
        .unwrap_or_else(|_| vault_path.to_path_buf());
    items.push(DiagnosticItem::pass(
        cat,
        "Vault Path",
        canonical.display().to_string(),
    ));

    // 2. Count markdown documents
    let mut md_count = 0;
    for entry in walkdir::WalkDir::new(vault_path)
        .into_iter()
        .filter_entry(|e| {
            if e.depth() == 0 {
                return true;
            }
            let name = e.file_name().to_string_lossy();
            !name.starts_with('.') && name != "node_modules" && name != "target"
        })
        .flatten()
    {
        if entry.file_type().is_file() {
            if let Some(ext) = entry.path().extension() {
                if ext == "md" || ext == "markdown" {
                    md_count += 1;
                }
            }
        }
    }

    if md_count > 0 {
        items.push(DiagnosticItem::pass(
            cat,
            "Markdown Documents",
            format!("{} markdown notes discovered", md_count),
        ));
    } else {
        items.push(DiagnosticItem::warn(
            cat,
            "Markdown Documents",
            "0 markdown notes discovered in vault",
            Some("Add .md documents or verify vault directory path".to_string()),
        ));
    }

    // 3. Knowledge hierarchy detection
    let hierarchy_dirs = [
        "projects",
        "decisions",
        "logs",
        "10_Projects",
        "20_Cards",
        "01_AI_Logs",
        "30_Logs",
        "40_Archives",
        "Projects",
        "Cards",
        "cards",
        "adr",
        "notes",
        "wiki",
        "docs",
        "Archive",
    ];
    let mut detected: Vec<String> = hierarchy_dirs
        .iter()
        .filter(|dir| vault_path.join(dir).is_dir())
        .map(|s| s.to_string())
        .collect();

    if let Some(idx_path) = crate::adapters::find_index_markdown(vault_path) {
        if let Some(name) = idx_path.file_name().and_then(|s| s.to_str()) {
            detected.push(name.to_string());
        }
    }

    if !detected.is_empty() {
        let is_llm_wiki = detected.iter().any(|d| {
            matches!(
                d.as_str(),
                "projects" | "decisions" | "logs" | "INDEX.md" | "index.md"
            )
        });
        let label = if is_llm_wiki {
            format!("Detected LLM-Wiki structure: {}", detected.join(", "))
        } else {
            format!("Detected structure: {}", detected.join(", "))
        };
        items.push(DiagnosticItem::pass(cat, "Knowledge Hierarchy", label));
    } else if md_count > 0 {
        items.push(DiagnosticItem::pass(
            cat,
            "Knowledge Hierarchy",
            "Detected structure: Flat LLM-Wiki (root markdown topology)".to_string(),
        ));
    } else {
        items.push(DiagnosticItem::warn(
            cat,
            "Knowledge Hierarchy",
            "Standard hierarchy folders (projects, decisions, logs, or 10_Projects, 20_Cards) not detected",
            Some(
                "Organizing notes into projects/, decisions/, and logs/ (or 10_Projects, 20_Cards) optimizes agent loadout context"
                    .to_string(),
            ),
        ));
    }

    items
}

/// Probe 3: Inspects disposable SQLite cache file, FTS5 index, vector tables, and embedding backend.
pub fn probe_storage(vault_path: &Path) -> Vec<DiagnosticItem> {
    let cat = "Storage & Search Engine";
    let mut items = Vec::new();

    let cache_dir = vault_path.join(".k0maru");
    let cache_path = cache_dir.join("cache.sqlite");

    if !cache_path.exists() {
        items.push(DiagnosticItem::warn(
            cat,
            "SQLite Cache",
            format!("Cache file does not exist ({})", cache_path.display()),
            Some("Run `k0maru sync` to initialize local cache index".to_string()),
        ));
    } else {
        // Cache file size
        let size = std::fs::metadata(&cache_path).map(|m| m.len()).unwrap_or(0);
        let size_kb = size as f64 / 1024.0;
        items.push(DiagnosticItem::pass(
            cat,
            "SQLite Cache",
            format!("{:.2} KB ({} bytes)", size_kb, size),
        ));

        // Connect read-only and inspect schema v3, FTS5 parity, section index, fragmentation, and vectors
        crate::storage::ensure_sqlite_vec_registered();
        match rusqlite::Connection::open_with_flags(
            &cache_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        ) {
            Ok(conn) => {
                let user_version: i32 = conn
                    .query_row("PRAGMA user_version", [], |r| r.get(0))
                    .unwrap_or(0);
                let supported = crate::storage::CURRENT_SCHEMA_VERSION;
                if user_version == supported {
                    items.push(DiagnosticItem::pass(
                        cat,
                        "Schema Version",
                        format!("v{}", user_version),
                    ));
                } else if user_version < supported {
                    items.push(DiagnosticItem::warn(
                        cat,
                        "Schema Version",
                        format!("Legacy v{} (v{} available)", user_version, supported),
                        Some(format!(
                            "Run `k0maru sync` to upgrade to schema v{} without losing vectors",
                            supported
                        )),
                    ));
                } else {
                    items.push(DiagnosticItem::fail(
                        cat,
                        "Schema Version",
                        format!(
                            "Unsupported v{} (supported up to v{})",
                            user_version, supported
                        ),
                        Some("Upgrade k0maru to match cache schema version".to_string()),
                    ));
                }

                // FTS5 (fts_documents) and fts_hash parity check
                let doc_count_res: Result<i64, _> =
                    conn.query_row("SELECT count(*) FROM documents", [], |r| r.get(0));
                let fts_exists: bool = conn
                    .query_row(
                        "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'fts_documents'",
                        [],
                        |r| Ok(r.get::<_, i64>(0)? > 0),
                    )
                    .unwrap_or(false);
                let has_fts_hash_col: bool = conn
                    .prepare("PRAGMA table_info(documents)")
                    .and_then(|mut s| {
                        let rows = s.query_map([], |r| r.get::<_, String>(1))?;
                        let mut found = false;
                        for col in rows {
                            if col.as_deref() == Ok("fts_hash") {
                                found = true;
                            }
                        }
                        Ok(found)
                    })
                    .unwrap_or(false);

                match (doc_count_res, fts_exists, has_fts_hash_col) {
                    (Ok(docs), true, true) => {
                        let expected_ver = k0maru_core::text::FTS_SEGMENTER_VERSION.to_string();
                        let mismatch_res: Result<i64, _> = conn.query_row(
                            "SELECT count(*) FROM documents WHERE fts_hash IS NULL OR fts_hash != (content_hash || ':' || ?1)",
                            rusqlite::params![expected_ver],
                            |r| r.get(0),
                        );
                        match mismatch_res {
                            Ok(0) => {
                                items.push(DiagnosticItem::pass(
                                    cat,
                                    "FTS5 Search Index",
                                    format!(
                                        "Operational (fts_documents, {} documents, 0 stale fts_hash rows)",
                                        docs
                                    ),
                                ));
                            }
                            Ok(mismatches) => {
                                items.push(DiagnosticItem::warn(
                                    cat,
                                    "FTS5 Search Index",
                                    format!(
                                        "{} of {} documents have stale or missing fts_hash",
                                        mismatches, docs
                                    ),
                                    Some(
                                        "Run `k0maru sync` to repair stale FTS entries".to_string(),
                                    ),
                                ));
                            }
                            Err(e) => {
                                items.push(DiagnosticItem::warn(
                                    cat,
                                    "FTS5 Search Index",
                                    format!("Failed to check fts_hash parity: {}", e),
                                    Some(
                                        "Run `k0maru sync --force` to rebuild search index"
                                            .to_string(),
                                    ),
                                ));
                            }
                        }
                    }
                    (Ok(docs), false, _) => {
                        items.push(DiagnosticItem::warn(
                            cat,
                            "FTS5 Search Index",
                            format!("fts_documents table missing ({} raw docs exist)", docs),
                            Some("Run `k0maru sync` to migrate to fts_documents".to_string()),
                        ));
                    }
                    (Ok(docs), true, false) => {
                        items.push(DiagnosticItem::warn(
                            cat,
                            "FTS5 Search Index",
                            format!(
                                "documents.fts_hash column missing ({} raw docs exist)",
                                docs
                            ),
                            Some(format!(
                                "Run `k0maru sync` to migrate schema to v{}",
                                supported
                            )),
                        ));
                    }
                    (Err(e), _, _) => {
                        items.push(DiagnosticItem::warn(
                            cat,
                            "FTS5 Search Index",
                            format!("Documents table not ready: {}", e),
                            Some("Run `k0maru sync` to initialize storage".to_string()),
                        ));
                    }
                }

                // Legacy documents_fts absence check
                let legacy_fts_present: bool = conn
                    .query_row(
                        "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'documents_fts'",
                        [],
                        |r| Ok(r.get::<_, i64>(0)? > 0),
                    )
                    .unwrap_or(false);
                if legacy_fts_present {
                    items.push(DiagnosticItem::warn(
                        cat,
                        "Legacy FTS5 Table",
                        "Legacy documents_fts table is still present",
                        Some("Run `k0maru sync` to drop legacy documents_fts".to_string()),
                    ));
                } else {
                    items.push(DiagnosticItem::pass(
                        cat,
                        "Legacy FTS5 Table",
                        "Absent (legacy documents_fts cleaned up)",
                    ));
                }

                // Reclaimable freelist check
                let freelist_count: i64 = conn
                    .query_row("PRAGMA freelist_count", [], |r| r.get(0))
                    .unwrap_or(0);
                let page_size: i64 = conn
                    .query_row("PRAGMA page_size", [], |r| r.get(0))
                    .unwrap_or(4096);
                let reclaimable_bytes =
                    (freelist_count.max(0) as u64).saturating_mul(page_size.max(0) as u64);
                let reclaimable_kb = reclaimable_bytes as f64 / 1024.0;
                if reclaimable_bytes > 1024 * 1024 {
                    items.push(DiagnosticItem::warn(
                        cat,
                        "Cache Fragmentation",
                        format!(
                            "{:.2} KB reclaimable ({} free pages)",
                            reclaimable_kb, freelist_count
                        ),
                        Some(
                            "Run `k0maru doctor --compact` to optimize FTS5 and VACUUM free pages"
                                .to_string(),
                        ),
                    ));
                } else {
                    items.push(DiagnosticItem::pass(
                        cat,
                        "Cache Fragmentation",
                        format!(
                            "{:.2} KB reclaimable ({} free pages)",
                            reclaimable_kb, freelist_count
                        ),
                    ));
                }

                // Section search index (v3)
                let doc_count = conn
                    .query_row("SELECT count(*) FROM documents", [], |r| r.get(0))
                    .unwrap_or(0);
                let has_doc_sections: bool = conn
                    .query_row(
                        "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'document_sections'",
                        [],
                        |r| Ok(r.get::<_, i64>(0)? > 0),
                    )
                    .unwrap_or(false);
                let has_fts_sections: bool = conn
                    .query_row(
                        "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'fts_sections'",
                        [],
                        |r| Ok(r.get::<_, i64>(0)? > 0),
                    )
                    .unwrap_or(false);
                let has_section_state: bool = conn
                    .query_row(
                        "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'document_section_state'",
                        [],
                        |r| Ok(r.get::<_, i64>(0)? > 0),
                    )
                    .unwrap_or(false);

                if has_doc_sections && has_fts_sections && has_section_state {
                    let sec_count: i64 = conn
                        .query_row("SELECT count(*) FROM document_sections", [], |r| r.get(0))
                        .unwrap_or(0);
                    let expected_sec_suffix = format!(
                        ":s{}:f{}",
                        k0maru_core::parser::sections::SECTION_PARSER_VERSION,
                        k0maru_core::text::FTS_SEGMENTER_VERSION
                    );
                    let stale_sec_res: Result<i64, _> = conn.query_row(
                        r#"
                        SELECT count(*)
                        FROM documents d
                        LEFT JOIN document_section_state s ON s.doc_id = d.id
                        WHERE s.sections_hash IS NULL OR s.sections_hash != (d.content_hash || ?1)
                        "#,
                        rusqlite::params![expected_sec_suffix],
                        |r| r.get(0),
                    );
                    match stale_sec_res {
                        Ok(0) => {
                            items.push(DiagnosticItem::pass(
                                cat,
                                "Section Search Index",
                                format!(
                                    "Operational (document_sections, {} sections across {} docs, 0 stale)",
                                    sec_count, doc_count
                                ),
                            ));
                        }
                        Ok(stale) => {
                            items.push(DiagnosticItem::warn(
                                cat,
                                "Section Search Index",
                                format!(
                                    "{} sections indexed ({} of {} docs have stale or missing sections)",
                                    sec_count, stale, doc_count
                                ),
                                Some(
                                    "Run `k0maru sync` to repair stale section indices".to_string(),
                                ),
                            ));
                        }
                        Err(e) => {
                            items.push(DiagnosticItem::warn(
                                cat,
                                "Section Search Index",
                                format!("Failed to check section index parity: {}", e),
                                Some("Run `k0maru sync` to rebuild section index".to_string()),
                            ));
                        }
                    }
                } else {
                    items.push(DiagnosticItem::warn(
                        cat,
                        "Section Search Index",
                        "Section index tables missing (legacy schema)".to_string(),
                        Some(format!(
                            "Run `k0maru sync` to upgrade to schema v{}",
                            supported
                        )),
                    ));
                }

                // Vector virtual table, model provenance, and section vectors
                let has_meta_table: bool = conn
                    .query_row(
                        "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'k0maru_meta'",
                        [],
                        |r| Ok(r.get::<_, i64>(0)? > 0),
                    )
                    .unwrap_or(false);
                let read_meta = |key: &str| -> Option<String> {
                    if !has_meta_table {
                        return None;
                    }
                    conn.query_row(
                        "SELECT value FROM k0maru_meta WHERE key = ?1",
                        rusqlite::params![key],
                        |r| r.get(0),
                    )
                    .ok()
                };
                let embed_model = read_meta("embed_model");
                let sec_vecs_complete = read_meta("section_vectors_complete");

                let vec_count_res: Result<i64, _> =
                    conn.query_row("SELECT count(*) FROM vector_metadata", [], |r| r.get(0));

                match vec_count_res {
                    Ok(vec_count) => {
                        let coverage = if doc_count > 0 {
                            (vec_count as f64 / doc_count as f64 * 100.0).min(100.0)
                        } else {
                            0.0
                        };
                        if vec_count > 0 {
                            match embed_model.as_deref() {
                                Some(model) => {
                                    items.push(DiagnosticItem::pass(
                                        cat,
                                        "Vector Embeddings",
                                        format!(
                                            "{} vectors indexed ({:.1}% coverage of {} docs, model: {})",
                                            vec_count, coverage, doc_count, model
                                        ),
                                    ));
                                }
                                None => {
                                    items.push(DiagnosticItem::warn(
                                        cat,
                                        "Vector Embeddings",
                                        format!(
                                            "{} vectors indexed ({:.1}% coverage of {} docs; model provenance unknown)",
                                            vec_count, coverage, doc_count
                                        ),
                                        Some(
                                            "Run `k0maru sync --vector` to record embed_model provenance"
                                                .to_string(),
                                        ),
                                    ));
                                }
                            }
                        } else {
                            items.push(DiagnosticItem::pass(
                                cat,
                                "Vector Embeddings",
                                format!(
                                    "{} vectors indexed ({:.1}% coverage of {} docs)",
                                    vec_count, coverage, doc_count
                                ),
                            ));
                        }
                    }
                    Err(e) => {
                        items.push(DiagnosticItem::warn(
                            cat,
                            "Vector Embeddings",
                            format!("Vector table query failed: {}", e),
                            Some(
                                "Run `k0maru sync --vector` to generate vector embeddings"
                                    .to_string(),
                            ),
                        ));
                    }
                }

                let has_sec_vec_meta: bool = conn
                    .query_row(
                        "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'section_vector_meta'",
                        [],
                        |r| Ok(r.get::<_, i64>(0)? > 0),
                    )
                    .unwrap_or(false);
                if has_sec_vec_meta && has_doc_sections {
                    let sec_vec_count: i64 = conn
                        .query_row("SELECT count(*) FROM section_vector_meta", [], |r| r.get(0))
                        .unwrap_or(0);
                    let sec_total: i64 = conn
                        .query_row("SELECT count(*) FROM document_sections", [], |r| r.get(0))
                        .unwrap_or(0);
                    if sec_vecs_complete.as_deref() == Some("1") {
                        items.push(DiagnosticItem::pass(
                            cat,
                            "Section Vectors",
                            format!(
                                "{} section vectors indexed (ready, model: {})",
                                sec_vec_count,
                                embed_model.as_deref().unwrap_or("unknown")
                            ),
                        ));
                    } else if sec_vec_count > 0 {
                        items.push(DiagnosticItem::warn(
                            cat,
                            "Section Vectors",
                            format!(
                                "{} of {} section vectors indexed (backfill incomplete)",
                                sec_vec_count, sec_total
                            ),
                            Some(
                                "Run `k0maru sync --vector` to finish section embedding backfill"
                                    .to_string(),
                            ),
                        ));
                    } else {
                        items.push(DiagnosticItem::pass(
                            cat,
                            "Section Vectors",
                            format!(
                                "0 of {} section vectors indexed (run `k0maru sync --vector` to enable section KNN)",
                                sec_total
                            ),
                        ));
                    }
                }
            }
            Err(e) => {
                items.push(DiagnosticItem::fail(
                    cat,
                    "SQLite Connection",
                    format!("Failed to open cache database: {}", e),
                    Some("Check file permissions or remove corrupted cache file".to_string()),
                ));
            }
        }
    }

    // Embedding engine backend probe
    let model_type = if let Ok(env_val) = std::env::var("K0MARU_EMBED_MODEL") {
        crate::vector::EmbeddingModelType::from_str(&env_val).unwrap_or_default()
    } else {
        crate::vector::EmbeddingModelType::default()
    };

    if std::env::var("K0MARU_FORCE_MOCK_EMBED").is_ok()
        || model_type == crate::vector::EmbeddingModelType::Mock
    {
        items.push(DiagnosticItem::pass(
            cat,
            "Embedding Engine",
            format!(
                "Mock Engine ready [model: {}, {} dimensions, zero-download]",
                model_type.canonical_name(),
                model_type.dimension()
            ),
        ));
    } else {
        #[cfg(feature = "fastembed")]
        {
            let model_dir = crate::vector::resolve_cache_dir(None);
            let weights_cached = model_dir.exists()
                && std::fs::read_dir(&model_dir)
                    .map(|mut d| d.next().is_some())
                    .unwrap_or(false);
            if weights_cached {
                items.push(DiagnosticItem::pass(
                    cat,
                    "Embedding Engine",
                    format!(
                        "FastEmbed (ONNX) ready [model: {}, {} dimensions, cache: {} (weights cached)]",
                        model_type.canonical_name(),
                        model_type.dimension(),
                        model_dir.display()
                    ),
                ));
            } else {
                items.push(DiagnosticItem::pass(
                    cat,
                    "Embedding Engine",
                    format!(
                        "FastEmbed (ONNX) ready [model: {}, {} dimensions, cache: {} (download on demand)]",
                        model_type.canonical_name(),
                        model_type.dimension(),
                        model_dir.display()
                    ),
                ));
            }
        }

        #[cfg(not(feature = "fastembed"))]
        {
            items.push(DiagnosticItem::pass(
                cat,
                "Embedding Engine",
                format!(
                    "Mock embedding engine [model: {}, {} dimensions] (fastembed feature disabled)",
                    model_type.canonical_name(),
                    model_type.dimension()
                ),
            ));
        }
    }

    items
}

/// Probe 4: Inspects configured AI agent MCP clients (Claude, Cursor, Gemini, Windsurf, Cline).
pub fn probe_ecosystem(home_override: Option<&Path>, vault_path: &Path) -> Vec<DiagnosticItem> {
    let cat = "Agent MCP Ecosystem";
    let mut items = Vec::new();

    let client_statuses = inspect_all_clients(home_override);
    let canonical_target = vault_path
        .canonicalize()
        .unwrap_or_else(|_| vault_path.to_path_buf());

    for status in client_statuses {
        let name = status.client.name();

        if !status.installed {
            items.push(DiagnosticItem::pass(
                cat,
                name,
                format!("Not installed ({})", status.config_path.display()),
            ));
            continue;
        }

        if !status.configured {
            items.push(DiagnosticItem::warn(
                cat,
                name,
                format!(
                    "Installed at {} but k0maru-memory is not configured",
                    status.config_path.display()
                ),
                Some(format!(
                    "Run `k0maru install --target {}` to configure MCP server",
                    target_flag_name(status.client)
                )),
            ));
            continue;
        }

        // Configured: verify target vault path
        match &status.vault_path {
            Some(configured_vault) => {
                let canonical_cfg = configured_vault
                    .canonicalize()
                    .unwrap_or_else(|_| configured_vault.clone());

                if canonical_cfg == canonical_target || configured_vault == vault_path {
                    items.push(DiagnosticItem::pass(
                        cat,
                        name,
                        format!(
                            "Mounted and targeting current vault ({})",
                            configured_vault.display()
                        ),
                    ));
                } else {
                    items.push(DiagnosticItem::warn(
                        cat,
                        name,
                        format!(
                            "Mounted to a different vault: {} (current: {})",
                            configured_vault.display(),
                            canonical_target.display()
                        ),
                        Some(format!(
                            "Run `k0maru install --target {}` to point this client to current vault",
                            target_flag_name(status.client)
                        )),
                    ));
                }
            }
            None => {
                items.push(DiagnosticItem::warn(
                    cat,
                    name,
                    format!(
                        "Mounted in {} but no --vault argument found",
                        status.config_path.display()
                    ),
                    Some(format!(
                        "Run `k0maru install --target {}` to set the target vault path",
                        target_flag_name(status.client)
                    )),
                ));
            }
        }
    }

    items
}

/// Helper to get CLI flag name for a client.
fn target_flag_name(client: McpClient) -> &'static str {
    match client {
        McpClient::Claude => "claude",
        McpClient::Cursor => "cursor",
        McpClient::Gemini => "gemini",
        McpClient::Antigravity => "antigravity",
        McpClient::Windsurf => "windsurf",
        McpClient::Cline => "cline",
        McpClient::Hermes => "hermes",
        McpClient::OpenClaw => "openclaw",
        McpClient::Omp => "omp",
    }
}

/// Optimizes `fts_documents`, runs `VACUUM`, and performs a post-compact KNN vector self-check.
pub fn compact_cache(vault_path: &Path) -> DiagnosticItem {
    let cat = "Storage & Search Engine";
    let cache_path = vault_path.join(".k0maru").join("cache.sqlite");
    if !cache_path.exists() {
        return DiagnosticItem::warn(
            cat,
            "Cache Compaction",
            format!("Cache file does not exist ({})", cache_path.display()),
            Some("Run `k0maru sync` to initialize local cache index".to_string()),
        );
    }

    let size_before = std::fs::metadata(&cache_path).map(|m| m.len()).unwrap_or(0);
    let storage = match crate::storage::SqliteStorage::open_rw(&cache_path) {
        Ok(s) => s,
        Err(e) => {
            return DiagnosticItem::fail(
                cat,
                "Cache Compaction",
                format!("Failed to open cache for compaction: {}", e),
                None,
            );
        }
    };

    if let Err(e) = storage.connection().execute(
        "INSERT INTO fts_documents(fts_documents) VALUES('optimize')",
        [],
    ) {
        return DiagnosticItem::fail(
            cat,
            "Cache Compaction",
            format!("FTS5 optimize failed: {}", e),
            None,
        );
    }

    let has_fts_sections: bool = storage
        .connection()
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'fts_sections'",
            [],
            |r| Ok(r.get::<_, i64>(0)? > 0),
        )
        .unwrap_or(false);
    if has_fts_sections {
        if let Err(e) = storage.connection().execute(
            "INSERT INTO fts_sections(fts_sections) VALUES('optimize')",
            [],
        ) {
            return DiagnosticItem::fail(
                cat,
                "Cache Compaction",
                format!("Section FTS5 optimize failed: {}", e),
                None,
            );
        }
    }

    if let Err(e) = storage.connection().execute_batch("VACUUM;") {
        return DiagnosticItem::fail(
            cat,
            "Cache Compaction",
            format!("SQLite VACUUM failed: {}", e),
            None,
        );
    }

    let knn_summary = match storage.has_vectors() {
        Ok(true) => {
            let dim = storage
                .get_vector_dimension()
                .ok()
                .flatten()
                .unwrap_or(crate::storage::VECTOR_DIMENSIONS);
            let mut query = vec![0.0f32; dim];
            if !query.is_empty() {
                query[0] = 1.0;
            }
            match storage.search_vectors(&query, 1) {
                Ok(hits) if !hits.is_empty() && hits[0].1.is_finite() => {
                    format!("KNN self-check passed ({}d)", dim)
                }
                Ok(_) => {
                    return DiagnosticItem::fail(
                        cat,
                        "Cache Compaction",
                        "Post-VACUUM KNN self-check returned empty or non-finite vector hits"
                            .to_string(),
                        Some(
                            "Run `k0maru sync --force --vector` to rebuild vector index"
                                .to_string(),
                        ),
                    );
                }
                Err(e) => {
                    return DiagnosticItem::fail(
                        cat,
                        "Cache Compaction",
                        format!("Post-VACUUM KNN self-check failed: {}", e),
                        Some(
                            "Run `k0maru sync --force --vector` to rebuild vector index"
                                .to_string(),
                        ),
                    );
                }
            }
        }
        Ok(false) => "no vectors indexed".to_string(),
        Err(e) => format!("vector check skipped ({})", e),
    };

    let size_after = std::fs::metadata(&cache_path).map(|m| m.len()).unwrap_or(0);
    DiagnosticItem::pass(
        cat,
        "Cache Compaction",
        format!(
            "FTS5 optimized + VACUUM completed ({:.2} KB -> {:.2} KB; {})",
            size_before as f64 / 1024.0,
            size_after as f64 / 1024.0,
            knn_summary
        ),
    )
}

/// Probe 5: Inspects optional read-only global vault path, schema version, embed_model parity, and sync age.
pub fn probe_global_vault(primary_vault: &Path, global_flag: Option<&Path>) -> Vec<DiagnosticItem> {
    let cat = "Global Vault";
    let mut items = Vec::new();

    if std::env::var("K0MARU_NO_GLOBAL").as_deref() == Ok("1") {
        return items;
    }

    let resolved =
        k0maru_core::storage::federated::resolve_global_vault(global_flag, primary_vault);
    let Some(global_path) = resolved else {
        let candidate = global_flag
            .filter(|p| !p.as_os_str().is_empty())
            .map(|p| p.to_path_buf())
            .or_else(|| {
                std::env::var_os("K0MARU_GLOBAL_VAULT")
                    .filter(|v| !v.is_empty() && v.to_str().is_some_and(|s| !s.trim().is_empty()))
                    .map(std::path::PathBuf::from)
            });
        if let Some(cand) = candidate {
            let canon_cand = std::fs::canonicalize(&cand).unwrap_or(cand);
            let canon_prim = std::fs::canonicalize(primary_vault)
                .unwrap_or_else(|_| primary_vault.to_path_buf());
            if canon_cand != canon_prim {
                let cache_path = canon_cand.join(".k0maru").join("cache.sqlite");
                items.push(DiagnosticItem::warn(
                    cat,
                    "Global Vault Cache",
                    format!(
                        "Global vault {} has no cache at {}",
                        canon_cand.display(),
                        cache_path.display()
                    ),
                    Some(format!(
                        "Run `k0maru sync --vault {}`",
                        canon_cand.display()
                    )),
                ));
            }
        }
        return items;
    };

    items.push(DiagnosticItem::pass(
        cat,
        "Global Vault Path",
        global_path.display().to_string(),
    ));

    let global_cache = global_path.join(".k0maru").join("cache.sqlite");
    let ro = match k0maru_core::storage::ReadOnlyStorage::open(&global_cache) {
        Ok(ro) => ro,
        Err(err) => {
            items.push(DiagnosticItem::warn(
                cat,
                "Global Vault Cache",
                format!(
                    "Failed to open global cache read-only ({}): {}",
                    global_cache.display(),
                    err
                ),
                Some(format!(
                    "Run `k0maru sync --vault {}`",
                    global_path.display()
                )),
            ));
            return items;
        }
    };

    let user_version: i32 = ro
        .storage()
        .connection()
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap_or(0);
    let supported = crate::storage::CURRENT_SCHEMA_VERSION;
    if user_version <= supported {
        items.push(DiagnosticItem::pass(
            cat,
            "Global Schema Version",
            format!("v{}", user_version),
        ));
    } else {
        items.push(DiagnosticItem::fail(
            cat,
            "Global Schema Version",
            format!(
                "Unsupported v{} (supported up to v{})",
                user_version, supported
            ),
            Some("Upgrade k0maru to match global cache schema version".to_string()),
        ));
    }

    let global_model = ro.storage().meta_get("embed_model").ok().flatten();
    let primary_cache = primary_vault.join(".k0maru").join("cache.sqlite");
    let primary_model = if primary_cache.is_file() {
        k0maru_core::storage::ReadOnlyStorage::open(&primary_cache)
            .ok()
            .and_then(|p_ro| p_ro.storage().meta_get("embed_model").ok().flatten())
    } else {
        None
    };

    match (global_model.as_deref(), primary_model.as_deref()) {
        (Some(g), Some(p)) if g == p => {
            items.push(DiagnosticItem::pass(
                cat,
                "Global Embed Model",
                format!("Matches primary ({})", g),
            ));
        }
        (Some(g), Some(p)) => {
            items.push(DiagnosticItem::warn(
                cat,
                "Global Embed Model",
                format!(
                    "Mismatch: global '{}' vs primary '{}' (global falls back to BM25-only)",
                    g, p
                ),
                Some(format!(
                    "Re-sync global vault with matching model: `k0maru sync --vault {} --vector --embed-model {}`",
                    global_path.display(),
                    p
                )),
            ));
        }
        (Some(g), None) => {
            items.push(DiagnosticItem::pass(
                cat,
                "Global Embed Model",
                format!("{} (primary has no embed_model recorded)", g),
            ));
        }
        (None, Some(p)) => {
            if ro.storage().has_vectors().unwrap_or(false) {
                items.push(DiagnosticItem::warn(
                    cat,
                    "Global Embed Model",
                    format!(
                        "Unknown provenance on global vectors (primary uses '{}')",
                        p
                    ),
                    Some(format!(
                        "Run `k0maru sync --vault {} --vector --embed-model {}`",
                        global_path.display(),
                        p
                    )),
                ));
            } else {
                items.push(DiagnosticItem::pass(
                    cat,
                    "Global Embed Model",
                    format!("None (BM25-only; primary uses {})", p),
                ));
            }
        }
        (None, None) => {
            if ro.storage().has_vectors().unwrap_or(false) {
                items.push(DiagnosticItem::warn(
                    cat,
                    "Global Embed Model",
                    "Unknown (vectors present without embed_model provenance)".to_string(),
                    Some(format!(
                        "Run `k0maru sync --vault {} --vector`",
                        global_path.display()
                    )),
                ));
            } else {
                items.push(DiagnosticItem::pass(
                    cat,
                    "Global Embed Model",
                    "None (no vectors indexed)".to_string(),
                ));
            }
        }
    }

    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let last_sync_secs = ro
        .storage()
        .meta_get("last_sync_at")
        .ok()
        .flatten()
        .and_then(|s| s.trim().parse::<i64>().ok());
    const STALE_GLOBAL_SECS: i64 = 7 * 86_400;

    match last_sync_secs {
        Some(ts) => {
            let age_secs = now_secs.saturating_sub(ts);
            if age_secs > STALE_GLOBAL_SECS {
                let age_days = age_secs / 86_400;
                items.push(DiagnosticItem::warn(
                    cat,
                    "Global Sync Age",
                    format!("Stale: last synced {} days ago (> 7d)", age_days),
                    Some(format!(
                        "Run `k0maru sync --vault {}`",
                        global_path.display()
                    )),
                ));
            } else {
                let age_hours = age_secs / 3_600;
                items.push(DiagnosticItem::pass(
                    cat,
                    "Global Sync Age",
                    format!("Fresh ({}h ago)", age_hours),
                ));
            }
        }
        None => {
            items.push(DiagnosticItem::warn(
                cat,
                "Global Sync Age",
                "Unknown (last_sync_at metadata missing)".to_string(),
                Some(format!(
                    "Run `k0maru sync --vault {}`",
                    global_path.display()
                )),
            ));
        }
    }

    items
}

/// Runs full diagnostic suite across all categories and generates a `DoctorReport`.
pub fn run_diagnostics(vault_path: &Path, home_override: Option<&Path>) -> DoctorReport {
    run_diagnostics_with_options(vault_path, home_override, false, None)
}

/// Runs full diagnostic suite, optionally compacting the SQLite cache first.
pub fn run_diagnostics_with_compact(
    vault_path: &Path,
    home_override: Option<&Path>,
    compact: bool,
) -> DoctorReport {
    run_diagnostics_with_options(vault_path, home_override, compact, None)
}

/// Runs full diagnostic suite with optional compaction and global vault diagnostics.
pub fn run_diagnostics_with_options(
    vault_path: &Path,
    home_override: Option<&Path>,
    compact: bool,
    global_vault: Option<&Path>,
) -> DoctorReport {
    let mut items = Vec::new();
    let compact_item = if compact {
        Some(compact_cache(vault_path))
    } else {
        None
    };

    items.extend(probe_binary());
    items.extend(probe_vault(vault_path));
    items.extend(probe_storage(vault_path));
    if let Some(c) = compact_item {
        items.push(c);
    }
    items.extend(probe_global_vault(vault_path, global_vault));
    items.extend(probe_ecosystem(home_override, vault_path));

    DoctorReport::from_items(items)
}

/// Formats a `DoctorReport` into a colorized, human-readable terminal string.
pub fn format_report(report: &DoctorReport) -> String {
    let mut out = String::new();
    out.push_str("🏥 k0maru doctor — System & Environment Health Report\n");
    out.push_str(
        "================================================================================\n\n",
    );

    // Group items by category, preserving probe order
    let mut categories: BTreeMap<usize, (&str, Vec<&DiagnosticItem>)> = BTreeMap::new();
    let cat_order = [
        "Binary & System",
        "Vault & Hierarchy",
        "Storage & Search Engine",
        "Global Vault",
        "Agent MCP Ecosystem",
    ];

    for item in &report.items {
        let order = cat_order
            .iter()
            .position(|&c| c == item.category)
            .unwrap_or(99);
        categories
            .entry(order)
            .or_insert_with(|| (&item.category, Vec::new()))
            .1
            .push(item);
    }

    for (_, (cat_name, items)) in categories {
        out.push_str(&format!("[{}]\n", cat_name));
        for item in items {
            let label = item.status.colored_label();
            out.push_str(&format!("  {}  {}: {}\n", label, item.name, item.detail));
            if let Some(suggestion) = &item.suggestion {
                out.push_str(&format!("          💡 Suggestion: {}\n", suggestion));
            }
        }
        out.push('\n');
    }

    out.push_str(
        "================================================================================\n",
    );
    let summary = &report.summary;
    out.push_str(&format!(
        "Summary: {} passed, {} warnings, {} failures (total: {})\n",
        summary.passed, summary.warnings, summary.failures, summary.total
    ));

    out
}
