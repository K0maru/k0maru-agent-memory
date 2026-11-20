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
    Project,
    Skill,
    Generic,
}

impl NoteCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            NoteCategory::Decision => "decision",
            NoteCategory::Log => "log",
            NoteCategory::Concept => "concept",
            NoteCategory::Project => "project",
            NoteCategory::Skill => "skill",
            NoteCategory::Generic => "generic",
        }
    }

    /// Parses an explicitly supported category or alias. Returns `None` for unrecognized values.
    pub fn parse_strict(s: &str) -> Option<Self> {
        let lower = s.trim().to_lowercase();
        match lower.as_str() {
            "decision" | "decisions" | "adr" | "adrs" => Some(NoteCategory::Decision),
            "" | "log" | "logs" | "journal" | "daily" | "records" => Some(NoteCategory::Log),
            "concept" | "concepts" | "wiki" | "card" | "cards" => Some(NoteCategory::Concept),
            "project" | "projects" => Some(NoteCategory::Project),
            "skill" | "skills" | "playbook" | "playbooks" | "recipe" | "recipes"
            | "troubleshooting" => Some(NoteCategory::Skill),
            _ => None,
        }
    }

    pub fn from_str_loose(s: &str) -> Self {
        Self::parse_strict(s).unwrap_or(NoteCategory::Generic)
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
    /// Per-category template contents discovered in template directories.
    #[serde(default)]
    pub category_templates: HashMap<String, String>,
}

impl VaultConvention {
    /// Resolves the best template for a given category.
    ///
    /// When category-specific templates exist, returns only a matching template
    /// (falling back to standard markdown instead of applying a wrong category's template).
    /// When only a single generic template was detected, falls back to `self.template`.
    pub fn template_for(&self, category: &str) -> Option<&str> {
        let norm_cat = category.trim().to_lowercase();
        if let Some(t) = self.category_templates.get(&norm_cat) {
            return Some(t.as_str());
        }
        let canonical = NoteCategory::from_str_loose(&norm_cat).as_str();
        if let Some(t) = self.category_templates.get(canonical) {
            return Some(t.as_str());
        }
        if canonical == "decision" {
            if let Some(t) = self.category_templates.get("concept") {
                return Some(t.as_str());
            }
        }
        if self.category_templates.is_empty() {
            self.template.as_deref()
        } else {
            None
        }
    }

    /// Resolve the relative target subdirectory for a given category.
    /// Defaults to structured subdirectories (`decisions`, `logs`, `projects`, `skills`)
    /// rather than writing directly into the vault root.
    pub fn target_subdir_for(&self, category: &str) -> PathBuf {
        let norm_cat = category.trim().to_lowercase();
        if let Some(sub) = self.target_subdirs.get(&norm_cat) {
            if !sub.as_os_str().is_empty() {
                return sub.clone();
            }
        }
        // Check standard category mapping
        let loose = NoteCategory::from_str_loose(&norm_cat);
        if let Some(sub) = self.target_subdirs.get(loose.as_str()) {
            if !sub.as_os_str().is_empty() {
                return sub.clone();
            }
        }
        match loose {
            NoteCategory::Decision | NoteCategory::Concept => PathBuf::from("decisions"),
            NoteCategory::Log => PathBuf::from("logs"),
            NoteCategory::Project => PathBuf::from("projects"),
            NoteCategory::Skill => PathBuf::from("skills"),
            NoteCategory::Generic => PathBuf::from("logs"),
        }
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
    /// Optional list of existing notes/ADRs that this note supersedes
    #[serde(default)]
    pub supersedes: Vec<String>,
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
    /// Relative paths of existing notes that were marked as superseded by this flush
    #[serde(default)]
    pub superseded_files: Vec<PathBuf>,
    /// Non-fatal warning when the note was written to disk but SQLite cache sync failed
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sync_warning: Option<String>,
}
