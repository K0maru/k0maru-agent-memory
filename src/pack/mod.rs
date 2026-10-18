//! Portable memory packs.
//!
//! A pack is a topology-agnostic bundle of markdown notes plus the Living ADR
//! guard invariants declared by those notes. Packs can be exported from any
//! vault layout (Johnny.Decimal SecondBrain, flat Karpathy-style LLM-Wiki,
//! engineering `docs/adr` wikis, PARA, ...) and imported into a heterogeneous
//! target vault. Import routing is delegated to `FlushEngine`, which sniffs the
//! target convention.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::adapters::GenericWikiAdapter;
use crate::convention::{generate_filename, slugify, ConventionSniffer, FlushRequest};
use crate::{Document, VaultAdapter};
use k0maru_core::guard::{extract_rules_from_document, GuardRule};

/// A single portable note inside a [`MemoryPack`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackNote {
    pub title: String,
    pub category: String,
    pub summary: Option<String>,
    pub tags: Vec<String>,
    pub related_notes: Vec<String>,
    pub content: String,
}

/// A portable bundle of notes and Living ADR guard rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryPack {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub notes: Vec<PackNote>,
    pub guard_rules: Vec<GuardRule>,
}

/// Options controlling [`export_pack`].
#[derive(Debug, Clone)]
pub struct PackExportOptions {
    pub vault_path: PathBuf,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub tag: Option<String>,
    pub query: Option<String>,
    pub files: Vec<PathBuf>,
}

/// Options controlling [`import_pack`].
#[derive(Debug, Clone)]
pub struct PackImportOptions {
    pub vault_path: PathBuf,
    pub dry_run: bool,
}

/// A note written (or predicted for a dry run) during import.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackImportItem {
    pub title: String,
    pub category: String,
    pub relative_path: PathBuf,
    pub created: bool,
    pub dry_run: bool,
}

/// Result of importing a pack into a target vault.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackImportResult {
    pub pack_name: String,
    pub vault_path: PathBuf,
    pub imported_notes: Vec<PackImportItem>,
    pub guard_rules_count: usize,
    pub dry_run: bool,
}

/// Inspection summary of a pack, optionally predicting target paths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackInspectReport {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub notes_count: usize,
    pub guard_rules_count: usize,
    pub note_titles: Vec<String>,
    pub predicted_paths: Vec<PathBuf>,
}

/// Rewrites folder-qualified WikiLinks so they are topology-agnostic.
///
/// Only the target portion inside `[[...]]` is rewritten: folder prefixes are
/// dropped, while aliases (`|Alias`), headings (`#Heading`), and block
/// references are preserved. Content outside WikiLinks is never modified.
pub fn normalize_wikilinks(markdown: &str) -> String {
    let mut out = String::with_capacity(markdown.len());
    let mut rest = markdown;

    while let Some(start) = rest.find("[[") {
        out.push_str(&rest[..start]);

        let after_open = &rest[start + 2..];
        match after_open.find("]]") {
            Some(end) => {
                let inner = &after_open[..end];
                out.push_str("[[");
                out.push_str(&normalize_wikilink_inner(inner));
                out.push_str("]]");
                rest = &after_open[end + 2..];
            }
            None => {
                // Malformed/unclosed WikiLink: leave the remainder untouched.
                out.push_str(&rest[start..]);
                rest = "";
            }
        }
    }

    out.push_str(rest);
    out
}

/// Normalizes the inside of a `[[...]]` pair, preserving alias and heading.
fn normalize_wikilink_inner(inner: &str) -> String {
    let (target, alias) = match inner.split_once('|') {
        Some((target, alias)) => (target, Some(alias)),
        None => (inner, None),
    };

    let normalized = normalize_wikilink_target(target);

    match alias {
        Some(alias) => format!("{}|{}", normalized, alias),
        None => normalized,
    }
}

/// Case-insensitive ASCII prefix check without allocating.
fn starts_with_ignore_ascii_case(text: &str, prefix: &str) -> bool {
    match text.get(..prefix.len()) {
        Some(head) => head.eq_ignore_ascii_case(prefix),
        None => false,
    }
}

