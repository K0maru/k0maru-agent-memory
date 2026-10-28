//! Context loadout builder engine.
//!
//! Extracts project vision, linked L3 evergreen rules, and recent L2 logs
//! into a compact, high-density, sub-300-token prompt package.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::core::models::HierarchyLevel;
use crate::core::traits::{CacheStorage, VaultAdapter};
use crate::git::ModifiedFile;
use crate::loadout::metadata::{
    extract_derived_aliases, extract_project_metadata, is_project_match, normalize_identifier,
    ProjectMetadata,
};
use crate::vector::EmbeddingEngine;

/// Summary of an associated L3 evergreen card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct L3CardSummary {
    pub name: String,
    pub path: String,
    pub concept: Option<String>,
}

/// Summary of an associated L2 AI log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct L2LogSummary {
    pub name: String,
    pub path: String,
}

/// Summary of a project discovered in the vault.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub name: String,
    pub status: String,
    pub path: String,
}

/// Complete compiled loadout context pack for a project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadoutResult {
    pub project_name: String,
    pub project_path: String,
    pub scope: String,
    pub l3_cards: Vec<L3CardSummary>,
    pub l2_logs: Vec<L2LogSummary>,
    pub estimated_tokens: usize,
    pub markdown: String,
}

/// Estimates token count for mixed English and CJK text.
pub fn estimate_tokens(text: &str) -> usize {
    let mut cjk_count: usize = 0;
    let mut non_cjk_chars: usize = 0;
    for c in text.chars() {
        if ('\u{4e00}'..='\u{9fff}').contains(&c)
            || ('\u{3400}'..='\u{4dbf}').contains(&c)
            || ('\u{f900}'..='\u{faff}').contains(&c)
        {
            cjk_count += 1;
        } else if !c.is_whitespace() {
            non_cjk_chars += 1;
        }
    }
    non_cjk_chars.div_ceil(4) + (cjk_count * 2).div_ceil(3)
}

/// Copies formatted string into system clipboard via `arboard`.
pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    match arboard::Clipboard::new() {
        Ok(mut clipboard) => clipboard
            .set_text(text)
            .map_err(|e| format!("Failed to set clipboard text: {}", e)),
        Err(e) => Err(format!("Clipboard unavailable: {}", e)),
    }
}

/// Extracts project scope/vision statement from markdown body or blockquotes.
pub fn extract_project_scope(body: &str) -> String {
    let mut lines = body.lines().peekable();

    // 1. Check for blockquote `>` lines
    while let Some(&line) = lines.peek() {
        let trimmed = line.trim();
        if trimmed.starts_with('>') {
            let mut quote_lines = Vec::new();
            while let Some(&q_line) = lines.peek() {
                let q_trimmed = q_line.trim();
                if q_trimmed.starts_with('>') {
                    let text = q_trimmed
                        .trim_start_matches('>')
                        .trim()
                        .trim_start_matches('#')
                        .trim();
                    if !text.starts_with("[!") && !text.is_empty() {
                        quote_lines.push(text);
                    }
                    lines.next();
                } else {
                    break;
                }
            }
            if !quote_lines.is_empty() {
                return quote_lines.join(" ");
            }
        } else {
            lines.next();
        }
    }

    // 2. Fallback: first non-empty paragraph that isn't a heading
    let mut para_lines = Vec::new();
    let mut started = false;
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if started {
                break;
            }
            continue;
        }
        if trimmed.starts_with('#') {
            if started {
                break;
            }
            continue;
        }
        started = true;
        let text = trimmed.trim_start_matches('#').trim();
        if !text.is_empty() {
            para_lines.push(text);
        }
    }

    if !para_lines.is_empty() {
        return para_lines.join(" ");
    }

    "No project summary available.".to_string()
}

/// Extracts 1-sentence core concept from an L3 card.
pub fn extract_card_concept(body: &str) -> Option<String> {
    let lines: Vec<&str> = body.lines().collect();

    // 1. Look for a heading declaring the core concept. Both English
    //    (`Core Concept`, `Summary`) and Chinese (`核心概念`, `核心结论`)
    //    headings are accepted so existing vault fixtures keep working.
    for (i, &line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            let lower = trimmed.to_lowercase();
            if lower.contains("core concept")
                || lower.contains("summary")
                || lower.contains("核心概念")
                || lower.contains("核心结论")
            {
                let mut section_lines = Vec::new();
                for &sub_line in lines[i + 1..].iter() {
                    let sub_trimmed = sub_line.trim();
                    if sub_trimmed.starts_with('#') {
                        break;
                    }
                    section_lines.push(sub_trimmed);
                }

                // Check for blockquote under section
                let mut quote_lines = Vec::new();
                for &s in &section_lines {
                    if s.starts_with('>') {
                        let text = s.trim_start_matches('>').trim();
                        if !text.starts_with("[!") && !text.is_empty() {
                            quote_lines.push(text);
                        }
                    } else if !quote_lines.is_empty() {
                        break;
                    }
                }
                if !quote_lines.is_empty() {
                    let joined = quote_lines.join(" ");
                    return Some(clean_and_truncate_concept(&joined));
                }

                // Fallback: first non-empty text line under section
                let text_lines: Vec<&str> = section_lines
                    .into_iter()
                    .filter(|s| !s.is_empty())
                    .collect();
                if !text_lines.is_empty() {
                    let joined = text_lines.join(" ");
                    return Some(clean_and_truncate_concept(&joined));
                }
            }
        }
    }

    // 2. Look for the first blockquote in the document
    let mut doc_quote_lines = Vec::new();
    let mut in_quote = false;
    for &line in &lines {
        let trimmed = line.trim();
        if trimmed.starts_with('>') {
            in_quote = true;
            let text = trimmed.trim_start_matches('>').trim();
            if !text.starts_with("[!") && !text.is_empty() {
                doc_quote_lines.push(text);
            }
        } else if in_quote {
            break;
        }
    }
    if !doc_quote_lines.is_empty() {
        let joined = doc_quote_lines.join(" ");
        return Some(clean_and_truncate_concept(&joined));
    }

    // 3. Fallback: first non-empty paragraph after first heading
    let mut para_lines = Vec::new();
    let mut seen_heading = false;
    for &line in &lines {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            if seen_heading && !para_lines.is_empty() {
                break;
            }
            seen_heading = true;
            continue;
        }
        if seen_heading {
            if trimmed.is_empty() {
                if !para_lines.is_empty() {
                    break;
                }
            } else {
                para_lines.push(trimmed);
            }
        }
    }
    if !para_lines.is_empty() {
        let joined = para_lines.join(" ");
        return Some(clean_and_truncate_concept(&joined));
    }

    None
}

fn clean_and_truncate_concept(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() > 100 {
        let truncated: String = trimmed.chars().take(90).collect();
        format!("{}...", truncated.trim_end())
    } else {
        trimmed.to_string()
    }
}

