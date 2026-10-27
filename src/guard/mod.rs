use std::collections::HashSet;
use std::fs;
use std::io::{IsTerminal, Read};
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::{Args, Subcommand};
use k0maru_core::adapters::GenericWikiAdapter;
use k0maru_core::core::traits::VaultAdapter;
use k0maru_core::guard::extractor::{
    extract_rules_from_config, extract_rules_from_document, extract_rules_from_vault,
};
use k0maru_core::guard::models::{GuardReport, GuardRule, RuleSeverity};
pub use k0maru_core::guard::GuardEngine;

#[derive(Args, Debug, Clone)]
pub struct GuardArgs {
    #[command(subcommand)]
    pub command: GuardSubcommand,
}

#[derive(Subcommand, Debug, Clone)]
pub enum GuardSubcommand {
    /// Audit Git diff or repository files against Living ADR architectural invariants
    Check(GuardCheckArgs),

    /// Install k0maru guard Git pre-commit hook into repository
    Install(GuardInstallArgs),
}

#[derive(Args, Debug, Clone)]
pub struct GuardCheckArgs {
    /// Path to markdown vault root directory
    #[arg(short, long, value_name = "PATH")]
    pub vault: Option<PathBuf>,

    /// Inspect Git working tree / index diff (default behavior)
    #[arg(short, long)]
    pub diff: bool,

    /// Only audit staged changes (`git diff --cached`); used by the pre-commit hook
    #[arg(long)]
    pub staged: bool,

    /// Target project name to filter ADR invariants for
    #[arg(short, long, value_name = "PROJECT")]
    pub project: Option<String>,

    /// Target Git base branch for comparison (e.g. 'main', 'dev')
    #[arg(short, long, value_name = "BRANCH")]
    pub branch: Option<String>,

    /// Output structured JSON report
    #[arg(long)]
    pub json: bool,

    /// Claude Code `Stop` hook mode: audit the working tree, print violations to
    /// stderr with exit code 2, and stay silent (exit 0) when the audit passes
    #[arg(long)]
    pub claude_stop_hook: bool,

    /// Strict mode: treat all warnings as blocking errors
    #[arg(long)]
    pub strict: bool,

    /// Specific file paths to check directly
    #[arg(value_name = "FILES")]
    pub files: Vec<PathBuf>,
}

#[derive(Args, Debug, Clone)]
pub struct GuardInstallArgs {
    /// Target hook type (default: pre-commit)
    #[arg(long, default_value = "pre-commit")]
    pub hook: String,

    /// Path to target git repository directory (default: current directory)
    #[arg(short, long, value_name = "REPO_DIR")]
    pub path: Option<PathBuf>,

    /// Path to the markdown vault so the installed hook audits the right ADRs
    #[arg(short, long, value_name = "PATH")]
    pub vault: Option<PathBuf>,
}

/// Collects guard rules declared under `<root>/.k0maru/rules`, which may be
/// either a single config file or a directory of `.toml` / `.yaml` / `.yml` /
/// `.md` rule files.
fn collect_dot_k0maru_rules(
    root: &Path,
    seen: &mut HashSet<(String, String)>,
    rules: &mut Vec<GuardRule>,
) {
    collect_rules_from_path(&root.join(".k0maru").join("rules"), seen, rules);
}

/// Collects rules from a rule path that may be a single file or a directory.
fn collect_rules_from_path(
    rules_path: &Path,
    seen: &mut HashSet<(String, String)>,
    rules: &mut Vec<GuardRule>,
) {
    let mut files: Vec<PathBuf> = Vec::new();

    if rules_path.is_file() {
        files.push(rules_path.to_path_buf());
    } else if rules_path.is_dir() {
        if let Ok(entries) = fs::read_dir(rules_path) {
            files = entries
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .filter(|path| path.is_file())
                .filter(|path| {
                    matches!(
                        path.extension()
                            .and_then(|ext| ext.to_str())
                            .map(|ext| ext.to_lowercase())
                            .as_deref(),
                        Some("toml" | "yaml" | "yml" | "md")
                    )
                })
                .collect();
            files.sort();
        }
    }

    for file in files {
        let is_markdown = file
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("md"))
            .unwrap_or(false);

        let extracted = if is_markdown {
            // Markdown rule notes carry their invariants in frontmatter/body,
            // so parse them as documents rather than YAML configs.
            let file_name = file
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.to_string())
                .unwrap_or_default();
            let adapter_root: PathBuf = if rules_path.is_dir() {
                rules_path.to_path_buf()
            } else {
                rules_path
                    .parent()
                    .map(|parent| parent.to_path_buf())
                    .unwrap_or_else(|| rules_path.to_path_buf())
            };
            let adapter = GenericWikiAdapter::new(adapter_root);
            adapter
                .read_document(Path::new(&file_name))
                .map(|doc| extract_rules_from_document(&doc))
                .unwrap_or_default()
        } else {
            extract_rules_from_config(&file).unwrap_or_default()
        };

        for rule in extracted {
            if seen.insert((rule.id.clone(), format!("{:?}", rule.condition))) {
                rules.push(rule);
            }
        }
    }
}

/// Resolves the canonical Git work-tree root that contains `start`.
///
/// When `start` points at a file its parent directory is used. The lookup
/// delegates to `git rev-parse --show-toplevel` so guard checks behave
/// identically from the repository root and from nested directories such as
/// `src/`. Non-Git locations fall back to the canonicalized input path.
pub fn resolve_repo_root(start: &Path) -> PathBuf {
    let base = if start.is_file() {
        start
            .parent()
            .map(|parent| parent.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
    } else {
        start.to_path_buf()
    };
    let base = if base.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        base
    };

    if let Ok(output) = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(&base)
        .output()
    {
        if output.status.success() {
            let toplevel = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !toplevel.is_empty() {
                let path = PathBuf::from(toplevel);
                return path.canonicalize().unwrap_or(path);
            }
        }
    }

    base.canonicalize().unwrap_or(base)
}

/// Pushes `path` onto `roots` unless an equivalent entry already exists.
fn push_unique_root(roots: &mut Vec<PathBuf>, path: &Path) {
    if path.as_os_str().is_empty() {
        return;
    }
    if !roots.iter().any(|existing| existing == path) {
        roots.push(path.to_path_buf());
    }
}

