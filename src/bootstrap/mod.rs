//! Zero-vault cold-start bootstrap engine for K0maru Agent Memory Hub.
//!
//! Scans unconfigured Git repositories, synthesizes an initial LLM-Wiki topology
//! (`10_Projects/`, `20_Cards/adr-001-...`, `01_AI_Logs/`), and indexes the resulting
//! knowledge artifacts into `.k0maru/cache.sqlite` for sub-5ms context loadouts and
//! instant Living ADR guard sentinel checks.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::adapters::ObsidianAdapter;
use crate::convention::slugify;
use crate::core::traits::CacheStorage;
use crate::scanner::IncrementalScanner;
use crate::storage::SqliteStorage;

/// Options controlling zero-vault cold-start initialization.
#[derive(Debug, Clone)]
pub struct BootstrapOptions {
    /// Target repository path (defaults to current working directory).
    pub repo_path: PathBuf,
    /// Custom target knowledge vault root. Defaults to `<repo_path>/.k0maru/wiki`.
    pub vault_path: Option<PathBuf>,
    /// Project name override (slugified for project filename).
    pub project_name: Option<String>,
    /// Whether to configure agent MCP clients and repo hooks (.git & .claude).
    pub install_clients: bool,
    /// Home directory override for client MCP installation.
    pub home_override: Option<PathBuf>,
}

/// Diagnostic report summarizing artifacts created and indexed during bootstrap.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapReport {
    pub repo_path: PathBuf,
    pub vault_path: PathBuf,
    pub project_slug: String,
    pub created_files: Vec<PathBuf>,
    pub indexed_docs: usize,
    pub guard_hook_installed: Option<PathBuf>,
    pub claude_hook_installed: Option<PathBuf>,
}

/// Detects the project slug from explicit override, package manifests, or directory name.
fn detect_project_slug(repo_path: &Path, explicit_override: Option<&str>) -> String {
    if let Some(name) = explicit_override {
        let trimmed = name.trim();
        if !trimmed.is_empty() {
            let s = slugify(trimmed);
            if !s.is_empty() {
                return s;
            }
        }
    }

    // 1. Cargo.toml
    let cargo_toml = repo_path.join("Cargo.toml");
    if cargo_toml.exists() {
        if let Ok(content) = fs::read_to_string(&cargo_toml) {
            let mut in_package = false;
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with('[') && trimmed.ends_with(']') {
                    in_package = trimmed == "[package]";
                } else if in_package && trimmed.starts_with("name") {
                    if let Some((_, val)) = trimmed.split_once('=') {
                        let name_val = val.trim().trim_matches('"').trim_matches('\'').trim();
                        if !name_val.is_empty() {
                            let s = slugify(name_val);
                            if !s.is_empty() {
                                return s;
                            }
                        }
                    }
                }
            }
        }
    }

    // 2. package.json
    let package_json = repo_path.join("package.json");
    if package_json.exists() {
        if let Ok(content) = fs::read_to_string(&package_json) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(name) = val.get("name").and_then(|v| v.as_str()) {
                    let clean = name.trim_start_matches('@').replace('/', "-");
                    let s = slugify(&clean);
                    if !s.is_empty() {
                        return s;
                    }
                }
            }
        }
    }

    // 3. pyproject.toml
    let pyproject = repo_path.join("pyproject.toml");
    if pyproject.exists() {
        if let Ok(content) = fs::read_to_string(&pyproject) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("name") {
                    if let Some((_, val)) = trimmed.split_once('=') {
                        let name_val = val.trim().trim_matches('"').trim_matches('\'').trim();
                        if !name_val.is_empty() {
                            let s = slugify(name_val);
                            if !s.is_empty() {
                                return s;
                            }
                        }
                    }
                }
            }
        }
    }

    // 4. go.mod
    let go_mod = repo_path.join("go.mod");
    if go_mod.exists() {
        if let Ok(content) = fs::read_to_string(&go_mod) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("module ") {
                    let mod_path = trimmed.strip_prefix("module ").unwrap().trim();
                    let file_part = mod_path.rsplit('/').next().unwrap_or(mod_path);
                    let s = slugify(file_part);
                    if !s.is_empty() {
                        return s;
                    }
                }
            }
        }
    }

    // 5. repo directory name
    if let Some(stem) = repo_path.file_name().and_then(|s| s.to_str()) {
        let s = slugify(stem);
        if !s.is_empty() {
            return s;
        }
    }

    "project".to_string()
}