fn is_subsequence(needle: &str, haystack: &str) -> bool {
    let mut needle_chars = needle.chars();
    let mut current_needle = needle_chars.next();
    for h in haystack.chars() {
        if let Some(n) = current_needle {
            if n.eq_ignore_ascii_case(&h) {
                current_needle = needle_chars.next();
            }
        } else {
            return true;
        }
    }
    current_needle.is_none()
}

/// Folder segments that identify project notes across Johnny.Decimal, PARA,
/// and flat LLM-Wiki topologies.
const PROJECT_DIR_SEGMENTS: &[&str] = &["10_projects", "projects", "1_projects"];

/// Folder segments that identify log/journal notes.
const LOG_DIR_SEGMENTS: &[&str] = &[
    "01_ai_logs",
    "30_logs",
    "logs",
    "ai_logs",
    "journal",
    "records",
    "postmortems",
];

/// Folder segments that identify ephemeral notes.
const DAILY_DIR_SEGMENTS: &[&str] = &["00_daily", "daily", "dailies", "inbox", "fleeting"];

/// Returns true when any path segment matches one of the candidates
/// (case-insensitive).
fn path_has_segment(path: &str, candidates: &[&str]) -> bool {
    path.split('/')
        .any(|segment| candidates.contains(&segment.to_lowercase().as_str()))
}

/// Returns true when the path lives in a project, log, or daily folder.
fn is_non_knowledge_path(path: &str) -> bool {
    path_has_segment(path, PROJECT_DIR_SEGMENTS)
        || path_has_segment(path, LOG_DIR_SEGMENTS)
        || path_has_segment(path, DAILY_DIR_SEGMENTS)
}

/// Strips a leading folder prefix (e.g. `20_Cards/`, `cards/`, `logs/`)
/// from a wikilink target, preserving the remainder verbatim.
fn strip_folder_prefix<'a>(target: &'a str, prefixes: &[&str]) -> &'a str {
    let lower = target.to_lowercase();
    for prefix in prefixes {
        let prefix_lower = prefix.to_lowercase();
        if lower.starts_with(&prefix_lower) {
            return &target[prefix.len()..];
        }
    }
    target
}

/// Extracts a card concept and folds in the Living ADR guard rules so that
/// architectural invariants surface inside the compact sub-300-token loadout.
fn extract_guard_aware_concept(doc: &crate::core::models::Document) -> Option<String> {
    let mut concept = extract_card_concept(&doc.body);
    let rules = k0maru_core::guard::extractor::extract_rules_from_document(doc);

    if !rules.is_empty() {
        let mut guards: Vec<String> = Vec::new();
        for rule in rules.iter().take(3) {
            let cond_str = match &rule.condition {
                k0maru_core::guard::models::RuleCondition::Forbidden(pattern) => {
                    format!("forbid `{}`", pattern)
                }
                k0maru_core::guard::models::RuleCondition::MustContain(pattern) => {
                    format!("require `{}`", pattern)
                }
            };

            let mut guard = format!("[Guard: {}", cond_str);
            if !rule.remediation.is_empty() {
                guard.push_str(&format!(" -> {}", rule.remediation));
            }
            guard.push(']');
            guards.push(guard);
        }

        let guard_block = guards.join("; ");
        concept = Some(match concept {
            Some(existing) => format!("{} {}", existing, guard_block),
            None => guard_block,
        });
    }

    concept
}

/// Sub-300-token context loadout builder.
pub struct LoadoutBuilder<'a, A: VaultAdapter, S: CacheStorage> {
    pub(crate) adapter: &'a A,
    pub(crate) storage: &'a S,
    pub(crate) embedder: Option<Arc<dyn EmbeddingEngine>>,
}

impl<'a, A: VaultAdapter, S: CacheStorage> LoadoutBuilder<'a, A, S> {
    /// Creates a new `LoadoutBuilder` with references to vault adapter and cache storage.
    pub fn new(adapter: &'a A, storage: &'a S) -> Self {
        Self {
            adapter,
            storage,
            embedder: None,
        }
    }

    /// Attaches an embedding engine for semantic vector fallback resolution (Tier 3).
    pub fn with_embedder(mut self, embedder: Arc<dyn EmbeddingEngine>) -> Self {
        self.embedder = Some(embedder);
        self
    }

    /// Detects whether this vault follows the Johnny.Decimal topology (`00_Daily/`).
    ///
    /// Generic open-source Git vaults have no `00_Daily/` folder, so loadouts must
    /// emit protocol-agnostic English governance rules instead of the hardcoded
    /// Johnny.Decimal protocol.
    fn has_daily_notes_dir(&self) -> bool {
        if self.adapter.read_document(Path::new("00_Daily")).is_ok() {
            return true;
        }

        self.adapter
            .scan_files()
            .map(|files| {
                files.iter().any(|path| {
                    path.to_string_lossy()
                        .replace('\\', "/")
                        .starts_with("00_Daily")
                })
            })
            .unwrap_or(false)
    }

    /// Discovers candidate project file paths in the vault.
    fn discover_project_paths(&self) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        let all_files = self.adapter.scan_files()?;

        // 1. Check if there are files in any known project folder
        //    (`10_Projects/`, `projects/`, `Projects/`, `1_Projects/`).
        let mut project_files: Vec<PathBuf> = all_files
            .iter()
            .filter(|p| {
                let s = p.to_string_lossy().replace('\\', "/");
                s.split('/').any(|segment| {
                    matches!(
                        segment.to_lowercase().as_str(),
                        "10_projects" | "projects" | "1_projects"
                    )
                })
            })
            .cloned()
            .collect();

