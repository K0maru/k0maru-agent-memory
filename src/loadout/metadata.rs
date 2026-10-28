//! Project metadata normalization and alias matching engine.

use std::collections::HashSet;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::core::models::Document;

/// Extracted project metadata with multi-dimensional aliases and tags.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectMetadata {
    pub path: PathBuf,
    pub stem: String,
    pub title: String,
    pub aliases: Vec<String>,
    pub tags: Vec<String>,
    pub derived_aliases: Vec<String>,
    pub slug: Option<String>,
}

/// Normalizes an identifier string by lowercasing, replacing hyphens/underscores with spaces,
/// and condensing whitespace.
pub fn normalize_identifier(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut last_was_space = true;

    for c in s.chars() {
        if c.is_alphanumeric() {
            for lc in c.to_lowercase() {
                result.push(lc);
            }
            last_was_space = false;
        } else if (c == '-' || c == '_' || c.is_whitespace() || c == '/' || c == ':' || c == '.')
            && !last_was_space
        {
            result.push(' ');
            last_was_space = true;
        }
    }

    if result.ends_with(' ') {
        result.pop();
    }
    result
}

/// Extracts derived slug aliases from a backlink source note stem or file name.
///
/// Strips date prefixes `YYYY-MM-DD-` and trailing suffixes like `-phase-*`, `-audit`, `-live-*`.
pub fn extract_derived_aliases(source_stem: &str) -> Vec<String> {
    let mut aliases = Vec::new();
    let trimmed = source_stem.trim().trim_end_matches(".md");

    // Check for date prefix: YYYY-MM-DD-
    let without_date = if trimmed.len() >= 11 {
        let bytes = trimmed.as_bytes();
        let is_date = bytes[0..4].iter().all(u8::is_ascii_digit)
            && bytes[4] == b'-'
            && bytes[5..7].iter().all(u8::is_ascii_digit)
            && bytes[7] == b'-'
            && bytes[8..10].iter().all(u8::is_ascii_digit)
            && bytes[10] == b'-';
        if is_date {
            &trimmed[11..]
        } else {
            trimmed
        }
    } else {
        trimmed
    };

    if without_date.is_empty() {
        return aliases;
    }

    // Add full stripped slug
    aliases.push(without_date.to_string());

    // Suffix markers to strip
    let suffix_markers = [
        "-phase-", "-phase", "-audit", "-live-", "-live", "-review", "-debug", "-session", "-spec",
        "-tickets",
    ];

    for marker in suffix_markers {
        if let Some(idx) = without_date.find(marker) {
            let base = without_date[..idx].trim_matches('-');
            if !base.is_empty() && !aliases.iter().any(|a| a == base) {
                aliases.push(base.to_string());
            }
        }
    }

    aliases
}

/// Extracts `ProjectMetadata` from a document and any pre-harvested derived aliases.
pub fn extract_project_metadata(doc: &Document, derived_aliases: &[String]) -> ProjectMetadata {
    let stem = doc
        .path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&doc.title)
        .to_string();

    let mut aliases: Vec<String> = Vec::new();
    let mut slug: Option<String> = None;

    // 1. Extract from frontmatter
    if let Some(val) = doc.frontmatter.get("slug").and_then(|v| v.as_str()) {
        let s = val.trim().to_string();
        if !s.is_empty() {
            slug = Some(s.clone());
            if !aliases.contains(&s) {
                aliases.push(s);
            }
        }
    }

    if let Some(val) = doc.frontmatter.get("repo").and_then(|v| v.as_str()) {
        let s = val.trim();
        // If "owner/repo", harvest repo name as well
        let repo_name = s.split('/').next_back().unwrap_or(s).to_string();
        if !repo_name.is_empty() && !aliases.contains(&repo_name) {
            aliases.push(repo_name);
        }
    }

    for key in &["aliases", "alias"] {
        if let Some(val) = doc.frontmatter.get(*key) {
            if let Some(arr) = val.as_array() {
                for item in arr {
                    if let Some(s) = item.as_str() {
                        let trimmed = s.trim().to_string();
                        if !trimmed.is_empty() && !aliases.contains(&trimmed) {
                            aliases.push(trimmed);
                        }
                    }
                }
            } else if let Some(s) = val.as_str() {
                for part in s.split(',') {
                    let trimmed = part.trim().to_string();
                    if !trimmed.is_empty() && !aliases.contains(&trimmed) {
                        aliases.push(trimmed);
                    }
                }
            }
        }
    }

    // 2. Extract and sanitize tags
    let mut tags = Vec::new();
    for raw_tag in &doc.tags {
        let clean = raw_tag.trim().trim_start_matches('#');
        let stripped = clean
            .strip_prefix("topic/")
            .or_else(|| clean.strip_prefix("project/"))
            .or_else(|| clean.strip_prefix("type/"))
            .unwrap_or(clean);

        let s = stripped.to_string();
        if !s.is_empty() && !tags.contains(&s) {
            tags.push(s);
        }
    }

    // Also check frontmatter tags
    if let Some(fm_tags) = doc.frontmatter.get("tags") {
        if let Some(arr) = fm_tags.as_array() {
            for item in arr {
                if let Some(s) = item.as_str() {
                    let clean = s.trim().trim_start_matches('#');
                    let stripped = clean
                        .strip_prefix("topic/")
                        .or_else(|| clean.strip_prefix("project/"))
                        .or_else(|| clean.strip_prefix("type/"))
                        .unwrap_or(clean);
                    let st = stripped.to_string();
                    if !st.is_empty() && !tags.contains(&st) {
                        tags.push(st);
                    }
                }
            }
        }
    }

    ProjectMetadata {
        path: doc.path.clone(),
        stem,
        title: doc.title.clone(),
        aliases,
        tags,
        derived_aliases: derived_aliases.to_vec(),
        slug,
    }
}

