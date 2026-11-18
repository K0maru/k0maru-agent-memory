//! WikiLink graph target resolution.

use std::collections::HashMap;
use std::path::Path;

/// Outcome of resolving a raw WikiLink target against a vault's indexed documents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    /// Resolved to an indexed document with the given database ID.
    Doc(i64),
    /// Target points to a non-Markdown vault asset (image, PDF, canvas, etc.).
    Asset,
    /// Target refers to the source document itself (empty/fragment-only or same doc ID).
    SelfRef,
    /// Target could not be resolved to any known document or asset.
    Broken,
}

const ASSET_EXTENSIONS: &[&str] = &[
    "png",
    "jpg",
    "jpeg",
    "gif",
    "webp",
    "svg",
    "pdf",
    "canvas",
    "excalidraw",
    "mp4",
    "webm",
    "mp3",
    "wav",
    "zip",
    "csv",
];

#[derive(Debug, Clone)]
struct DocEntry {
    id: i64,
    path_noext: String,
    parent_dir: String,
}

/// Resolves Obsidian-style WikiLink targets to document IDs, assets, self-references, or broken links.
#[derive(Debug, Clone)]
pub struct LinkResolver {
    docs: Vec<DocEntry>,
    by_path_noext: HashMap<String, usize>,
    by_stem: HashMap<String, Vec<usize>>,
    by_title: HashMap<String, Vec<usize>>,
}

impl LinkResolver {
    /// Builds a resolver index from `(doc_id, rel_path, title)` tuples.
    pub fn from_docs<I>(docs: I) -> Self
    where
        I: IntoIterator<Item = (i64, String, String)>,
    {
        let mut entries = Vec::new();
        let mut by_path_noext = HashMap::new();
        let mut by_stem: HashMap<String, Vec<usize>> = HashMap::new();
        let mut by_title: HashMap<String, Vec<usize>> = HashMap::new();

        for (id, rel_path, title) in docs {
            let path_noext = normalize_noext(&rel_path);
            if path_noext.is_empty() {
                continue;
            }
            let parent_dir = parent_of(&path_noext).to_string();
            let stem = stem_of(&path_noext).to_string();
            let title_norm = title.trim().to_lowercase();

            let idx = entries.len();
            entries.push(DocEntry {
                id,
                path_noext: path_noext.clone(),
                parent_dir,
            });

            by_path_noext.insert(path_noext, idx);
            by_stem.entry(stem).or_default().push(idx);
            if !title_norm.is_empty() {
                by_title.entry(title_norm).or_default().push(idx);
            }
        }

        Self {
            docs: entries,
            by_path_noext,
            by_stem,
            by_title,
        }
    }

