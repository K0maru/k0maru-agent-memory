use std::path::{Path, PathBuf};

use crate::adapters::{
    parse_hierarchy_override, read_document_impl, resolve_unique_path, scan_markdown_files,
};
use crate::core::models::{Document, HierarchyLevel};
use crate::core::traits::VaultAdapter;

/// Vault adapter for generic flat LLM-Wiki structures (e.g. Karpathy-style wiki).
#[derive(Debug, Clone)]
pub struct GenericWikiAdapter {
    root: PathBuf,
    default_hierarchy: HierarchyLevel,
}

impl GenericWikiAdapter {
    /// Creates a new `GenericWikiAdapter` mounted at the given root path, defaulting to `L3Evergreen`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            default_hierarchy: HierarchyLevel::L3Evergreen,
        }
    }

    /// Sets the default hierarchy level to be used when frontmatter does not specify one.
    pub fn with_default_hierarchy(mut self, hierarchy: HierarchyLevel) -> Self {
        self.default_hierarchy = hierarchy;
        self
    }

    /// Returns a reference to the wiki root directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns the configured default hierarchy level.
    pub fn default_hierarchy(&self) -> HierarchyLevel {
        self.default_hierarchy
    }
}

impl VaultAdapter for GenericWikiAdapter {
    fn scan_files(&self) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        scan_markdown_files(&self.root)
    }

    fn read_document(&self, relative_path: &Path) -> Result<Document, Box<dyn std::error::Error>> {
        let default_hierarchy = self.default_hierarchy;
        read_document_impl(&self.root, relative_path, move |frontmatter, _rel_path| {
            if let Some(level) = parse_hierarchy_override(frontmatter) {
                level
            } else {
                default_hierarchy
            }
        })
    }

    fn write_card(
        &self,
        title: &str,
        content: &str,
        _hierarchy: HierarchyLevel,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        std::fs::create_dir_all(&self.root)?;

        let (full_path, filename) = resolve_unique_path(&self.root, title);
        std::fs::write(&full_path, content)?;

        Ok(PathBuf::from(filename))
    }

    fn root_path(&self) -> Option<&Path> {
        Some(&self.root)
    }
}
