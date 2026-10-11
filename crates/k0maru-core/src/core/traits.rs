use std::path::{Path, PathBuf};

use crate::core::models::{CachedDocMeta, Document, HierarchyLevel, WikiLink};

/// Seam trait abstracting vault access over different physical folder layouts (Obsidian, generic LLM-Wiki).
pub trait VaultAdapter {
    /// Scans all markdown files in the vault, returning their relative paths.
    fn scan_files(&self) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>>;

    /// Reads and parses a markdown document by its relative path.
    fn read_document(&self, relative_path: &Path) -> Result<Document, Box<dyn std::error::Error>>;

    /// Writes an atomic card (e.g. L3 evergreen note) to the vault and returns its relative path.
    fn write_card(
        &self,
        title: &str,
        content: &str,
        hierarchy: HierarchyLevel,
    ) -> Result<PathBuf, Box<dyn std::error::Error>>;

    /// Returns the root filesystem directory of the vault, if applicable.
    fn root_path(&self) -> Option<&Path> {
        None
    }

    /// Fast-path check: returns the filesystem mtime (in seconds) of a relative path in the vault.
    fn file_mtime(&self, relative_path: &Path) -> Result<Option<u64>, Box<dyn std::error::Error>> {
        if let Some(root) = self.root_path() {
            let full_path = root.join(relative_path);
            let metadata = std::fs::metadata(&full_path)?;
            let mtime = metadata
                .modified()?
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            Ok(Some(mtime))
        } else {
            Ok(None)
        }
    }

    /// Fast-path check: computes the xxHash3-64 hexadecimal hash of a relative file's raw content.
    fn compute_content_hash(
        &self,
        relative_path: &Path,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        if let Some(root) = self.root_path() {
            let full_path = root.join(relative_path);
            let bytes = std::fs::read(&full_path)?;
            let hash = xxhash_rust::xxh3::xxh3_64(&bytes);
            Ok(Some(format!("{:016x}", hash)))
        } else {
            Ok(None)
        }
    }
}

/// Seam trait abstracting ephemeral SQLite cache operations.
pub trait CacheStorage {
    /// Initializes tables and FTS5 indices.
    fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>>;

    /// Drops all tables and indices and re-initializes clean schema.
    fn wipe_and_rebuild(&mut self) -> Result<(), Box<dyn std::error::Error>>;

    /// Inserts or updates document record, adjacency links, and search indices.
    fn upsert_document(&mut self, doc: &Document) -> Result<(), Box<dyn std::error::Error>>;

    /// Deletes a document and its associated links from the cache.
    fn delete_document(&mut self, path: &Path) -> Result<(), Box<dyn std::error::Error>>;

    /// BM25 full-text search across indexed documents.
    fn search_fts(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<Document>, Box<dyn std::error::Error>>;

    /// Returns all outgoing WikiLinks from a given document path.
    fn get_outgoing_links(&self, path: &Path) -> Result<Vec<WikiLink>, Box<dyn std::error::Error>>;

    /// Returns paths of all documents linking to a target title (backlinks).
    fn get_backlinks(&self, target_title: &str)
        -> Result<Vec<PathBuf>, Box<dyn std::error::Error>>;

    /// Returns metadata (path, mtime, content_hash) for all documents currently in the cache.
    fn get_cached_metadata(&self) -> Result<Vec<CachedDocMeta>, Box<dyn std::error::Error>> {
        Ok(Vec::new())
    }

    /// Retrieves a document by its relative path if present in cache.
    fn get_document(&self, _path: &Path) -> Result<Option<Document>, Box<dyn std::error::Error>> {
        Ok(None)
    }
}
