use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

use chrono::NaiveDate;

use super::{NamingStyle, VaultConvention};

pub struct ConventionSniffer;

impl ConventionSniffer {
    /// Sniff the convention of a vault located at `vault_root`.
    /// Strictly stays within `vault_root` with traversal defense.
    pub fn sniff(vault_root: &Path) -> VaultConvention {
        // Resolve canonical absolute path
        let canonical_root = if vault_root.is_absolute() {
            vault_root
                .canonicalize()
                .unwrap_or_else(|_| vault_root.to_path_buf())
        } else {
            let current = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let joined = current.join(vault_root);
            joined.canonicalize().unwrap_or(joined)
        };

        let mut target_subdirs = HashMap::new();
        let mut has_explicit_rules = false;
        let mut rule_source = None;
        let mut template = None;

        // 1. Probe for explicit rules files
        let rule_files = [
            "AGENTS.md",
            "CLAUDE.md",
            "RULES.md",
            "CONVENTIONS.md",
            ".k0maru/rules.md",
        ];

        for candidate in rule_files {
            let rule_path = canonical_root.join(candidate);
            if rule_path.is_file() {
                has_explicit_rules = true;
                rule_source = Some(rule_path.clone());
                if let Ok(content) = fs::read_to_string(&rule_path) {
                    parse_explicit_rules(&content, &mut target_subdirs);
                }
                break;
            }
        }

        // 2. Probe for templates
        let template_dirs = ["templates", "_templates", ".obsidian/templates"];
        for dir in template_dirs {
            let tdir = canonical_root.join(dir);
            if tdir.is_dir() {
                if let Some(t_content) = probe_template_in_dir(&tdir) {
                    template = Some(t_content);
                    break;
                }
            }
        }

        // 3. Statistical Topology: Probe for category directories if not defined by explicit rules
        probe_category_topology(&canonical_root, &mut target_subdirs);

        // 4. Filename naming style & frontmatter heuristics
        let (naming_style, has_frontmatter) =
            probe_naming_and_frontmatter(&canonical_root, &target_subdirs);

        VaultConvention {
            vault_root: canonical_root,
            has_explicit_rules,
            rule_source,
            target_subdirs,
            naming_style,
            has_frontmatter,
            template,
        }
    }
}