/// Scans the first ~40 non-empty lines of README.md for project overview.
fn scan_readme_summary(repo_path: &Path) -> String {
    let candidates = [
        "README.md",
        "readme.md",
        "README",
        "readme.markdown",
        "docs/README.md",
    ];
    for name in &candidates {
        let path = repo_path.join(name);
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                let non_empty_lines: Vec<&str> = content
                    .lines()
                    .map(|l| l.trim())
                    .filter(|l| !l.is_empty())
                    .take(40)
                    .collect();
                if !non_empty_lines.is_empty() {
                    return non_empty_lines.join("\n\n");
                }
            }
        }
    }
    "Project codebase and operational assets.".to_string()
}

/// Detects tech stack signatures from manifests and key project files.
fn detect_tech_stack(repo_path: &Path) -> Vec<String> {
    let mut stack = Vec::new();
    if repo_path.join("Cargo.toml").exists() {
        stack.push("Rust (Cargo)".to_string());
    }
    if repo_path.join("package.json").exists() {
        if repo_path.join("tsconfig.json").exists() {
            stack.push("TypeScript / Node.js".to_string());
        } else {
            stack.push("JavaScript / Node.js".to_string());
        }
    }
    if repo_path.join("pyproject.toml").exists() || repo_path.join("requirements.txt").exists() {
        stack.push("Python".to_string());
    }
    if repo_path.join("go.mod").exists() {
        stack.push("Go".to_string());
    }
    if repo_path.join("pom.xml").exists() || repo_path.join("build.gradle").exists() {
        stack.push("Java / JVM".to_string());
    }
    if repo_path.join("CMakeLists.txt").exists() {
        stack.push("C / C++ (CMake)".to_string());
    }
    if stack.is_empty() {
        stack.push("Polyglot / Generic Codebase".to_string());
    }
    stack
}

/// Scans existing agent constitutions or instructions (AGENTS.md, CLAUDE.md, .cursorrules).
fn scan_agent_rules(repo_path: &Path) -> String {
    let candidates = [
        "AGENTS.md",
        "CLAUDE.md",
        ".cursorrules",
        ".github/copilot-instructions.md",
    ];
    let mut rules = Vec::new();
    for name in &candidates {
        let path = repo_path.join(name);
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                let lines: Vec<&str> = content
                    .lines()
                    .map(|l| l.trim())
                    .filter(|l| !l.is_empty())
                    .take(25)
                    .collect();
                if !lines.is_empty() {
                    rules.push(format!("### Source: {}\n{}", name, lines.join("\n")));
                }
            }
        }
    }
    if rules.is_empty() {
        "- Adhere to clean architecture and test-driven invariant verification.\n- Never bypass living architectural guardrails."
            .to_string()
    } else {
        rules.join("\n\n")
    }
}

/// Captures recent git commits for initial handoff milestones.
fn scan_recent_commits(repo_path: &Path) -> Vec<String> {
    if let Ok(output) = std::process::Command::new("git")
        .args([
            "-C",
            &repo_path.to_string_lossy(),
            "log",
            "-n",
            "5",
            "--oneline",
        ])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let commits: Vec<String> = stdout
                .lines()
                .map(|l| l.trim())
                .filter(|l| !l.is_empty())
                .map(|l| format!("- {}", l))
                .collect();
            if !commits.is_empty() {
                return commits;
            }
        }
    }
    vec!["- Initial repository bootstrap and memory architecture setup.".to_string()]
}

