use std::collections::HashSet;
use std::fs;
use std::io::{IsTerminal, Read};
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::{Args, Subcommand};
use k0maru_core::adapters::GenericWikiAdapter;
use k0maru_core::core::traits::VaultAdapter;
use k0maru_core::guard::extractor::{extract_rules_from_document, extract_rules_from_vault};
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
}

/// Resolves active GuardRules from vault configuration files and document notes.
pub fn resolve_guard_rules(
    vault_path: &Path,
    project_filter: Option<&str>,
) -> Result<Vec<GuardRule>, Box<dyn std::error::Error>> {
    let mut rules = Vec::new();
    let mut seen = HashSet::new();

    // 1. Vault root config file (.k0maru/guard.toml, guard.yaml, etc.)
    if let Ok(config_rules) = extract_rules_from_vault(vault_path) {
        for r in config_rules {
            if seen.insert((r.id.clone(), format!("{:?}", r.condition))) {
                rules.push(r);
            }
        }
    }

    // Also check current working directory if different from vault_path
    if let Ok(cwd) = std::env::current_dir() {
        if cwd != vault_path {
            if let Ok(config_rules) = extract_rules_from_vault(&cwd) {
                for r in config_rules {
                    if seen.insert((r.id.clone(), format!("{:?}", r.condition))) {
                        rules.push(r);
                    }
                }
            }
        }
    }

    // 2. Scan vault notes for living ADR invariants (topology-agnostic)
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

                let in_project_dir = segments
                    .iter()
                    .any(|segment| matches!(*segment, "10_projects" | "projects" | "1_projects"));
                let is_project_note = in_project_dir
                    || doc
                        .tags
                        .iter()
                        .any(|tag| tag.eq_ignore_ascii_case("project"))
                    || doc
                        .frontmatter
                        .get("type")
                        .and_then(|value| value.as_str())
                        .map(|value| value.eq_ignore_ascii_case("project"))
                        .unwrap_or(false);

                // Project notes only contribute when they match the active filter.
                if let Some(filter) = project_filter {
                    if is_project_note {
                        let stem = relative_path
                            .file_stem()
                            .and_then(|stem| stem.to_str())
                            .unwrap_or_default()
                            .to_lowercase();
                        if !stem.contains(&filter.to_lowercase()) {
                            continue;
                        }
                    }
                }

                for rule in extract_rules_from_document(&doc) {
                    if seen.insert((rule.id.clone(), format!("{:?}", rule.condition))) {
                        rules.push(rule);
                    }
                }
            }
        }
    }

    Ok(rules)
}

/// Obtains git diff output for the current repository.
pub fn get_git_diff(
    repo_root: &Path,
    branch: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut diff_str = String::new();

    if let Some(target_branch) = branch {
        let triple_dot = format!("{}...HEAD", target_branch);
        let output = Command::new("git")
            .arg("diff")
            .arg(&triple_dot)
            .current_dir(repo_root)
            .output();

        if let Ok(out) = output {
            if out.status.success() {
                diff_str = String::from_utf8_lossy(&out.stdout).to_string();
            }
        }
    }

    if diff_str.is_empty() {
        // Try `git diff HEAD`
        let head_output = Command::new("git")
            .arg("diff")
            .arg("HEAD")
            .current_dir(repo_root)
            .output();

        if let Ok(out) = head_output {
            if out.status.success() {
                diff_str = String::from_utf8_lossy(&out.stdout).to_string();
            }
        }
    }

    // Fallback: combine unstaged and staged
    if diff_str.is_empty() {
        let mut combined = String::new();
        if let Ok(out) = Command::new("git")
            .arg("diff")
            .current_dir(repo_root)
            .output()
        {
            if out.status.success() {
                combined.push_str(&String::from_utf8_lossy(&out.stdout));
            }
        }

        if let Ok(out) = Command::new("git")
            .arg("diff")
            .arg("--cached")
            .current_dir(repo_root)
            .output()
        {
            if out.status.success() {
                combined.push_str(&String::from_utf8_lossy(&out.stdout));
            }
        }

        diff_str = combined;
    }

    Ok(diff_str)
}

/// Executes `k0maru guard check`.
pub fn run_guard_check(
    args: GuardCheckArgs,
    vault_path: &Path,
) -> Result<GuardReport, Box<dyn std::error::Error>> {
    let rules = resolve_guard_rules(vault_path, args.project.as_deref())?;

    let mut report = if !args.files.is_empty() {
        let mut file_tuples = Vec::new();
        for file in &args.files {
            if file.exists() {
                let content = fs::read_to_string(file)?;
                file_tuples.push((file.clone(), content));
            }
        }
        GuardEngine::check_files(&file_tuples, &rules)
    } else {
        let diff_text = if !std::io::stdin().is_terminal() {
            let mut stdin_str = String::new();
            std::io::stdin().read_to_string(&mut stdin_str)?;
            if !stdin_str.trim().is_empty() {
                stdin_str
            } else {
                let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                get_git_diff(&cwd, args.branch.as_deref())?
            }
        } else {
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            get_git_diff(&cwd, args.branch.as_deref())?
        };

        GuardEngine::check_diff(&diff_text, &rules)
    };

    if args.strict && !report.violations.is_empty() {
        report.is_passed = false;
    }

    Ok(report)
}

/// Installs the `k0maru guard check --diff` pre-commit hook into `.git/hooks/`.
pub fn run_guard_install(args: GuardInstallArgs) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let repo_dir = args.path.unwrap_or_else(|| PathBuf::from("."));
    let git_dir = repo_dir.join(".git");

    if !git_dir.exists() {
        return Err(format!(
            "Not a git repository: '{}' does not contain a .git directory",
            repo_dir.display()
        )
        .into());
    }

    let hooks_dir = git_dir.join("hooks");
    fs::create_dir_all(&hooks_dir)?;

    let hook_path = hooks_dir.join(&args.hook);

    if hook_path.exists() {
        let existing = fs::read_to_string(&hook_path)?;
        if existing.contains("k0maru guard check") {
            return Ok(hook_path);
        }

        let chained = format!(
            "{}\n\n# --- k0maru guard sentinel ---\nif command -v k0maru >/dev/null 2>&1; then\n    k0maru guard check --diff\nfi\n",
            existing.trim_end()
        );
        fs::write(&hook_path, chained)?;
    } else {
        let script = r#"#!/bin/sh
# k0maru guard pre-commit hook
# Automatically generated by `k0maru guard install`

if command -v k0maru >/dev/null 2>&1; then
    k0maru guard check --diff
else
    echo "⚠️  k0maru not found in PATH; skipping architectural guard check."
fi
"#;
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