/// Candidate directories whose `.k0maru` configuration must be considered.
///
/// A vault mounted at `<repo>/.k0maru/wiki` still belongs to `<repo>`, so the
/// `.k0maru` directory and the repository root are always inspected. This
/// keeps `k0maru guard check` CWD-independent.
fn guard_config_roots(vault_path: &Path, repo_root: Option<&Path>) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    push_unique_root(&mut roots, vault_path);

    let under_dot_k0maru = vault_path
        .ancestors()
        .any(|ancestor| ancestor.file_name().and_then(|name| name.to_str()) == Some(".k0maru"));
    if under_dot_k0maru {
        if let Some(parent) = vault_path.parent() {
            push_unique_root(&mut roots, parent);
            if let Some(grandparent) = parent.parent() {
                push_unique_root(&mut roots, grandparent);
            }
        }
    }

    if let Some(root) = repo_root {
        push_unique_root(&mut roots, root);
    }

    roots
}

/// Collects every project slug declared by a document: either from its
/// location under a project directory or from `project` / `projects`
/// frontmatter. Slugs are normalized with [`crate::convention::slugify`].
fn extract_doc_project_slugs(
    doc: &crate::Document,
    segments: &[&str],
    relative_path: &Path,
) -> Vec<String> {
    let mut projects: Vec<String> = Vec::new();
    let mut push = |raw: &str| {
        let slug = crate::convention::slugify(raw);
        if !slug.is_empty() && slug != "untitled" && !projects.iter().any(|p| p == &slug) {
            projects.push(slug);
        }
    };

    if segments
        .iter()
        .any(|segment| matches!(*segment, "10_projects" | "projects" | "1_projects"))
    {
        if let Some(stem) = relative_path.file_stem().and_then(|stem| stem.to_str()) {
            push(stem);
        }
    }

    for key in ["project", "projects"] {
        let Some(value) = doc.frontmatter.get(key) else {
            continue;
        };
        let mut raw_values: Vec<String> = Vec::new();
        if let Some(single) = value.as_str() {
            raw_values.push(single.to_string());
        } else if let Some(array) = value.as_array() {
            for item in array {
                if let Some(single) = item.as_str() {
                    raw_values.push(single.to_string());
                }
            }
        }

        for raw in raw_values {
            let mut clean = raw.trim();
            if let Some(stripped) = clean.strip_prefix("[[") {
                clean = stripped;
            }
            if let Some(stripped) = clean.strip_suffix("]]") {
                clean = stripped;
            }
            if let Some((before_alias, _)) = clean.split_once('|') {
                clean = before_alias.trim();
            }
            if let Some(stripped) = clean.strip_suffix(".md") {
                clean = stripped;
            }
            let mut clean = clean.trim().trim_matches('/');
            for prefix in ["10_Projects/", "10_projects/", "1_Projects/", "1_projects/"] {
                if let Some(stripped) = clean.strip_prefix(prefix) {
                    clean = stripped;
                    break;
                }
            }
            for prefix in ["projects/", "Projects/", "PROJECTS/"] {
                if let Some(stripped) = clean.strip_prefix(prefix) {
                    clean = stripped;
                    break;
                }
            }
            let lower = clean.to_ascii_lowercase();
            if let Some(idx) = lower.find("projects/") {
                clean = &clean[(idx + "projects/".len())..];
            }
            if clean.is_empty() {
                continue;
            }
            push(clean);
        }
    }

    projects
}

/// Resolves active GuardRules from vault configuration files and document notes.
pub fn resolve_guard_rules(
    vault_path: &Path,
    project_filter: Option<&str>,
) -> Result<Vec<GuardRule>, Box<dyn std::error::Error>> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let root = resolve_repo_root(&cwd);
    resolve_guard_rules_with_repo(vault_path, project_filter, Some(&root))
}

/// Resolves active GuardRules with an explicit repository root.
///
/// `repo_root` anchors `.k0maru` discovery, project-slug isolation and the
/// same-commit supersession guard (a commit that flips an ADR to
/// `status: superseded` must not silently disable that ADR's invariants).
pub fn resolve_guard_rules_with_repo(
    vault_path: &Path,
    project_filter: Option<&str>,
    repo_root: Option<&Path>,
) -> Result<Vec<GuardRule>, Box<dyn std::error::Error>> {
    let mut rules = Vec::new();
    let mut seen = HashSet::new();

    // 1. Configuration files: vault root, the enclosing `.k0maru` directory
    //    (for `<repo>/.k0maru/wiki` vaults) and the Git repository root.
    for config_root in guard_config_roots(vault_path, repo_root) {
        if let Ok(config_rules) = extract_rules_from_vault(&config_root) {
            for r in config_rules {
                if seen.insert((r.id.clone(), format!("{:?}", r.condition))) {
                    rules.push(r);
                }
            }
        }

        // 1b. `.k0maru/rules` (single file or rule directory).
        collect_dot_k0maru_rules(&config_root, &mut seen, &mut rules);

        // A `.k0maru` directory passed directly as the vault root owns
        // `<root>/rules` rather than `<root>/.k0maru/rules`.
        if config_root.file_name().and_then(|name| name.to_str()) == Some(".k0maru") {
            collect_rules_from_path(&config_root.join("rules"), &mut seen, &mut rules);
        }
    }

    // 2. Scan vault notes for living ADR invariants (topology-agnostic).
    let mut scanned: Vec<(PathBuf, crate::Document, Vec<String>)> = Vec::new();
    let mut all_vault_projects: HashSet<String> = HashSet::new();

    if vault_path.is_dir() {
        let adapter = GenericWikiAdapter::new(vault_path);

        if let Ok(files) = adapter.scan_files() {
            for relative_path in files {
                let path_lower = relative_path
                    .to_string_lossy()
                    .replace('\\', "/")
                    .to_lowercase();
                let segments: Vec<&str> = path_lower.split('/').collect();

                // Skip ephemeral and log directories: they never own invariants.
                if segments.iter().any(|segment| {
                    matches!(
                        *segment,
                        "00_daily" | "01_ai_logs" | "30_logs" | "daily" | "logs" | "journal"
                    )
                }) {
                    continue;
                }

                let doc = match adapter.read_document(&relative_path) {
                    Ok(doc) => doc,
                    Err(_) => continue,
                };

                let doc_projects = extract_doc_project_slugs(&doc, &segments, &relative_path);
                for project in &doc_projects {
                    all_vault_projects.insert(project.clone());
                }

                scanned.push((relative_path, doc, doc_projects));
            }
        }
    }

    // 2b. Project isolation. `--project` (or the repository the check runs in)
    // selects exactly one slug: `beta` never matches `payment-beta-legacy`, and
    // a `project: alpha` card never leaks into the `beta` repository. Notes
    // without project frontmatter are vault-global and always apply.
    let active_project_slug: Option<String> = if let Some(filter) = project_filter {
        Some(crate::convention::slugify(filter))
    } else if let Some(root) = repo_root {
        if all_vault_projects.is_empty() {
            None
        } else {
            root.file_name()
                .and_then(|name| name.to_str())
                .map(crate::convention::slugify)
                .filter(|slug| !slug.is_empty())
        }
    } else {
        None
    };

    for (_relative_path, doc, doc_projects) in &scanned {
        if let Some(target_slug) = &active_project_slug {
            if !doc_projects.is_empty() && !doc_projects.iter().any(|p| p == target_slug) {
                continue;
            }
        }

        for rule in extract_rules_from_document(doc) {
            if seen.insert((rule.id.clone(), format!("{:?}", rule.condition))) {
                rules.push(rule);
            }
        }
    }

    // 3. Same-commit supersession defense: a changeset that flips an active ADR
    // to `status: superseded` while also introducing the violation must still
    // be audited against the ADR that was active at HEAD.
    if let Some(root) = repo_root {
        collect_head_active_rules(root, vault_path, &scanned, &mut seen, &mut rules);
    }

    Ok(rules)
}

