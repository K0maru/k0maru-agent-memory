use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Semantic lifecycle hierarchy level of notes in the knowledge hub.
/// - L0 (Ephemeral): Daily notes, scratchpad, transient context
/// - L1 (Resource): Static references, cheatsheets, project assets
/// - L2 (Log): AI analysis traces, evaluation runs, technical logs
/// - L3 (Evergreen): Atomic permanent notes, architectural principles
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub enum HierarchyLevel {
    #[default]
    L0Ephemeral,
    L1Resource,
    L2Log,
    L3Evergreen,
}

impl HierarchyLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::L0Ephemeral => "L0Ephemeral",
            Self::L1Resource => "L1Resource",
            Self::L2Log => "L2Log",
            Self::L3Evergreen => "L3Evergreen",
        }
    }
}

impl std::fmt::Display for HierarchyLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
/// Lifecycle status of a document/note.
/// - Active: Current, valid knowledge (default)
/// - Superseded: Replaced by a newer document/ADR
/// - Deprecated: Outdated or marked obsolete, but not directly superseded by a specific document
/// - Archived: Historical record, retired from active consideration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum NoteLifecycleStatus {
    #[default]
    Active,
    Superseded,
    Deprecated,
    Archived,
}

impl NoteLifecycleStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Superseded => "superseded",
            Self::Deprecated => "deprecated",
            Self::Archived => "archived",
        }
    }
}

impl std::fmt::Display for NoteLifecycleStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A parsed WikiLink reference between documents (e.g. `[[Target]]` or `[[Target|Alias]]`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WikiLink {
    /// Target file/card name without file extension
    pub target: String,
    /// Optional display alias
    pub alias: Option<String>,
    /// Exact raw markdown syntax string
    pub raw_text: String,
}

impl WikiLink {
    pub fn new(
        target: impl Into<String>,
        alias: Option<impl Into<String>>,
        raw_text: impl Into<String>,
    ) -> Self {
        Self {
            target: target.into(),
            alias: alias.map(|a| a.into()),
            raw_text: raw_text.into(),
        }
    }
}

/// Canonical document entity representing an immutable parsed Markdown file in the vault.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Document {
    /// Relative path within the vault root
    pub path: PathBuf,
    /// Title extracted from frontmatter or first heading or filename
    pub title: String,
    /// Semantic hierarchy level
    pub hierarchy: HierarchyLevel,
    /// Extracted YAML frontmatter as JSON value
    pub frontmatter: serde_json::Value,
    /// Outgoing WikiLinks extracted from document body
    pub links: Vec<WikiLink>,
    /// Tags extracted from frontmatter or inline `#tags`
    pub tags: Vec<String>,
    /// xxHash64/xxh3 hash of file content for dirty checking
    pub content_hash: String,
    /// Last modification timestamp (seconds since Unix epoch)
    pub mtime: u64,
    /// Raw document body without frontmatter
    pub body: String,
}
impl Document {
    /// Determines the lifecycle status of this document based on frontmatter,
    /// relations/supersession metadata, status tags, and markdown body heuristics.
    pub fn lifecycle_status(&self) -> NoteLifecycleStatus {
        // 1. Check frontmatter "status" field
        if let Some(status_str) = self.frontmatter.get("status").and_then(|v| v.as_str()) {
            let s = status_str.trim().to_lowercase();
            if s == "superseded" || s.starts_with("superseded ") || s.starts_with("superseded_by") {
                return NoteLifecycleStatus::Superseded;
            } else if s == "deprecated"
                || s == "obsolete"
                || s == "rejected"
                || s.starts_with("deprecated ")
                || s.starts_with("rejected ")
            {
                return NoteLifecycleStatus::Deprecated;
            } else if s == "archived" || s == "retired" || s.starts_with("archived ") {
                return NoteLifecycleStatus::Archived;
            }
        }

        // 2. Check if this document has been superseded by another document
        if let Some(s) = self.superseded_by() {
            if !s.trim().is_empty() {
                return NoteLifecycleStatus::Superseded;
            }
        }

        // 3. Check tags for status indicators (e.g. status/superseded, #status/deprecated)
        for tag in &self.tags {
            let normalized = tag.trim().trim_start_matches('#').to_lowercase();
            match normalized.as_str() {
                "status/superseded" => return NoteLifecycleStatus::Superseded,
                "status/deprecated" | "status/obsolete" => return NoteLifecycleStatus::Deprecated,
                "status/archived" => return NoteLifecycleStatus::Archived,
                _ => {}
            }
        }

        // 4. Check first 5 lines of body for callouts
        for line in self.body.lines().take(5) {
            let trimmed_upper = line.trim_start().to_uppercase();
            if trimmed_upper.starts_with("> [SUPERSEDED]")
                || trimmed_upper.starts_with("> [!WARN] SUPERSEDED")
                || trimmed_upper.starts_with("> [WARN] SUPERSEDED")
            {
                return NoteLifecycleStatus::Superseded;
            }
        }
        NoteLifecycleStatus::Active
    }

