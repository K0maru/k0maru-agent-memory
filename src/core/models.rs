use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Semantic lifecycle hierarchy level of notes in the knowledge hub.
/// - L0 (Ephemeral): Daily notes, scratchpad, transient context
/// - L1 (Resource): Static references, cheatsheets, project assets
/// - L2 (Log): AI analysis traces, evaluation runs, technical logs
/// - L3 (Evergreen): Atomic permanent notes, architectural principles
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum HierarchyLevel {
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
