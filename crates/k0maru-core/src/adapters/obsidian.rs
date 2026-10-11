use std::path::{Path, PathBuf};

use crate::adapters::{
    parse_hierarchy_override, read_document_impl, resolve_unique_path, scan_markdown_files,
};
use crate::core::models::{Document, HierarchyLevel};
use crate::core::traits::VaultAdapter;

/// Vault adapter for Obsidian SecondBrain structures.
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

    /// Determines the hierarchy level for a document in an Obsidian vault.
    ///
    /// Priority order:
    /// 1. Explicit frontmatter `hierarchy:` or `level:` declaration.
    /// 2. Folder taxonomy heuristics:
    ///    - `20_Cards` or `10_Projects` -> `L3Evergreen`
    ///    - `01_AI_Logs` -> `L2Log`
    ///    - `00_Daily` -> `L0Ephemeral`
    ///    - Default -> `L1Resource`
    fn determine_hierarchy(
        frontmatter: &serde_json::Value,
        relative_path: &Path,
    ) -> HierarchyLevel {
        if let Some(level) = parse_hierarchy_override(frontmatter) {
            return level;
        }

        let path_str = relative_path.to_string_lossy().replace('\\', "/");
        let segments: Vec<&str> = path_str.split('/').collect();

        if segments.contains(&"20_Cards") || segments.contains(&"10_Projects") {
            HierarchyLevel::L3Evergreen
        } else if segments.contains(&"01_AI_Logs") {
            HierarchyLevel::L2Log
        } else if segments.contains(&"00_Daily") {
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
