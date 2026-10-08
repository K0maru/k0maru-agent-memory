use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub mod flush;
pub mod sniffer;
pub mod synthesizer;

pub use flush::FlushEngine;
pub use sniffer::{generate_filename, slugify, ConventionSniffer};
pub use synthesizer::synthesize_markdown;

/// Note naming style convention detected or configured for the vault.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum NamingStyle {
    /// ISO Date prefixed: `YYYY-MM-DD-<slug>.md`
    #[default]
    DateSlug,
    /// Timestamp prefixed: `YYYYMMDDHHMM-<slug>.md`
    TimestampSlug,
    /// Slug only: `<slug>.md`
    SlugOnly,
    /// Preserves original Title case: `<Title>.md`
    TitleCase,
}

/// Semantic category of the note being crystallized.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NoteCategory {
    Decision,
    Log,
    Concept,
    Skill,
    Generic,
}

impl NoteCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            NoteCategory::Decision => "decision",
            NoteCategory::Log => "log",
            NoteCategory::Concept => "concept",
            NoteCategory::Skill => "skill",
            NoteCategory::Generic => "generic",
        }
    }

    pub fn from_str_loose(s: &str) -> Self {
        let lower = s.trim().to_lowercase();
        match lower.as_str() {
            "decision" | "decisions" | "adr" => NoteCategory::Decision,
            "log" | "logs" | "journal" | "daily" | "records" => NoteCategory::Log,
            "concept" | "concepts" | "wiki" | "card" | "cards" => NoteCategory::Concept,
            "skill" | "skills" | "playbook" | "playbooks" | "recipe" | "recipes"
            | "troubleshooting" => NoteCategory::Skill,
            _ => NoteCategory::Generic,
        }
    }
}

/// Sniffed conventions and structural configuration of a target vault.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaultConvention {
    /// Absolute canonical path to the vault root.
    pub vault_root: PathBuf,
    /// Whether explicit convention rules were detected (e.g. AGENTS.md, RULES.md).
    pub has_explicit_rules: bool,
    /// File path where explicit rules were discovered, if any.
    pub rule_source: Option<PathBuf>,
    /// Target subdirectories mapped by category (e.g. "decision" -> "decisions").
    pub target_subdirs: HashMap<String, PathBuf>,
    /// Dominant filename naming style.
    pub naming_style: NamingStyle,
    /// Whether existing notes commonly include YAML frontmatter.
    pub has_frontmatter: bool,
    /// Discovered template content if available.
    pub template: Option<String>,
}

impl VaultConvention {
    /// Resolve the relative target subdirectory for a given category.
    /// Falls back to empty path (meaning vault root) if not mapped.
    pub fn target_subdir_for(&self, category: &str) -> PathBuf {
        let norm_cat = category.trim().to_lowercase();
        if let Some(sub) = self.target_subdirs.get(&norm_cat) {
            return sub.clone();
        }
        // Check standard category mapping
        let loose = NoteCategory::from_str_loose(&norm_cat);
        if let Some(sub) = self.target_subdirs.get(loose.as_str()) {
            return sub.clone();
        }
        PathBuf::from("")
    }
}

/// Request to crystallize agent memory/learning into the target vault.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlushRequest {
    pub title: String,
    pub summary: Option<String>,
    pub content: String,
    pub category: String,
    pub tags: Vec<String>,
    pub related_notes: Vec<String>,
    pub dry_run: bool,
}

/// Result of a flush operation (simulated if dry_run, executed on disk otherwise).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlushResult {
    pub file_path: PathBuf,
    pub relative_path: PathBuf,
    pub category: String,
    pub created: bool,
    pub appended: bool,
    pub content_preview: String,
    pub dry_run: bool,
}