/// Strips every folder prefix from a link target while retaining heading/block
/// fragments and external URLs.
fn normalize_wikilink_target(target: &str) -> String {
    let trimmed = target.trim();
    if starts_with_ignore_ascii_case(trimmed, "http://")
        || starts_with_ignore_ascii_case(trimmed, "https://")
        || starts_with_ignore_ascii_case(trimmed, "mailto:")
    {
        return target.to_string();
    }

    let (path_part, fragment) = match trimmed.split_once('#') {
        Some((path, fragment)) => (path, Some(fragment)),
        None => (trimmed, None),
    };

    if !path_part.contains('/') && !path_part.contains('\\') {
        return target.to_string();
    }

    let last_segment = path_part
        .rsplit(['/', '\\'])
        .find(|segment| !segment.is_empty())
        .unwrap_or(path_part);

    match fragment {
        Some(fragment) => format!("{}#{}", last_segment, fragment),
        None => last_segment.to_string(),
    }
}

/// Exports notes (and their Living ADR invariants) from a vault into a pack.
pub fn export_pack(options: &PackExportOptions) -> Result<MemoryPack, Box<dyn std::error::Error>> {
    let adapter = GenericWikiAdapter::new(&options.vault_path);
    let explicit_selection = !options.files.is_empty();

    let candidate_paths: Vec<PathBuf> = if explicit_selection {
        options
            .files
            .iter()
            .map(|file| resolve_export_path(&options.vault_path, file))
            .collect::<Result<Vec<_>, _>>()?
    } else {
        adapter.scan_files()?
    };

    let tag_filter = options.tag.as_deref().map(normalize_tag);
    let query_filter = options.query.as_deref().map(|q| q.trim().to_lowercase());

    let mut notes = Vec::new();
    let mut guard_rules: Vec<GuardRule> = Vec::new();
    let mut seen_rule_ids: HashSet<String> = HashSet::new();

    for relative_path in candidate_paths {
        if !explicit_selection && is_ephemeral(&relative_path) {
            continue;
        }

        let doc = adapter.read_document(&relative_path)?;

        if let Some(tag_filter) = &tag_filter {
            if !document_matches_tag(&doc, tag_filter) {
                continue;
            }
        }

        if let Some(query_filter) = &query_filter {
            if !document_matches_query(&doc, query_filter) {
                continue;
            }
        }

        let extracted_rules = extract_rules_from_document(&doc);
        let category = infer_category(&doc, &relative_path, &extracted_rules);

        for rule in extracted_rules {
            if seen_rule_ids.insert(rule.id.clone()) {
                guard_rules.push(rule);
            }
        }
        let summary = doc
            .frontmatter
            .get("summary")
            .and_then(|value| value.as_str())
            .map(String::from);
        let tags = collect_document_tags(&doc);
        let related_notes = collect_related_notes(&doc);
        let content = note_content(&options.vault_path, &doc);

        notes.push(PackNote {
            title: note_title(&doc, &relative_path),
            category,
            summary,
            tags,
            related_notes,
            content,
        });
    }

    Ok(MemoryPack {
        name: options.name.clone(),
        version: options.version.clone(),
        description: options.description.clone(),
        notes,
        guard_rules,
    })
}

/// Imports every note in a pack into the target vault via the convention-aware
/// `FlushEngine`.
pub fn import_pack(
    pack: &MemoryPack,
    options: &PackImportOptions,
) -> Result<PackImportResult, Box<dyn std::error::Error>> {
    let engine = crate::convention::FlushEngine::new(options.vault_path.clone())
        .map_err(|e| -> Box<dyn std::error::Error> { e.to_string().into() })?;

    let mut imported_notes = Vec::with_capacity(pack.notes.len());

    for note in &pack.notes {
        let request = FlushRequest {
            title: note.title.clone(),
            summary: note.summary.clone(),
            content: note.content.clone(),
            category: note.category.clone(),
            tags: note.tags.clone(),
            related_notes: note.related_notes.clone(),
            dry_run: options.dry_run,
        };

        let result = engine
            .flush(request)
            .map_err(|e| -> Box<dyn std::error::Error> { e.to_string().into() })?;

        imported_notes.push(PackImportItem {
            title: note.title.clone(),
            category: note.category.clone(),
            relative_path: result.relative_path,
            created: result.created,
            dry_run: options.dry_run,
        });
    }

    Ok(PackImportResult {
        pack_name: pack.name.clone(),
        vault_path: options.vault_path.clone(),
        imported_notes,
        guard_rules_count: pack.guard_rules.len(),
        dry_run: options.dry_run,
    })
}