        // Filter out index, readme, template
        project_files.retain(|p| {
            if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                let lower = stem.to_lowercase();
                lower != "readme" && lower != "index" && lower != "template"
            } else {
                false
            }
        });

        if !project_files.is_empty() {
            project_files.sort();
            return Ok(project_files);
        }

        // 2. If no 10_Projects/, scan for notes with project tag or frontmatter status
        let mut fallback_projects = Vec::new();
        for path in &all_files {
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            let stem_lower = stem.to_lowercase();
            if stem_lower == "readme" || stem_lower == "index" || stem_lower == "template" {
                continue;
            }

            if let Ok(doc) = self.adapter.read_document(path) {
                let has_project_tag = doc.tags.iter().any(|t| t.to_lowercase() == "project");
                let has_status = doc.frontmatter.get("status").is_some();
                if has_project_tag || has_status {
                    fallback_projects.push(path.clone());
                }
            }
        }

        if fallback_projects.is_empty() {
            for path in &all_files {
                let stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default();
                let stem_lower = stem.to_lowercase();
                if stem_lower != "readme" && stem_lower != "index" && stem_lower != "template" {
                    fallback_projects.push(path.clone());
                }
            }
        }

        fallback_projects.sort();
        Ok(fallback_projects)
    }

    /// Lists all projects discovered in the vault.
    pub fn list_projects(&self) -> Result<Vec<ProjectInfo>, Box<dyn std::error::Error>> {
        let paths = self.discover_project_paths()?;
        let mut projects = Vec::with_capacity(paths.len());

        for path in paths {
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();

            let status = match self.adapter.read_document(&path) {
                Ok(doc) => doc
                    .frontmatter
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("active")
                    .to_string(),
                Err(_) => "unknown".to_string(),
            };

            projects.push(ProjectInfo {
                name: stem,
                status,
                path: path.to_string_lossy().to_string(),
            });
        }

        projects.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(projects)
    }

    /// Finds a matching project by query string using the 4-tier resolution funnel:
    /// - Tier 1: Normalized Exact, Alias, Tag, and Backlink Derived-Alias Match.
    /// - Tier 2: Scoped FTS5 SQL Push-Down.
    /// - Tier 3: Vector Semantic Fallback (via Cosine Similarity if embedder available).
    /// - Tier 4: Ambient Workspace / CWD directory matching.
    pub fn find_project(&self, query: &str) -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
        let trimmed = query.trim();
        let paths = self.discover_project_paths()?;
        if paths.is_empty() {
            return Ok(None);
        }

        // Pre-harvest project metadata for all candidates
        let mut candidates_meta: Vec<(PathBuf, ProjectMetadata)> = Vec::with_capacity(paths.len());
        for path in &paths {
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();

            if let Ok(doc) = self.adapter.read_document(path) {
                // Collect backlinks targeting this note to harvest derived aliases
                let mut derived_aliases = Vec::new();
                if let Ok(backlinks) = self.storage.get_backlinks(&stem) {
                    for bl in backlinks {
                        if let Some(bl_stem) = bl.file_stem().and_then(|s| s.to_str()) {
                            derived_aliases.extend(extract_derived_aliases(bl_stem));
                        }
                    }
                }
                if doc.title != stem {
                    if let Ok(backlinks_title) = self.storage.get_backlinks(&doc.title) {
                        for bl in backlinks_title {
                            if let Some(bl_stem) = bl.file_stem().and_then(|s| s.to_str()) {
                                derived_aliases.extend(extract_derived_aliases(bl_stem));
                            }
                        }
                    }
                }
                derived_aliases.sort();
                derived_aliases.dedup();

                let meta = extract_project_metadata(&doc, &derived_aliases);
                candidates_meta.push((path.clone(), meta));
            }
        }

        if !trimmed.is_empty() {
            // === TIER 1: Normalized Exact, Alias, Tag, and Derived-Alias Match ===
            // Pass 1A: Exact stem, exact title, exact slug, or exact alias
            let q_norm = normalize_identifier(trimmed);
            for (path, meta) in &candidates_meta {
                if meta.stem.eq_ignore_ascii_case(trimmed)
                    || meta.title.eq_ignore_ascii_case(trimmed)
                    || meta
                        .slug
                        .as_deref()
                        .map(|s| s.eq_ignore_ascii_case(trimmed))
                        .unwrap_or(false)
                    || meta.aliases.iter().any(|a| a.eq_ignore_ascii_case(trimmed))
                    || normalize_identifier(&meta.stem) == q_norm
                    || normalize_identifier(&meta.title) == q_norm
                {
                    return Ok(Some(path.clone()));
                }
            }

            // Pass 1B: Prefix, substring, tags, token subset, derived aliases via is_project_match
            for (path, meta) in &candidates_meta {
                if is_project_match(trimmed, meta) {
                    return Ok(Some(path.clone()));
                }
            }

            // Fuzzy subsequence match on stem fallback
            let q_lower = trimmed.to_lowercase();
            for (path, meta) in &candidates_meta {
                let stem_lower = meta.stem.to_lowercase();
                if is_subsequence(&q_lower, &stem_lower) {
                    return Ok(Some(path.clone()));
                }
            }

            // === TIER 2: Scoped FTS5 SQL Push-down ===
            // First try 10_Projects/ scope
            if let Ok(fts_matches) =
                self.storage
                    .search_scoped_fts(trimmed, Some("10_Projects/"), None, 5)
            {
                for m in fts_matches {
                    if paths.contains(&m.path) {
                        return Ok(Some(m.path));
                    }
                    if let Some(matching_path) =
                        paths.iter().find(|p| p.file_stem() == m.path.file_stem())
                    {
                        return Ok(Some(matching_path.clone()));
                    }
                }
            }

            // Also try L1Resource hierarchy scope for generic wiki layouts
            if let Ok(fts_matches) =
                self.storage
                    .search_scoped_fts(trimmed, None, Some("L1Resource"), 5)
            {
                for m in fts_matches {
                    if paths.contains(&m.path) {
                        return Ok(Some(m.path));
                    }
                    if let Some(matching_path) =
                        paths.iter().find(|p| p.file_stem() == m.path.file_stem())
                    {
                        return Ok(Some(matching_path.clone()));
                    }
                }
            }

            // === TIER 3: Vector Semantic Fallback ===
            if let Some(ref embedder) = self.embedder {
                if let Ok(query_emb) = embedder.embed(trimmed) {
                    if let Ok(vector_hits) = self.storage.search_vectors(&query_emb, 10) {
                        for (doc_id, distance) in vector_hits {
                            // Cosine similarity > 0.65 threshold (distance <= 0.35)
                            if distance <= 0.35 {
                                let hit_path = PathBuf::from(&doc_id);
                                if paths.contains(&hit_path) {
                                    return Ok(Some(hit_path));
                                }
                                if let Some(matching_path) =
                                    paths.iter().find(|p| p.file_stem() == hit_path.file_stem())
                                {
                                    return Ok(Some(matching_path.clone()));
                                }
                            }
                        }
                    }
                }
            }
        }

        // === TIER 4: Ambient Workspace / CWD Resolution ===
        // Activates if query is empty or matches the current workspace / CWD directory name
        if trimmed.is_empty() {
            if let Ok(cwd) = std::env::current_dir() {
                if let Some(dir_name) = cwd.file_name().and_then(|s| s.to_str()) {
                    let dir_norm = normalize_identifier(dir_name);
                    for (path, meta) in &candidates_meta {
                        if meta.stem.eq_ignore_ascii_case(dir_name)
                            || meta
                                .slug
                                .as_deref()
                                .map(|s| s.eq_ignore_ascii_case(dir_name))
                                .unwrap_or(false)
                            || is_project_match(dir_name, meta)
                            || normalize_identifier(&meta.stem) == dir_norm
                        {
                            return Ok(Some(path.clone()));
                        }
                    }
                }
            }
        } else if let Ok(cwd) = std::env::current_dir() {
            if let Some(dir_name) = cwd.file_name().and_then(|s| s.to_str()) {
                if dir_name.eq_ignore_ascii_case(trimmed)
                    || normalize_identifier(dir_name) == normalize_identifier(trimmed)
                {
                    for (path, meta) in &candidates_meta {
                        if meta.stem.eq_ignore_ascii_case(dir_name)
                            || meta
                                .slug
                                .as_deref()
                                .map(|s| s.eq_ignore_ascii_case(dir_name))
                                .unwrap_or(false)
                            || is_project_match(dir_name, meta)
                        {
                            return Ok(Some(path.clone()));
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Resolves an L3 card note from a link target name across heterogeneous
    /// vault topologies (`20_Cards/`, `cards/`, `decisions/`, `adr/`,
    /// `concepts/`, `notes/`, `wiki/`, or flat root notes).
    fn resolve_l3_card(&self, raw_target: &str) -> Option<(String, PathBuf, Option<String>)> {
        let mut visited: Vec<String> = Vec::new();
        self.resolve_l3_card_with_depth(raw_target, 0, &mut visited)
    }

    fn resolve_l3_card_with_depth(
        &self,
        raw_target: &str,
        depth: usize,
        visited: &mut Vec<String>,
    ) -> Option<(String, PathBuf, Option<String>)> {
        if depth > 16 {
            return None;
        }
        let mut trimmed_target = raw_target.trim();
        if let Some(s) = trimmed_target.strip_prefix("[[") {
            trimmed_target = s;
        }
        if let Some(s) = trimmed_target.strip_suffix("]]") {
            trimmed_target = s;
        }
        if let Some((before_pipe, _)) = trimmed_target.split_once('|') {
            trimmed_target = before_pipe.trim();
        }
        let normalized = trimmed_target.trim().replace('\\', "/");
        let without_extension = normalized.strip_suffix(".md").unwrap_or(&normalized);
        let stripped = strip_folder_prefix(
            without_extension,
            &[
                "20_Cards/",
                "cards/",
                "decisions/",
                "adr/",
                "concepts/",
                "notes/",
                "wiki/",
            ],
        );
        let clean = stripped.rsplit('/').next().unwrap_or("").trim();

        if clean.is_empty() {
            return None;
        }

        if visited.iter().any(|seen| seen.eq_ignore_ascii_case(clean)) {
            return None;
        }
        visited.push(clean.to_string());

        let candidate_dirs = [
            "20_Cards",
            "cards",
            "decisions",
            "adr",
            "concepts",
            "notes",
            "wiki",
            "",
        ];

        for dir in candidate_dirs {
            let candidate = if dir.is_empty() {
                PathBuf::from(format!("{clean}.md"))
            } else {
                PathBuf::from(format!("{dir}/{clean}.md"))
            };
            let candidate_lower = candidate.to_string_lossy().to_lowercase();
            if is_non_knowledge_path(&candidate_lower) {
                continue;
            }
            if let Ok(doc) = self.adapter.read_document(&candidate) {
                if !doc.is_active() {
                    if let Some(replacement) = doc.superseded_by() {
                        let first_rep =
                            replacement.split(',').next().unwrap_or(&replacement).trim();
                        if !first_rep.is_empty() {
                            if let Some(resolved) =
                                self.resolve_l3_card_with_depth(first_rep, depth + 1, visited)
                            {
                                return Some(resolved);
                            }
                        }
                    }
                    // A broken or over-long supersession chain must never make
                    // the card vanish; surface the superseded note itself.
                    let concept = extract_guard_aware_concept(&doc);
                    return Some((clean.to_string(), doc.path.clone(), concept));
                }
                let doc_path_lower = doc.path.to_string_lossy().replace('\\', "/").to_lowercase();
                if is_non_knowledge_path(&doc_path_lower) {
                    continue;
                }
                // Explicit card folders are accepted as-is; root notes must
                // resolve to an evergreen hierarchy.
                if doc.hierarchy == HierarchyLevel::L3Evergreen || !dir.is_empty() {
                    let concept = extract_guard_aware_concept(&doc);
                    return Some((clean.to_string(), doc.path, concept));
                }
            }
        }

        // Topology-agnostic fallback: scan every note outside project/log/daily
        // folders for a matching file stem.
        if let Ok(files) = self.adapter.scan_files() {
            for relative_path in files {
                let path_lower = relative_path
                    .to_string_lossy()
                    .replace('\\', "/")
                    .to_lowercase();
                if is_non_knowledge_path(&path_lower) {
                    continue;
                }
                let stem_matches = relative_path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .map(|stem| stem.eq_ignore_ascii_case(clean))
                    .unwrap_or(false);
                if !stem_matches {
                    continue;
                }
                if let Ok(doc) = self.adapter.read_document(&relative_path) {
                    if !doc.is_active() {
                        if let Some(replacement) = doc.superseded_by() {
                            let first_rep =
                                replacement.split(',').next().unwrap_or(&replacement).trim();
                            if !first_rep.is_empty() {
                                if let Some(resolved) =
                                    self.resolve_l3_card_with_depth(first_rep, depth + 1, visited)
                                {
                                    return Some(resolved);
                                }
                            }
                        }
                        let concept = extract_guard_aware_concept(&doc);
                        return Some((clean.to_string(), doc.path.clone(), concept));
                    }
                    let concept = extract_guard_aware_concept(&doc);
                    return Some((clean.to_string(), doc.path, concept));
                }
            }
        }

        None
    }

    /// Resolves an L2 log note from a link target name across heterogeneous
    /// vault topologies (`01_AI_Logs/`, `logs/`, `journal/`, `records/`).
    fn resolve_l2_log(&self, raw_target: &str) -> Option<(String, PathBuf)> {
        let normalized = raw_target.trim().replace('\\', "/");
        let without_extension = normalized.strip_suffix(".md").unwrap_or(&normalized);
        let stripped = strip_folder_prefix(
            without_extension,
            &["01_AI_Logs/", "logs/", "journal/", "records/"],
        );
        let clean = stripped.rsplit('/').next().unwrap_or("").trim();

        if clean.is_empty() {
            return None;
        }

        let candidate_dirs = ["01_AI_Logs", "logs", "journal", "records", ""];

        for dir in candidate_dirs {
            let candidate = if dir.is_empty() {
                PathBuf::from(format!("{clean}.md"))
            } else {
                PathBuf::from(format!("{dir}/{clean}.md"))
            };
            if let Ok(doc) = self.adapter.read_document(&candidate) {
                // Explicit log folders are accepted as-is; root notes must
                // resolve to a log hierarchy.
                if doc.hierarchy == HierarchyLevel::L2Log || !dir.is_empty() {
                    return Some((clean.to_string(), doc.path));
                }
            }
        }

        // Topology-agnostic fallback: scan for a matching stem in any log-like
        // folder or a note explicitly declared as L2Log.
        if let Ok(files) = self.adapter.scan_files() {
            for relative_path in files {
                let path_lower = relative_path
                    .to_string_lossy()
                    .replace('\\', "/")
                    .to_lowercase();
                let stem_matches = relative_path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .map(|stem| stem.eq_ignore_ascii_case(clean))
                    .unwrap_or(false);
                if !stem_matches {
                    continue;
                }
                if let Ok(doc) = self.adapter.read_document(&relative_path) {
                    if doc.hierarchy == HierarchyLevel::L2Log
                        || path_has_segment(&path_lower, LOG_DIR_SEGMENTS)
                    {
                        return Some((clean.to_string(), doc.path));
                    }
                }
            }
        }

        None
    }

    /// Builds a sub-300 token context loadout for the specified project query.
    pub fn build(&self, query: &str) -> Result<Option<LoadoutResult>, Box<dyn std::error::Error>> {
        let project_path = match self.find_project(query)? {
            Some(p) => p,
            None => return Ok(None),
        };

        let project_doc = self.adapter.read_document(&project_path)?;
        let project_stem = project_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let scope = extract_project_scope(&project_doc.body);

        // Resolve L3 cards (up to 5 cards)
        let mut l3_cards = Vec::new();
        let mut seen_cards = HashSet::new();

        // Resolve L2 logs (up to 3 logs)
        let mut l2_logs = Vec::new();
        let mut seen_logs = HashSet::new();

        for link in &project_doc.links {
            let target_str = link.target.replace('\\', "/");

            // Check if card
            let is_card_link = target_str.contains("20_Cards/")
                || (!target_str.contains("01_AI_Logs/") && !target_str.contains("00_Daily/"));

            if is_card_link && l3_cards.len() < 5 {
                if let Some((name, path, concept)) = self.resolve_l3_card(&target_str) {
                    if !seen_cards.contains(&name) {
                        seen_cards.insert(name.clone());
                        l3_cards.push(L3CardSummary {
                            name,
                            path: path.to_string_lossy().to_string(),
                            concept,
                        });
                    }
                }
            }

            // Check if log
            let is_log_link = target_str.contains("01_AI_Logs/")
                || (target_str.len() >= 10
                    && target_str.as_bytes()[..4]
                        .iter()
                        .all(|b| b.is_ascii_digit())
                    && target_str.as_bytes()[4] == b'-'
                    && target_str.as_bytes()[7] == b'-');

            if is_log_link && l2_logs.len() < 3 {
                if let Some((name, path)) = self.resolve_l2_log(&target_str) {
                    if !seen_logs.contains(&name) {
                        seen_logs.insert(name.clone());
                        l2_logs.push(L2LogSummary {
                            name,
                            path: path.to_string_lossy().to_string(),
                        });
                    }
                }
            }
        }

        // Also check storage outgoing links if cache has more links
        if l3_cards.len() < 5 || l2_logs.len() < 3 {
            if let Ok(cached_links) = self.storage.get_outgoing_links(&project_path) {
                for link in cached_links {
                    let target_str = link.target.replace('\\', "/");
                    if l3_cards.len() < 5 {
                        if let Some((name, path, concept)) = self.resolve_l3_card(&target_str) {
                            if !seen_cards.contains(&name) {
                                seen_cards.insert(name.clone());
                                l3_cards.push(L3CardSummary {
                                    name,
                                    path: path.to_string_lossy().to_string(),
                                    concept,
                                });
                            }
                        }
                    }
                    if l2_logs.len() < 3 {
                        if let Some((name, path)) = self.resolve_l2_log(&target_str) {
                            if !seen_logs.contains(&name) {
                                seen_logs.insert(name.clone());
                                l2_logs.push(L2LogSummary {
                                    name,
                                    path: path.to_string_lossy().to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }

        // Auto-discover active L3 cards (20_Cards/) and recent L2 logs (01_AI_Logs/)
        let default_vault_root = PathBuf::from(".");
        let vault_root = self.adapter.root_path().unwrap_or(&default_vault_root);
        let projects_dir = vault_root.join("10_Projects");
        let project_md_count = if projects_dir.is_dir() {
            std::fs::read_dir(&projects_dir)
                .map(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
                        .count()
                })
                .unwrap_or(0)
        } else {
            0
        };
        let is_single_project_vault = project_md_count <= 1;

        if l3_cards.len() < 5 {
            let cards_dir = vault_root.join("20_Cards");
            if cards_dir.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&cards_dir) {
                    let mut card_entries: Vec<_> = entries
                        .filter_map(|e| e.ok())
                        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
                        .collect();

                    card_entries.sort_by(|a, b| {
                        let mtime_a = a.metadata().and_then(|m| m.modified()).ok();
                        let mtime_b = b.metadata().and_then(|m| m.modified()).ok();
                        match (mtime_a, mtime_b) {
                            (Some(ta), Some(tb)) if ta != tb => tb.cmp(&ta),
                            _ => b.file_name().cmp(&a.file_name()),
                        }
                    });

                    for entry in card_entries {
                        if l3_cards.len() >= 5 {
                            break;
                        }
                        let rel_path = match entry.path().strip_prefix(vault_root) {
                            Ok(p) => p.to_path_buf(),
                            Err(_) => entry.path(),
                        };
                        let rel_path_str = rel_path.to_string_lossy().to_string();

                        if let Ok(doc) = self.adapter.read_document(&rel_path) {
                            if !doc.is_active() {
                                continue;
                            }

                            let links_to_project = doc.links.iter().any(|l| {
                                let clean = l.target.trim_end_matches(".md");
                                clean == project_stem
                                    || clean.ends_with(&format!("/{}", project_stem))
                            });

                            let frontmatter_matches = doc
                                .frontmatter
                                .get("project")
                                .or_else(|| doc.frontmatter.get("projects"))
                                .is_some_and(|val| {
                                    if let Some(s) = val.as_str() {
                                        let s_clean =
                                            s.trim_start_matches("[[").trim_end_matches("]]");
                                        let s_clean = s_clean.trim_end_matches(".md");
                                        s_clean == project_stem
                                            || s_clean.ends_with(&format!("/{}", project_stem))
                                    } else if let Some(arr) = val.as_array() {
                                        arr.iter().any(|item| {
                                            if let Some(s) = item.as_str() {
                                                let s_clean = s
                                                    .trim_start_matches("[[")
                                                    .trim_end_matches("]]");
                                                let s_clean = s_clean.trim_end_matches(".md");
                                                s_clean == project_stem
                                                    || s_clean
                                                        .ends_with(&format!("/{}", project_stem))
                                            } else {
                                                false
                                            }
                                        })
                                    } else {
                                        false
                                    }
                                });

                            let is_adr_or_guard =
                                !k0maru_core::guard::extractor::extract_rules_from_document(&doc)
                                    .is_empty()
                                    || rel_path_str.to_lowercase().contains("adr")
                                    || rel_path_str.to_lowercase().contains("decision")
                                    || doc
                                        .frontmatter
                                        .get("category")
                                        .and_then(|c| c.as_str())
                                        .is_some_and(|c| c == "decision" || c == "adr");

                            let should_include = links_to_project
                                || frontmatter_matches
                                || (is_single_project_vault && is_adr_or_guard);

                            if should_include {
                                if let Some((name, resolved_path, concept)) =
                                    self.resolve_l3_card(&rel_path_str)
                                {
                                    if !seen_cards.contains(&name) {
                                        seen_cards.insert(name.clone());
                                        l3_cards.push(L3CardSummary {
                                            name,
                                            path: resolved_path.to_string_lossy().to_string(),
                                            concept,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if l2_logs.len() < 3 {
            let logs_dir = vault_root.join("01_AI_Logs");
            if logs_dir.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&logs_dir) {
                    let mut log_entries: Vec<_> = entries
                        .filter_map(|e| e.ok())
                        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
                        .collect();

                    log_entries.sort_by(|a, b| {
                        let mtime_a = a.metadata().and_then(|m| m.modified()).ok();
                        let mtime_b = b.metadata().and_then(|m| m.modified()).ok();
                        match (mtime_a, mtime_b) {
                            (Some(ta), Some(tb)) if ta != tb => tb.cmp(&ta),
                            _ => b.file_name().cmp(&a.file_name()),
                        }
                    });

                    for entry in log_entries {
                        if l2_logs.len() >= 3 {
                            break;
                        }
                        let rel_path = match entry.path().strip_prefix(vault_root) {
                            Ok(p) => p.to_path_buf(),
                            Err(_) => entry.path(),
                        };
                        let rel_path_str = rel_path.to_string_lossy().to_string();

                        let should_include = if is_single_project_vault {
                            true
                        } else if let Ok(doc) = self.adapter.read_document(&rel_path) {
                            doc.links.iter().any(|l| {
                                let clean = l.target.trim_end_matches(".md");
                                clean == project_stem
                                    || clean.ends_with(&format!("/{}", project_stem))
                            }) || doc.body.contains(&project_stem)
                        } else {
                            false
                        };

                        if should_include {
                            if let Some((name, resolved_path)) = self.resolve_l2_log(&rel_path_str)
                            {
                                if !seen_logs.contains(&name) {
                                    seen_logs.insert(name.clone());
                                    l2_logs.push(L2LogSummary {
                                        name,
                                        path: resolved_path.to_string_lossy().to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        // Assemble markdown
        let has_daily_notes = self.has_daily_notes_dir();
        let mut lines = Vec::new();
        lines.push(format!("# 🎒 Agent Task Loadout: {}", project_stem));
        lines.push(format!("> **Project Scope**: {}", scope));
        lines.push(String::new());
        lines.push("## 🧠 Active Knowledge & Constraints (L3)".to_string());
        if l3_cards.is_empty() {
            lines.push("- *(No linked L3 architecture cards yet)*".to_string());
        } else {
            for card in &l3_cards {
                let link_target = if card.path.contains("20_Cards") {
                    format!("20_Cards/{}", card.name)
                } else {
                    card.name.clone()
                };
                if let Some(concept) = &card.concept {
                    lines.push(format!("- **[[{}]]**: {}", link_target, concept));
                } else {
                    lines.push(format!("- **[[{}]]**", link_target));
                }
            }
        }

        lines.push(String::new());
        lines.push("## 📝 Recent Context & Decisions (L2)".to_string());
        if l2_logs.is_empty() {
            lines.push("- *(No recent L2 engineering logs)*".to_string());
        } else {
            for log in &l2_logs {
                let link_target = if log.path.contains("01_AI_Logs") {
                    format!("01_AI_Logs/{}", log.name)
                } else {
                    log.name.clone()
                };
                lines.push(format!("- [[{}]]", link_target));
            }
        }

        lines.push(String::new());
        if has_daily_notes {
            lines.push("## 🛡️ Operating Rules (Obsidian Vault Protocol)".to_string());
            lines.push(
                "1. **WikiLinks First**: Use bare native WikiLinks `[[...]]`; never wrap them in backticks."
                    .to_string(),
            );
            lines.push(
                "2. **Symbolic Offloading**: Write outputs longer than 50 lines to external logs under `01_AI_Logs/`; keep only the Mermaid map in the main context."
                    .to_string(),
            );
            lines.push(
                "3. **Discuss Before Persisting**: Confirm major changes first; never pollute `00_Daily` or the flat `20_Cards/` tree."
                    .to_string(),
            );
        } else {
            lines.push("## 🛡️ Agent Autonomous Protocol (Zero-Human-in-the-Loop)".to_string());
            lines.push(
                "1. Run `k0maru guard check --diff` before committing; use `k0maru flush --supersedes <old-adr>` when replacing decisions."
                    .to_string(),
            );
        }

        let markdown = lines.join("\n");
        let estimated_tokens = estimate_tokens(&markdown);

        Ok(Some(LoadoutResult {
            project_name: project_stem,
            project_path: project_path.to_string_lossy().to_string(),
            scope,
            l3_cards,
            l2_logs,
            estimated_tokens,
            markdown,
        }))
    }

    /// Builds a change-aware dynamic context loadout strictly bounded to <300 tokens,
    /// activating ADRs and invariants relevant to the detected Git changes.
    pub fn build_with_diff(
        &self,
        query: &str,
        modified_files: &[ModifiedFile],
    ) -> Result<Option<LoadoutResult>, Box<dyn std::error::Error>> {
        let project_path = match self.find_project(query)? {
            Some(p) => p,
            None => return Ok(None),
        };

        let project_doc = self.adapter.read_document(&project_path)?;
        let project_stem = project_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let mut scope = extract_project_scope(&project_doc.body);
        if scope.chars().count() > 100 {
            scope = format!("{}...", scope.chars().take(97).collect::<String>());
        }

        let has_daily_notes = self.has_daily_notes_dir();

        // 1. Extract keywords and changed modules from modified files (excluding internal .k0maru/ artifacts)
        let code_modified_files: Vec<&ModifiedFile> = modified_files
            .iter()
            .filter(|f| {
                let norm = f.path.to_string_lossy().replace('\\', "/");
                !norm.starts_with(".k0maru/") && !norm.contains("/.k0maru/")
            })
            .collect();

        let mut all_keywords = Vec::new();
        let mut seen_kw = HashSet::new();
        let mut changed_modules = Vec::new();
        let mut seen_mod = HashSet::new();

        for file in &code_modified_files {
            for kw in file.extract_keywords() {
                if seen_kw.insert(kw.clone()) {
                    all_keywords.push(kw.clone());
                }
                if seen_mod.insert(kw.clone()) && changed_modules.len() < 3 {
                    changed_modules.push(kw);
                }
            }
        }

        // Active changes line
        let active_changes_line = if code_modified_files.is_empty() {
            "> **Active Changes**: Clean working tree (no uncommitted changes)".to_string()
        } else {
            let mods_summary = if changed_modules.is_empty() {
                "various files".to_string()
            } else {
                changed_modules
                    .iter()
                    .map(|m| {
                        let clamped = if m.chars().count() > 30 {
                            format!("{}...", m.chars().take(27).collect::<String>())
                        } else {
                            m.clone()
                        };
                        format!("`{}`", clamped)
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let line = format!(
                "> **Active Changes**: {} modified modules: {} ({} files)",
                changed_modules.len().max(1),
                mods_summary,
                code_modified_files.len()
            );
            if line.chars().count() > 100 {
                let truncated: String = line.chars().take(97).collect();
                format!("{}...", truncated)
            } else {
                line
            }
        };

        // 2. Discover all candidate ADR cards from project links and vault
        let mut candidate_cards: Vec<ScoredCard> = Vec::new();
        let mut seen_card_names = HashSet::new();
        seen_card_names.insert(project_stem.clone());
        seen_card_names.insert(format!("{}.md", project_stem));

        // 2a. Direct project links
        for link in &project_doc.links {
            let target_str = link.target.replace('\\', "/");
            let is_card_link = target_str.contains("20_Cards/")
                || (!target_str.contains("01_AI_Logs/") && !target_str.contains("00_Daily/"));

            if is_card_link {
                if let Some((name, path, concept)) = self.resolve_l3_card(&target_str) {
                    if name != project_stem
                        && path != project_path
                        && !path.to_string_lossy().contains("10_Projects")
                        && seen_card_names.insert(name.clone())
                    {
                        let score = score_card_relevance(
                            &name,
                            &path.to_string_lossy(),
                            concept.as_deref(),
                            &all_keywords,
                            true,
                        );
                        candidate_cards.push(ScoredCard {
                            name,
                            path: path.to_string_lossy().to_string(),
                            concept,
                            score,
                            is_project_linked: true,
                        });
                    }
                }
            }
        }

        // 2b. Outgoing links from storage cache
        if let Ok(cached_links) = self.storage.get_outgoing_links(&project_path) {
            for link in cached_links {
                let target_str = link.target.replace('\\', "/");
                if let Some((name, path, concept)) = self.resolve_l3_card(&target_str) {
                    if name != project_stem
                        && path != project_path
                        && !path.to_string_lossy().contains("10_Projects")
                        && seen_card_names.insert(name.clone())
                    {
                        let score = score_card_relevance(
                            &name,
                            &path.to_string_lossy(),
                            concept.as_deref(),
                            &all_keywords,
                            true,
                        );
                        candidate_cards.push(ScoredCard {
                            name,
                            path: path.to_string_lossy().to_string(),
                            concept,
                            score,
                            is_project_linked: true,
                        });
                    }
                }
            }
        }

        // 2c. Vault-wide scan for cards in 20_Cards, decisions, adr
        if let Ok(all_files) = self.adapter.scan_files() {
            for f in all_files {
                let f_str = f.to_string_lossy();
                if f_str.contains("20_Cards") || f_str.contains("adr") || f_str.contains("decision")
                {
                    if let Some((name, path, concept)) = self.resolve_l3_card(&f_str) {
                        if name != project_stem
                            && path != project_path
                            && !path.to_string_lossy().contains("10_Projects")
                            && seen_card_names.insert(name.clone())
                        {
                            let score = score_card_relevance(
                                &name,
                                &path.to_string_lossy(),
                                concept.as_deref(),
                                &all_keywords,
                                false,
                            );
                            candidate_cards.push(ScoredCard {
                                name,
                                path: path.to_string_lossy().to_string(),
                                concept,
                                score,
                                is_project_linked: false,
                            });
                        }
                    }
                }
            }
        }

        // 2d. FTS search fallback for top keywords
        for kw in all_keywords.iter().take(3) {
            if let Ok(docs) = self.storage.search_fts(kw, 3) {
                for doc in docs {
                    let doc_path_str = doc.path.to_string_lossy();
                    if doc.hierarchy == HierarchyLevel::L3Evergreen
                        || doc_path_str.contains("20_Cards")
                        || doc_path_str.contains("adr")
                        || doc_path_str.contains("decision")
                    {
                        if let Some((name, path, concept)) = self.resolve_l3_card(&doc_path_str) {
                            if name != project_stem
                                && path != project_path
                                && !path.to_string_lossy().contains("10_Projects")
                                && seen_card_names.insert(name.clone())
                            {
                                let score = score_card_relevance(
                                    &name,
                                    &path.to_string_lossy(),
                                    concept.as_deref(),
                                    &all_keywords,
                                    false,
                                );
                                candidate_cards.push(ScoredCard {
                                    name,
                                    path: path.to_string_lossy().to_string(),
                                    concept,
                                    score,
                                    is_project_linked: false,
                                });
                            }
                        }
                    }
                }
            }
        }

        // 3. Sort candidate cards by relevance score descending
        candidate_cards.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.is_project_linked.cmp(&a.is_project_linked))
                .then_with(|| a.name.len().cmp(&b.name.len()))
        });

        // 4. Greedy Packing under Safety Budget <= 280 tokens (Hard Bound < 300)
        let mut l2_logs: Vec<L2LogSummary> = self
            .build(query)
            .ok()
            .flatten()
            .map(|r| r.l2_logs.into_iter().take(2).collect())
            .unwrap_or_default();

        let max_token_budget = 280;
        let mut selected_cards = Vec::new();

        for scored in &candidate_cards {
            if selected_cards.len() >= 4 {
                break;
            }

            // Truncate concept to <= 220 characters so multiple `[Guard: ...]`
            // invariants (e.g. ADR-001 private keys + conflict markers) survive packing.
            let truncated_concept = scored.concept.as_ref().map(|c| {
                let trimmed = c.trim();
                // Never cut off an appended `[Guard: ...]` invariant: trim only the
                // prose prefix before the first `[Guard:` if needed.
                if let Some(guard_idx) = trimmed.find(" [Guard:") {
                    let guard = &trimmed[guard_idx..];
                    let prefix_chars: Vec<char> = trimmed[..guard_idx].chars().collect();
                    if prefix_chars.len() > 60 {
                        let prefix: String = prefix_chars.into_iter().take(60).collect();
                        return format!("{}...{}", prefix.trim_end(), guard);
                    }
                    return trimmed.to_string();
                }

                if trimmed.chars().count() <= 220 {
                    return trimmed.to_string();
                }

                let truncated: String = trimmed.chars().take(217).collect();
                format!("{}...", truncated)
            });

            let candidate_summary = L3CardSummary {
                name: scored.name.clone(),
                path: scored.path.clone(),
                concept: truncated_concept,
            };

            let mut test_cards = selected_cards.clone();
            test_cards.push(candidate_summary.clone());

            let test_md = assemble_diff_loadout_markdown(
                &project_stem,
                &active_changes_line,
                &scope,
                &test_cards,
                &l2_logs,
                has_daily_notes,
            );

            if estimate_tokens(&test_md) <= max_token_budget {
                selected_cards.push(candidate_summary);
            } else if selected_cards.is_empty() {
                // If even the first card exceeds budget, truncate scope further
                let shortened_scope: String = scope.chars().take(60).collect();
                let test_shortened_md = assemble_diff_loadout_markdown(
                    &project_stem,
                    &active_changes_line,
                    &shortened_scope,
                    &test_cards,
                    &l2_logs,
                    has_daily_notes,
                );
                if estimate_tokens(&test_shortened_md) <= max_token_budget {
                    scope = format!("{}...", shortened_scope);
                    selected_cards.push(candidate_summary);
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        // 5. Final Assembly and Hard Guarantee Enforcement
        let mut final_markdown = assemble_diff_loadout_markdown(
            &project_stem,
            &active_changes_line,
            &scope,
            &selected_cards,
            &l2_logs,
            has_daily_notes,
        );
        let mut final_tokens = estimate_tokens(&final_markdown);

        while final_tokens >= 300 && !l2_logs.is_empty() {
            l2_logs.pop();
            final_markdown = assemble_diff_loadout_markdown(
                &project_stem,
                &active_changes_line,
                &scope,
                &selected_cards,
                &l2_logs,
                has_daily_notes,
            );
            final_tokens = estimate_tokens(&final_markdown);
        }

        while final_tokens >= 300 && !selected_cards.is_empty() {
            selected_cards.pop();
            final_markdown = assemble_diff_loadout_markdown(
                &project_stem,
                &active_changes_line,
                &scope,
                &selected_cards,
                &l2_logs,
                has_daily_notes,
            );
            final_tokens = estimate_tokens(&final_markdown);
        }

        if final_tokens >= 300 {
            scope = scope.chars().take(40).collect();
            final_markdown = assemble_diff_loadout_markdown(
                &project_stem,
                &active_changes_line,
                &scope,
                &selected_cards,
                &l2_logs,
                has_daily_notes,
            );
            final_tokens = estimate_tokens(&final_markdown);
        }

        if final_tokens >= 300 {
            scope = scope.chars().take(20).collect();
            final_markdown = assemble_diff_loadout_markdown(
                &project_stem,
                &active_changes_line,
                &scope,
                &selected_cards,
                &l2_logs,
                has_daily_notes,
            );
            final_tokens = estimate_tokens(&final_markdown);
        }

        if final_tokens >= 300 {
            scope = String::new();
            final_markdown = assemble_diff_loadout_markdown(
                &project_stem,
                &active_changes_line,
                &scope,
                &selected_cards,
                &l2_logs,
                has_daily_notes,
            );
            final_tokens = estimate_tokens(&final_markdown);
        }

        // Hard clamp / pop on markdown if token estimate is >= 300
        while final_tokens >= 300 && !final_markdown.is_empty() {
            if let Some(idx) = final_markdown.rfind('\n') {
                final_markdown.truncate(idx);
            } else {
                let count = final_markdown.chars().count();
                if count <= 10 {
                    final_markdown.clear();
                } else {
                    final_markdown = final_markdown.chars().take(count - 10).collect();
                }
            }
            final_tokens = estimate_tokens(&final_markdown);
        }

        Ok(Some(LoadoutResult {
            project_name: project_stem,
            project_path: project_path.to_string_lossy().to_string(),
            scope,
            l3_cards: selected_cards,
            l2_logs,
            estimated_tokens: final_tokens,
            markdown: final_markdown,
        }))
    }
}

#[derive(Debug, Clone)]
struct ScoredCard {
    name: String,
    path: String,
    concept: Option<String>,
    score: f64,
    is_project_linked: bool,
}

fn score_card_relevance(
    card_name: &str,
    card_path: &str,
    card_concept: Option<&str>,
    keywords: &[String],
    is_project_linked: bool,
) -> f64 {
    let mut score = 0.0;
    let name_lower = card_name.to_lowercase();
    let path_lower = card_path.to_lowercase();
    let concept_lower = card_concept.unwrap_or("").to_lowercase();

    for kw in keywords {
        let kw_lower = kw.to_lowercase();
        if kw_lower.len() < 2 {
            continue;
        }

        // Exact name match
        if name_lower == kw_lower {
            score += 20.0;
        } else if name_lower.contains(&kw_lower) {
            score += 10.0;
        }

        // Path match
        if path_lower.contains(&kw_lower) {
            score += 4.0;
        }

        // Concept text match
        if concept_lower.contains(&kw_lower) {
            score += 3.0;
        }
    }

    if is_project_linked {
        score += 1.0;
    }

    score
}

fn assemble_diff_loadout_markdown(
    project_stem: &str,
    active_changes: &str,
    scope: &str,
    cards: &[L3CardSummary],
    l2_logs: &[L2LogSummary],
    has_daily_notes: bool,
) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "# 🎒 Agent Task Loadout: {} [Diff-Aware]",
        project_stem
    ));
    lines.push(active_changes.to_string());
    lines.push(format!("> **Project Scope**: {}", scope));
    lines.push(String::new());
    lines.push("## 🧠 Active Knowledge & Constraints (L3)".to_string());

    if cards.is_empty() {
        lines.push(
            "- *(No module-specific ADR rules matched; follow the repository architecture invariants)*"
                .to_string(),
        );
    } else {
        for card in cards {
            let link_target = if card.path.contains("20_Cards") {
                format!("20_Cards/{}", card.name)
            } else {
                card.name.clone()
            };
            if let Some(concept) = &card.concept {
                lines.push(format!("- **[[{}]]**: {}", link_target, concept));
            } else {
                lines.push(format!("- **[[{}]]**", link_target));
            }
        }
    }

    lines.push(String::new());
    lines.push("## 📝 Recent Context & Decisions (L2)".to_string());
    if l2_logs.is_empty() {
        lines.push("- *(No recent L2 engineering logs)*".to_string());
    } else {
        for log in l2_logs {
            let link_target = if log.path.contains("01_AI_Logs") {
                format!("01_AI_Logs/{}", log.name)
            } else {
                log.name.clone()
            };
            lines.push(format!("- [[{}]]", link_target));
        }
    }

    lines.push(String::new());
    if has_daily_notes {
        lines.push("## 🛡️ Operating Rules (Obsidian Vault Protocol)".to_string());
        lines.push(
            "1. **WikiLinks First**: Use bare native WikiLinks `[[...]]`; never wrap them in backticks."
                .to_string(),
        );
        lines.push(
            "2. **Symbolic Offloading**: Write outputs longer than 50 lines to external logs under `01_AI_Logs/`; keep only the Mermaid map in the main context."
                .to_string(),
        );
        lines.push(
            "3. **Discuss Before Persisting**: Confirm major changes first; never pollute `00_Daily` or the flat `20_Cards/` tree."
                .to_string(),
        );
    } else {
        lines.push("## 🛡️ Agent Autonomous Protocol (Zero-Human-in-the-Loop)".to_string());
        lines.push(
            "1. Run `k0maru guard check --diff` before committing; use `k0maru flush --supersedes <old-adr>` when replacing decisions."
                .to_string(),
        );
    }

    lines.join("\n")
}