    /// Returns true if the document has an Active lifecycle status.
    pub fn is_active(&self) -> bool {
        self.lifecycle_status() == NoteLifecycleStatus::Active
    }

    /// Returns the target document(s) that supersede this note, if any.
    /// Checks frontmatter `superseded_by` (or `relations.superseded_by`),
    /// or markdown body callout header (e.g. `> [SUPERSEDED] Superseded by [[ADR-002]]`).
    pub fn superseded_by(&self) -> Option<String> {
        let fm_val = self.frontmatter.get("superseded_by").or_else(|| {
            self.frontmatter
                .get("relations")
                .and_then(|r| r.get("superseded_by"))
        });

        let fm_list = extract_string_or_array(fm_val);
        if !fm_list.is_empty() {
            return Some(fm_list.join(", "));
        }

        // Fallback: check first 5 lines of markdown body
        for line in self.body.lines().take(5) {
            let trimmed_upper = line.trim_start().to_uppercase();
            if trimmed_upper.starts_with("> [SUPERSEDED]")
                || trimmed_upper.starts_with("> [!WARN] SUPERSEDED")
                || trimmed_upper.starts_with("> [WARN] SUPERSEDED")
            {
                let mut links = Vec::new();
                let mut rest = line;
                while let Some(start) = rest.find("[[") {
                    if let Some(end) = rest[start..].find("]]") {
                        links.push(&rest[start..start + end + 2]);
                        rest = &rest[start + end + 2..];
                    } else {
                        break;
                    }
                }
                if !links.is_empty() {
                    return Some(links.join(", "));
                }

                let lower = line.to_lowercase();
                for pattern in &["superseded by", "superseded_by"] {
                    if let Some(pos) = lower.find(pattern) {
                        let remainder = line[pos + pattern.len()..].trim();
                        let remainder = remainder.strip_prefix(':').unwrap_or(remainder).trim();
                        if !remainder.is_empty() {
                            return Some(remainder.to_string());
                        }
                    }
                }
            }
        }

        None
    }

    /// Returns the list of document identifiers or paths that this document supersedes.
    /// Checks frontmatter `supersedes` or `relations.supersedes` (string or array of strings).
    pub fn supersedes(&self) -> Vec<String> {
        let fm_val = self.frontmatter.get("supersedes").or_else(|| {
            self.frontmatter
                .get("relations")
                .and_then(|r| r.get("supersedes"))
        });

        extract_string_or_array(fm_val)
    }

    /// Google OKF compatibility: returns expiry/staleness deadline if declared in frontmatter.
    pub fn stale_after(&self) -> Option<String> {
        self.frontmatter
            .get("stale_after")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }
}