/// Checks whether a query string matches the project metadata across any dimension:
/// stem, title, slug, aliases, derived aliases, tags, or cross-field token subset.
pub fn is_project_match(query: &str, meta: &ProjectMetadata) -> bool {
    let q = query.trim();
    if q.is_empty() {
        return false;
    }

    let q_norm = normalize_identifier(q);
    let q_lower = q.to_lowercase();
    let q_tokens: Vec<&str> = q_norm.split_whitespace().collect();
    if q_tokens.is_empty() {
        return false;
    }

    // Identifiers: stem, title, slug, aliases, derived aliases
    let mut idents: Vec<&str> = Vec::new();
    idents.push(&meta.stem);
    idents.push(&meta.title);
    if let Some(ref s) = meta.slug {
        idents.push(s);
    }
    for a in &meta.aliases {
        idents.push(a);
    }
    for da in &meta.derived_aliases {
        idents.push(da);
    }

    for cand in &idents {
        let cand_lower = cand.to_lowercase();
        // 1. Exact match
        if cand.eq_ignore_ascii_case(q) {
            return true;
        }

        let cand_norm = normalize_identifier(cand);
        // 2. Normalized exact match
        if cand_norm == q_norm {
            return true;
        }

        // 3. Slug equality (hyphen/underscore insensitive)
        if cand
            .replace(['_', ' '], "-")
            .eq_ignore_ascii_case(&q.replace(['_', ' '], "-"))
        {
            return true;
        }

        // 4. Prefix match
        if cand_lower.starts_with(&q_lower) || cand_norm.starts_with(&q_norm) {
            return true;
        }
        if cand_norm.len() >= 3 && q_norm.starts_with(&cand_norm) {
            return true;
        }

        // 5. Substring match
        if cand_lower.contains(&q_lower) || cand_norm.contains(&q_norm) {
            return true;
        }

        // 6. Token subset match
        let cand_tokens: Vec<&str> = cand_norm.split_whitespace().collect();
        if !cand_tokens.is_empty() {
            if q_tokens.iter().all(|tok| cand_tokens.contains(tok)) {
                return true;
            }
            if cand_tokens.iter().all(|tok| q_tokens.contains(tok)) && cand_tokens.len() >= 2 {
                return true;
            }
        }
    }

    // Tags matching:
    // Only match tags if query matches tag exactly, prefix, or query tokens match tag tokens
    for tag in &meta.tags {
        if tag.eq_ignore_ascii_case("project") || tag.eq_ignore_ascii_case("active") {
            continue; // Skip generic classification tags
        }
        let tag_norm = normalize_identifier(tag);
        if tag_norm == q_norm || tag.eq_ignore_ascii_case(q) {
            return true;
        }
        if tag_norm.starts_with(&q_norm) {
            return true;
        }
        let tag_tokens: Vec<&str> = tag_norm.split_whitespace().collect();
        if !tag_tokens.is_empty() && q_tokens.iter().all(|tok| tag_tokens.contains(tok)) {
            return true;
        }
    }

    // Multi-field cross-token coverage:
    // Every query token must be covered somewhere in the candidate's normalized tokens or substrings,
    // EXCLUDING generic tags like "project"
    let mut all_cand_tokens: HashSet<String> = HashSet::new();
    let mut all_cand_normalized: Vec<String> = Vec::new();
    for cand in &idents {
        let norm = normalize_identifier(cand);
        for tok in norm.split_whitespace() {
            all_cand_tokens.insert(tok.to_string());
        }
        all_cand_normalized.push(norm);
    }
    for tag in &meta.tags {
        if !tag.eq_ignore_ascii_case("project") && !tag.eq_ignore_ascii_case("active") {
            let norm = normalize_identifier(tag);
            for tok in norm.split_whitespace() {
                all_cand_tokens.insert(tok.to_string());
            }
            all_cand_normalized.push(norm);
        }
    }

    let all_covered = q_tokens.iter().all(|&q_tok| {
        if all_cand_tokens.contains(q_tok) {
            return true;
        }
        all_cand_normalized.iter().any(|norm| norm.contains(q_tok))
    });

    if all_covered && !all_cand_tokens.is_empty() {
        return true;
    }

    false
}
