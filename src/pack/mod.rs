//! Portable memory packs.
//!
//! A pack is a topology-agnostic bundle of markdown notes plus the Living ADR
//! guard invariants declared by those notes. Packs can be exported from any
//! vault layout (Johnny.Decimal / Obsidian Vault, flat Karpathy-style LLM-Wiki,
//! engineering `docs/adr` wikis, PARA, ...) and imported into a heterogeneous
//! target vault. Import routing is delegated to `FlushEngine`, which sniffs the
//! target convention.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::adapters::GenericWikiAdapter;
use crate::convention::{generate_filename, slugify, ConventionSniffer, FlushRequest};
use crate::{
    Document, IncrementalScanner, ObsidianAdapter, SqliteStorage, VaultAdapter, VectorSyncEngine,
};
use k0maru_core::guard::{
    extract_rules_from_document, extract_rules_from_vault, GuardRule, RuleCondition,
};

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

/// Options controlling [`install_pack`].
#[derive(Debug, Clone)]
pub struct PackInstallOptions {
    pub vault_path: PathBuf,
    pub source_path: PathBuf,
    pub home_override: Option<PathBuf>,
    pub dry_run: bool,
}

/// Metadata persisted alongside an installed pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledPackManifest {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub source_kind: String,
    pub notes_count: usize,
    pub guard_rules_count: usize,
    pub installed_skills: Vec<String>,
}

/// Result of installing a pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackInstallResult {
    pub manifest: InstalledPackManifest,
    pub install_dir: PathBuf,
    pub skill_paths: Vec<PathBuf>,
    pub dry_run: bool,
}

/// Result of uninstalling a pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackUninstallResult {
    pub name: String,
    pub removed_dir: PathBuf,
    pub removed_skills: Vec<String>,
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
            supersedes: Vec::new(),
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

/// Installs a pack from a `.pack.json` file or an OpenHarness `DSH` directory
/// into `<vault>/.k0maru/packs/<slug>/`, additionally exporting any bundled
/// skills to `<home>/.omp/agent/skills/<skill>/SKILL.md`.
pub fn install_pack(
    options: &PackInstallOptions,
) -> Result<PackInstallResult, Box<dyn std::error::Error>> {
    if options.source_path.is_file() {
        install_pack_from_file(options)
    } else if options.source_path.is_dir() {
        install_pack_from_directory(options)
    } else {
        Err(format!(
            "Pack source {} does not exist",
            options.source_path.display()
        )
        .into())
    }
}

/// Installs a serialized [`MemoryPack`] note by note.
fn install_pack_from_file(
    options: &PackInstallOptions,
) -> Result<PackInstallResult, Box<dyn std::error::Error>> {
    let raw = fs::read_to_string(&options.source_path)?;
    let pack: MemoryPack = serde_json::from_str(&raw).map_err(|error| {
        format!(
            "Failed to parse pack {}: {}",
            options.source_path.display(),
            error
        )
    })?;

    let pack_slug = validated_pack_slug(&pack.name)?;
    let install_dir = packs_root(&options.vault_path).join(&pack_slug);

    if !options.dry_run {
        fs::create_dir_all(&install_dir)?;
        for note in &pack.notes {
            let file_name = format!("{}.md", slugify(&note.title));
            fs::write(install_dir.join(file_name), render_pack_note(note))?;
        }
    }

    let manifest = InstalledPackManifest {
        name: pack.name,
        version: pack.version,
        description: pack.description,
        source_kind: "pack".to_string(),
        notes_count: pack.notes.len(),
        guard_rules_count: pack.guard_rules.len(),
        installed_skills: Vec::new(),
    };

    if !options.dry_run {
        write_pack_manifest(&install_dir, &manifest)?;
        sync_vault_cache(&options.vault_path)?;
    }

    Ok(PackInstallResult {
        manifest,
        install_dir,
        skill_paths: Vec::new(),
        dry_run: options.dry_run,
    })
}