fn extract_string_or_array(val: Option<&serde_json::Value>) -> Vec<String> {
    match val {
        Some(serde_json::Value::String(s)) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                Vec::new()
            } else {
                vec![trimmed.to_string()]
            }
        }
        Some(serde_json::Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        _ => Vec::new(),
    }
}

/// Metrics and statistics summarizing a cache sync or scan execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SyncStats {
    pub added: usize,
    pub modified: usize,
    pub deleted: usize,
    pub unchanged: usize,
    pub duration_ms: u128,
}

impl SyncStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn total_processed(&self) -> usize {
        self.added + self.modified + self.deleted + self.unchanged
    }
}

/// Lightweight metadata for cached documents used in incremental sync dirty checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachedDocMeta {
    pub path: PathBuf,
    pub mtime: u64,
    pub content_hash: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_lifecycle_status_and_supersession() {
        let mut doc = Document {
            path: PathBuf::from("ADR-001.md"),
            title: "ADR 001".to_string(),
            hierarchy: HierarchyLevel::L3Evergreen,
            frontmatter: json!({}),
            links: Vec::new(),
            tags: Vec::new(),
            content_hash: "hash".to_string(),
            mtime: 0,
            body: "# Header\nSome text".to_string(),
        };

        // Default is active
        assert_eq!(doc.lifecycle_status(), NoteLifecycleStatus::Active);
        assert!(doc.is_active());
        assert_eq!(doc.superseded_by(), None);
        assert!(doc.supersedes().is_empty());
        assert_eq!(doc.stale_after(), None);

        // Frontmatter status: superseded
        doc.frontmatter = json!({ "status": "superseded" });
        assert_eq!(doc.lifecycle_status(), NoteLifecycleStatus::Superseded);
        assert!(!doc.is_active());

        // Frontmatter status: deprecated
        doc.frontmatter = json!({ "status": "deprecated" });
        assert_eq!(doc.lifecycle_status(), NoteLifecycleStatus::Deprecated);

        // Frontmatter status: archived
        doc.frontmatter = json!({ "status": "archived" });
        assert_eq!(doc.lifecycle_status(), NoteLifecycleStatus::Archived);

        // Frontmatter superseded_by
        doc.frontmatter = json!({
            "status": "active",
            "superseded_by": "ADR-002.md",
            "stale_after": "2026-12-31"
        });
        assert_eq!(doc.lifecycle_status(), NoteLifecycleStatus::Superseded);
        assert_eq!(doc.superseded_by(), Some("ADR-002.md".to_string()));
        assert_eq!(doc.stale_after(), Some("2026-12-31".to_string()));

        // Supersedes array
        doc.frontmatter = json!({
            "supersedes": ["ADR-001.md", "ADR-000.md"]
        });
        assert_eq!(doc.lifecycle_status(), NoteLifecycleStatus::Active);
        assert_eq!(doc.supersedes(), vec!["ADR-001.md", "ADR-000.md"]);

        // Relations object
        doc.frontmatter = json!({
            "relations": {
                "superseded_by": ["ADR-003.md", "ADR-004.md"],
                "supersedes": "ADR-001.md"
            }
        });
        assert_eq!(doc.lifecycle_status(), NoteLifecycleStatus::Superseded);
        assert_eq!(
            doc.superseded_by(),
            Some("ADR-003.md, ADR-004.md".to_string())
        );
        assert_eq!(doc.supersedes(), vec!["ADR-001.md"]);

        // Tag status
        doc.frontmatter = json!({});
        doc.tags = vec!["#status/obsolete".to_string()];
        assert_eq!(doc.lifecycle_status(), NoteLifecycleStatus::Deprecated);

        // Body callout
        doc.tags.clear();
        doc.body = "> [SUPERSEDED] Superseded by [[ADR-005.md]]\n\nContent here".to_string();
        assert_eq!(doc.lifecycle_status(), NoteLifecycleStatus::Superseded);
        assert_eq!(doc.superseded_by(), Some("[[ADR-005.md]]".to_string()));
    }
}
