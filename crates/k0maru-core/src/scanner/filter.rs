//! Vault scan filtering backed by `.k0maruignore` gitignore rules,
//! built-in binary/canvas markdown exclusions, and opt-in canonical-sibling rules.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use ignore::gitignore::{Gitignore, GitignoreBuilder};

const IGNORE_FILE_NAME: &str = ".k0maruignore";
const CANONICAL_SIBLING_DIRECTIVE: &str = "k0maru:canonical-sibling";

/// Path-based scan filter evaluated during vault directory traversal.
///
/// Combines:
/// 1. Built-in exclusion of `*.excalidraw.md` files (case-insensitive).
/// 2. Gitignore-style patterns loaded from `<vault_root>/.k0maruignore` (if present).
/// 3. Opt-in `# k0maru:canonical-sibling = <filename>` directives inside `.k0maruignore`
///    (or configured via [`ScanFilter::with_canonical_siblings`]): when a directory
///    contains `<filename>`, all other `*.md` files directly in that directory are excluded.
#[derive(Debug)]
pub struct ScanFilter {
    root: PathBuf,
    gitignore: Gitignore,
    canonical_siblings: Vec<String>,
    canonical_dir_cache: Mutex<HashMap<PathBuf, bool>>,
}

impl Clone for ScanFilter {
    fn clone(&self) -> Self {
        Self {
            root: self.root.clone(),
            gitignore: self.gitignore.clone(),
            canonical_siblings: self.canonical_siblings.clone(),
            canonical_dir_cache: Mutex::new(HashMap::new()),
        }
    }
}

