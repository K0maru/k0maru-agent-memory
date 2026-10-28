use std::path::{Path, PathBuf};

use crate::adapters::{
    parse_hierarchy_override, read_document_impl, resolve_unique_path, scan_markdown_files,
};
use crate::core::models::{Document, HierarchyLevel};
use crate::core::traits::VaultAdapter;

/// Vault adapter for Obsidian / Johnny.Decimal vault structures.
#[derive(Debug, Clone)]
pub struct ObsidianAdapter {
    root: PathBuf,
}

impl ObsidianAdapter {
    /// Creates a new `ObsidianAdapter` mounted at the given vault root path.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Returns a reference to the vault root directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Determines the hierarchy level for a document in a vault.
    ///
    /// Priority order:
    /// 1. Explicit frontmatter `hierarchy:` or `level:` declaration.
    /// 2. Folder taxonomy heuristics (case-insensitive, topology-agnostic):
    ///    - Evergreen/project/decision folders -> `L3Evergreen`
    ///    - Log/journal folders -> `L2Log`
    ///    - Daily/inbox folders -> `L0Ephemeral`
    ///    - Default -> `L1Resource`
    ///
    /// This keeps Obsidian / Johnny.Decimal vault (`10_Projects`, `20_Cards`, `01_AI_Logs`, `00_Daily`)
    /// fully supported while also accepting flat LLM-Wikis, engineering
    /// `docs/adr` trees, and PARA layouts.
    fn determine_hierarchy(
        frontmatter: &serde_json::Value,
        relative_path: &Path,
    ) -> HierarchyLevel {
        if let Some(level) = parse_hierarchy_override(frontmatter) {
            return level;
        }

        let path_str = relative_path.to_string_lossy().replace('\\', "/");
        let segments: Vec<String> = path_str
            .split('/')
            .map(|segment| segment.to_lowercase())
            .collect();

        const EVERGREEN_SEGMENTS: &[&str] = &[
            "20_cards",
            "10_projects",
            "cards",
            "concepts",
            "decisions",
            "adr",
            "wiki",
            "rules",
            "skills",
            "playbooks",
            "evergreen",
        ];
        const LOG_SEGMENTS: &[&str] = &[
            "01_ai_logs",
            "30_logs",
            "logs",
            "ai_logs",
            "journal",
            "records",
            "postmortems",
        ];
        const EPHEMERAL_SEGMENTS: &[&str] = &["00_daily", "daily", "dailies", "inbox", "fleeting"];

        let has_segment = |candidates: &[&str]| {
            segments
                .iter()
                .any(|segment| candidates.contains(&segment.as_str()))
        };

        if has_segment(EVERGREEN_SEGMENTS) {
            HierarchyLevel::L3Evergreen
        } else if has_segment(LOG_SEGMENTS) {
            HierarchyLevel::L2Log
        } else if has_segment(EPHEMERAL_SEGMENTS) {
            HierarchyLevel::L0Ephemeral
        } else {
            HierarchyLevel::L1Resource
        }
    }
}

impl VaultAdapter for ObsidianAdapter {
    fn scan_files(&self) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        scan_markdown_files(&self.root)
    }

    fn read_document(&self, relative_path: &Path) -> Result<Document, Box<dyn std::error::Error>> {
        read_document_impl(&self.root, relative_path, Self::determine_hierarchy)
    }

    fn write_card(
        &self,
        title: &str,
        content: &str,
        hierarchy: HierarchyLevel,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let subfolder = match hierarchy {
            HierarchyLevel::L3Evergreen => "20_Cards",
            HierarchyLevel::L2Log => "01_AI_Logs",
            HierarchyLevel::L0Ephemeral => "00_Daily",
            HierarchyLevel::L1Resource => "10_Projects",
        };

        let target_dir = self.root.join(subfolder);
        std::fs::create_dir_all(&target_dir)?;

        let (full_path, filename) = resolve_unique_path(&target_dir, title);
        std::fs::write(&full_path, content)?;

        Ok(PathBuf::from(subfolder).join(filename))
    }

    fn root_path(&self) -> Option<&Path> {
        Some(&self.root)
    }
}
