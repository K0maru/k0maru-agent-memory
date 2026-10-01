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
        "10_Projects",
        "20_Cards",
        "30_Logs",
        "40_Archives",
        "Projects",
        "Cards",
        "Archive",
    ];
    let detected: Vec<&str> = hierarchy_dirs
        .iter()
        .copied()
        .filter(|dir| vault_path.join(dir).is_dir())
        .collect();

    if !detected.is_empty() {
        items.push(DiagnosticItem::pass(
            cat,
            "Knowledge Hierarchy",
            format!("Detected structure: {}", detected.join(", ")),
        ));
    } else {
        items.push(DiagnosticItem::warn(
            cat,
            "Knowledge Hierarchy",
            "Standard hierarchy folders (10_Projects, 20_Cards) not detected",
            Some(
                "Organizing notes with 10_Projects and 20_Cards optimizes agent loadout context"
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

        // Connect and test FTS5 and vector tables
        match rusqlite::Connection::open(&cache_path) {
            Ok(conn) => {
                // Ensure sqlite-vec auto extension is registered
                crate::storage::ensure_sqlite_vec_registered();

                // FTS5 and document count
                let doc_count_res: Result<i64, _> =
                    conn.query_row("SELECT count(*) FROM documents", [], |r| r.get(0));
                let fts_count_res: Result<i64, _> =
                    conn.query_row("SELECT count(*) FROM documents_fts", [], |r| r.get(0));

                match (doc_count_res, fts_count_res) {
                    (Ok(docs), Ok(fts)) => {
                        items.push(DiagnosticItem::pass(
                            cat,
                            "FTS5 Search Index",
                            format!("Operational ({} documents, {} in FTS index)", docs, fts),
                        ));
                    }
                    (Ok(docs), Err(e)) => {
                        items.push(DiagnosticItem::warn(
                            cat,
                            "FTS5 Search Index",
                            format!("FTS5 table error: {} ({} raw docs exist)", e, docs),
                            Some("Run `k0maru sync --force` to rebuild search index".to_string()),
                        ));
                    }
                    (Err(e), _) => {
                        items.push(DiagnosticItem::warn(
                            cat,
                            "FTS5 Search Index",
                            format!("Documents table not ready: {}", e),
                            Some("Run `k0maru sync` to initialize storage".to_string()),
                        ));
                    }
                }

                // Vector virtual table and coverage
                let vec_count_res: Result<i64, _> =
                    conn.query_row("SELECT count(*) FROM vector_metadata", [], |r| r.get(0));
                let doc_count = conn
                    .query_row("SELECT count(*) FROM documents", [], |r| r.get(0))
                    .unwrap_or(0);

                match vec_count_res {
                    Ok(vec_count) => {
                        let coverage = if doc_count > 0 {
                            (vec_count as f64 / doc_count as f64 * 100.0).min(100.0)
                        } else {
                            0.0
                        };
                        items.push(DiagnosticItem::pass(
                            cat,
                            "Vector Embeddings",
                            format!(
                                "{} vectors indexed ({:.1}% coverage of {} docs)",
                                vec_count, coverage, doc_count
                            ),
                        ));
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
    #[cfg(feature = "fastembed")]
    {
        let model_dir = crate::vector::fastembed::resolve_cache_dir(None);
        if model_dir.exists() {
            items.push(DiagnosticItem::pass(
                cat,
                "Embedding Engine",
                format!(
                    "FastEmbed (ONNX) ready, model cache: {}",
                    model_dir.display()
                ),
            ));
        } else {
            items.push(DiagnosticItem::pass(
                cat,
                "Embedding Engine",
                format!(
                    "FastEmbed (ONNX) ready (cache will download to {})",
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
            "Mock embedding engine (fastembed feature disabled)".to_string(),
        ));
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
        McpClient::Windsurf => "windsurf",
        McpClient::Cline => "cline",
    }
}

/// Runs full diagnostic suite across all categories and generates a `DoctorReport`.
pub fn run_diagnostics(vault_path: &Path, home_override: Option<&Path>) -> DoctorReport {
    let mut items = Vec::new();

    items.extend(probe_binary());
    items.extend(probe_vault(vault_path));
    items.extend(probe_storage(vault_path));
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
