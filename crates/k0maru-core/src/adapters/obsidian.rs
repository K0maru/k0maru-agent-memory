use std::path::{Path, PathBuf};

use crate::adapters::{
    determine_llm_wiki_hierarchy, read_document_impl, resolve_unique_path, scan_markdown_files,
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
        determine_llm_wiki_hierarchy(frontmatter, relative_path, HierarchyLevel::L1Resource)
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
            HierarchyLevel::L3Evergreen => {
                if self.root.join("decisions").is_dir() {
                    "decisions"
                } else if self.root.join("cards").is_dir() {
                    "cards"
                } else if self.root.join("20_Cards").is_dir() {
                    "20_Cards"
                } else {
                    "decisions"
                }
            }
            HierarchyLevel::L2Log => {
                if self.root.join("logs").is_dir() {
                    "logs"
                } else if self.root.join("01_AI_Logs").is_dir() {
                    "01_AI_Logs"
                } else {
                    "logs"
                }
            }
            HierarchyLevel::L0Ephemeral => {
                if self.root.join("daily").is_dir() {
                    "daily"
                } else if self.root.join("00_Daily").is_dir() {
                    "00_Daily"
                } else {
                    "daily"
                }
            }
            HierarchyLevel::L1Resource => {
                if self.root.join("resources").is_dir() {
                    "resources"
                } else if self.root.join("30_Resources").is_dir() {
                    "30_Resources"
                } else {
                    "resources"
                }
            }
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