/// Normalizes a wiki-link / path reference from `superseded_by` or `supersedes`
/// frontmatter into an exact slug (`[[20_Cards/ADR-002|alias]]` -> `adr-002`).
fn normalize_doc_ref(raw: &str) -> String {
    let mut clean = raw.trim();
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
    let name = clean.rsplit('/').next().unwrap_or(clean);
    let slug = crate::convention::slugify(name);
    if slug == "untitled" {
        String::new()
    } else {
        slug
    }
}

/// Slug of a document path's file stem, used for supersession matching.
fn doc_path_slug(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(crate::convention::slugify)
        .unwrap_or_default()
}

/// Normalized vault-relative path used for comparisons (forward slashes).
fn normalized_rel(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Finds the working-tree document whose vault-relative path matches `vault_rel`.
fn find_working_doc<'a>(
    scanned: &'a [(PathBuf, crate::Document, Vec<String>)],
    vault_rel: &Path,
) -> Option<&'a crate::Document> {
    let target = normalized_rel(vault_rel);
    scanned
        .iter()
        .find(|(path, _, _)| normalized_rel(path) == target)
        .map(|(_, doc, _)| doc)
}

/// True when an active working-tree document explicitly declares `supersedes`
/// for `vault_rel`'s stem/slug.
fn has_active_superseder(
    scanned: &[(PathBuf, crate::Document, Vec<String>)],
    vault_rel: &Path,
) -> bool {
    let own_slug = doc_path_slug(vault_rel);
    if own_slug.is_empty() {
        return false;
    }
    scanned.iter().any(|(path, doc, _)| {
        doc.is_active()
            && normalized_rel(path) != normalized_rel(vault_rel)
            && doc
                .supersedes()
                .iter()
                .any(|target| normalize_doc_ref(target) == own_slug)
    })
}

/// True when a superseded working-tree document points at an active
/// replacement, or an active document declares it supersedes this note.
fn is_legitimately_superseded(
    current_doc: &crate::Document,
    vault_rel: &Path,
    scanned: &[(PathBuf, crate::Document, Vec<String>)],
) -> bool {
    if let Some(targets) = current_doc.superseded_by() {
        for raw in targets.split(',') {
            let slug = normalize_doc_ref(raw);
            if slug.is_empty() {
                continue;
            }
            let replacement_exists = scanned.iter().any(|(path, doc, _)| {
                doc.is_active()
                    && doc_path_slug(path) == slug
                    && normalized_rel(path) != normalized_rel(vault_rel)
            });
            if replacement_exists {
                return true;
            }
        }
    }
    has_active_superseder(scanned, vault_rel)
}

/// True for guard configuration paths whose `HEAD` rules must not silently
/// disappear in the same commit (`.k0maru/guard.toml`, `.k0maru/rules/*`).
fn is_guard_config_path(rel: &str) -> bool {
    let lower = rel.replace('\\', "/").to_lowercase();
    let file_name = lower.rsplit('/').next().unwrap_or("");
    let under_k0maru = lower.contains(".k0maru/");
    let is_guard_config = matches!(file_name, "guard.toml" | "guard.yaml" | "guard.yml");
    let under_rules = lower.contains(".k0maru/rules/");
    under_k0maru && (is_guard_config || under_rules)
}

/// Builds a transient [`crate::Document`] from raw markdown content.
fn document_from_content(path: &Path, content: &str) -> crate::Document {
    let (frontmatter, body) = k0maru_core::parser::parse_frontmatter(content);
    crate::Document {
        path: path.to_path_buf(),
        title: String::new(),
        hierarchy: crate::HierarchyLevel::L3Evergreen,
        frontmatter,
        links: Vec::new(),
        tags: Vec::new(),
        content_hash: String::new(),
        mtime: 0,
        body: body.to_string(),
    }
}