    /// Resolves `raw_target` as linked from `src_path`.
    pub fn resolve(&self, src_path: &str, raw_target: &str) -> Resolution {
        let trimmed_raw = raw_target.trim();
        if trimmed_raw.is_empty() {
            return Resolution::Broken;
        }

        // 1. Strip `#...` heading and `^...` block suffixes; empty after stripping is a self-link.
        let without_fragment = match trimmed_raw.find(['#', '^']) {
            Some(idx) => trimmed_raw[..idx].trim(),
            None => trimmed_raw,
        };
        if without_fragment.is_empty() {
            return Resolution::SelfRef;
        }

        let target = normalize_noext(without_fragment);
        if target.is_empty() {
            return Resolution::Broken;
        }

        // 2. Non-`.md` asset extensions resolve as `Asset`.
        let last_seg = stem_of(&target);
        if let Some(ext) = Path::new(last_seg).extension().and_then(|e| e.to_str()) {
            if ASSET_EXTENSIONS.contains(&ext) {
                return Resolution::Asset;
            }
        }

        let src_noext = normalize_noext(src_path);
        let src_id = self
            .by_path_noext
            .get(&src_noext)
            .map(|&idx| self.docs[idx].id);
        let src_parent = parent_of(&src_noext);

        let normalized_slash = without_fragment.replace('\\', "/");
        let is_explicit_parent_rel = normalized_slash == ".."
            || normalized_slash.starts_with("../")
            || normalized_slash.contains("/../");
        let is_explicit_cur_rel = normalized_slash == "." || normalized_slash.starts_with("./");

        if is_explicit_parent_rel || is_explicit_cur_rel {
            match resolve_relative_noext(src_parent, &normalized_slash) {
                Some(rel_target) if !rel_target.is_empty() => {
                    if let Some(&idx) = self.by_path_noext.get(&rel_target) {
                        return self.finish(idx, src_id);
                    }
                    if is_explicit_parent_rel {
                        return Resolution::Broken;
                    }
                    // Compatibility: `./<vault-root-path>` in WikiLinks may omit `src_parent`.
                    if let Some(&idx) = self.by_path_noext.get(&target) {
                        return self.finish(idx, src_id);
                    }
                    return Resolution::Broken;
                }
                _ => return Resolution::Broken,
            }
        }

        // 3. Exact normalized path without `.md`.
        if let Some(&idx) = self.by_path_noext.get(&target) {
            return self.finish(idx, src_id);
        }

        // 3b. Subpath relative to source parent directory (e.g. `sub/detail.md` from `guide/intro.md`).
        if !normalized_slash.starts_with('/') && !src_parent.is_empty() && target.contains('/') {
            if let Some(rel_target) = resolve_relative_noext(src_parent, &normalized_slash) {
                if let Some(&idx) = self.by_path_noext.get(&rel_target) {
                    return self.finish(idx, src_id);
                }
            }
        }

        // 4. Relative subpath suffix `/<target>` when target contains `/`.
        if target.contains('/') {
            let suffix = format!("/{target}");
            if let Some(indices) = self.by_stem.get(last_seg) {
                let matches: Vec<usize> = indices
                    .iter()
                    .copied()
                    .filter(|&idx| self.docs[idx].path_noext.ends_with(&suffix))
                    .collect();
                if let Some(best) = self.pick_best(&matches, src_parent) {
                    return self.finish(best, src_id);
                }
            }
        }

        // 5. File stem matching the last segment of target.
        if let Some(indices) = self.by_stem.get(last_seg) {
            if let Some(best) = self.pick_best(indices, src_parent) {
                return self.finish(best, src_id);
            }
        }

        // 6. Document title fallback (compatibility with title-based links).
        if let Some(indices) = self.by_title.get(&target) {
            if let Some(best) = self.pick_best(indices, src_parent) {
                return self.finish(best, src_id);
            }
        }

        Resolution::Broken
    }

    fn finish(&self, idx: usize, src_id: Option<i64>) -> Resolution {
        let doc_id = self.docs[idx].id;
        if Some(doc_id) == src_id {
            Resolution::SelfRef
        } else {
            Resolution::Doc(doc_id)
        }
    }

    // 7. Tie-break: same parent dir as source, then shortest path, then lexicographic.
    fn pick_best(&self, candidates: &[usize], src_parent: &str) -> Option<usize> {
        candidates.iter().copied().min_by(|&a, &b| {
            let da = &self.docs[a];
            let db = &self.docs[b];
            let a_diff_parent = u8::from(da.parent_dir != src_parent);
            let b_diff_parent = u8::from(db.parent_dir != src_parent);
            a_diff_parent
                .cmp(&b_diff_parent)
                .then_with(|| da.path_noext.len().cmp(&db.path_noext.len()))
                .then_with(|| da.path_noext.cmp(&db.path_noext))
                .then_with(|| da.id.cmp(&db.id))
        })
    }
}

fn resolve_relative_noext(src_parent: &str, raw_rel: &str) -> Option<String> {
    let mut parts: Vec<String> = src_parent
        .split('/')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_lowercase())
        .collect();

    let lowered = raw_rel.replace('\\', "/").trim().to_lowercase();
    let no_ext = lowered.strip_suffix(".md").unwrap_or(&lowered).trim();

    for seg in no_ext.split('/') {
        match seg {
            "" | "." => continue,
            ".." => {
                parts.pop()?;
            }
            other => parts.push(other.to_string()),
        }
    }

    Some(parts.join("/"))
}

fn normalize_noext(raw: &str) -> String {
    let mut s = raw.replace('\\', "/").trim().to_lowercase();
    loop {
        if let Some(rest) = s.strip_prefix("./") {
            s = rest.to_string();
        } else if let Some(rest) = s.strip_prefix('/') {
            s = rest.to_string();
        } else {
            break;
        }
    }
    let trimmed = s.trim();
    let no_ext = trimmed.strip_suffix(".md").unwrap_or(trimmed).trim();
    no_ext.to_string()
}

fn parent_of(path_noext: &str) -> &str {
    match path_noext.rsplit_once('/') {
        Some((parent, _)) => parent,
        None => "",
    }
}

fn stem_of(path_noext: &str) -> &str {
    match path_noext.rsplit_once('/') {
        Some((_, stem)) => stem,
        None => path_noext,
    }
}
