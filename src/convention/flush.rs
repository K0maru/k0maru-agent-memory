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
                if existing_content.trim() == content.trim() {
                    let rel_path = candidate_path
                        .strip_prefix(&self.vault_root)
                        .unwrap_or(&candidate_path)
                        .to_path_buf();
                    let superseded_files = self.apply_supersedes(&request, &rel_path)?;
                    return Ok(FlushResult {
                        file_path: candidate_path,
                        relative_path: rel_path,
                        category: request.category,
                        created: false,
                        appended: false,
                        content_preview: content,
                        dry_run: request.dry_run,
                        superseded_files,
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
                    if existing_content.trim() == content.trim() {
                        let rel_path = next_candidate
                            .strip_prefix(&self.vault_root)
                            .unwrap_or(&next_candidate)
                            .to_path_buf();
                        let superseded_files = self.apply_supersedes(&request, &rel_path)?;
                        return Ok(FlushResult {
                            file_path: next_candidate,
                            relative_path: rel_path,
                            category: request.category,
                            created: false,
                            appended: false,
                            content_preview: content,
                            dry_run: request.dry_run,
                            superseded_files,
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

        // 6. Pre-validate every supersedes target BEFORE writing anything, so a
        // typo or ambiguous reference never leaves an orphan note on disk.
        let resolved_supersedes = self.resolve_supersedes(&request, &rel_path)?;

        // 6b. Dry run handling
        if request.dry_run {
            return Ok(FlushResult {
                file_path: candidate_path,
                relative_path: rel_path,
                category: request.category,
                created: true,
                appended: false,
                content_preview: content,
                dry_run: true,
                superseded_files: Vec::new(),
            });
        }

        // 7. Live write to disk
        if let Some(parent) = candidate_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(&candidate_path, &content)?;

        // 7b. Mark superseded notes before cache sync
        let superseded_files = Self::apply_resolved_supersedes(&resolved_supersedes, &rel_path)?;

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
            superseded_files,
        })
    }

    /// Resolves every `--supersedes` reference without touching the disk.
    fn resolve_supersedes(
        &self,
        request: &FlushRequest,
        new_note_rel_path: &Path,
    ) -> Result<Vec<(PathBuf, PathBuf)>, Box<dyn std::error::Error + Send + Sync>> {
        let mut resolved = Vec::new();
        for target in &request.supersedes {
            if let Some(entry) =
                resolve_supersede_target(&self.vault_root, target, Some(new_note_rel_path))?
            {
                resolved.push(entry);
            }
        }
        Ok(resolved)
    }

    /// Resolves then marks all superseded notes (idempotent early-return path).
    fn apply_supersedes(
        &self,
        request: &FlushRequest,
        new_note_rel_path: &Path,
    ) -> Result<Vec<PathBuf>, Box<dyn std::error::Error + Send + Sync>> {
        if request.dry_run || request.supersedes.is_empty() {
            return Ok(Vec::new());
        }
        let resolved = self.resolve_supersedes(request, new_note_rel_path)?;
        Self::apply_resolved_supersedes(&resolved, new_note_rel_path)
    }

    /// Rewrites pre-resolved supersede targets and returns their relative paths.
    fn apply_resolved_supersedes(
        resolved: &[(PathBuf, PathBuf)],
        new_note_rel_path: &Path,
    ) -> Result<Vec<PathBuf>, Box<dyn std::error::Error + Send + Sync>> {
        let mut files = Vec::new();
        for (canonical_target, rel_path) in resolved {
            rewrite_superseded_note(canonical_target, new_note_rel_path)?;
            files.push(rel_path.clone());
        }
        Ok(files)
    }
}

fn collect_markdown_files(dir: &Path, results: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                if name.starts_with('.') || name == "node_modules" || name == "target" {
                    continue;
                }
            }
            if path.is_dir() {
                collect_markdown_files(&path, results);
            } else if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md") {
                results.push(path);
            }
        }
    }
}