/// Extracts rules from `HEAD` guard-configuration content.
fn extract_rules_from_config_content(rel_path: &str, content: &str) -> Vec<GuardRule> {
    let extension = Path::new(rel_path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase();

    if extension == "md" {
        let doc = document_from_content(Path::new(rel_path), content);
        return extract_rules_from_document(&doc);
    }
    if !matches!(extension.as_str(), "toml" | "yaml" | "yml") {
        return Vec::new();
    }

    // `extract_rules_from_config` reads from disk, so stage the HEAD blob in a
    // uniquely named temporary file and clean it up immediately.
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let tmp_path = std::env::temp_dir().join(format!(
        "k0maru-head-rules-{}-{}.{}",
        std::process::id(),
        nanos,
        extension
    ));
    if fs::write(&tmp_path, content).is_err() {
        return Vec::new();
    }
    let extracted = extract_rules_from_config(&tmp_path).unwrap_or_default();
    let _ = fs::remove_file(&tmp_path);
    extracted
}

/// Re-adds invariants declared by documents that were active at `HEAD` but are
/// no longer active (or were deleted/weakened) in the working tree.
///
/// Without this, `status: active -> superseded`, or deleting a `forbidden:`
/// line, in the very same commit as a violating change would silently disable
/// the rule being violated. A *legitimate* supersession (an active replacement
/// ADR that exists in the working tree, or an active ADR declaring
/// `supersedes`) is honored and does not resurrect the old rule.
fn collect_head_active_rules(
    repo_root: &Path,
    vault_path: &Path,
    scanned: &[(PathBuf, crate::Document, Vec<String>)],
    seen: &mut HashSet<(String, String)>,
    rules: &mut Vec<GuardRule>,
) {
    // Only notes inside the vault can own invariants; map the vault onto the
    // repository so `git show HEAD:<path>` paths can be filtered. Guard config
    // files are repository-relative and audited even when the vault is mounted
    // outside the repository.
    let canonical_vault = vault_path
        .canonicalize()
        .unwrap_or_else(|_| vault_path.to_path_buf());
    let canonical_root = repo_root
        .canonicalize()
        .unwrap_or_else(|_| repo_root.to_path_buf());
    let vault_prefix = canonical_vault
        .strip_prefix(&canonical_root)
        .ok()
        .map(|prefix| prefix.to_path_buf());

    let mut changed: Vec<String> = Vec::new();
    for args in [
        ["diff", "HEAD", "--name-only", "--diff-filter=MD"],
        ["diff", "--cached", "--name-only", "--diff-filter=MD"],
    ] {
        let output = match Command::new("git")
            .args(args)
            .current_dir(repo_root)
            .output()
        {
            Ok(output) if output.status.success() => output,
            _ => continue,
        };

        for line in String::from_utf8_lossy(&output.stdout).lines() {
            let rel = line.trim();
            if rel.is_empty() {
                continue;
            }
            let is_markdown = rel.ends_with(".md");
            let is_config = is_guard_config_path(rel);
            if !is_markdown && !is_config {
                continue;
            }
            if is_markdown {
                let inside_vault = vault_prefix
                    .as_ref()
                    .map(|prefix| Path::new(rel).starts_with(prefix))
                    .unwrap_or(false);
                if !inside_vault && !is_config {
                    continue;
                }
            }
            if !changed.iter().any(|existing| existing == rel) {
                changed.push(rel.to_string());
            }
        }
    }

    for rel_path in changed {
        let output = match Command::new("git")
            .args(["show", &format!("HEAD:{rel_path}")])
            .current_dir(repo_root)
            .output()
        {
            Ok(output) if output.status.success() => output,
            _ => continue,
        };
        let head_content = String::from_utf8_lossy(&output.stdout).to_string();

        // Guard configuration (including `.k0maru/rules/*.md`) is audited from
        // HEAD regardless of where the vault is mounted.
        if is_guard_config_path(&rel_path) {
            for mut rule in extract_rules_from_config_content(&rel_path, &head_content) {
                // Report the repository path, not the transient HEAD temp file.
                rule.source_note = rel_path.clone();
                if seen.insert((rule.id.clone(), format!("{:?}", rule.condition))) {
                    rules.push(rule);
                }
            }
            continue;
        }

        if !rel_path.ends_with(".md") {
            continue;
        }

        let Some(prefix) = vault_prefix.as_ref() else {
            continue;
        };
        let Ok(vault_rel) = Path::new(&rel_path).strip_prefix(prefix) else {
            continue;
        };
        let vault_rel = vault_rel.to_path_buf();

        let head_doc = document_from_content(Path::new(&rel_path), &head_content);
        if !head_doc.is_active() {
            continue;
        }

        match find_working_doc(scanned, &vault_rel) {
            Some(current_doc) => {
                // A legitimate same-commit supersession (retiring an ADR and
                // replacing it with an active one) must not resurrect the old
                // rule. A bare `status: superseded` flip without a valid
                // replacement still must.
                if !current_doc.is_active()
                    && is_legitimately_superseded(current_doc, &vault_rel, scanned)
                {
                    continue;
                }
                // Active in the working tree is not enough: the commit may
                // have deleted or weakened the rule itself, so HEAD's missing
                // rules are re-added (deduplicated against `seen`).
            }
            None => {
                // Deleted in the working tree: resurrect unless an active
                // document explicitly supersedes it.
                if has_active_superseder(scanned, &vault_rel) {
                    continue;
                }
            }
        }

        for rule in extract_rules_from_document(&head_doc) {
            if seen.insert((rule.id.clone(), format!("{:?}", rule.condition))) {
                rules.push(rule);
            }
        }
    }
}

/// Flags that force `git diff` to emit plain, parseable unified diffs.
///
/// Without these, `git config color.ui always`, `diff.mnemonicPrefix`,
/// `.gitattributes -diff`, `textconv` filters, or binary NUL bytes can hide a
/// violation from [`GuardEngine::check_diff`].
const HARDENED_DIFF_FLAGS: [&str; 6] = [
    "--no-color",
    "--no-ext-diff",
    "--no-textconv",
    "--src-prefix=a/",
    "--dst-prefix=b/",
    "--text",
];

/// Runs `git diff` with extra arguments, returning stdout on success.
fn git_diff_output(repo_root: &Path, extra_args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("diff")
        .args(HARDENED_DIFF_FLAGS)
        .args(extra_args)
        .current_dir(repo_root)
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        None
    }
}

/// Maximum size of an untracked file that is synthesized into the diff.
const MAX_UNTRACKED_BYTES: u64 = 1024 * 1024;

/// Obtains git diff output for the current repository.
///
/// The result always covers staged (`git diff --cached`) and unstaged
/// (`git diff`) changes, so staging a violation and then reverting the working
/// tree can never hide it. Untracked files are synthesized into unified diff
/// blocks so brand-new files are audited too.
pub fn get_git_diff(
    repo_root: &Path,
    branch: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    get_git_diff_with_mode(repo_root, branch, false)
}