/// Installs an OpenHarness `DSH` (or plain skill-pack) directory.
fn install_pack_from_directory(
    options: &PackInstallOptions,
) -> Result<PackInstallResult, Box<dyn std::error::Error>> {
    let (name, version, description) = read_dsh_metadata(&options.source_path)?;
    let pack_slug = validated_pack_slug(&name)?;
    let install_dir = packs_root(&options.vault_path).join(&pack_slug);

    let adapter = GenericWikiAdapter::new(&options.source_path);
    let note_files: Vec<PathBuf> = adapter
        .scan_files()?
        .into_iter()
        .filter(|relative| !is_skill_path(relative))
        .collect();

    let mut guard_rules: Vec<GuardRule> = Vec::new();
    let mut seen_rule_ids: HashSet<String> = HashSet::new();

    for relative in &note_files {
        let doc = adapter.read_document(relative)?;
        for rule in extract_rules_from_document(&doc) {
            if seen_rule_ids.insert(rule.id.clone()) {
                guard_rules.push(rule);
            }
        }
    }

    if let Ok(vault_rules) = extract_rules_from_vault(&options.source_path) {
        for rule in vault_rules {
            if seen_rule_ids.insert(rule.id.clone()) {
                guard_rules.push(rule);
            }
        }
    }

    let (installed_skills, skill_paths) = discover_and_install_skills(options)?;

    let manifest = InstalledPackManifest {
        name,
        version,
        description,
        source_kind: "dsh".to_string(),
        notes_count: note_files.len(),
        guard_rules_count: guard_rules.len(),
        installed_skills,
    };

    if !options.dry_run {
        fs::create_dir_all(&install_dir)?;
        for relative in &note_files {
            let destination = install_dir.join(relative);
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(options.source_path.join(relative), &destination)?;
        }

        if !guard_rules.is_empty() {
            fs::write(
                install_dir.join("pack-guard-rules.md"),
                render_guard_note(&manifest.name, &guard_rules),
            )?;
        }

        write_pack_manifest(&install_dir, &manifest)?;
        sync_vault_cache(&options.vault_path)?;
    }

    Ok(PackInstallResult {
        manifest,
        install_dir,
        skill_paths,
        dry_run: options.dry_run,
    })
}

/// Discovers skills under `<source>/skills/` and copies them into the OMP
/// skills root. Returns `(installed skill names, target SKILL.md paths)`.
fn discover_and_install_skills(
    options: &PackInstallOptions,
) -> Result<(Vec<String>, Vec<PathBuf>), Box<dyn std::error::Error>> {
    let skills_root = options.source_path.join("skills");
    if !skills_root.is_dir() {
        return Ok((Vec::new(), Vec::new()));
    }

    let omp_root = omp_skills_root(options.home_override.as_deref());
    let mut discovered: Vec<(String, PathBuf)> = Vec::new();

    for entry in fs::read_dir(&skills_root)? {
        let entry = entry?;
        let source = entry.path();
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            let skill_name = entry.file_name().to_string_lossy().to_string();
            if !is_safe_component(&skill_name) {
                continue;
            }

            let canonical_skill = source.join("SKILL.md");
            let fallback_skill = source.join(format!("{}.md", skill_name));
            let source_skill = if canonical_skill.is_file() {
                Some(canonical_skill)
            } else if fallback_skill.is_file() {
                Some(fallback_skill)
            } else {
                None
            };

            let Some(source_skill) = source_skill else {
                continue;
            };

            let target_dir = omp_root.join(&skill_name);
            discovered.push((skill_name, target_dir.join("SKILL.md")));

            if !options.dry_run {
                fs::create_dir_all(&target_dir)?;
                copy_directory_contents(&source, &target_dir)?;
                if !source_skill
                    .file_name()
                    .map(|name| name == "SKILL.md")
                    .unwrap_or(false)
                {
                    fs::copy(&source_skill, target_dir.join("SKILL.md"))?;
                }
            }
        } else if file_type.is_file() {
            let is_markdown = source
                .extension()
                .and_then(|extension| extension.to_str())
                .map(|extension| extension.eq_ignore_ascii_case("md"))
                .unwrap_or(false);
            if !is_markdown {
                continue;
            }

            let skill_name = source
                .file_stem()
                .map(|stem| stem.to_string_lossy().to_string())
                .unwrap_or_default();
            if !is_safe_component(&skill_name) {
                continue;
            }

            let target_dir = omp_root.join(&skill_name);
            discovered.push((skill_name, target_dir.join("SKILL.md")));

            if !options.dry_run {
                fs::create_dir_all(&target_dir)?;
                fs::copy(&source, target_dir.join("SKILL.md"))?;
            }
        }
    }

    discovered.sort_by(|left, right| left.0.cmp(&right.0));
    discovered.dedup_by(|left, right| left.0 == right.0);

    let mut installed_skills = Vec::with_capacity(discovered.len());
    let mut skill_paths = Vec::with_capacity(discovered.len());
    for (skill_name, skill_path) in discovered {
        installed_skills.push(skill_name);
        skill_paths.push(skill_path);
    }

    Ok((installed_skills, skill_paths))
}