/// Resolves a `--supersedes` target to `(canonical_path, vault_relative_path)`.
///
/// Returns `Ok(None)` when the target resolves to the note being written
/// (a self-reference is a no-op). Missing and ambiguous targets are hard
/// errors so a typo in `supersedes` never silently leaves a stale ADR active.
fn resolve_supersede_target(
    vault_root: &Path,
    target: &str,
    new_note_rel_path: Option<&Path>,
) -> Result<Option<(PathBuf, PathBuf)>, Box<dyn std::error::Error + Send + Sync>> {
    let ambiguous = || -> Box<dyn std::error::Error + Send + Sync> {
        format!(
            "Supersedes target '{}' not found or ambiguous",
            target.trim()
        )
        .into()
    };

    let mut clean = target.trim();
    if let Some(stripped) = clean.strip_prefix("[[") {
        clean = stripped;
    }
    if let Some(stripped) = clean.strip_suffix("]]") {
        clean = stripped;
    }
    if let Some((before_alias, _)) = clean.split_once('|') {
        clean = before_alias.trim();
    }
    clean = clean.trim();
    if let Some(stripped) = clean.strip_suffix(".md") {
        clean = stripped.trim();
    }
    if clean.is_empty() {
        return Err(ambiguous());
    }

    let canonical_root = vault_root
        .canonicalize()
        .unwrap_or_else(|_| vault_root.to_path_buf());

    // 1. Direct relative path: unambiguous by construction.
    let direct_with_ext = vault_root.join(format!("{}.md", clean));
    let direct_as_is = vault_root.join(clean);
    let found_path = if direct_with_ext.is_file() {
        Some(direct_with_ext)
    } else if direct_as_is.is_file() {
        Some(direct_as_is)
    } else {
        None
    };

    // 2. Vault-wide exact resolution (relative path, stem or slug). Substring
    //    matching is deliberately not used: `ADR-01` must never silently
    //    supersede `ADR-010-legacy`.
    let target_file = match found_path {
        Some(path) => path,
        None => {
            let stem = Path::new(clean)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(clean);
            let stem_lower = stem.to_lowercase();
            let stem_slug = crate::convention::slugify(stem);

            let mut all_mds = Vec::new();
            collect_markdown_files(vault_root, &mut all_mds);

            // Never let the note being written supersede itself.
            let candidates: Vec<PathBuf> = all_mds
                .into_iter()
                .filter(|path| match path.canonicalize() {
                    Ok(canonical) => canonical
                        .strip_prefix(&canonical_root)
                        .map(|rel| {
                            new_note_rel_path
                                .map(|new_note| rel != new_note)
                                .unwrap_or(true)
                        })
                        .unwrap_or(true),
                    Err(_) => true,
                })
                .collect();

            let exact_matches: Vec<PathBuf> = candidates
                .iter()
                .filter(|path| {
                    path.file_stem()
                        .and_then(|s| s.to_str())
                        .map(|s| {
                            s.eq_ignore_ascii_case(stem)
                                || s.to_lowercase() == stem_lower
                                || (stem_slug != "untitled"
                                    && crate::convention::slugify(s) == stem_slug)
                        })
                        .unwrap_or(false)
                })
                .cloned()
                .collect();

            if exact_matches.len() > 1 {
                return Err(ambiguous());
            }
            if let Some(single) = exact_matches.into_iter().next() {
                single
            } else {
                // Flushed filenames frequently carry a date prefix or a title
                // suffix. Accept only a unique match anchored on a slug
                // boundary; never a free substring (ADR-01 must not match
                // ADR-010-legacy).
                let prefix = format!("{}-", stem_slug);
                let suffix = format!("-{}", stem_slug);
                let boundary_matches: Vec<PathBuf> = candidates
                    .iter()
                    .filter(|path| {
                        path.file_stem()
                            .and_then(|s| s.to_str())
                            .map(|s| {
                                let slug = crate::convention::slugify(s);
                                slug == stem_slug
                                    || slug.starts_with(&prefix)
                                    || slug.ends_with(&suffix)
                            })
                            .unwrap_or(false)
                    })
                    .cloned()
                    .collect();

                if boundary_matches.len() != 1 {
                    return Err(ambiguous());
                }
                boundary_matches.into_iter().next().ok_or_else(&ambiguous)?
            }
        }
    };

    let canonical_target = target_file.canonicalize().map_err(|_| ambiguous())?;
    if !canonical_target.starts_with(&canonical_root) {
        return Err(ambiguous());
    }
    let rel_path = canonical_target
        .strip_prefix(&canonical_root)
        .map_err(|_| ambiguous())?
        .to_path_buf();
    if new_note_rel_path.map(|new_note| new_note == rel_path.as_path()) == Some(true) {
        return Ok(None);
    }

    Ok(Some((canonical_target, rel_path)))
}

