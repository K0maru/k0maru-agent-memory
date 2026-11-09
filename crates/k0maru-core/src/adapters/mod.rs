//! Storage adapters implementing the `VaultAdapter` trait for Obsidian and generic LLM-Wiki.

pub mod generic;
pub mod obsidian;

pub use generic::GenericWikiAdapter;
pub use obsidian::ObsidianAdapter;

use std::path::{Path, PathBuf};

use crate::core::models::{Document, HierarchyLevel};

/// Internal helper to scan Markdown files using `walkdir`, excluding hidden folders and files.
pub(crate) fn scan_markdown_files(root: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut paths = Vec::new();
    let walker = walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            if entry.depth() == 0 {
                return true;
            }
            let file_name = entry.file_name().to_string_lossy();
            if entry.file_type().is_dir()
                && (file_name.starts_with('.')
                    || file_name == "trash"
                    || file_name == "node_modules"
                    || file_name == "target")
            {
                return false;
            }
            true
        });

    for entry in walker {
        let entry = entry?;
        if entry.file_type().is_file() {
            let file_name = entry.file_name().to_string_lossy();
            if file_name.starts_with('.') {
                continue;
            }
            if let Some(ext) = entry.path().extension() {
                if ext.eq_ignore_ascii_case("md") {
                    if let Ok(rel_path) = entry.path().strip_prefix(root) {
                        paths.push(rel_path.to_path_buf());
                    }
                }
            }
        }
    }

    // The primary walk intentionally skips hidden directories, so pack-provided
    // markdown under `.k0maru/packs/` must be indexed explicitly.
    let packs_root = root.join(".k0maru").join("packs");
    if packs_root.is_dir() {
        let pack_walker = walkdir::WalkDir::new(&packs_root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| {
                if entry.depth() == 0 {
                    return true;
                }
                !entry.file_name().to_string_lossy().starts_with('.')
            });

        for entry in pack_walker {
            let entry = entry?;
            if !entry.file_type().is_file() {
                continue;
            }
            if entry.file_name().to_string_lossy().starts_with('.') {
                continue;
            }
            if let Some(ext) = entry.path().extension() {
                if ext.eq_ignore_ascii_case("md") {
                    if let Ok(rel_path) = entry.path().strip_prefix(root) {
                        paths.push(rel_path.to_path_buf());
                    }
                }
            }
        }
    }

    paths.sort();
    Ok(paths)
}

/// Extracts first H1 heading from markdown body using pulldown-cmark AST.
pub(crate) fn extract_first_h1(body: &str) -> Option<String> {
    use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
    let parser = Parser::new(body);
    let mut in_h1 = false;
    let mut h1_text = String::new();

    for event in parser {
        match event {
            Event::Start(Tag::Heading {
                level: HeadingLevel::H1,
                ..
            }) => {
                in_h1 = true;
                h1_text.clear();
            }
            Event::End(TagEnd::Heading(HeadingLevel::H1)) => {
                let trimmed = h1_text.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
                in_h1 = false;
            }
            Event::Text(t) | Event::Code(t) if in_h1 => {
                h1_text.push_str(&t);
            }
            _ => {}
        }
    }

    let trimmed = h1_text.trim();
    if in_h1 && !trimmed.is_empty() {
        Some(trimmed.to_string())
    } else {
        None
    }
}

