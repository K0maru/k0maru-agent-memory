//! Context loadout builder engine.
//!
//! Extracts project vision, linked L3 evergreen rules, and recent L2 logs
//! into a compact, high-density, sub-300-token prompt package.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

use crate::core::models::HierarchyLevel;
use crate::core::traits::{CacheStorage, VaultAdapter};

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
                    let text = q_trimmed.trim_start_matches('>').trim();
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
        para_lines.push(trimmed);
    }

    if !para_lines.is_empty() {
        return para_lines.join(" ");
    }

    "暂无项目简述".to_string()
}

/// Extracts 1-sentence core concept from an L3 card.
pub fn extract_card_concept(body: &str) -> Option<String> {
    let lines: Vec<&str> = body.lines().collect();

    // 1. Look for heading containing `核心概念` or `Core Concept`
    for (i, &line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            let lower = trimmed.to_lowercase();
            if lower.contains("核心概念") || lower.contains("core concept") {
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

/// Sub-300-token context loadout builder.
pub struct LoadoutBuilder<'a, A: VaultAdapter, S: CacheStorage> {
    pub(crate) adapter: &'a A,
    pub(crate) storage: &'a S,
}

impl<'a, A: VaultAdapter, S: CacheStorage> LoadoutBuilder<'a, A, S> {
    /// Creates a new `LoadoutBuilder` with references to vault adapter and cache storage.
    pub fn new(adapter: &'a A, storage: &'a S) -> Self {
        Self { adapter, storage }
    }

    /// Discovers candidate project file paths in the vault.
    fn discover_project_paths(&self) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        let all_files = self.adapter.scan_files()?;

        // 1. Check if there are files in 10_Projects/
        let mut project_files: Vec<PathBuf> = all_files
            .iter()
            .filter(|p| {
                let s = p.to_string_lossy().replace('\\', "/");
                s.contains("10_Projects/")
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

    /// Finds a matching project by query string (exact, prefix, substring, or fuzzy).
    pub fn find_project(&self, query: &str) -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }
        let q_lower = trimmed.to_lowercase();

        let paths = self.discover_project_paths()?;
        if paths.is_empty() {
            return Ok(None);
        }

        // 1. Exact match on stem or document title
        for path in &paths {
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if stem.eq_ignore_ascii_case(trimmed) {
                return Ok(Some(path.clone()));
            }
            if let Ok(doc) = self.adapter.read_document(path) {
                if doc.title.eq_ignore_ascii_case(trimmed) {
                    return Ok(Some(path.clone()));
                }
            }
        }

        // 2. Prefix match on stem or title
        for path in &paths {
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            let stem_lower = stem.to_lowercase();
            if stem_lower.starts_with(&q_lower) {
                return Ok(Some(path.clone()));
            }
            if let Ok(doc) = self.adapter.read_document(path) {
                if doc.title.to_lowercase().starts_with(&q_lower) {
                    return Ok(Some(path.clone()));
                }
            }
        }

        // 3. Substring match on stem or title
        for path in &paths {
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            let stem_lower = stem.to_lowercase();
            if stem_lower.contains(&q_lower) {
                return Ok(Some(path.clone()));
            }
            if let Ok(doc) = self.adapter.read_document(path) {
                if doc.title.to_lowercase().contains(&q_lower) {
                    return Ok(Some(path.clone()));
                }
            }
        }

        // 4. Fuzzy subsequence match
        for path in &paths {
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            let stem_lower = stem.to_lowercase();
            if is_subsequence(&q_lower, &stem_lower) {
                return Ok(Some(path.clone()));
            }
        }

        // 5. FTS search fallback in SQLite cache
        if let Ok(fts_matches) = self.storage.search_fts(trimmed, 5) {
            for m in fts_matches {
                if paths.contains(&m.path) {
                    return Ok(Some(m.path));
                }
            }
        }

        Ok(None)
    }

    /// Resolves an L3 card note from a link target name.
    fn resolve_l3_card(&self, raw_target: &str) -> Option<(String, PathBuf, Option<String>)> {
        let mut clean = raw_target.trim();
        if let Some(stripped) = clean.strip_prefix("20_Cards/") {
            clean = stripped;
        }
        if let Some(stripped) = clean.strip_suffix(".md") {
            clean = stripped;
        }
        clean = clean.trim();

        if clean.is_empty() {
            return None;
        }

        let candidate_paths = [
            PathBuf::from(format!("20_Cards/{}.md", clean)),
            PathBuf::from(format!("{}.md", clean)),
            PathBuf::from(clean),
        ];

        for cand in &candidate_paths {
            if let Ok(doc) = self.adapter.read_document(cand) {
                if doc.hierarchy == HierarchyLevel::L3Evergreen
                    || cand.to_string_lossy().contains("20_Cards")
                {
                    let concept = extract_card_concept(&doc.body);
                    return Some((clean.to_string(), doc.path, concept));
                }
            }
        }

        None
    }

    /// Resolves an L2 log note from a link target name.
    fn resolve_l2_log(&self, raw_target: &str) -> Option<(String, PathBuf)> {
        let mut clean = raw_target.trim();
        if let Some(stripped) = clean.strip_prefix("01_AI_Logs/") {
            clean = stripped;
        }
        if let Some(stripped) = clean.strip_suffix(".md") {
            clean = stripped;
        }
        clean = clean.trim();

        if clean.is_empty() {
            return None;
        }

        let candidate_paths = [
            PathBuf::from(format!("01_AI_Logs/{}.md", clean)),
            PathBuf::from(format!("{}.md", clean)),
            PathBuf::from(clean),
        ];

        for cand in &candidate_paths {
            if let Ok(doc) = self.adapter.read_document(cand) {
                if doc.hierarchy == HierarchyLevel::L2Log
                    || cand.to_string_lossy().contains("01_AI_Logs")
                {
                    return Some((clean.to_string(), doc.path));
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
                || target_str.starts_with("2026-")
                || target_str.contains("eval")
                || target_str.contains("run");

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

        // Assemble markdown
        let mut lines = Vec::new();
        lines.push(format!("# 🎒 Agent Task Loadout: {}", project_stem));
        lines.push(format!("> **Project Scope**: {}", scope));
        lines.push(String::new());
        lines.push("## 🧠 Active Knowledge & Constraints (L3)".to_string());
        if l3_cards.is_empty() {
            lines.push("- *(暂无绑定的原子卡片，请遵循通用架构规约)*".to_string());
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
            lines.push("- *(无历史前置研发日志)*".to_string());
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
        lines.push("## 🛡️ Operating Rules (SecondBrain Protocol)".to_string());
        lines.push(
            "1. **双链优先 (WikiLinks)**: 原生裸双链 `[[...]]`，禁止反引号包裹。".to_string(),
        );
        lines.push(
            "2. **符号化卸载**: 输出 >50 行写外部日志，主上下文仅维护 Mermaid 图谱。".to_string(),
        );
        lines.push(
            "3. **先讨论后落盘**: 重大改动先确认；严禁污染 00_Daily 或 20_Cards。".to_string(),
        );

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
}