/// Runs zero-vault cold-start initialization on a repository.
pub fn run_bootstrap(
    options: BootstrapOptions,
) -> Result<BootstrapReport, Box<dyn std::error::Error>> {
    let repo_path = if let Ok(canon) = fs::canonicalize(&options.repo_path) {
        canon
    } else {
        options.repo_path.clone()
    };

    let vault_path = match &options.vault_path {
        Some(v) => {
            if let Ok(canon) = fs::canonicalize(v) {
                canon
            } else if v.is_absolute() {
                v.clone()
            } else {
                repo_path.join(v)
            }
        }
        None => repo_path.join(".k0maru").join("wiki"),
    };

    // Ensure wiki folders exist
    let projects_dir = vault_path.join("10_Projects");
    let cards_dir = vault_path.join("20_Cards");
    let logs_dir = vault_path.join("01_AI_Logs");

    fs::create_dir_all(&projects_dir)?;
    fs::create_dir_all(&cards_dir)?;
    fs::create_dir_all(&logs_dir)?;

    let project_slug = detect_project_slug(&repo_path, options.project_name.as_deref());
    let today = chrono::Local::now()
        .date_naive()
        .format("%Y-%m-%d")
        .to_string();

    let readme_summary = scan_readme_summary(&repo_path);
    let tech_stack = detect_tech_stack(&repo_path);
    let agent_rules = scan_agent_rules(&repo_path);
    let recent_commits = scan_recent_commits(&repo_path);

    let mut created_files = Vec::new();

    // 1. 10_Projects/<project_slug>.md
    let project_note_path = projects_dir.join(format!("{}.md", project_slug));
    if !project_note_path.exists() {
        let tech_stack_bullets: Vec<String> =
            tech_stack.iter().map(|s| format!("- {}", s)).collect();
        let project_content = format!(
            r#"---
title: "{title}"
date: "{date}"
status: active
tags:
  - type/project
  - bootstrapped
repo: "{repo}"
---

# {title}

> {scope}

## Project Overview
{readme}

## Tech Stack
{tech_stack}

## Core Architecture & Rules
{rules}

## Recent Milestones
{commits}

## Key Artifacts & Invariants
- Architecture Guardrails: [[20_Cards/adr-001-repository-guardrails]]
- Initial Bootstrap Log: [[01_AI_Logs/{date}-initial-bootstrap]]
"#,
            title = project_slug,
            date = today,
            repo = project_slug,
            scope = readme_summary
                .lines()
                .next()
                .unwrap_or("Core project repository"),
            readme = readme_summary,
            tech_stack = tech_stack_bullets.join("\n"),
            rules = agent_rules,
            commits = recent_commits.join("\n"),
        );
        fs::write(&project_note_path, project_content)?;
        created_files.push(project_note_path);
    }

    // 2. 20_Cards/adr-001-repository-guardrails.md
    let adr_path = cards_dir.join("adr-001-repository-guardrails.md");
    if !adr_path.exists() {
        let adr_content = r#"---
title: "ADR-001 Repository Architectural & Security Guardrails"
category: decision
tags:
  - type/card
  - adr
  - guardrails
invariants:
  - id: no-hardcoded-private-keys
    title: "No Hardcoded Private Keys"
    forbidden: "-----BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY-----"
    file_pattern: "**/*"
    severity: "deny"
    remediation: "Never commit private keys; use environment variables or secret managers."
  - id: no-conflict-markers
    title: "No Git Merge Conflict Markers"
    forbidden: "^(<<<<<<< |>>>>>>> )"
    file_pattern: "**/*"
    severity: "deny"
    remediation: "Resolve all Git merge conflict markers before committing."
---

# ADR-001 Repository Architectural & Security Guardrails

## Context
Automated coding agents and human developers commit changesets at high velocity. To prevent accidental leakage of cryptographic private keys and prevent unresolved merge conflict markers from polluting production builds, mandatory living invariants are enforced.

## Non-Negotiable Invariants
1. **Never commit cryptographic private keys**:
   - Condition: forbidden
   - Pattern: `-----BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY-----`
   - Scope: all files (`**/*`)
2. **Never commit Git merge conflict markers**:
   - Condition: forbidden
   - Pattern: `^(<<<<<<< |>>>>>>> )`
   - Scope: application code trees

## Verification & Enforcement
These invariants are evaluated automatically on every changeset via:
```sh
k0maru guard check --diff
```
"#;
        fs::write(&adr_path, adr_content)?;
        created_files.push(adr_path);
    }

    // 3. 01_AI_Logs/<today>-initial-bootstrap.md
    let log_path = logs_dir.join(format!("{}-initial-bootstrap.md", today));
    if !log_path.exists() {
        let log_content = format!(
            r#"---
title: Initial Repository Bootstrap
date: "{date}"
tags:
  - type/ai-log
  - bootstrap
project: "[[10_Projects/{slug}]]"
---

# Initial Repository Bootstrap

## Executive Summary
Bootstrapped zero-config repository LLM-Wiki and Living ADR Sentinel with K0maru Agent Memory Hub.

## Handoff Context
- **Repository Slug**: `{slug}`
- **Repository Path**: `{repo}`
- **Vault Location**: `{vault}`

## Living ADR Guardrails
- Enforced via `[[20_Cards/adr-001-repository-guardrails]]`.

## Recent Commits
{commits}
"#,
            date = today,
            slug = project_slug,
            repo = repo_path.display(),
            vault = vault_path.display(),
            commits = recent_commits.join("\n"),
        );
        fs::write(&log_path, log_content)?;
        created_files.push(log_path);
    }

    // Immediately index vault artifacts into cache.sqlite
    let cache_dir = vault_path.join(".k0maru");
    fs::create_dir_all(&cache_dir)?;

    // Keep the binary SQLite index (and model/ref caches) out of Git when the
    // `.k0maru` directory lives inside the repository.
    if cache_dir.starts_with(&repo_path) {
        fs::write(
            cache_dir.join(".gitignore"),
            "cache.sqlite*\nmodels/\nrefs/\n",
        )?;
        let top_k0maru = repo_path.join(".k0maru");
        if top_k0maru.is_dir() {
            fs::write(
                top_k0maru.join(".gitignore"),
                "**/cache.sqlite*\n**/models/\n**/refs/\n",
            )?;
        }
    }

    let cache_path = cache_dir.join("cache.sqlite");

    let mut storage = SqliteStorage::open(&cache_path)?;
    storage.initialize()?;
    let adapter = ObsidianAdapter::new(&vault_path);
    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    let sync_stats = scanner.sync(false)?;
    let indexed_docs = storage
        .document_count()
        .unwrap_or(sync_stats.added + sync_stats.modified);

    // Optional client configuration and hook installation
    let (guard_hook_installed, claude_hook_installed) = if options.install_clients {
        let hooks = crate::install::install_repo_hooks(&repo_path, &vault_path, &project_slug);
        let install_opts = crate::install::InstallOptions {
            vault_path: vault_path.clone(),
            target: crate::install::InstallTarget::All,
            dry_run: false,
            home_override: options.home_override.clone(),
        };
        let _ = crate::install::run_install(install_opts);
        hooks
    } else {
        (None, None)
    };

    Ok(BootstrapReport {
        repo_path,
        vault_path,
        project_slug,
        created_files,
        indexed_docs,
        guard_hook_installed,
        claude_hook_installed,
    })
}