/// Produces a human-readable inspection report for a pack.
pub fn inspect_pack(pack: &MemoryPack, target_vault: Option<&Path>) -> PackInspectReport {
    let convention = target_vault
        .filter(|vault| vault.exists())
        .map(ConventionSniffer::sniff);

    let predicted_paths = pack
        .notes
        .iter()
        .map(|note| match &convention {
            Some(convention) => {
                convention
                    .target_subdir_for(&note.category)
                    .join(generate_filename(
                        &note.title,
                        &convention.naming_style,
                        None,
                    ))
            }
            None => PathBuf::from(format!("{}.md", slugify(&note.title))),
        })
        .collect();

    PackInspectReport {
        name: pack.name.clone(),
        version: pack.version.clone(),
        description: pack.description.clone(),
        notes_count: pack.notes.len(),
        guard_rules_count: pack.guard_rules.len(),
        note_titles: pack.notes.iter().map(|note| note.title.clone()).collect(),
        predicted_paths,
    }
}

/// Converts an exported file selection into a path relative to the vault root.
fn resolve_export_path(
    vault_path: &Path,
    file: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if !file.is_absolute() {
        return Ok(file.to_path_buf());
    }

    if let Ok(stripped) = file.strip_prefix(vault_path) {
        return Ok(stripped.to_path_buf());
    }

    let canonical_vault = vault_path
        .canonicalize()
        .unwrap_or_else(|_| vault_path.to_path_buf());
    if let Ok(canonical_file) = file.canonicalize() {
        if let Ok(stripped) = canonical_file.strip_prefix(&canonical_vault) {
            return Ok(stripped.to_path_buf());
        }
    }

    Err(format!(
        "File {} is not inside vault {}",
        file.display(),
        vault_path.display()
    )
    .into())
}

/// Returns true for daily/ephemeral notes that are excluded from bulk exports.
fn is_ephemeral(path: &Path) -> bool {
    path.components().any(|component| {
        let segment = component.as_os_str().to_string_lossy().to_lowercase();
        matches!(
            segment.as_str(),
            "00_daily" | "daily" | "templates" | "_templates" | "99_system"
        )
    })
}

/// Case-insensitive, `#`-agnostic tag comparison.
fn normalize_tag(tag: &str) -> String {
    tag.trim().trim_start_matches('#').to_lowercase()
}

/// Checks document and frontmatter tags against a normalized filter.
fn document_matches_tag(doc: &Document, filter: &str) -> bool {
    doc.tags
        .iter()
        .chain(frontmatter_tags(&doc.frontmatter).iter())
        .any(|tag| normalize_tag(tag) == filter)
}

/// Case-insensitive free-text match across title, body, and relative path.
fn document_matches_query(doc: &Document, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }

    doc.title.to_lowercase().contains(query)
        || doc.body.to_lowercase().contains(query)
        || doc.path.to_string_lossy().to_lowercase().contains(query)
}

/// Extracts tag strings from a JSON/YAML frontmatter value.
fn frontmatter_tags(frontmatter: &serde_json::Value) -> Vec<String> {
    match frontmatter.get("tags") {
        Some(serde_json::Value::Array(values)) => values
            .iter()
            .filter_map(|value| value.as_str())
            .map(String::from)
            .collect(),
        Some(serde_json::Value::String(tags)) => tags
            .split(',')
            .map(str::trim)
            .filter(|tag| !tag.is_empty())
            .map(String::from)
            .collect(),
        _ => Vec::new(),
    }
}

/// Merges `doc.tags` with frontmatter tags, preserving order and removing
/// duplicates (case-insensitive, leading `#` ignored).
fn collect_document_tags(doc: &Document) -> Vec<String> {
    let mut tags = Vec::new();
    let mut seen = HashSet::new();

    for tag in doc
        .tags
        .iter()
        .chain(frontmatter_tags(&doc.frontmatter).iter())
    {
        let cleaned = tag.trim().trim_start_matches('#').to_string();
        if cleaned.is_empty() {
            continue;
        }
        if seen.insert(cleaned.to_lowercase()) {
            tags.push(cleaned);
        }
    }

    tags
}

/// Returns the last path segment of every outgoing WikiLink target.
fn collect_related_notes(doc: &Document) -> Vec<String> {
    let mut related = Vec::new();
    let mut seen = HashSet::new();

    for link in &doc.links {
        let target = link
            .target
            .split('#')
            .next()
            .unwrap_or(link.target.as_str())
            .trim();
        let segment = target
            .rsplit(['/', '\\'])
            .find(|segment| !segment.is_empty())
            .unwrap_or(target);
        let name = segment.trim_end_matches(".md").to_string();

        if !name.is_empty() && seen.insert(name.clone()) {
            related.push(name);
        }
    }

    related
}