/// Convert a title string into a clean lowercase kebab-case slug.
pub fn slugify(title: &str) -> String {
    let mut slug = String::new();
    let mut prev_dash = false;

    for c in title.chars() {
        if c.is_alphanumeric() {
            for lc in c.to_lowercase() {
                slug.push(lc);
            }
            prev_dash = false;
        } else if !prev_dash {
            slug.push('-');
            prev_dash = true;
        }
    }

    let trimmed = slug.trim_matches('-');
    if trimmed.is_empty() {
        "untitled".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Generate a standardized filename based on title, naming style, and optional date.
pub fn generate_filename(title: &str, style: &NamingStyle, date: Option<NaiveDate>) -> String {
    match style {
        NamingStyle::DateSlug => {
            let d = date.unwrap_or_else(|| chrono::Local::now().date_naive());
            let slug = slugify(title);
            format!("{}-{}.md", d.format("%Y-%m-%d"), slug)
        }
        NamingStyle::TimestampSlug => {
            let ts = match date {
                Some(d) => format!("{}0000", d.format("%Y%m%d")),
                None => chrono::Local::now().format("%Y%m%d%H%M").to_string(),
            };
            let slug = slugify(title);
            format!("{}-{}.md", ts, slug)
        }
        NamingStyle::SlugOnly => {
            let slug = slugify(title);
            format!("{}.md", slug)
        }
        NamingStyle::TitleCase => {
            // Strip invalid filesystem characters and sanitize
            let sanitized: String = title
                .chars()
                .filter(|&c| !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
                .collect();
            let trimmed = sanitized.trim();
            if trimmed.is_empty() {
                "Untitled.md".to_string()
            } else {
                format!("{}.md", trimmed)
            }
        }
    }
}

/// Validate that a relative path does not escape root (no `..` components, not absolute).
fn is_safe_subpath(path_str: &str) -> bool {
    let p = Path::new(path_str);
    if p.is_absolute() {
        return false;
    }
    for comp in p.components() {
        if comp == Component::ParentDir {
            return false;
        }
    }
    true
}

/// Parse explicit directory routing rules from markdown/text.
fn parse_explicit_rules(content: &str, target_subdirs: &mut HashMap<String, PathBuf>) {
    for line in content.lines() {
        let trimmed = line.trim();
        // Skip comments or non-rule lines
        let clean = trimmed.trim_start_matches(['-', '*', ' ']);
        if let Some((raw_key, raw_val)) = clean.split_once(':') {
            let key = raw_key.trim().trim_matches(['*', '_', '`']).to_lowercase();
            let val = raw_val
                .trim()
                .trim_matches(['\'', '"', '`'])
                .trim_end_matches('/');

            if val.is_empty() || !is_safe_subpath(val) {
                continue;
            }

            match key.as_str() {
                "decision" | "decisions" | "adr" => {
                    target_subdirs.insert("decision".to_string(), PathBuf::from(val));
                }
                "log" | "logs" | "journal" | "daily" | "records" => {
                    target_subdirs.insert("log".to_string(), PathBuf::from(val));
                }
                "concept" | "concepts" | "wiki" | "card" | "cards" => {
                    target_subdirs.insert("concept".to_string(), PathBuf::from(val));
                }
                _ => {}
            }
        }
    }
}

/// Inspect template directory for matching markdown templates.
fn probe_template_in_dir(tdir: &Path) -> Option<String> {
    // Prioritized template filenames
    let priority_names = [
        "decision.md",
        "adr.md",
        "log.md",
        "journal.md",
        "note.md",
        "default.md",
    ];

    for name in priority_names {
        let p = tdir.join(name);
        if p.is_file() {
            if let Ok(c) = fs::read_to_string(&p) {
                return Some(c);
            }
        }
    }

    // Fallback: read first valid .md file in templates directory
    if let Ok(entries) = fs::read_dir(tdir) {
        let mut md_files = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md") {
                md_files.push(path);
            }
        }
        md_files.sort();
        if let Some(first) = md_files.first() {
            if let Ok(c) = fs::read_to_string(first) {
                return Some(c);
            }
        }
    }

    None
}

/// Statistical topology matching for standard category folders.
fn probe_category_topology(vault_root: &Path, target_subdirs: &mut HashMap<String, PathBuf>) {
    // Decision candidates
    if !target_subdirs.contains_key("decision") {
        let decision_candidates = ["decisions", "adr", "docs/adr", "architecture"];
        let mut found = None;
        for cand in decision_candidates {
            if vault_root.join(cand).is_dir() {
                found = Some(PathBuf::from(cand));
                break;
            }
        }
        target_subdirs.insert(
            "decision".to_string(),
            found.unwrap_or_else(|| PathBuf::from("")),
        );
    }

    // Log candidates
    if !target_subdirs.contains_key("log") {
        let log_candidates = ["logs", "journal", "daily", "records"];
        let mut found = None;
        for cand in log_candidates {
            if vault_root.join(cand).is_dir() {
                found = Some(PathBuf::from(cand));
                break;
            }
        }
        target_subdirs.insert(
            "log".to_string(),
            found.unwrap_or_else(|| PathBuf::from("")),
        );
    }

    // Concept candidates
    if !target_subdirs.contains_key("concept") {
        let concept_candidates = ["concepts", "cards", "wiki", "notes"];
        let mut found = None;
        for cand in concept_candidates {
            if vault_root.join(cand).is_dir() {
                found = Some(PathBuf::from(cand));
                break;
            }
        }
        target_subdirs.insert(
            "concept".to_string(),
            found.unwrap_or_else(|| PathBuf::from("")),
        );
    }

    // Skill candidates
    if !target_subdirs.contains_key("skill") {
        let skill_candidates = [
            "skills",
            "playbooks",
            "recipes",
            "troubleshooting",
            "cards",
            "20_Cards",
        ];
        let mut found = None;
        for cand in skill_candidates {
            if vault_root.join(cand).is_dir() {
                found = Some(PathBuf::from(cand));
                break;
            }
        }
        target_subdirs.insert(
            "skill".to_string(),
            found.unwrap_or_else(|| PathBuf::from("")),
        );
    }

    // Generic defaults to vault root
    target_subdirs
        .entry("generic".to_string())
        .or_insert_with(|| PathBuf::from(""));
}

/// Inspect sampled markdown files to infer naming convention and YAML frontmatter presence.
fn probe_naming_and_frontmatter(
    vault_root: &Path,
    target_subdirs: &HashMap<String, PathBuf>,
) -> (NamingStyle, bool) {
    let mut sampled_files = Vec::new();

    // Check specific subdirectories first, then root
    let mut dirs_to_check = Vec::new();
    for sub in target_subdirs.values() {
        let dir = vault_root.join(sub);
        if dir.is_dir() && !dirs_to_check.contains(&dir) {
            dirs_to_check.push(dir);
        }
    }
    if !dirs_to_check.contains(&vault_root.to_path_buf()) {
        dirs_to_check.push(vault_root.to_path_buf());
    }

    for dir in dirs_to_check {
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("md") {
                    // Ignore hidden files and special markdown files
                    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                        if file_name.starts_with('.')
                            || file_name.eq_ignore_ascii_case("agents.md")
                            || file_name.eq_ignore_ascii_case("claude.md")
                            || file_name.eq_ignore_ascii_case("rules.md")
                            || file_name.eq_ignore_ascii_case("conventions.md")
                        {
                            continue;
                        }
                        sampled_files.push(path);
                        if sampled_files.len() >= 50 {
                            break;
                        }
                    }
                }
            }
        }
        if sampled_files.len() >= 50 {
            break;
        }
    }

    if sampled_files.is_empty() {
        // Default conventions for a fresh or empty vault
        return (NamingStyle::DateSlug, true);
    }

    let mut date_slug_count = 0;
    let mut timestamp_slug_count = 0;
    let mut title_case_count = 0;
    let mut slug_only_count = 0;
    let mut frontmatter_count = 0;

    for file_path in &sampled_files {
        if let Some(stem) = file_path.file_stem().and_then(|s| s.to_str()) {
            if is_iso_date_prefixed(stem) {
                date_slug_count += 1;
            } else if is_timestamp_prefixed(stem) {
                timestamp_slug_count += 1;
            } else if is_title_case(stem) {
                title_case_count += 1;
            } else {
                slug_only_count += 1;
            }
        }

        // Check frontmatter presence
        if let Ok(content) = fs::read_to_string(file_path) {
            let trimmed = content.trim_start();
            if trimmed.starts_with("---") {
                frontmatter_count += 1;
            }
        }
    }

    let dominant_style = if date_slug_count > 0 && date_slug_count >= slug_only_count {
        NamingStyle::DateSlug
    } else if slug_only_count > 0 && slug_only_count >= title_case_count {
        NamingStyle::SlugOnly
    } else if title_case_count > 0 {
        NamingStyle::TitleCase
    } else if timestamp_slug_count > 0 {
        NamingStyle::TimestampSlug
    } else {
        NamingStyle::DateSlug
    };

    // If any sampled notes have frontmatter, default to true
    let has_frontmatter = frontmatter_count > 0 || sampled_files.is_empty();

    (dominant_style, has_frontmatter)
}