/// Formats a BootstrapReport for clean terminal output.
pub fn format_bootstrap_report(report: &BootstrapReport) -> String {
    let mut out = String::new();
    out.push_str("🚀 K0maru Agent Memory Hub: Zero-Config Bootstrap\n");
    out.push_str("==================================================\n");
    out.push_str(&format!(
        "📁 Target Repo:  {}\n",
        report.repo_path.display()
    ));
    out.push_str(&format!(
        "🧠 Wiki Vault:   {}\n",
        report.vault_path.display()
    ));
    out.push_str(&format!("🏷️  Project Slug: {}\n", report.project_slug));
    out.push_str(&format!("📚 Indexed Docs: {}\n", report.indexed_docs));
    out.push_str("--------------------------------------------------\n");
    out.push_str("📝 Initialized Knowledge Artifacts:\n");
    for file in &report.created_files {
        out.push_str(&format!("   ✓ {}\n", file.display()));
    }
    out.push_str("--------------------------------------------------\n");
    if let Some(gh) = &report.guard_hook_installed {
        out.push_str(&format!("⚓ Git Pre-commit Guard: {}\n", gh.display()));
    }
    if let Some(ch) = &report.claude_hook_installed {
        out.push_str(&format!("🤖 Claude SessionStart Hook: {}\n", ch.display()));
    }
    out.push_str("--------------------------------------------------\n");
    out.push_str("⚡ Next Steps:\n");
    out.push_str(&format!(
        "   • Context Briefing:  k0maru loadout {}\n",
        report.project_slug
    ));
    out.push_str("   • Guard Sentinel:    k0maru guard check --diff\n");
    out.push_str("   • Full Diagnostics:  k0maru doctor\n");
    out
}