/// Infers the semantic category for a document from frontmatter and path.
fn infer_category(doc: &Document, path: &Path, rules: &[GuardRule]) -> String {
    if let Some(value) = doc
        .frontmatter
        .get("category")
        .or_else(|| doc.frontmatter.get("type"))
        .and_then(|value| value.as_str())
    {
        let cleaned = value.trim();
        if !cleaned.is_empty() {
            return cleaned.to_string();
        }
    }

    let path_lower = path.to_string_lossy().to_lowercase();

    if !rules.is_empty() || path_lower.contains("decision") || path_lower.contains("adr") {
        "decision".to_string()
    } else if path_lower.contains("skill") || path_lower.contains("playbook") {
        "skill".to_string()
    } else if path_lower.contains("log") || path_lower.contains("01_ai_logs") {
        "log".to_string()
    } else {
        "concept".to_string()
    }
}

/// Uses the parsed title, falling back to the file stem when it is empty.
fn note_title(doc: &Document, path: &Path) -> String {
    if !doc.title.trim().is_empty() {
        return doc.title.clone();
    }

    path.file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_else(|| doc.title.clone())
}

/// Produces portable note content while preserving Living ADR declarations.
fn note_content(vault_path: &Path, doc: &Document) -> String {
    let raw_path = vault_path.join(&doc.path);

    if let Ok(raw) = fs::read_to_string(raw_path) {
        return normalize_wikilinks(&raw);
    }

    // Fall back to the parsed body. If the note carries frontmatter invariants,
    // re-render the frontmatter so importing it into another vault keeps the
    // Living ADR rules intact.
    let body = normalize_wikilinks(&doc.body);
    if frontmatter_has_invariants(&doc.frontmatter) {
        if let Ok(yaml) = serde_yaml::to_string(&doc.frontmatter) {
            let terminator = if yaml.ends_with('\n') {
                String::new()
            } else {
                "\n".to_string()
            };
            return format!("---\n{}{}---\n\n{}", yaml, terminator, body);
        }
    }

    body
}

/// Returns true if frontmatter declares Living ADR invariants.
fn frontmatter_has_invariants(frontmatter: &serde_json::Value) -> bool {
    ["invariants", "guard_rules", "rules"].iter().any(|key| {
        frontmatter
            .get(*key)
            .map(|value| match value {
                serde_json::Value::Null => false,
                serde_json::Value::Array(values) => !values.is_empty(),
                serde_json::Value::Object(values) => !values.is_empty(),
                _ => true,
            })
            .unwrap_or(false)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_folder_prefix_and_preserves_alias() {
        assert_eq!(
            normalize_wikilinks("See [[20_Cards/Redis_Lock|Redis Lock]] for details."),
            "See [[Redis_Lock|Redis Lock]] for details."
        );
    }

    #[test]
    fn strips_nested_paths_and_preserves_headings() {
        assert_eq!(
            normalize_wikilinks("[[10_Projects/Payment/Auth#Section]]"),
            "[[Auth#Section]]"
        );
        assert_eq!(
            normalize_wikilinks("[[10_Projects/Payment/Auth#Section|Auth]]"),
            "[[Auth#Section|Auth]]"
        );
    }

    #[test]
    fn leaves_plain_links_and_outside_text_untouched() {
        let input = "Folder 20_Cards/Redis_Lock stays; [[Plain_Note|Plain]] too.";
        assert_eq!(normalize_wikilinks(input), input);
    }

    #[test]
    fn normalizes_every_wikilink_in_a_document() {
        assert_eq!(
            normalize_wikilinks(
                "![[20_Cards/Diagram]] and [[10_Projects/App#API]] and [[docs/adr/ADR-1.md]]"
            ),
            "![[Diagram]] and [[App#API]] and [[ADR-1.md]]"
        );
    }

    #[test]
    fn leaves_malformed_links_unchanged() {
        assert_eq!(
            normalize_wikilinks("[[20_Cards/Unclosed"),
            "[[20_Cards/Unclosed"
        );
    }

    #[test]
    fn does_not_rewrite_embedded_urls_or_fragment_only_links() {
        assert_eq!(
            normalize_wikilinks("[[https://example.com/a/b]]"),
            "[[https://example.com/a/b]]"
        );
        assert_eq!(normalize_wikilinks("[[#Heading]]"), "[[#Heading]]");
    }
}