/// Extracts document title prioritizing the first Markdown H1 heading, then frontmatter title, then file stem.
pub(crate) fn extract_title(body: &str, frontmatter: &serde_json::Value, path: &Path) -> String {
    if let Some(h1) = extract_first_h1(body) {
        return h1;
    }
    if let Some(t) = frontmatter.get("title").and_then(|v| v.as_str()) {
        let trimmed = t.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    if let Some(stem) = path.file_stem() {
        return stem.to_string_lossy().to_string();
    }
    "Untitled".to_string()
}

/// Parses explicit `hierarchy:` or `level:` from frontmatter.
pub(crate) fn parse_hierarchy_override(frontmatter: &serde_json::Value) -> Option<HierarchyLevel> {
    let val = frontmatter
        .get("hierarchy")
        .or_else(|| frontmatter.get("level"))
        .or_else(|| frontmatter.get("Hierarchy"))
        .or_else(|| frontmatter.get("Level"))?;

    if let Some(n) = val.as_i64() {
        return match n {
            0 => Some(HierarchyLevel::L0Ephemeral),
            1 => Some(HierarchyLevel::L1Resource),
            2 => Some(HierarchyLevel::L2Log),
            3 => Some(HierarchyLevel::L3Evergreen),
            _ => None,
        };
    }

    if let Some(s) = val.as_str() {
        let trimmed = s.trim().to_lowercase();
        return match trimmed.as_str() {
            "l0" | "l0ephemeral" | "ephemeral" | "daily" | "0" => Some(HierarchyLevel::L0Ephemeral),
            "l1" | "l1resource" | "resource" | "1" => Some(HierarchyLevel::L1Resource),
            "l2" | "l2log" | "log" | "2" => Some(HierarchyLevel::L2Log),
            "l3" | "l3evergreen" | "evergreen" | "permanent" | "3" => {
                Some(HierarchyLevel::L3Evergreen)
            }
            _ => None,
        };
    }

    None
}

/// Returns true if any standard structured vault or LLM-Wiki directory exists under `vault_path`.
pub fn is_structured_vault(vault_path: &Path) -> bool {
    const CANDIDATES: &[&str] = &[
        ".obsidian",
        "projects",
        "decisions",
        "cards",
        "logs",
        "10_Projects",
        "20_Cards",
        "01_AI_Logs",
    ];
    CANDIDATES.iter().any(|c| vault_path.join(c).is_dir())
}

/// Resolves the LLM-Wiki hierarchy level for a document according to Karpathy-style and
/// structured knowledge topologies.
///
/// Hierarchy determination logic:
/// 1. Check `parse_hierarchy_override(frontmatter)` first.
/// 2. Check frontmatter `type` or `category` (case-insensitive string):
///    - `"decision" | "adr" | "card" | "concept" | "project" | "architecture" | "evergreen"` -> `HierarchyLevel::L3Evergreen`
///    - `"log" | "ai-log" | "session" | "postmortem" | "record"` -> `HierarchyLevel::L2Log`
///    - `"daily" | "ephemeral" | "scratch" | "inbox"` -> `HierarchyLevel::L0Ephemeral`
///    - `"resource" | "reference" | "doc"` -> `HierarchyLevel::L1Resource`
/// 3. Inspect directory components of `relative_path` (all components except the filename, case-insensitive):
///    - `"20_cards" | "2_cards" | "cards" | "decisions" | "concepts" | "architecture" | "adr" | "specs" | "rules" | "projects" | "10_projects" | "1_projects"` -> `HierarchyLevel::L3Evergreen`
///    - `"01_ai_logs" | "30_logs" | "logs" | "ai_logs" | "journal" | "records" | "postmortems"` -> `HierarchyLevel::L2Log`
///    - `"00_daily" | "0_daily" | "daily" | "ephemeral" | "scratch" | "inbox"` -> `HierarchyLevel::L0Ephemeral`
///    - `"30_resources" | "3_resources" | "resources" | "references" | "docs"` -> `HierarchyLevel::L1Resource`
/// 4. If `relative_path.parent()` is `None` or `Some(Path::new(""))` (root-level `.md` file), return `HierarchyLevel::L3Evergreen`.
/// 5. Otherwise return `fallback`.
pub fn determine_llm_wiki_hierarchy(
    frontmatter: &serde_json::Value,
    relative_path: &Path,
    fallback: HierarchyLevel,
) -> HierarchyLevel {
    // 1. Explicit hierarchy override
    if let Some(level) = parse_hierarchy_override(frontmatter) {
        return level;
    }

    // 2. Frontmatter type or category
    let type_or_cat = frontmatter
        .get("type")
        .or_else(|| frontmatter.get("category"))
        .and_then(|v| v.as_str());

    if let Some(kind) = type_or_cat {
        let lower = kind.trim().to_lowercase();
        match lower.as_str() {
            "decision" | "adr" | "card" | "concept" | "project" | "architecture" | "evergreen" => {
                return HierarchyLevel::L3Evergreen;
            }
            "log" | "ai-log" | "session" | "postmortem" | "record" => {
                return HierarchyLevel::L2Log;
            }
            "daily" | "ephemeral" | "scratch" | "inbox" => {
                return HierarchyLevel::L0Ephemeral;
            }
            "resource" | "reference" | "doc" => {
                return HierarchyLevel::L1Resource;
            }
            _ => {}
        }
    }

    // 3. Inspect directory components of relative_path
    if let Some(parent) = relative_path.parent() {
        if !parent.as_os_str().is_empty() {
            let segments: Vec<String> = parent
                .components()
                .map(|c| c.as_os_str().to_string_lossy().to_lowercase())
                .collect();

            let has_segment = |candidates: &[&str]| {
                segments
                    .iter()
                    .any(|seg| candidates.contains(&seg.as_str()))
            };

            const EVERGREEN_DIRS: &[&str] = &[
                "20_cards",
                "2_cards",
                "cards",
                "decisions",
                "concepts",
                "architecture",
                "adr",
                "specs",
                "rules",
                "projects",
                "10_projects",
                "1_projects",
                "wiki",
                "skills",
                "playbooks",
                "evergreen",
            ];
            const LOG_DIRS: &[&str] = &[
                "01_ai_logs",
                "30_logs",
                "logs",
                "ai_logs",
                "journal",
                "records",
                "postmortems",
            ];
            const EPHEMERAL_DIRS: &[&str] = &[
                "00_daily",
                "0_daily",
                "daily",
                "ephemeral",
                "scratch",
                "inbox",
                "dailies",
                "fleeting",
            ];
            const RESOURCE_DIRS: &[&str] = &[
                "30_resources",
                "3_resources",
                "resources",
                "references",
                "docs",
            ];

            if has_segment(EVERGREEN_DIRS) {
                return HierarchyLevel::L3Evergreen;
            } else if has_segment(LOG_DIRS) {
                return HierarchyLevel::L2Log;
            } else if has_segment(EPHEMERAL_DIRS) {
                return HierarchyLevel::L0Ephemeral;
            } else if has_segment(RESOURCE_DIRS) {
                return HierarchyLevel::L1Resource;
            }
        }
    }

    // 4. Root-level .md file defaults to L3Evergreen
    if relative_path
        .parent()
        .map(|p| p.as_os_str().is_empty())
        .unwrap_or(true)
    {
        return HierarchyLevel::L3Evergreen;
    }

    // 5. Fallback
    fallback
}

/// Extracts and merges tags from frontmatter and body.
pub(crate) fn extract_tags_combined(
    frontmatter: &serde_json::Value,
    body_tags: Vec<String>,
) -> Vec<String> {
    let mut tags = Vec::new();
    if let Some(fm_tags) = frontmatter.get("tags").or_else(|| frontmatter.get("tag")) {
        if let Some(arr) = fm_tags.as_array() {
            for t in arr {
                if let Some(s) = t.as_str() {
                    let tag = s.trim();
                    if !tag.is_empty() && !tags.iter().any(|existing: &String| existing == tag) {
                        tags.push(tag.to_string());
                    }
                }
            }
        } else if let Some(s) = fm_tags.as_str() {
            if s.contains(',') {
                for part in s.split(',') {
                    let tag = part.trim();
                    if !tag.is_empty() && !tags.iter().any(|existing: &String| existing == tag) {
                        tags.push(tag.to_string());
                    }
                }
            } else {
                let tag = s.trim();
                if !tag.is_empty() && !tags.iter().any(|existing: &String| existing == tag) {
                    tags.push(tag.to_string());
                }
            }
        }
    }
    for bt in body_tags {
        if !tags.contains(&bt) {
            tags.push(bt);
        }
    }
    tags
}

/// Sanitizes title string into a safe file name across major OS filesystems.
pub(crate) fn sanitize_filename(title: &str) -> String {
    let clean: String = title
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0' => '_',
            _ => c,
        })
        .collect();
    let clean_trimmed = clean.trim().trim_matches('.');
    if clean_trimmed.is_empty() {
        "untitled".to_string()
    } else {
        clean_trimmed.to_string()
    }
}