/// Obtains git diff output, optionally restricted to the staged index.
///
/// `staged_only = true` inspects only `git diff --cached` (staged changes vs
/// `HEAD`). The pre-commit hook uses this so an untracked scratch file or an
/// unstaged edit cannot falsely block an unrelated `git commit`.
pub fn get_git_diff_with_mode(
    repo_root: &Path,
    branch: Option<&str>,
    staged_only: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    let root = resolve_repo_root(repo_root);
    let mut combined = String::new();

    // Pre-commit audits only what is actually being committed.
    if staged_only {
        combined.push_str(&git_diff_output(&root, &["--cached"]).unwrap_or_default());
        return Ok(combined);
    }

    if let Some(target_branch) = branch {
        let verified = Command::new("git")
            .args(["rev-parse", "--verify", target_branch])
            .current_dir(&root)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false);
        if !verified {
            return Err(format!("Git branch or revision '{}' not found", target_branch).into());
        }

        let range = format!("{}...HEAD", target_branch);
        combined.push_str(&git_diff_output(&root, &[&range]).unwrap_or_default());
    }

    // Staged index vs HEAD, then working tree vs index.
    combined.push_str(&git_diff_output(&root, &["--cached"]).unwrap_or_default());
    combined.push_str(&git_diff_output(&root, &[]).unwrap_or_default());

    // Untracked files never appear in `git diff`; audit them explicitly.
    if let Ok(output) = Command::new("git")
        .args(["ls-files", "--others", "--exclude-standard"])
        .current_dir(&root)
        .output()
    {
        if output.status.success() {
            for line in String::from_utf8_lossy(&output.stdout).lines() {
                let rel = line.trim();
                if rel.is_empty()
                    || rel.starts_with(".k0maru/")
                    || rel.contains("/.k0maru/")
                    || rel.ends_with("cache.sqlite")
                {
                    continue;
                }

                let full_path = root.join(rel);
                match fs::metadata(&full_path) {
                    Ok(metadata) if metadata.is_file() && metadata.len() <= MAX_UNTRACKED_BYTES => {
                    }
                    _ => continue,
                }

                // Read bytes and decode lossily: a NUL byte must not make an
                // untracked file invisible to the audit.
                let bytes = match fs::read(&full_path) {
                    Ok(bytes) => bytes,
                    Err(_) => continue,
                };
                let content = String::from_utf8_lossy(&bytes);
                let content = content.trim_start_matches('\u{feff}');

                let lines: Vec<&str> = content.lines().collect();
                if lines.is_empty() {
                    continue;
                }

                combined.push_str(&format!(
                    "\ndiff --git a/{rel} b/{rel}\nnew file mode 100644\n--- /dev/null\n+++ b/{rel}\n@@ -0,0 +1,{} @@\n",
                    lines.len()
                ));
                for line in lines {
                    combined.push('+');
                    combined.push_str(line);
                    combined.push('\n');
                }
            }
        }
    }

    Ok(combined)
}

/// Builds the "does this file still satisfy the invariant?" resolver used by
/// diff audits.
///
/// The staged index is consulted first (a staged deletion must fail closed even
/// when the working tree still holds the old content), then the working tree,
/// with relative paths anchored at the repository root.
pub fn repo_content_reader(root: &Path) -> impl Fn(&Path, &dyn Fn(&str) -> bool) -> bool + '_ {
    move |rel_path: &Path, pred: &dyn Fn(&str) -> bool| {
        let rel_str = rel_path.to_string_lossy().replace('\\', "/");

        if let Ok(output) = Command::new("git")
            .args(["show", &format!(":{rel_str}")])
            .current_dir(root)
            .output()
        {
            if output.status.success() {
                let staged_content = String::from_utf8_lossy(&output.stdout).to_string();
                if !pred(&staged_content) {
                    return false;
                }
            }
        }

        let full_path = if rel_path.is_absolute() {
            rel_path.to_path_buf()
        } else {
            root.join(rel_path)
        };

        if full_path.exists() {
            return fs::read_to_string(&full_path)
                .map(|content| pred(&content))
                .unwrap_or(false);
        }

        false
    }
}

/// Executes `k0maru guard check`.
///
/// In `--claude-stop-hook` mode this behaves like a Claude Code `Stop` hook:
/// a passing audit exits silently, a failing audit writes the report to stderr
/// and exits 2 (the code Claude Code feeds back to the agent). The hook payload
/// on stdin is read only to honor `stop_hook_active`, so a continuation can
/// never loop forever.
pub fn run_guard_check(
    args: GuardCheckArgs,
    vault_path: &Path,
) -> Result<GuardReport, Box<dyn std::error::Error>> {
    let wall_start = std::time::Instant::now();

    // Stop-hook payloads arrive as JSON on stdin, never as a unified diff.
    let hook_payload: Option<String> = if args.claude_stop_hook && !std::io::stdin().is_terminal() {
        let mut payload = String::new();
        match std::io::stdin().read_to_string(&mut payload) {
            Ok(_) => Some(payload),
            Err(_) => None,
        }
    } else {
        None
    };

    if let Some(payload) = hook_payload.as_deref() {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(payload) {
            if value.get("stop_hook_active").and_then(|v| v.as_bool()) == Some(true) {
                // Already continuing because of this hook: exit 0 silently.
                return Ok(GuardReport::new(0, 0, Vec::new(), 0));
            }
        }
    }

    if !vault_path.exists() {
        if args.claude_stop_hook {
            // An uninitialized repository must never trap the agent.
            return Ok(GuardReport::new(0, 0, Vec::new(), 0));
        }
        return Err(format!(
            "[FAIL] Vault path does not exist: '{}'. Run 'k0maru init --install' to initialize this repository.",
            vault_path.display()
        )
        .into());
    }

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let root = resolve_repo_root(&cwd);
    let rules = resolve_guard_rules_with_repo(vault_path, args.project.as_deref(), Some(&root))?;

    let mut report = if !args.files.is_empty() {
        let mut file_tuples = Vec::new();
        for file in &args.files {
            if !file.exists() {
                return Err(format!("Target file does not exist: '{}'", file.display()).into());
            }
            let content = fs::read_to_string(file)?;
            file_tuples.push((file.clone(), content));
        }
        GuardEngine::check_files(&file_tuples, &rules)
    } else {
        let diff_text = if hook_payload.is_some() {
            get_git_diff_with_mode(&root, args.branch.as_deref(), args.staged)?
        } else if !std::io::stdin().is_terminal() {
            let mut stdin_str = String::new();
            std::io::stdin().read_to_string(&mut stdin_str)?;
            if !stdin_str.trim().is_empty() {
                stdin_str
            } else {
                get_git_diff_with_mode(&root, args.branch.as_deref(), args.staged)?
            }
        } else {
            get_git_diff_with_mode(&root, args.branch.as_deref(), args.staged)?
        };

        // Resolve "does this file still satisfy the invariant?" against the
        // staged index first (a staged deletion must fail closed) and then the
        // working tree, always relative to the repository root.
        GuardEngine::check_diff_with_reader(&diff_text, &rules, repo_content_reader(&root))
    };

    if args.strict && !report.violations.is_empty() {
        report.is_passed = false;
    }

    // Report end-to-end wall time: rule resolution + git diff + engine audit.
    report.duration_ms = wall_start.elapsed().as_millis();

    if args.claude_stop_hook {
        if report.is_passed {
            return Ok(report);
        }
        // Exit code 2 is Claude Code's blocking hook contract: stderr is fed
        // straight back to the agent as the reason the turn cannot end.
        eprintln!("{}", format_guard_report(&report));
        std::process::exit(2);
    }

    if !report.is_passed && !args.json {
        // Surface the same recovery protocol on both streams so pipelines that
        // only capture stdout or only capture stderr both see it.
        println!("{}", format_guard_report(&report));
        if let Some(protocol) = self_heal_protocol(&report) {
            eprintln!("{}", protocol);
        }
        std::process::exit(1);
    }

    Ok(report)
}

