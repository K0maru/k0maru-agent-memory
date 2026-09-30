//! Incremental scanner and vault cache synchronization engine.
//!
//! Provides dirty-checking change detection and synchronization between
//! physical Markdown vaults ([`VaultAdapter`]) and ephemeral SQLite cache ([`CacheStorage`]).

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::Instant;

use crate::core::models::{CachedDocMeta, SyncStats};
use crate::core::traits::{CacheStorage, VaultAdapter};

/// Categorized file paths representing filesystem vs cache dirty sets.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DirtySets {
    /// Files present on disk but not yet recorded in cache.
    pub added: Vec<PathBuf>,
    /// Files present in both disk and cache whose mtime or content hash changed.
    pub modified: Vec<PathBuf>,
    /// Files present in cache whose corresponding disk file has been removed.
    pub deleted: Vec<PathBuf>,
    /// Files present in both disk and cache with identical mtime and content hash.
    pub unchanged: Vec<PathBuf>,
}

impl DirtySets {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn total_processed(&self) -> usize {
        self.added.len() + self.modified.len() + self.deleted.len() + self.unchanged.len()
    }
}

/// High-throughput incremental file scanner and cache synchronizer.
pub struct IncrementalScanner<'a, A: VaultAdapter, S: CacheStorage> {
    adapter: &'a A,
    storage: &'a mut S,
}

impl<'a, A: VaultAdapter, S: CacheStorage> IncrementalScanner<'a, A, S> {
    /// Creates a new `IncrementalScanner` bound to the given vault adapter and cache storage.
    pub fn new(adapter: &'a A, storage: &'a mut S) -> Self {
        Self { adapter, storage }
    }

    /// Returns an immutable reference to the bound vault adapter.
    pub fn adapter(&self) -> &A {
        self.adapter
    }

    /// Returns an immutable reference to the bound cache storage.
    pub fn storage(&self) -> &S {
        self.storage
    }

    /// Returns a mutable reference to the bound cache storage.
    pub fn storage_mut(&mut self) -> &mut S {
        self.storage
    }

    /// Computes dirty sets (`added`, `modified`, `deleted`, `unchanged`) by comparing
    /// on-disk vault file metadata against SQLite cache metadata.
    ///
    /// If `force_full` is true, all existing cached files present on disk are treated as modified.
    pub fn compute_dirty_sets(
        &self,
        force_full: bool,
    ) -> Result<DirtySets, Box<dyn std::error::Error>> {
        let disk_files = self.adapter.scan_files()?;
        let cached_metas = self.storage.get_cached_metadata()?;

        let mut cached_map: HashMap<PathBuf, CachedDocMeta> =
            HashMap::with_capacity(cached_metas.len());
        for meta in cached_metas {
            cached_map.insert(meta.path.clone(), meta);
        }

        let mut added = Vec::new();
        let mut modified = Vec::new();
        let mut unchanged = Vec::new();
        let mut seen_cached_paths = HashSet::with_capacity(disk_files.len());

        for path in disk_files {
            if let Some(cached) = cached_map.get(&path) {
                seen_cached_paths.insert(path.clone());

                if force_full {
                    modified.push(path);
                } else {
                    let disk_mtime = self.adapter.file_mtime(&path)?;
                    let disk_hash = self.adapter.compute_content_hash(&path)?;

                    let is_modified = match (disk_mtime, disk_hash) {
                        (Some(dm), Some(ref dh)) => {
                            dm != cached.mtime || dh != &cached.content_hash
                        }
                        (Some(dm), None) => dm != cached.mtime,
                        (None, Some(ref dh)) => dh != &cached.content_hash,
                        (None, None) => {
                            let doc = self.adapter.read_document(&path)?;
                            doc.content_hash != cached.content_hash || doc.mtime != cached.mtime
                        }
                    };

                    if is_modified {
                        modified.push(path);
                    } else {
                        unchanged.push(path);
                    }
                }
            } else {
                added.push(path);
            }
        }

        let mut deleted = Vec::new();
        for cached_path in cached_map.keys() {
            if !seen_cached_paths.contains(cached_path) {
                deleted.push(cached_path.clone());
            }
        }

        added.sort();
        modified.sort();
        deleted.sort();
        unchanged.sort();

        Ok(DirtySets {
            added,
            modified,
            deleted,
            unchanged,
        })
    }

    /// Synchronizes changes into cache storage based on computed dirty sets.
    ///
    /// Added and modified documents are re-read through `adapter.read_document()` and upserted.
    /// Deleted documents are pruned via `storage.delete_document()`.
    /// Returns execution statistics as [`SyncStats`].
    pub fn sync(&mut self, force_full: bool) -> Result<SyncStats, Box<dyn std::error::Error>> {
        let start_time = Instant::now();
        let dirty = self.compute_dirty_sets(force_full)?;

        for path in &dirty.added {
            let doc = self.adapter.read_document(path)?;
            self.storage.upsert_document(&doc)?;
        }

        for path in &dirty.modified {
            let doc = self.adapter.read_document(path)?;
            self.storage.upsert_document(&doc)?;
        }

        for path in &dirty.deleted {
            self.storage.delete_document(path)?;
        }

        let duration_ms = start_time.elapsed().as_millis();

        Ok(SyncStats {
            added: dirty.added.len(),
            modified: dirty.modified.len(),
            deleted: dirty.deleted.len(),
            unchanged: dirty.unchanged.len(),
            duration_ms,
        })
    }
}
