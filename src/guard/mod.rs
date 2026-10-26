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

    /// Target project name to filter ADR invariants for
    #[arg(short, long, value_name = "PROJECT")]
    pub project: Option<String>,

    /// Target Git base branch for comparison (e.g. 'main', 'dev')
    #[arg(short, long, value_name = "BRANCH")]
    pub branch: Option<String>,

    /// Output structured JSON report
    #[arg(long)]
    pub json: bool,

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
    // a `project: alpha` card never leaks into the `beta` repository.
    let active_project_slug: Option<String> = if let Some(filter) = project_filter {
        Some(crate::convention::slugify(filter))
    } else if let Some(root) = repo_root {
        root.file_name()
            .and_then(|name| name.to_str())
            .map(crate::convention::slugify)
            .filter(|root_slug| {
                all_vault_projects.contains(root_slug)
                    || (!root_slug.starts_with("tmp") && all_vault_projects.len() > 1)
            })
    } else {
        None
    };

    for (_relative_path, doc, doc_projects) in scanned {
        if let Some(target_slug) = &active_project_slug {
            if !doc_projects.is_empty() && !doc_projects.iter().any(|p| p == target_slug) {
                continue;
            }
        }

        for rule in extract_rules_from_document(&doc) {
            if seen.insert((rule.id.clone(), format!("{:?}", rule.condition))) {
                rules.push(rule);
            }
        }
    }

    // 3. Same-commit supersession defense: a changeset that flips an active ADR
    // to `status: superseded` while also introducing the violation must still
    // be audited against the ADR that was active at HEAD.
    if let Some(root) = repo_root {
        collect_head_active_rules(root, vault_path, &mut seen, &mut rules);
    }

    Ok(rules)
}

