//! Incremental scanner and vault cache synchronization engine.
//!
//! Provides dirty-checking change detection and synchronization between
//! physical Markdown vaults ([`VaultAdapter`]) and ephemeral SQLite cache ([`CacheStorage`]).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use crate::core::models::{CachedDocMeta, SyncStats};
use crate::core::traits::{CacheStorage, VaultAdapter};
use crate::scanner::vector_sync::{VectorSyncEngine, VectorSyncStats};
use crate::storage::SqliteStorage;
use crate::vector::EmbeddingEngine;

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
    section_repair_budget: Option<usize>,
}

impl<'a, A: VaultAdapter, S: CacheStorage> IncrementalScanner<'a, A, S> {
    /// Creates a new `IncrementalScanner` bound to the given vault adapter and cache storage.
    pub fn new(adapter: &'a A, storage: &'a mut S) -> Self {
        Self {
            adapter,
            storage,
            section_repair_budget: Some(256),
        }
    }

    /// Sets the maximum number of stale documents to section-index during `sync()` (`None` = unlimited).
    pub fn with_section_repair_budget(mut self, budget: Option<usize>) -> Self {
        self.section_repair_budget = budget;
        self
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
                    let disk_mtime = match self.adapter.file_mtime(&path) {
                        Ok(m) => m,
                        Err(err) => {
                            eprintln!(
                                "[k0maru] warning: skipping unreadable file {:?}: {}",
                                path, err
                            );
                            continue;
                        }
                    };
                    let disk_hash = match self.adapter.compute_content_hash(&path) {
                        Ok(h) => h,
                        Err(err) => {
                            eprintln!(
                                "[k0maru] warning: skipping unreadable file {:?}: {}",
                                path, err
                            );
                            continue;
                        }
                    };

                    let is_modified = match (disk_mtime, disk_hash) {
                        (Some(dm), Some(ref dh)) => {
                            dm != cached.mtime || dh != &cached.content_hash
                        }
                        (Some(dm), None) => dm != cached.mtime,
                        (None, Some(ref dh)) => dh != &cached.content_hash,
                        (None, None) => {
                            let doc = match self.adapter.read_document(&path) {
                                Ok(d) => d,
                                Err(err) => {
                                    eprintln!(
                                        "[k0maru] warning: skipping unreadable file {:?}: {}",
                                        path, err
                                    );
                                    continue;
                                }
                            };
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
    /// Added and modified documents are re-read through `adapter.read_document()` and upserted,
    /// unless excluded by frontmatter (`doc.is_index_excluded()`) in which case any stale entry is
    /// deleted and counted under `SyncStats.excluded`. Unreadable or non-UTF8 files are skipped with
    /// a warning instead of aborting the entire vault sync.
    /// Deleted documents are pruned via `storage.delete_document()`.
    /// Returns execution statistics as [`SyncStats`].
    pub fn sync(&mut self, force_full: bool) -> Result<SyncStats, Box<dyn std::error::Error>> {
        let start_time = Instant::now();
        let dirty = self.compute_dirty_sets(force_full)?;

        let mut added = 0usize;
        let mut modified = 0usize;
        let mut deleted = 0usize;
        let mut excluded = 0usize;

        for path in &dirty.added {
            let doc = match self.adapter.read_document(path) {
                Ok(d) => d,
                Err(err) if is_skippable_read_error(&*err) => {
                    eprintln!(
                        "[k0maru] warning: skipping unreadable file {:?}: {}",
                        path, err
                    );
                    continue;
                }
                Err(err) => return Err(err),
            };
            if doc.is_index_excluded() {
                self.storage.delete_document(path)?;
                excluded += 1;
            } else {
                self.storage.upsert_document(&doc)?;
                added += 1;
            }
        }

        for path in &dirty.modified {
            let doc = match self.adapter.read_document(path) {
                Ok(d) => d,
                Err(err) if is_skippable_read_error(&*err) => {
                    eprintln!(
                        "[k0maru] warning: skipping unreadable file {:?}: {}",
                        path, err
                    );
                    continue;
                }
                Err(err) => return Err(err),
            };
            if doc.is_index_excluded() {
                self.storage.delete_document(path)?;
                excluded += 1;
            } else {
                self.storage.upsert_document(&doc)?;
                modified += 1;
            }
        }

        for path in &dirty.deleted {
            self.storage.delete_document(path)?;
            deleted += 1;
        }

        // Heal any stale fts_hash rows (e.g. written by an older binary or segmenter bump)
        // and clean up any stray legacy documents_fts table.
        self.storage.repair_stale_fts()?;
        self.storage
            .repair_stale_sections(self.section_repair_budget)?;
        self.storage.rebuild_link_edges_if_dirty()?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.storage.meta_set("last_sync_at", &now.to_string())?;

        let duration_ms = start_time.elapsed().as_millis();

        Ok(SyncStats {
            added,
            modified,
            deleted,
            unchanged: dirty.unchanged.len(),
            excluded,
            duration_ms,
        })
    }
}

fn is_skippable_read_error(err: &(dyn std::error::Error + 'static)) -> bool {
    if let Some(io_err) = err.downcast_ref::<std::io::Error>() {
        return matches!(
            io_err.kind(),
            std::io::ErrorKind::InvalidData
                | std::io::ErrorKind::PermissionDenied
                | std::io::ErrorKind::NotFound
        );
    }
    false
}

impl<'a, A: VaultAdapter> IncrementalScanner<'a, A, SqliteStorage> {
    /// Synchronizes filesystem vault changes into SQLite storage and updates vector embeddings.
    ///
    /// If `embedder` is `Some`, calls [`VectorSyncEngine`] to incrementally embed new/modified
    /// documents and purge deleted ones. If `None`, vector synchronization is skipped.
    pub fn sync_vault_with_vector(
        &mut self,
        _vault_root: &Path,
        embedder: Option<Arc<dyn EmbeddingEngine>>,
    ) -> Result<(SyncStats, VectorSyncStats), Box<dyn std::error::Error>> {
        let sync_stats = self.sync(false)?;
        let vector_stats = if let Some(ref engine) = embedder {
            let doc_stats = VectorSyncEngine::sync(self.storage, &**engine)?;
            VectorSyncEngine::new().sync_sections(self.storage, &**engine, None)?;
            doc_stats
        } else {
            VectorSyncStats::default()
        };
        Ok((sync_stats, vector_stats))
    }

    /// Backwards-compatible convenience method to synchronize vault into SQLite storage.
    pub fn sync_vault(
        &mut self,
        vault_root: &Path,
    ) -> Result<SyncStats, Box<dyn std::error::Error>> {
        self.sync_vault_with_vector(vault_root, None)
            .map(|(s, _)| s)
    }
}