/// Check if string starts with ISO date format: `YYYY-MM-DD-` or `YYYY-MM-DD`
fn is_iso_date_prefixed(stem: &str) -> bool {
    let bytes = stem.as_bytes();
    if bytes.len() < 10 {
        return false;
    }
    // Check YYYY-MM-DD
    for (i, &b) in bytes.iter().enumerate().take(10) {
        if i == 4 || i == 7 {
            if b != b'-' {
                return false;
            }
        } else if !b.is_ascii_digit() {
            return false;
        }
    }
    if bytes.len() == 10 {
        return true;
    }
    bytes[10] == b'-' || bytes[10] == b'_' || bytes[10] == b' '
}

/// Check if string starts with numeric timestamp prefix: e.g. `20261004...-`
fn is_timestamp_prefixed(stem: &str) -> bool {
    let bytes = stem.as_bytes();
    if bytes.len() < 9 {
        return false;
    }
    let digits = bytes.iter().take_while(|b| b.is_ascii_digit()).count();
    if (digits == 8 || digits == 12 || digits == 14) && bytes.len() > digits {
        return bytes[digits] == b'-' || bytes[digits] == b'_';
    }
    false
}

/// Check if string looks like Title Case (e.g. contains spaces and capital letters)
fn is_title_case(stem: &str) -> bool {
    if stem.contains(' ') {
        return true;
    }
    // Capitalized first letter and not all-uppercase or kebab-case
    let mut chars = stem.chars();
    if let Some(first) = chars.next() {
        if first.is_uppercase() && stem.chars().any(|c| c.is_lowercase()) && !stem.contains('-') {
            return true;
        }
    }
    false
}