/// Re-adds invariants declared by documents that were active at `HEAD` but are
/// no longer active (or were deleted) in the working tree.
///
/// Without this, `status: active -> superseded` in the very same commit as a
/// violating change would silently disable the rule being violated.
fn collect_head_active_rules(
    repo_root: &Path,
    vault_path: &Path,
    seen: &mut HashSet<(String, String)>,
    rules: &mut Vec<GuardRule>,
) {
    // Only notes inside the vault can own invariants; map the vault onto the
    // repository so `git show HEAD:<path>` paths can be filtered.
    let canonical_vault = vault_path
        .canonicalize()
        .unwrap_or_else(|_| vault_path.to_path_buf());
    let canonical_root = repo_root
        .canonicalize()
        .unwrap_or_else(|_| repo_root.to_path_buf());
    let vault_prefix = match canonical_vault.strip_prefix(&canonical_root) {
        Ok(prefix) => prefix.to_path_buf(),
        Err(_) => return,
    };

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
            if rel.is_empty() || !rel.ends_with(".md") {
                continue;
            }
            if !Path::new(rel).starts_with(&vault_prefix) {
                continue;
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
        let (frontmatter, body) = k0maru_core::parser::parse_frontmatter(&head_content);
        let head_doc = crate::Document {
            path: PathBuf::from(&rel_path),
            title: String::new(),
            hierarchy: crate::HierarchyLevel::L3Evergreen,
            frontmatter,
            links: Vec::new(),
            tags: Vec::new(),
            content_hash: String::new(),
            mtime: 0,
            body: body.to_string(),
        };

        if !head_doc.is_active() {
            continue;
        }

        // Still active in the working tree: its rules were already collected.
        let working_copy = repo_root.join(&rel_path);
        if working_copy.is_file() {
            if let Ok(current_content) = fs::read_to_string(&working_copy) {
                let (current_fm, current_body) =
                    k0maru_core::parser::parse_frontmatter(&current_content);
                let current_doc = crate::Document {
                    path: PathBuf::from(&rel_path),
                    title: String::new(),
                    hierarchy: crate::HierarchyLevel::L3Evergreen,
                    frontmatter: current_fm,
                    links: Vec::new(),
                    tags: Vec::new(),
                    content_hash: String::new(),
                    mtime: 0,
                    body: current_body.to_string(),
                };
                if current_doc.is_active() {
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

/// Runs `git diff` with extra arguments, returning stdout on success.
fn git_diff_output(repo_root: &Path, extra_args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("diff")
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
    let root = resolve_repo_root(repo_root);
    let mut combined = String::new();

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
        combined.push_str(&git_diff_output(&root, &["--no-ext-diff", &range]).unwrap_or_default());
    }

    // Staged index vs HEAD, then working tree vs index.
    combined.push_str(&git_diff_output(&root, &["--cached", "--no-ext-diff"]).unwrap_or_default());
    combined.push_str(&git_diff_output(&root, &["--no-ext-diff"]).unwrap_or_default());

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

                let content = match fs::read_to_string(&full_path) {
                    Ok(content) => content,
                    Err(_) => continue,
                };

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
pub fn run_guard_check(
    args: GuardCheckArgs,
    vault_path: &Path,
) -> Result<GuardReport, Box<dyn std::error::Error>> {
    let wall_start = std::time::Instant::now();

    if !vault_path.exists() {
        return Err(format!("Vault path does not exist: '{}'", vault_path.display()).into());
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
        let diff_text = if !std::io::stdin().is_terminal() {
            let mut stdin_str = String::new();
            std::io::stdin().read_to_string(&mut stdin_str)?;
            if !stdin_str.trim().is_empty() {
                stdin_str
            } else {
                get_git_diff(&root, args.branch.as_deref())?
            }
        } else {
            get_git_diff(&root, args.branch.as_deref())?
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

/// Builds the fail-closed pre-commit guard snippet.
///
/// The snippet aborts the commit when the audit fails (`|| exit 1`) and also
/// when no `k0maru` binary can be located, unless `K0MARU_GUARD_OPTIONAL=1`.
fn build_guard_sentinel(vault: Option<&Path>) -> String {
    let vault_arg = match vault {
        Some(path) => format!(" --vault \"{}\"", path.display()),
        None => String::new(),
    };

    format!(
        r#"# --- k0maru guard sentinel ---
# k0maru guard check --diff (fail-closed)
if command -v k0maru >/dev/null 2>&1; then
    K0MARU_BIN="$(command -v k0maru)"
elif [ -x "$HOME/.local/bin/k0maru" ]; then
    K0MARU_BIN="$HOME/.local/bin/k0maru"
elif [ -x "$HOME/.cargo/bin/k0maru" ]; then
    K0MARU_BIN="$HOME/.cargo/bin/k0maru"
else
    K0MARU_BIN=""
fi

if [ -z "$K0MARU_BIN" ]; then
    if [ "${{K0MARU_GUARD_OPTIONAL:-0}}" = "1" ]; then
        echo "k0maru not found; K0MARU_GUARD_OPTIONAL=1 so the architectural guard check is skipped."
    else
        echo "ERROR: k0maru guard binary not found (PATH, ~/.local/bin, ~/.cargo/bin)." >&2
        echo "       Install k0maru, or set K0MARU_GUARD_OPTIONAL=1 to skip the guard." >&2
        exit 1
    fi
else
    "$K0MARU_BIN" guard check --diff{vault_arg} || exit 1
fi
# --- end k0maru guard sentinel ---"#
    )
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
    let sentinel = build_guard_sentinel(vault_for_hook.as_deref());

    if hook_path.exists() {
        let existing = fs::read_to_string(&hook_path)?;
        let already_installed = existing.lines().any(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with('#') && trimmed.contains("k0maru guard check")
        });
        if already_installed {
            return Ok(hook_path);
        }

        // Insert before a trailing `exit 0` so the guard actually executes.
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
        fs::write(&hook_path, chained)?;
    } else {
        let script = format!(
            "#!/bin/sh\n# k0maru guard pre-commit hook\n# Automatically generated by `k0maru guard install`\n\n{}\n",
            sentinel
        );
        fs::write(&hook_path, script)?;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = fs::metadata(&hook_path) {
            let mut perms = metadata.permissions();
            perms.set_mode(0o755);
            let _ = fs::set_permissions(&hook_path, perms);
        }
    }

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

    out
}
