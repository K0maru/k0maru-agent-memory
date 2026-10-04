use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::adapters::{GenericWikiAdapter, ObsidianAdapter};
use crate::convention::{
    generate_filename, synthesize_markdown, ConventionSniffer, FlushRequest, FlushResult,
    VaultConvention,
};
use crate::scanner::{IncrementalScanner, VectorSyncEngine};
use crate::storage::SqliteStorage;

/// Safe write-back and memory crystallization engine adhering to vault conventions.
pub struct FlushEngine {
    vault_root: PathBuf,
    convention: VaultConvention,
}

impl FlushEngine {
    /// Creates a new `FlushEngine` targeting the given vault root.
    /// Sniffs the vault's conventions dynamically.
    pub fn new(
        vault_root: impl Into<PathBuf>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let raw_path = vault_root.into();
        let canonical_root = if raw_path.is_absolute() {
            raw_path.canonicalize().unwrap_or(raw_path)
        } else {
            let current = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let joined = current.join(&raw_path);
            joined.canonicalize().unwrap_or(joined)
        };

        if !canonical_root.exists() {
            return Err(format!(
                "Vault root directory does not exist: {}",
                canonical_root.display()
            )
            .into());
        }

        let convention = ConventionSniffer::sniff(&canonical_root);
        Ok(Self {
            vault_root: canonical_root,
            convention,
        })
    }

    /// Sets or overrides the detected vault convention.
    pub fn with_convention(mut self, convention: VaultConvention) -> Self {
        self.convention = convention;
        self
    }

    /// Returns the canonical path to the vault root directory.
    pub fn vault_root(&self) -> &Path {
        &self.vault_root
    }

    /// Returns the active vault convention.
    pub fn convention(&self) -> &VaultConvention {
        &self.convention
    }

    /// Flushes crystallized memory or notes into the vault.
    pub fn flush(
        &self,
        request: FlushRequest,
    ) -> Result<FlushResult, Box<dyn std::error::Error + Send + Sync>> {
        // 1. Determine target directory
        let rel_target_dir = self.convention.target_subdir_for(&request.category);

        // Security check: ensure target subdirectory does not attempt traversal
        for comp in rel_target_dir.components() {
            if matches!(
                comp,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            ) {
                return Err("Security error: target directory escapes vault root".into());
            }
        }
        let target_dir = self.vault_root.join(&rel_target_dir);

        // 2. Generate base filename
        let base_filename = generate_filename(&request.title, &self.convention.naming_style, None);

        // Security check: ensure base filename does not attempt traversal
        let file_path_base = Path::new(&base_filename);
        if file_path_base.components().any(|c| {
            matches!(
                c,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        }) {
            return Err("Security error: generated filename attempts directory traversal".into());
        }

        // 3. Synthesize markdown content
        let content = synthesize_markdown(&request, &self.convention);

        // 4. Resolve stem and extension for anti-collision
        let stem = file_path_base
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(&base_filename);
        let ext = file_path_base
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("md");

        // 5. Anti-Collision Resolution
        let mut candidate_path = target_dir.join(&base_filename);

        if candidate_path.exists() {
            // Check if existing content is identical
            if let Ok(existing_content) = fs::read_to_string(&candidate_path) {
                if existing_content == content {
                    let rel_path = candidate_path
                        .strip_prefix(&self.vault_root)
                        .unwrap_or(&candidate_path)
                        .to_path_buf();
                    return Ok(FlushResult {
                        file_path: candidate_path,
                        relative_path: rel_path,
                        category: request.category,
                        created: false,
                        appended: false,
                        content_preview: content,
                        dry_run: request.dry_run,
                    });
                }
            }

            // File exists with different content: find next available suffix
            let mut counter = 2;
            loop {
                let candidate_filename = format!("{}-v{}.{}", stem, counter, ext);
                let next_candidate = target_dir.join(&candidate_filename);
                if !next_candidate.exists() {
                    candidate_path = next_candidate;
                    break;
                }
                if let Ok(existing_content) = fs::read_to_string(&next_candidate) {
                    if existing_content == content {
                        let rel_path = next_candidate
                            .strip_prefix(&self.vault_root)
                            .unwrap_or(&next_candidate)
                            .to_path_buf();
                        return Ok(FlushResult {
                            file_path: next_candidate,
                            relative_path: rel_path,
                            category: request.category,
                            created: false,
                            appended: false,
                            content_preview: content,
                            dry_run: request.dry_run,
                        });
                    }
                }
                counter += 1;
            }
        }

        // Security check: final candidate path must reside within vault root
        if !candidate_path.starts_with(&self.vault_root) {
            return Err("Security error: destination path escapes vault root".into());
        }

        let rel_path = candidate_path
            .strip_prefix(&self.vault_root)
            .unwrap_or(&candidate_path)
            .to_path_buf();

        // 6. Dry run handling
        if request.dry_run {
            return Ok(FlushResult {
                file_path: candidate_path,
                relative_path: rel_path,
                category: request.category,
                created: true,
                appended: false,
                content_preview: content,
                dry_run: true,
            });
        }

        // 7. Live write to disk
        if let Some(parent) = candidate_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(&candidate_path, &content)?;

        // 8. Auto-sync cache if SQLite cache exists
        let cache_path = self.vault_root.join(".k0maru").join("cache.sqlite");
        if cache_path.exists() {
            if let Ok(mut storage) = SqliteStorage::open(&cache_path) {
                let is_obsidian = self.vault_root.join("10_Projects").is_dir()
                    || self.vault_root.join(".obsidian").exists()
                    || self.vault_root.join("20_Cards").is_dir();

                let model_cache = if self.vault_root.join(".k0maru").join("models").exists() {
                    Some(self.vault_root.join(".k0maru").join("models"))
                } else {
                    None
                };
                let embedder = crate::vector::default_embedding_engine(model_cache).ok();

                let has_vectors = storage.has_vectors().unwrap_or(false);

                if is_obsidian {
                    let adapter = ObsidianAdapter::new(&self.vault_root);
                    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
                    let _ = scanner.sync(false);
                    if has_vectors {
                        if let Some(ref emb) = embedder {
                            let _ = VectorSyncEngine::sync(scanner.storage_mut(), &**emb);
                        }
                    }
                } else {
                    let adapter = GenericWikiAdapter::new(&self.vault_root);
                    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
                    let _ = scanner.sync(false);
                    if has_vectors {
                        if let Some(ref emb) = embedder {
                            let _ = VectorSyncEngine::sync(scanner.storage_mut(), &**emb);
                        }
                    }
                }
            }
        }

        Ok(FlushResult {
            file_path: candidate_path,
            relative_path: rel_path,
            category: request.category,
            created: true,
            appended: false,
            content_preview: content,
            dry_run: false,
        })
    }
}