/// Removes an installed pack and every skill it exported.
pub fn uninstall_pack(
    vault_path: &Path,
    name: &str,
    home_override: Option<&Path>,
) -> Result<PackUninstallResult, Box<dyn std::error::Error>> {
    let root = packs_root(vault_path);

    let mut candidates = Vec::new();
    if is_safe_component(name) {
        candidates.push(root.join(name));
    }
    let slug = slugify(name);
    if is_safe_component(&slug)
        && !candidates
            .iter()
            .any(|candidate| candidate.ends_with(&slug))
    {
        candidates.push(root.join(&slug));
    }

    let pack_dir = candidates
        .into_iter()
        .find(|candidate| candidate.is_dir())
        .ok_or_else(|| format!("Pack '{}' is not installed in {}", name, root.display()))?;

    let manifest_path = pack_dir.join("pack-manifest.json");
    let manifest = if manifest_path.is_file() {
        let raw = fs::read_to_string(&manifest_path)?;
        Some(
            serde_json::from_str::<InstalledPackManifest>(&raw).map_err(|error| {
                format!("Failed to parse {}: {}", manifest_path.display(), error)
            })?,
        )
    } else {
        None
    };

    let omp_root = omp_skills_root(home_override);
    let mut removed_skills = Vec::new();

    if let Some(manifest) = &manifest {
        for skill in &manifest.installed_skills {
            if !is_safe_component(skill) {
                continue;
            }
            let skill_dir = omp_root.join(skill);
            if skill_dir.is_dir() {
                fs::remove_dir_all(&skill_dir)?;
                removed_skills.push(skill.clone());
            } else if skill_dir.is_file() {
                fs::remove_file(&skill_dir)?;
                removed_skills.push(skill.clone());
            }
        }
    }

    fs::remove_dir_all(&pack_dir)?;
    sync_vault_cache(vault_path)?;

    Ok(PackUninstallResult {
        name: manifest
            .map(|manifest| manifest.name)
            .unwrap_or_else(|| name.to_string()),
        removed_dir: pack_dir,
        removed_skills,
    })
}

/// Lists every installed pack manifest, sorted by pack name.
pub fn list_installed_packs(
    vault_path: &Path,
) -> Result<Vec<InstalledPackManifest>, Box<dyn std::error::Error>> {
    let root = packs_root(vault_path);
    if !root.is_dir() {
        return Ok(Vec::new());
    }

    let mut manifests = Vec::new();
    for entry in fs::read_dir(&root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let manifest_path = entry.path().join("pack-manifest.json");
        if !manifest_path.is_file() {
            continue;
        }

        let raw = fs::read_to_string(&manifest_path)?;
        let manifest = serde_json::from_str::<InstalledPackManifest>(&raw)
            .map_err(|error| format!("Failed to parse {}: {}", manifest_path.display(), error))?;
        manifests.push(manifest);
    }

    manifests.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(manifests)
}

/// Returns the vault-local pack install root.
fn packs_root(vault_path: &Path) -> PathBuf {
    vault_path.join(".k0maru").join("packs")
}

/// Verifies a pack name cannot escape its install root, then slugs it.
fn validated_pack_slug(name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Pack name must not be empty".into());
    }
    if trimmed.contains("..") || trimmed.contains('/') || trimmed.contains('\\') {
        return Err(format!("Invalid pack name '{}'", name).into());
    }

    let slug = slugify(trimmed);
    if slug.is_empty() || slug == "." || slug == ".." || slug.contains('/') || slug.contains('\\') {
        return Err(format!("Invalid pack name '{}'", name).into());
    }

    Ok(slug)
}

/// Returns true when a relative path lives under the pack's `skills/` folder.
fn is_skill_path(relative: &Path) -> bool {
    relative
        .components()
        .next()
        .map(|component| {
            component
                .as_os_str()
                .to_string_lossy()
                .eq_ignore_ascii_case("skills")
        })
        .unwrap_or(false)
}

/// Rejects empty names, `.`/`..`, and any name containing path separators.
fn is_safe_component(name: &str) -> bool {
    let trimmed = name.trim();
    !trimmed.is_empty()
        && trimmed != "."
        && trimmed != ".."
        && !trimmed.contains('/')
        && !trimmed.contains('\\')
}

/// Resolves the OMP skills root from an override, `OMP_HOME`, or the user home.
fn omp_skills_root(home_override: Option<&Path>) -> PathBuf {
    if let Some(home) = home_override {
        return home.join(".omp").join("agent").join("skills");
    }

    if let Ok(omp_home) = std::env::var("OMP_HOME") {
        if !omp_home.trim().is_empty() {
            return PathBuf::from(omp_home).join("skills");
        }
    }

    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".omp")
        .join("agent")
        .join("skills")
}

/// Reads `harness.json` metadata when present; otherwise derives it from the
/// directory name.
fn read_dsh_metadata(
    source: &Path,
) -> Result<(String, String, Option<String>), Box<dyn std::error::Error>> {
    let harness_path = source.join("harness.json");
    if !harness_path.is_file() {
        return Ok((fallback_pack_name(source), "0.1.0".to_string(), None));
    }

    let raw = fs::read_to_string(&harness_path)?;
    let value: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|error| format!("Failed to parse {}: {}", harness_path.display(), error))?;

    let name = ["name", "id"]
        .iter()
        .find_map(|key| value.get(*key).and_then(|value| value.as_str()))
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(String::from)
        .unwrap_or_else(|| fallback_pack_name(source));

    let version = value
        .get("version")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|version| !version.is_empty())
        .unwrap_or("0.1.0")
        .to_string();

    let description = value
        .get("description")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|description| !description.is_empty())
        .map(String::from);

    Ok((name, version, description))
}