/// Resolves the directory Git actually executes hooks from, honoring
/// `core.hooksPath` (Husky / worktrees) and `git rev-parse --git-path hooks`.
fn resolve_hooks_dir(repo_dir: &Path) -> PathBuf {
    let configured = Command::new("git")
        .args(["config", "--get", "core.hooksPath"])
        .current_dir(repo_dir)
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .filter(|value| !value.is_empty());

    if let Some(hooks_path) = configured {
        let path = PathBuf::from(hooks_path);
        return if path.is_absolute() {
            path
        } else {
            repo_dir.join(path)
        };
    }

    if let Ok(out) = Command::new("git")
        .args(["rev-parse", "--git-path", "hooks"])
        .current_dir(repo_dir)
        .output()
    {
        if out.status.success() {
            let resolved = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !resolved.is_empty() {
                let path = PathBuf::from(resolved);
                return if path.is_absolute() {
                    path
                } else {
                    repo_dir.join(path)
                };
            }
        }
    }

    repo_dir.join(".git").join("hooks")
}

/// Canonicalizes `path`, resolving the nearest existing ancestor when the leaf
/// does not exist yet (e.g. an uninitialized `.k0maru/wiki`).
///
/// This keeps generated hook paths stable across macOS `/var` -> `/private/var`
/// symlinks without requiring the target directory to exist.
pub(crate) fn canonicalize_best_effort(path: &Path) -> PathBuf {
    if let Ok(canonical) = path.canonicalize() {
        return canonical;
    }

    let mut missing: Vec<std::ffi::OsString> = Vec::new();
    let mut current = path;
    loop {
        let Some(parent) = current.parent() else {
            return path.to_path_buf();
        };
        if let Some(name) = current.file_name() {
            missing.push(name.to_os_string());
        }
        if let Ok(canonical_parent) = parent.canonicalize() {
            let mut resolved = canonical_parent;
            for name in missing.iter().rev() {
                resolved.push(name);
            }
            return resolved;
        }
        current = parent;
    }
}

/// Renders the `--vault "<path>"` argument for a generated Git hook.
///
/// A vault living inside the repository is referenced through `$REPO_ROOT`
/// (resolved at hook runtime) so moving or cloning the repository never leaves
/// a machine-specific `/Users/...` or `/private/tmp/...` path baked into
/// `.git/hooks`. Only a vault outside the repository keeps an absolute path.
fn hook_vault_arg(vault: Option<&Path>, repo_dir: &Path) -> String {
    let Some(vault) = vault else {
        return String::new();
    };

    let canonical_repo = canonicalize_best_effort(repo_dir);
    let canonical_vault = canonicalize_best_effort(vault);

    let relative = canonical_vault
        .strip_prefix(&canonical_repo)
        .ok()
        .or_else(|| vault.strip_prefix(repo_dir).ok());

    match relative {
        Some(relative) if !relative.as_os_str().is_empty() => {
            let slashed = relative.to_string_lossy().replace('\\', "/");
            format!(" --vault \"$REPO_ROOT/{}\"", slashed.trim_matches('/'))
        }
        _ => format!(" --vault \"{}\"", vault.display()),
    }
}

/// Resolves the `k0maru` binary at hook runtime, with a hard-coded fallback to
/// the binary that installed the hook (PATH may not include it).
fn hook_binary_resolver(exe: Option<&str>) -> String {
    let exe_fallback = exe
        .filter(|path| !path.is_empty())
        .map(|path| format!("elif [ -x \"{path}\" ]; then\n    K0MARU_BIN=\"{path}\"\n"))
        .unwrap_or_default();

    format!(
        r#"if command -v k0maru >/dev/null 2>&1; then
    K0MARU_BIN="$(command -v k0maru)"
elif [ -x "$HOME/.local/bin/k0maru" ]; then
    K0MARU_BIN="$HOME/.local/bin/k0maru"
elif [ -x "$HOME/.cargo/bin/k0maru" ]; then
    K0MARU_BIN="$HOME/.cargo/bin/k0maru"
{exe_fallback}else
    K0MARU_BIN=""
fi
"#,
        exe_fallback = exe_fallback
    )
}

/// Builds the fail-closed pre-commit guard snippet.
///
/// The snippet aborts the commit when the audit fails (`|| exit 1`) and also
/// when no `k0maru` binary can be located, unless `K0MARU_GUARD_OPTIONAL=1`.
fn build_guard_sentinel(vault: Option<&Path>, repo_dir: &Path) -> String {
    let vault_arg = hook_vault_arg(vault, repo_dir);

    // Fall back to the binary that installed the hook: PATH may not include the
    // non-standard location the user is currently running from.
    let exe_fallback = std::env::current_exe()
        .ok()
        .map(|path| path.display().to_string());

    format!(
        r#"# --- k0maru guard sentinel ---
# k0maru guard check --diff --staged (fail-closed)
REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
{binary_resolver}
if [ -z "$K0MARU_BIN" ]; then
    if [ "${{K0MARU_GUARD_OPTIONAL:-0}}" = "1" ]; then
        echo "k0maru not found; K0MARU_GUARD_OPTIONAL=1 so the architectural guard check is skipped."
    else
        echo "ERROR: k0maru guard binary not found (PATH, ~/.local/bin, ~/.cargo/bin)." >&2
        echo "       Install k0maru, or set K0MARU_GUARD_OPTIONAL=1 to skip the guard." >&2
        exit 1
    fi
else
    "$K0MARU_BIN" guard check --diff --staged{vault_arg} || exit 1
fi
# --- end k0maru guard sentinel ---"#,
        binary_resolver = hook_binary_resolver(exe_fallback.as_deref()),
        vault_arg = vault_arg
    )
}