impl ScanFilter {
    /// Creates an empty filter rooted at `root` (still enforcing built-in defaults such as `*.excalidraw.md`).
    pub fn empty(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            gitignore: Gitignore::empty(),
            canonical_siblings: Vec::new(),
            canonical_dir_cache: Mutex::new(HashMap::new()),
        }
    }

    /// Loads `.k0maruignore` rules and directives from `<root>/.k0maruignore` if present.
    pub fn from_vault_root(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let ignore_path = root.join(IGNORE_FILE_NAME);
        if !ignore_path.is_file() {
            return Self::empty(root);
        }

        let mut canonical_siblings = Vec::new();
        match std::fs::read(&ignore_path) {
            Ok(bytes) => {
                let text = String::from_utf8_lossy(&bytes);
                for line in text.lines() {
                    parse_canonical_sibling_directive(line, &mut canonical_siblings);
                }
            }
            Err(err) => {
                eprintln!(
                    "[k0maru] warning: failed to read {}: {}",
                    ignore_path.display(),
                    err
                );
            }
        }

        let mut builder = GitignoreBuilder::new(&root);
        if let Some(err) = builder.add(&ignore_path) {
            eprintln!(
                "[k0maru] warning: failed to parse {}: {}",
                ignore_path.display(),
                err
            );
        }

        let gitignore = match builder.build() {
            Ok(gi) => gi,
            Err(err) => {
                eprintln!(
                    "[k0maru] warning: failed to compile {} rules: {}",
                    ignore_path.display(),
                    err
                );
                Gitignore::empty()
            }
        };

        Self {
            root,
            gitignore,
            canonical_siblings,
            canonical_dir_cache: Mutex::new(HashMap::new()),
        }
    }

    /// Appends opt-in canonical sibling filenames to this filter.
    ///
    /// When a directory contains a regular file matching any configured sibling name,
    /// all other `*.md` files directly inside that directory are excluded at scan time.
    pub fn with_canonical_siblings<I, S>(mut self, siblings: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for raw in siblings {
            let trimmed = raw.into().trim().to_string();
            if !trimmed.is_empty() && !self.canonical_siblings.iter().any(|s| s == &trimmed) {
                self.canonical_siblings.push(trimmed);
            }
        }
        if let Ok(mut cache) = self.canonical_dir_cache.lock() {
            cache.clear();
        }
        self
    }

    /// Returns the configured canonical sibling filenames.
    pub fn canonical_siblings(&self) -> &[String] {
        &self.canonical_siblings
    }

    /// Returns `true` if `rel_path` (relative to the vault root) should be excluded from scanning.
    pub fn is_excluded(&self, rel_path: &Path, is_dir: bool) -> bool {
        let rel_path = rel_path.strip_prefix(".").unwrap_or(rel_path);
        if rel_path.as_os_str().is_empty() {
            return false;
        }

        // 1. Built-in exclusion: *.excalidraw.md (case-insensitive)
        if !is_dir {
            if let Some(file_name) = rel_path.file_name().and_then(|n| n.to_str()) {
                if file_name.to_lowercase().ends_with(".excalidraw.md") {
                    return true;
                }
            }
        }

        // 2. .k0maruignore gitignore-style matching (respecting parent dir matches and `!` negations)
        if self
            .gitignore
            .matched_path_or_any_parents(rel_path, is_dir)
            .is_ignore()
        {
            return true;
        }

        // 3. Opt-in canonical-sibling exclusion for *.md files in the same directory
        if !is_dir && !self.canonical_siblings.is_empty() {
            let is_md = rel_path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("md"));
            if is_md {
                if let Some(file_name) = rel_path.file_name().and_then(|n| n.to_str()) {
                    if !self.canonical_siblings.iter().any(|c| c == file_name) {
                        let parent_rel = rel_path.parent().unwrap_or_else(|| Path::new(""));
                        if self.dir_has_canonical_sibling(parent_rel) {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    fn dir_has_canonical_sibling(&self, parent_rel: &Path) -> bool {
        if let Ok(cache) = self.canonical_dir_cache.lock() {
            if let Some(&cached) = cache.get(parent_rel) {
                return cached;
            }
        }

        let abs_dir = if parent_rel.as_os_str().is_empty() {
            self.root.clone()
        } else {
            self.root.join(parent_rel)
        };

        let mut found = false;
        if let Ok(entries) = std::fs::read_dir(&abs_dir) {
            for entry in entries.flatten() {
                let is_file = entry.file_type().map(|ft| ft.is_file()).unwrap_or(false);
                if !is_file {
                    continue;
                }
                if let Some(name) = entry.file_name().to_str() {
                    if self.canonical_siblings.iter().any(|c| c == name) {
                        let candidate_rel = if parent_rel.as_os_str().is_empty() {
                            PathBuf::from(name)
                        } else {
                            parent_rel.join(name)
                        };
                        if !self
                            .gitignore
                            .matched_path_or_any_parents(&candidate_rel, false)
                            .is_ignore()
                        {
                            found = true;
                            break;
                        }
                    }
                }
            }
        }

        if let Ok(mut cache) = self.canonical_dir_cache.lock() {
            cache.insert(parent_rel.to_path_buf(), found);
        }
        found
    }
}

/// Parses `# k0maru:canonical-sibling = <filename>[, <filename2>...]` comment directives.
fn parse_canonical_sibling_directive(line: &str, out: &mut Vec<String>) {
    let trimmed = line.trim();
    let Some(after_hash) = trimmed.strip_prefix('#') else {
        return;
    };
    let after_hash = after_hash.trim_start();
    let Some(after_key) = after_hash.strip_prefix(CANONICAL_SIBLING_DIRECTIVE) else {
        return;
    };
    let after_key = after_key.trim_start();
    let Some(rhs) = after_key.strip_prefix('=') else {
        return;
    };

    for part in rhs.split(',') {
        let cleaned = part.trim().trim_matches(|c| c == '"' || c == '\'').trim();
        if !cleaned.is_empty() && !out.iter().any(|existing| existing == cleaned) {
            out.push(cleaned.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_excalidraw_suffix_exclusion_case_insensitive() {
        let tmp = tempdir().unwrap();
        let filter = ScanFilter::empty(tmp.path());

        assert!(filter.is_excluded(Path::new("diagram.excalidraw.md"), false));
        assert!(filter.is_excluded(Path::new("sub/Diagram.Excalidraw.MD"), false));
        assert!(!filter.is_excluded(Path::new("sub/normal.md"), false));
    }

    #[test]
    fn test_k0maruignore_patterns_and_negation() {
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        fs::write(
            root.join(".k0maruignore"),
            "drafts/\n*.tmp.md\nnotes/*.md\n!notes/keep.md\n/root_only.md\n",
        )
        .unwrap();

        let filter = ScanFilter::from_vault_root(root);

        assert!(filter.is_excluded(Path::new("drafts"), true));
        assert!(filter.is_excluded(Path::new("drafts/wip.md"), false));
        assert!(filter.is_excluded(Path::new("sub/scratch.tmp.md"), false));
        assert!(filter.is_excluded(Path::new("notes/drop.md"), false));
        assert!(!filter.is_excluded(Path::new("notes/keep.md"), false));
        assert!(filter.is_excluded(Path::new("root_only.md"), false));
        assert!(!filter.is_excluded(Path::new("nested/root_only.md"), false));
    }

    #[test]
    fn test_canonical_sibling_directive_parsing_and_matching() {
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        fs::write(
            root.join(".k0maruignore"),
            "# k0maru:canonical-sibling = primary.md, \"index.md\"\n# k0maru:canonical-sibling = overview.md\n",
        )
        .unwrap();

        fs::create_dir_all(root.join("bundle/nested")).unwrap();
        fs::write(root.join("bundle/primary.md"), "# Primary\n").unwrap();
        fs::write(root.join("bundle/variant_a.md"), "# Variant A\n").unwrap();
        fs::write(root.join("bundle/nested/child.md"), "# Child\n").unwrap();

        fs::create_dir_all(root.join("plain")).unwrap();
        fs::write(root.join("plain/note.md"), "# Note\n").unwrap();

        let filter = ScanFilter::from_vault_root(root);
        assert_eq!(
            filter.canonical_siblings(),
            &["primary.md", "index.md", "overview.md"]
        );

        // In bundle/, primary.md is kept while variant_a.md in the same directory is excluded.
        assert!(!filter.is_excluded(Path::new("bundle/primary.md"), false));
        assert!(filter.is_excluded(Path::new("bundle/variant_a.md"), false));
        // Nested subdirectories are not affected unless they also contain a canonical sibling.
        assert!(!filter.is_excluded(Path::new("bundle/nested"), true));
        assert!(!filter.is_excluded(Path::new("bundle/nested/child.md"), false));
        // Directories without a canonical sibling keep all files.
        assert!(!filter.is_excluded(Path::new("plain/note.md"), false));
    }
}