/// Derives a pack name from its source directory.
fn fallback_pack_name(source: &Path) -> String {
    source
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "pack".to_string())
}

/// Renders a `.pack.json` note, adding frontmatter only when it has none.
fn render_pack_note(note: &PackNote) -> String {
    if note.content.trim_start().starts_with("---") {
        return note.content.clone();
    }

    let frontmatter = serde_json::json!({
        "title": note.title.clone(),
        "tags": note.tags.clone(),
    });

    match serde_yaml::to_string(&frontmatter) {
        Ok(yaml) => format!("---\n{}\n---\n\n{}", yaml.trim_end(), note.content),
        Err(_) => note.content.clone(),
    }
}

/// Renders guard rules as a Living ADR note so the vault cache can extract them.
fn render_guard_note(pack_name: &str, rules: &[GuardRule]) -> String {
    let invariants: Vec<serde_json::Value> = rules
        .iter()
        .map(|rule| {
            let mut item = serde_json::Map::new();
            item.insert("id".to_string(), rule.id.clone().into());
            item.insert("title".to_string(), rule.title.clone().into());
            if let Some(pattern) = &rule.file_pattern {
                item.insert("file_pattern".to_string(), pattern.clone().into());
            }
            item.insert("severity".to_string(), rule.severity.as_str().into());
            match &rule.condition {
                RuleCondition::Forbidden(value) => {
                    item.insert("forbidden".to_string(), value.clone().into());
                }
                RuleCondition::MustContain(value) => {
                    item.insert("must_contain".to_string(), value.clone().into());
                }
            }
            item.insert("remediation".to_string(), rule.remediation.clone().into());
            serde_json::Value::Object(item)
        })
        .collect();

    let frontmatter = serde_json::json!({
        "title": format!("{} Guard Rules", pack_name),
        "tags": ["guard", "living-adr"],
        "invariants": invariants,
    });

    match serde_yaml::to_string(&frontmatter) {
        Ok(yaml) => format!(
            "---\n{}\n---\n\n# {} Guard Rules\n\nAuto-generated from the pack's Living ADR configuration.\n",
            yaml.trim_end(),
            pack_name
        ),
        Err(_) => format!("# {} Guard Rules\n", pack_name),
    }
}

/// Persists the install manifest inside a pack directory.
fn write_pack_manifest(
    install_dir: &Path,
    manifest: &InstalledPackManifest,
) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string_pretty(manifest)?;
    fs::write(install_dir.join("pack-manifest.json"), json)?;
    Ok(())
}

/// Recursively copies a directory's files into `destination`.
fn copy_directory_contents(
    source: &Path,
    destination: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in walkdir::WalkDir::new(source).follow_links(false) {
        let entry = entry?;
        let relative = entry.path().strip_prefix(source)?;
        if relative.as_os_str().is_empty() {
            continue;
        }

        let target = destination.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), &target)?;
        }
    }

    Ok(())
}

/// Re-scans the vault's existing SQLite cache after pack mutations.
fn sync_vault_cache(vault_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let cache_path = vault_path.join(".k0maru").join("cache.sqlite");
    if !cache_path.exists() {
        return Ok(());
    }

    let mut storage = SqliteStorage::open(&cache_path)?;
    let is_obsidian = vault_path.join("10_Projects").is_dir()
        || vault_path.join(".obsidian").exists()
        || vault_path.join("20_Cards").is_dir();

    let has_vectors = storage.has_vectors().unwrap_or(false);
    let embedder = if has_vectors {
        let model_cache = vault_path.join(".k0maru").join("models");
        crate::vector::default_embedding_engine(model_cache.exists().then_some(model_cache)).ok()
    } else {
        None
    };

    if is_obsidian {
        let adapter = ObsidianAdapter::new(vault_path);
        let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
        scanner.sync(false)?;
        if has_vectors {
            if let Some(embedder) = &embedder {
                let _ = VectorSyncEngine::sync(scanner.storage_mut(), &**embedder);
            }
        }
    } else {
        let adapter = GenericWikiAdapter::new(vault_path);
        let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
        scanner.sync(false)?;
        if has_vectors {
            if let Some(embedder) = &embedder {
                let _ = VectorSyncEngine::sync(scanner.storage_mut(), &**embedder);
            }
        }
    }

    Ok(())
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