/// Builds the `post-commit` memory-capture snippet.
///
/// It refreshes the vault cache after every commit and, for conventional
/// commit messages (`feat:`, `fix(scope):`, `adr!:` ...), records a log note.
/// Every command is best-effort (`|| true`) so a vault problem can never fail
/// the commit, and rebase/merge replays are skipped so synthetic commits are
/// not captured.
fn build_post_commit_sentinel(vault: Option<&Path>, repo_dir: &Path) -> String {
    let vault_path_expr = match hook_vault_arg(vault, repo_dir) {
        // `--vault "$REPO_ROOT/.k0maru/wiki"` -> `"$REPO_ROOT/.k0maru/wiki"`
        arg if arg.starts_with(" --vault ") => arg[" --vault ".len()..].to_string(),
        _ => "\"$REPO_ROOT/.k0maru/wiki\"".to_string(),
    };

    let exe_fallback = std::env::current_exe()
        .ok()
        .map(|path| path.display().to_string());

    format!(
        r#"# --- k0maru post-commit sentinel ---
REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
{binary_resolver}
VAULT_PATH={vault_path_expr}

if [ -n "$K0MARU_BIN" ]; then
    "$K0MARU_BIN" sync --vault "$VAULT_PATH" >/dev/null 2>&1 || true

    if [ ! -d "$REPO_ROOT/.git/rebase-merge" ] && [ ! -d "$REPO_ROOT/.git/rebase-apply" ]; then
        COMMIT_MSG="$(git log -1 --pretty=%s)"
        if printf '%s' "$COMMIT_MSG" | grep -Eq '^(feat|fix|refactor|perf|adr)(\(.+\))?!?:'; then
            "$K0MARU_BIN" flush --vault "$VAULT_PATH" --category log --title "Auto-Commit $(git rev-parse --short HEAD): $COMMIT_MSG" --content "$(git show --stat --oneline HEAD)" >/dev/null 2>&1 || true
            "$K0MARU_BIN" sync --vault "$VAULT_PATH" >/dev/null 2>&1 || true
        fi
    fi
fi
# --- end k0maru post-commit sentinel ---"#,
        binary_resolver = hook_binary_resolver(exe_fallback.as_deref()),
        vault_path_expr = vault_path_expr
    )
}

/// Sentinel markers identifying an already-installed k0maru hook block.
struct SentinelMarkers {
    start: &'static str,
    end: &'static str,
}

const GUARD_SENTINEL: SentinelMarkers = SentinelMarkers {
    start: "# --- k0maru guard sentinel ---",
    end: "# --- end k0maru guard sentinel ---",
};

const POST_COMMIT_SENTINEL: SentinelMarkers = SentinelMarkers {
    start: "# --- k0maru post-commit sentinel ---",
    end: "# --- end k0maru post-commit sentinel ---",
};

/// Writes (or upgrades in place) a sentinel block inside a Git hook script.
///
/// Idempotent: an identical block is a no-op, an older block is replaced, and
/// an existing hook keeps its own logic — the sentinel is inserted before a
/// trailing `exit 0` so it actually executes.
fn install_sentinel_block(
    hook_path: &Path,
    sentinel: &str,
    markers: &SentinelMarkers,
    header: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if hook_path.exists() {
        let existing = fs::read_to_string(hook_path)?;

        // The marker — not a textual mention inside an echo or comment — is the
        // source of truth for "already installed".
        if existing.contains(sentinel) {
            return Ok(());
        }
        if let Some(start) = existing.find(markers.start) {
            if let Some(end_rel) = existing[start..].find(markers.end) {
                // Upgrade an older sentinel in place (`--diff` -> `--diff --staged`).
                let end = start + end_rel + markers.end.len();
                let rewritten = format!("{}{}{}", &existing[..start], sentinel, &existing[end..]);
                fs::write(hook_path, rewritten)?;
                return Ok(());
            }
        }

        // Insert before a trailing `exit 0` so the sentinel actually executes.
        let trimmed_end = existing.trim_end();
        let chained = match trimmed_end.strip_suffix("exit 0") {
            Some(prefix) if !prefix.trim().is_empty() => {
                format!("{}\n{}\nexit 0\n", prefix.trim_end(), sentinel)
            }
            _ => {
                let first_line = existing.lines().next().unwrap_or("");
                if first_line.starts_with("#!") {
                    let body = existing
                        .split_once('\n')
                        .map(|(_, rest)| rest.trim_end())
                        .unwrap_or("");
                    if body.is_empty() {
                        format!("{}\n\n{}\n", first_line, sentinel)
                    } else {
                        format!("{}\n\n{}\n\n{}\n", first_line, sentinel, body)
                    }
                } else {
                    format!("{}\n\n{}\n", trimmed_end, sentinel)
                }
            }
        };
        fs::write(hook_path, chained)?;
    } else {
        let script = format!("#!/bin/sh\n{}\n\n{}\n", header, sentinel);
        fs::write(hook_path, script)?;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = fs::metadata(hook_path) {
            let mut perms = metadata.permissions();
            perms.set_mode(0o755);
            let _ = fs::set_permissions(hook_path, perms);
        }
    }

    Ok(())
}