/// Rewrites an already-resolved note so its frontmatter and body mark it as
/// superseded by the newly flushed note.
fn rewrite_superseded_note(
    canonical_target: &Path,
    new_note_rel_path: &Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let without_ext = new_note_rel_path.with_extension("");
    let new_wikilink = without_ext.to_string_lossy().replace('\\', "/");

    let content = fs::read_to_string(canonical_target)?;
    // A UTF-8 BOM must not hide the frontmatter delimiter.
    let content = content.strip_prefix('\u{feff}').unwrap_or(&content);

    let updated_content = if content.starts_with("---\n") || content.starts_with("---\r\n") {
        let line_ending = if content.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        let lines: Vec<&str> = content.lines().collect();
        let mut end_fm_idx = None;
        for (idx, line) in lines.iter().enumerate().skip(1) {
            if line.trim() == "---" {
                end_fm_idx = Some(idx);
                break;
            }
        }

        if let Some(end_idx) = end_fm_idx {
            let mut fm_lines: Vec<String> =
                lines[1..end_idx].iter().map(|s| s.to_string()).collect();
            let body_lines = &lines[(end_idx + 1)..];
            let body_str = body_lines.join(line_ending);

            let mut status_found = false;
            for line in &mut fm_lines {
                // Only rewrite a top-level key; a nested `review:\n  status:`
                // block must not be clobbered.
                if line.starts_with("status:") {
                    *line = "status: superseded".to_string();
                    status_found = true;
                    break;
                }
            }
            if !status_found {
                fm_lines.push("status: superseded".to_string());
            }

            let target_wikilink_entry = format!("superseded_by: \"[[{}]]\"", new_wikilink);
            let mut superseded_by_found = false;
            for line in &mut fm_lines {
                if line.starts_with("superseded_by:") {
                    *line = target_wikilink_entry.clone();
                    superseded_by_found = true;
                    break;
                }
            }
            if !superseded_by_found {
                fm_lines.push(target_wikilink_entry);
            }

            let callout = format!("> [SUPERSEDED] Superseded by [[{}]]", new_wikilink);
            let body_trimmed = body_str.trim_start();
            let new_body = if body_trimmed.contains("> [SUPERSEDED]") {
                body_trimmed.to_string()
            } else if body_trimmed.is_empty() {
                callout
            } else {
                format!("{}\n\n{}", callout, body_trimmed)
            };

            format!("---\n{}\n---\n\n{}\n", fm_lines.join("\n"), new_body)
        } else {
            let callout = format!("> [SUPERSEDED] Superseded by [[{}]]", new_wikilink);
            let body_trimmed = content.trim_start();
            let new_body = if body_trimmed.contains("> [SUPERSEDED]") {
                body_trimmed.to_string()
            } else if body_trimmed.is_empty() {
                callout
            } else {
                format!("{}\n\n{}", callout, body_trimmed)
            };
            format!(
                "---\nstatus: superseded\nsuperseded_by: \"[[{}]]\"\n---\n\n{}\n",
                new_wikilink, new_body
            )
        }
    } else {
        let callout = format!("> [SUPERSEDED] Superseded by [[{}]]", new_wikilink);
        let body_trimmed = content.trim_start();
        let new_body = if body_trimmed.contains("> [SUPERSEDED]") {
            body_trimmed.to_string()
        } else if body_trimmed.is_empty() {
            callout
        } else {
            format!("{}\n\n{}", callout, body_trimmed)
        };
        format!(
            "---\nstatus: superseded\nsuperseded_by: \"[[{}]]\"\n---\n\n{}\n",
            new_wikilink, new_body
        )
    };

    fs::write(canonical_target, updated_content)?;
    Ok(())
}

/// Marks an existing note/ADR as superseded by a newly flushed note.
/// Updates YAML frontmatter `status: superseded`, `superseded_by: "[[{new_wikilink}]]"`,
/// and inserts `> [SUPERSEDED] Superseded by [[{new_wikilink}]]` into the body.
///
/// Returns `Ok(None)` when the target resolves to the note being written
/// (a self-reference is a no-op). Missing and ambiguous targets are hard
/// errors so a typo in `supersedes` never silently leaves a stale ADR active.
pub fn mark_note_superseded(
    vault_root: &Path,
    target: &str,
    new_note_rel_path: &Path,
) -> Result<Option<PathBuf>, Box<dyn std::error::Error + Send + Sync>> {
    match resolve_supersede_target(vault_root, target, Some(new_note_rel_path))? {
        Some((canonical_target, rel_path)) => {
            rewrite_superseded_note(&canonical_target, new_note_rel_path)?;
            Ok(Some(rel_path))
        }
        None => Ok(None),
    }
}