/// Resolves a non-colliding file path in `dir` for a card with `title`.
pub(crate) fn resolve_unique_path(dir: &Path, title: &str) -> (PathBuf, String) {
    let sanitized = sanitize_filename(title);
    let base = if let Some(stripped) = sanitized.strip_suffix(".md") {
        stripped
    } else {
        &sanitized
    };

    let filename = format!("{}.md", base);
    let full_path = dir.join(&filename);
    if !full_path.exists() {
        return (full_path, filename);
    }

    let mut counter = 1;
    loop {
        let candidate_filename = format!("{}_{}.md", base, counter);
        let candidate_path = dir.join(&candidate_filename);
        if !candidate_path.exists() {
            return (candidate_path, candidate_filename);
        }
        counter += 1;
    }
}

/// Shared document reading and parsing implementation parameterized by a hierarchy resolver.
pub(crate) fn read_document_impl<F>(
    root: &Path,
    relative_path: &Path,
    hierarchy_resolver: F,
) -> Result<Document, Box<dyn std::error::Error>>
where
    F: FnOnce(&serde_json::Value, &Path) -> HierarchyLevel,
{
    let rel_path = if relative_path.is_absolute() {
        relative_path
            .strip_prefix(root)
            .unwrap_or(relative_path)
            .to_path_buf()
    } else {
        relative_path.to_path_buf()
    };
    let full_path = root.join(&rel_path);

    let content = std::fs::read_to_string(&full_path)?;
    let metadata = std::fs::metadata(&full_path)?;
    let mtime = metadata
        .modified()?
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let hash_num = xxhash_rust::xxh3::xxh3_64(content.as_bytes());
    let content_hash = format!("{:016x}", hash_num);

    let (frontmatter, body) = crate::parser::parse_frontmatter(&content);
    let (links, body_tags) = crate::parser::extract_elements(body);
    let tags = extract_tags_combined(&frontmatter, body_tags);
    let title = extract_title(body, &frontmatter, &rel_path);
    let hierarchy = hierarchy_resolver(&frontmatter, &rel_path);

    Ok(Document {
        path: rel_path,
        title,
        hierarchy,
        frontmatter,
        links,
        tags,
        content_hash,
        mtime,
        body: body.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn scan_markdown_files_includes_pack_markdown() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        fs::write(root.join("Root.md"), "# Root\n").unwrap();
        let pack_dir = root.join(".k0maru").join("packs").join("demo");
        fs::create_dir_all(pack_dir.join("skills")).unwrap();
        fs::write(pack_dir.join("AGENTS.md"), "# Pack\n").unwrap();
        fs::write(pack_dir.join("skills").join("demo.md"), "# Skill\n").unwrap();
        fs::write(pack_dir.join(".hidden.md"), "# Hidden\n").unwrap();

        let paths = scan_markdown_files(root).unwrap();
        let relative: Vec<String> = paths
            .iter()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .collect();

        assert_eq!(
            relative,
            vec![
                ".k0maru/packs/demo/AGENTS.md",
                ".k0maru/packs/demo/skills/demo.md",
                "Root.md",
            ]
        );
    }

    #[test]
    fn test_determine_llm_wiki_hierarchy_behaviors() {
        use serde_json::json;

        // 1. Explicit override
        let fm_override = json!({"hierarchy": "L2Log"});
        assert_eq!(
            determine_llm_wiki_hierarchy(
                &fm_override,
                Path::new("decisions/adr.md"),
                HierarchyLevel::L1Resource
            ),
            HierarchyLevel::L2Log
        );

        // 2. Category / type
        let fm_type = json!({"type": "adr"});
        assert_eq!(
            determine_llm_wiki_hierarchy(
                &fm_type,
                Path::new("other/something.md"),
                HierarchyLevel::L1Resource
            ),
            HierarchyLevel::L3Evergreen
        );
        let fm_cat = json!({"category": "log"});
        assert_eq!(
            determine_llm_wiki_hierarchy(
                &fm_cat,
                Path::new("other/something.md"),
                HierarchyLevel::L1Resource
            ),
            HierarchyLevel::L2Log
        );

        // 3. Directory components
        let fm_empty = json!({});
        assert_eq!(
            determine_llm_wiki_hierarchy(
                &fm_empty,
                Path::new("projects/acme.md"),
                HierarchyLevel::L1Resource
            ),
            HierarchyLevel::L3Evergreen
        );
        assert_eq!(
            determine_llm_wiki_hierarchy(
                &fm_empty,
                Path::new("decisions/adr-001.md"),
                HierarchyLevel::L1Resource
            ),
            HierarchyLevel::L3Evergreen
        );
        assert_eq!(
            determine_llm_wiki_hierarchy(
                &fm_empty,
                Path::new("logs/2026-11-09.md"),
                HierarchyLevel::L1Resource
            ),
            HierarchyLevel::L2Log
        );
        assert_eq!(
            determine_llm_wiki_hierarchy(
                &fm_empty,
                Path::new("daily/notes.md"),
                HierarchyLevel::L1Resource
            ),
            HierarchyLevel::L0Ephemeral
        );
        assert_eq!(
            determine_llm_wiki_hierarchy(
                &fm_empty,
                Path::new("resources/spec.md"),
                HierarchyLevel::L0Ephemeral
            ),
            HierarchyLevel::L1Resource
        );

        // 4. Root file
        assert_eq!(
            determine_llm_wiki_hierarchy(
                &fm_empty,
                Path::new("INDEX.md"),
                HierarchyLevel::L1Resource
            ),
            HierarchyLevel::L3Evergreen
        );

        // 5. Fallback for nested unrecognized folder
        assert_eq!(
            determine_llm_wiki_hierarchy(
                &fm_empty,
                Path::new("unrecognized/file.md"),
                HierarchyLevel::L1Resource
            ),
            HierarchyLevel::L1Resource
        );
    }

    #[test]
    fn test_is_structured_vault_detection() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        assert!(!is_structured_vault(root));

        fs::create_dir_all(root.join("decisions")).unwrap();
        assert!(is_structured_vault(root));
    }
}