/// Installs the `k0maru guard check --diff` pre-commit hook into the repo hooks directory.
pub fn run_guard_install(args: GuardInstallArgs) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let requested = args.path.clone().unwrap_or_else(|| PathBuf::from("."));
    let repo_dir = resolve_repo_root(&requested);
    let git_dir = repo_dir.join(".git");

    if !git_dir.exists() {
        return Err(format!(
            "Not a git repository: '{}' does not contain a .git directory",
            repo_dir.display()
        )
        .into());
    }

    // Bind the hook to a concrete vault so commits cannot audit an empty
    // rule set merely because the committing shell had a different CWD.
    let vault_for_hook: Option<PathBuf> = args.vault.clone().or_else(|| {
        let candidate = repo_dir.join(".k0maru").join("wiki");
        candidate.is_dir().then_some(candidate)
    });

    let hooks_dir = resolve_hooks_dir(&repo_dir);
    fs::create_dir_all(&hooks_dir)?;

    let hook_path = hooks_dir.join(&args.hook);
    let sentinel = build_guard_sentinel(vault_for_hook.as_deref(), &repo_dir);

    if hook_path.exists() {
        let existing = fs::read_to_string(&hook_path)?;

        // The marker — not a textual mention of `k0maru guard check` inside an
        // echo or comment — is the source of truth for "already installed".
        if existing.contains(sentinel.as_str()) {
            // The pre-commit guard is already current; still keep the
            // post-commit capture hook in sync below.
            if args.hook == "pre-commit" {
                install_post_commit_hook(&hooks_dir, vault_for_hook.as_deref(), &repo_dir)?;
            }
            return Ok(hook_path);
        }
    }

    install_sentinel_block(
        &hook_path,
        &sentinel,
        &GUARD_SENTINEL,
        "# k0maru guard pre-commit hook\n# Automatically generated by `k0maru guard install`",
    )?;

    if args.hook == "pre-commit" {
        install_post_commit_hook(&hooks_dir, vault_for_hook.as_deref(), &repo_dir)?;
    }

    Ok(hook_path)
}

/// Installs the best-effort `post-commit` capture hook next to the guard hook.
fn install_post_commit_hook(
    hooks_dir: &Path,
    vault: Option<&Path>,
    repo_dir: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let hook_path = hooks_dir.join("post-commit");
    let sentinel = build_post_commit_sentinel(vault, repo_dir);
    install_sentinel_block(
        &hook_path,
        &sentinel,
        &POST_COMMIT_SENTINEL,
        "# k0maru post-commit memory capture hook\n# Automatically generated by `k0maru guard install`",
    )?;
    Ok(hook_path)
}

/// Formats a GuardReport into a human-readable terminal presentation.
pub fn format_guard_report(report: &GuardReport) -> String {
    let mut out = String::new();

    let deny_count = report
        .violations
        .iter()
        .filter(|v| v.severity == RuleSeverity::Deny)
        .count();
    let warn_count = report
        .violations
        .iter()
        .filter(|v| v.severity == RuleSeverity::Warn)
        .count();

    if report.is_passed && report.violations.is_empty() {
        out.push_str("🛡️  k0maru guard: All architectural invariants passed!\n");
        out.push_str("--------------------------------------------------\n");
        out.push_str(&format!(
            "Checked: {} files, {} diff hunks in {}ms\n",
            report.checked_files, report.checked_diff_hunks, report.duration_ms
        ));
        out.push_str("Violations: 0\n");
        out.push_str("--------------------------------------------------\n");
        out.push_str("✅ Ready to commit.\n");
        return out;
    }

    if report.is_passed {
        out.push_str("🛡️  k0maru guard: Invariant checks completed with warnings.\n");
    } else {
        out.push_str("🛡️  k0maru guard: Invariant violations detected!\n");
    }

    out.push_str("--------------------------------------------------\n");
    out.push_str(&format!(
        "Checked: {} files, {} diff hunks in {}ms\n",
        report.checked_files, report.checked_diff_hunks, report.duration_ms
    ));
    out.push_str(&format!(
        "Violations: {} ({} deny, {} warn)\n",
        report.violations.len(),
        deny_count,
        warn_count
    ));
    out.push_str("--------------------------------------------------\n\n");

    for v in &report.violations {
        let sev_tag = match v.severity {
            RuleSeverity::Deny => "[DENY]",
            RuleSeverity::Warn => "[WARN]",
        };
        out.push_str(&format!("{} {}: {}\n", sev_tag, v.rule_id, v.rule_title));

        if let Some(line) = v.line_number {
            out.push_str(&format!("  File: {}:{}\n", v.file_path, line));
        } else {
            out.push_str(&format!("  File: {}\n", v.file_path));
        }

        if let Some(content) = &v.matched_content {
            out.push_str(&format!("  Matched: `{}`\n", content.trim()));
        }

        out.push_str(&format!("  Source:  {}\n", v.source_note));

        if !v.remediation.is_empty() {
            out.push_str(&format!("  Remedy:  {}\n", v.remediation));
        }
        out.push('\n');
    }

    out.push_str("--------------------------------------------------\n");
    if report.is_passed {
        out.push_str("⚠️  Passed with advisory warnings.\n");
    } else {
        out.push_str(&format!(
            "❌ Commit blocked: Fix {} blocking architectural invariant(s) before proceeding.\n",
            deny_count
        ));
    }

    if let Some(protocol) = self_heal_protocol(report) {
        out.push('\n');
        out.push_str(&protocol);
    }

    out
}

/// Builds the copy-pasteable recovery protocol appended to a failing report.
///
/// Both the human CLI and the Claude Code `Stop` hook surface this block, so an
/// agent can either fix the code or legitimately replace the ADR in a single
/// commit instead of disabling the check (`--no-verify`).
pub fn self_heal_protocol(report: &GuardReport) -> Option<String> {
    if report.is_passed {
        return None;
    }

    let source_note = report
        .violations
        .iter()
        .find(|violation| violation.severity == RuleSeverity::Deny)
        .or_else(|| report.violations.first())
        .map(|violation| violation.source_note.clone())
        .filter(|note| !note.trim().is_empty())
        .unwrap_or_else(|| "<SOURCE_NOTE>".to_string());

    Some(format!(
        r#"[AGENT_SELF_HEAL_PROTOCOL]
1. Fix the code: Refactor the violating line(s) to satisfy the Remedy above, then re-run `k0maru guard check --diff`.
2. Legitimate Architecture Replacement (ONLY if the user asked to replace this ADR):
   k0maru flush --category decision --title "ADR-New: <Title>" --supersedes "{source_note}" --content $'## Non-Negotiable Invariants\n- [<new-rule-id>] (deny) [src/**/*] forbidden: "<pattern>" | remediation: "<remedy>"'
   Then stage the updated vault (`git add .k0maru/wiki`) in the same commit. Never use `--no-verify`.
"#,
        source_note = source_note
    ))
}
