//! Zero-LLM Passive Lifecycle Hooks & Progressive Disclosure (`timeline_memory` / `inspect_note`).
//!
//! Provides `<5ms` pure-Rust session capture (`SessionStart`, `PostToolFailure`, `Stop`, `SessionEnd`)
//! and chronological timeline / note inspection APIs shared across CLI, MCP, and WebUI.

use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

use chrono::Local;
use serde::{Deserialize, Serialize};

use crate::adapters::open_vault_adapter;
use crate::config::K0maruConfig;
use crate::core::models::HierarchyLevel;
use crate::guard::GuardEngine;
use crate::loadout::{estimate_tokens, LoadoutBuilder};
use crate::scanner::IncrementalScanner;
use crate::storage::SqliteStorage;
use crate::{CacheStorage, VaultAdapter};
use k0maru_core::graph::{LinkResolver, Resolution};

/// Lifecycle hook events supported by `k0maru hook <event>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HookEvent {
    SessionStart,
    PostToolFailure,
    Stop,
    SessionEnd,
}

/// Options for executing a lifecycle hook invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookOptions {
    pub event: HookEvent,
    pub vault_path: PathBuf,
    pub repo_path: Option<PathBuf>,
    pub project: Option<String>,
    pub tool_name: Option<String>,
    pub summary: Option<String>,
    pub stdin_payload: Option<String>,
}

/// Outcome returned by `run_hook`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookReport {
    pub event: HookEvent,
    pub handled: bool,
    pub skipped_reason: Option<String>,
    pub output_markdown: Option<String>,
    pub observation_path: Option<PathBuf>,
    pub guard_passed: bool,
}

/// A single chronological event in the progressive-disclosure timeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimelineEntry {
    pub id: String,
    pub timestamp: String,
    pub date: String,
    pub kind: String,
    pub hierarchy: String,
    pub title: String,
    pub relative_path: String,
    pub summary: String,
    pub status: String,
    pub project: Option<String>,
}

/// Query parameters for `build_timeline` (`timeline_memory` MCP tool & `GET /api/timeline`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineQuery {
    pub anchor: Option<String>,
    pub window: usize,
    pub project: Option<String>,
}

impl Default for TimelineQuery {
    fn default() -> Self {
        Self {
            anchor: None,
            window: 10,
            project: None,
        }
    }
}

/// Detailed note inspection payload (`inspect_note` MCP tool & `GET /api/note`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteDetailReport {
    pub relative_path: String,
    pub title: String,
    pub hierarchy: String,
    pub status: String,
    pub frontmatter: serde_json::Value,
    pub superseded_by: Option<String>,
    pub supersedes: Vec<String>,
    pub outgoing_links: Vec<String>,
    pub incoming_links: Vec<String>,
    pub tags: Vec<String>,
    pub body: String,
}

/// Refreshes SQLite full-text search and backlink indexes incrementally.
fn sync_vault_cache(vault_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let cache_dir = vault_path.join(".k0maru");
    let cache_path = cache_dir.join("cache.sqlite");
    if let Ok(mut storage) = SqliteStorage::open(&cache_path) {
        let adapter = open_vault_adapter(vault_path);
        let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
        let _ = scanner.sync(false);
    }
    Ok(())
}

/// Locates the most recent observation or log note to harvest a brief session continuity snippet.
fn find_recent_session_summary(vault_path: &Path) -> Option<(String, String)> {
    let mut candidates = Vec::new();
    for dir_name in &["logs", "01_AI_Logs"] {
        let dir = vault_path.join(dir_name);
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file()
                    && p.extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
                {
                    candidates.push(p);
                }
            }
        }
    }
    candidates.sort_by(|a, b| b.file_name().cmp(&a.file_name()));

    for file in candidates {
        if let Ok(content) = fs::read_to_string(&file) {
            let file_stem = file
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            for line in content.lines().rev() {
                let t = line.trim();
                if t.starts_with("### ")
                    || t.starts_with("- **Error**:")
                    || t.starts_with("- **Modified Files**:")
                {
                    let cleaned = t.trim_start_matches("### ").trim();
                    let snippet: String = cleaned.chars().take(80).collect();
                    return Some((file_stem, snippet));
                }
            }
        }
    }
    None
}

/// Strips ANSI terminal escape sequences from raw tool outputs.
fn strip_ansi_escapes(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                while let Some(&next_c) = chars.peek() {
                    chars.next();
                    if next_c.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Determines the directory for observation logs inside the vault using [`ConventionSniffer`].
fn resolve_logs_dir(vault_path: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let convention = crate::convention::ConventionSniffer::sniff(vault_path);
    let log_subdir = convention.target_subdir_for("log");
    let target = vault_path.join(log_subdir);
    if !target.exists() {
        fs::create_dir_all(&target)?;
    }
    Ok(target)
}

/// Inspects modified Git repository files, filtering out internal `.k0maru` and session observation notes.
fn get_git_modified_files(repo_root: &Path) -> Vec<String> {
    let output = std::process::Command::new("git")
        .args(["--no-optional-locks", "status", "--porcelain"])
        .current_dir(repo_root)
        .output();

    let mut modified = Vec::new();
    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let path_part = if line.len() > 3 {
                line[3..].trim()
            } else {
                trimmed
            };
            if path_part.contains(".k0maru/")
                || path_part.starts_with(".k0maru")
                || path_part.ends_with("-session-observations.md")
            {
                continue;
            }
            modified.push(path_part.to_string());
        }
    }
    modified
}

/// Executes a passive lifecycle hook (`SessionStart`, `PostToolFailure`, `Stop`, `SessionEnd`).
pub fn run_hook(options: &HookOptions) -> Result<HookReport, Box<dyn std::error::Error>> {
    run_hook_with_global(options, None)
}

/// Executes a passive lifecycle hook with an optional global vault path override.
pub fn run_hook_with_global(
    options: &HookOptions,
    global_vault: Option<&Path>,
) -> Result<HookReport, Box<dyn std::error::Error>> {
    let config = K0maruConfig::load();

    if matches!(
        options.event,
        HookEvent::PostToolFailure | HookEvent::Stop | HookEvent::SessionEnd
    ) && !config.passive_capture
    {
        return Ok(HookReport {
            event: options.event,
            handled: false,
            skipped_reason: Some("passive_capture disabled".into()),
            output_markdown: None,
            observation_path: None,
            guard_passed: true,
        });
    }

    match options.event {
        HookEvent::SessionStart => {
            let repo_root = options
                .repo_path
                .clone()
                .unwrap_or_else(|| crate::guard::resolve_repo_root(&options.vault_path));

            let modified_files =
                crate::git::GitInspector::inspect_diff(&repo_root, None).unwrap_or_default();
            let project_query = options
                .project
                .clone()
                .or_else(|| {
                    repo_root
                        .file_name()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_string())
                })
                .unwrap_or_default();

            let cache_dir = options.vault_path.join(".k0maru");
            let cache_path = cache_dir.join("cache.sqlite");
            let mut storage = match SqliteStorage::open(&cache_path) {
                Ok(s) => s,
                Err(_) => SqliteStorage::in_memory()?,
            };

            let adapter = open_vault_adapter(&options.vault_path);
            let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
            let _ = scanner.sync(false);

            let global_path =
                k0maru_core::storage::resolve_global_vault(global_vault, &options.vault_path);
            let global_ro = global_path
                .as_deref()
                .and_then(crate::open_global_storage_with_warning);

            let mut builder = LoadoutBuilder::new(&adapter, &storage);
            if let Some(ref ro) = global_ro {
                builder = builder.with_global(ro);
            }
            let loadout_res = if !modified_files.is_empty() {
                builder.build_with_diff(&project_query, &modified_files)?
            } else {
                builder.build(&project_query)?
            };

            let mut markdown = loadout_res.map(|r| r.markdown).unwrap_or_else(|| {
                format!(
                    "# {}\n*No specific project context pack found.*",
                    project_query
                )
            });

            let budget = config.loadout_token_budget.min(300);
            let handoff_project = options
                .project
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty());
            if let Ok(Some(hint)) = crate::handoff::HandoffStore::new(&options.vault_path)
                .latest_pending_hint(handoff_project)
            {
                let candidate = format!("{}\n\n{}", markdown, hint);
                if estimate_tokens(&candidate) <= budget {
                    markdown = candidate;
                }
            }

            if let Some((date_or_file, summary_line)) =
                find_recent_session_summary(&options.vault_path)
            {
                let continuity = format!(
                    "\n\n## Recent Session Continuity\n- Last session ({}): {}\n",
                    date_or_file, summary_line
                );
                let candidate = format!("{}{}", markdown, continuity);
                if estimate_tokens(&candidate) <= budget {
                    markdown = candidate;
                }
            }

            Ok(HookReport {
                event: HookEvent::SessionStart,
                handled: true,
                skipped_reason: None,
                output_markdown: Some(markdown),
                observation_path: None,
                guard_passed: true,
            })
        }

        HookEvent::PostToolFailure => {
            let raw_failure = options
                .summary
                .as_deref()
                .or(options.stdin_payload.as_deref())
                .unwrap_or("");
            let clean = strip_ansi_escapes(raw_failure);
            let failure_text: String = clean.chars().take(4096).collect();
            let failure_text = failure_text.trim();

            if failure_text.is_empty() {
                return Ok(HookReport {
                    event: HookEvent::PostToolFailure,
                    handled: false,
                    skipped_reason: Some("empty failure payload".into()),
                    output_markdown: None,
                    observation_path: None,
                    guard_passed: true,
                });
            }

            let logs_dir = resolve_logs_dir(&options.vault_path)?;
            let today = Local::now().format("%Y-%m-%d").to_string();
            let obs_path = logs_dir.join(format!("{}-session-observations.md", today));
            let now_time = Local::now().format("%H:%M:%S").to_string();
            let tool_name = options.tool_name.as_deref().unwrap_or("unknown");

            if !obs_path.exists() {
                let initial = format!(
                    "---\ntitle: \"{} Session Observations\"\ndate: {}\nhierarchy: L2Log\ntags:\n  - session-observation\n  - tool-failure\n---\n\n# Session Observations ({})\n\n",
                    today, today, today
                );
                fs::write(&obs_path, initial)?;
            }

            let mut file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&obs_path)?;
            let entry = format!(
                "### [{}] Tool Failure ({})\n- **Error**: {}\n\n",
                now_time, tool_name, failure_text
            );
            file.write_all(entry.as_bytes())?;

            let _ = sync_vault_cache(&options.vault_path);

            Ok(HookReport {
                event: HookEvent::PostToolFailure,
                handled: true,
                skipped_reason: None,
                output_markdown: None,
                observation_path: Some(obs_path),
                guard_passed: true,
            })
        }

        HookEvent::Stop => {
            if let Some(payload) = &options.stdin_payload {
                if payload.contains("\"stop_hook_active\": true")
                    || payload.contains("\"stop_hook_active\":true")
                {
                    return Ok(HookReport {
                        event: HookEvent::Stop,
                        handled: false,
                        skipped_reason: Some("stop_hook_active".into()),
                        output_markdown: None,
                        observation_path: None,
                        guard_passed: true,
                    });
                }
                if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(payload) {
                    if json_val.get("stop_hook_active") == Some(&serde_json::Value::Bool(true)) {
                        return Ok(HookReport {
                            event: HookEvent::Stop,
                            handled: false,
                            skipped_reason: Some("stop_hook_active".into()),
                            output_markdown: None,
                            observation_path: None,
                            guard_passed: true,
                        });
                    }
                }
            }

            let repo_root = options
                .repo_path
                .clone()
                .unwrap_or_else(|| crate::guard::resolve_repo_root(&options.vault_path));

            let resolved_global =
                k0maru_core::storage::resolve_global_vault(global_vault, &options.vault_path);
            let (rules, global_origins) = crate::guard::resolve_guard_rules_with_global(
                &options.vault_path,
                options.project.as_deref(),
                Some(&repo_root),
                resolved_global.as_deref(),
            )?;

            let diff_text = crate::guard::get_git_diff_with_mode(&repo_root, None, false)?;
            let guard_report = GuardEngine::check_diff_with_reader(
                &diff_text,
                &rules,
                crate::guard::repo_content_reader(&repo_root),
            );

            if !guard_report.is_passed {
                let formatted =
                    crate::guard::format_guard_report_with_global(&guard_report, &global_origins);
                return Ok(HookReport {
                    event: HookEvent::Stop,
                    handled: true,
                    skipped_reason: Some("guard invariants failed".into()),
                    output_markdown: Some(formatted),
                    observation_path: None,
                    guard_passed: false,
                });
            }

            let modified_files = get_git_modified_files(&repo_root);
            if modified_files.is_empty() {
                return Ok(HookReport {
                    event: HookEvent::Stop,
                    handled: true,
                    skipped_reason: Some("no repo files modified".into()),
                    output_markdown: None,
                    observation_path: None,
                    guard_passed: true,
                });
            }

            let logs_dir = resolve_logs_dir(&options.vault_path)?;
            let today = Local::now().format("%Y-%m-%d").to_string();
            let obs_path = logs_dir.join(format!("{}-session-observations.md", today));
            let now_time = Local::now().format("%H:%M:%S").to_string();

            if !obs_path.exists() {
                let initial = format!(
                    "---\ntitle: \"{} Session Observations\"\ndate: {}\nhierarchy: L2Log\ntags:\n  - session-observation\n---\n\n# Session Observations ({})\n\n",
                    today, today, today
                );
                fs::write(&obs_path, initial)?;
            }

            let mut file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&obs_path)?;
            let entry = format!(
                "### [{}] Session Checkpoint\n- **Modified Files**: {}\n\n",
                now_time,
                modified_files.join(", ")
            );
            file.write_all(entry.as_bytes())?;

            let _ = sync_vault_cache(&options.vault_path);

            Ok(HookReport {
                event: HookEvent::Stop,
                handled: true,
                skipped_reason: None,
                output_markdown: None,
                observation_path: Some(obs_path),
                guard_passed: true,
            })
        }

        HookEvent::SessionEnd => {
            let repo_root = options
                .repo_path
                .clone()
                .unwrap_or_else(|| crate::guard::resolve_repo_root(&options.vault_path));

            let modified_files = get_git_modified_files(&repo_root);
            let has_summary = options
                .summary
                .as_ref()
                .is_some_and(|s| !s.trim().is_empty());

            if modified_files.is_empty() && !has_summary {
                return Ok(HookReport {
                    event: HookEvent::SessionEnd,
                    handled: false,
                    skipped_reason: Some("no changes to hand off".into()),
                    output_markdown: None,
                    observation_path: None,
                    guard_passed: true,
                });
            }

            let logs_dir = resolve_logs_dir(&options.vault_path)?;
            let today = Local::now().format("%Y-%m-%d").to_string();
            let obs_path = logs_dir.join(format!("{}-session-observations.md", today));
            let now_time = Local::now().format("%H:%M:%S").to_string();

            if !obs_path.exists() {
                let initial = format!(
                    "---\ntitle: \"{} Session Observations\"\ndate: {}\nhierarchy: L2Log\ntags:\n  - session-observation\n---\n\n# Session Observations ({})\n\n",
                    today, today, today
                );
                fs::write(&obs_path, initial)?;
            }

            let mut file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&obs_path)?;

            let summary_text = if let Some(s) = &options.summary {
                s.trim().to_string()
            } else {
                format!("Modified files: {}", modified_files.join(", "))
            };

            let entry = format!(
                "### [{}] Session Handoff Summary\n{}\n\n",
                now_time, summary_text
            );
            file.write_all(entry.as_bytes())?;

            let _ = sync_vault_cache(&options.vault_path);

            Ok(HookReport {
                event: HookEvent::SessionEnd,
                handled: true,
                skipped_reason: None,
                output_markdown: None,
                observation_path: Some(obs_path),
                guard_passed: true,
            })
        }
    }
}

/// Helper to parse an ISO `YYYY-MM-DD` date prefix.
fn extract_iso_date(s: &str) -> Option<String> {
    let trimmed = s.trim();
    if let Some(candidate) = trimmed.get(..10) {
        let parts: Vec<&str> = candidate.split('-').collect();
        if parts.len() == 3
            && parts[0].len() == 4
            && parts[1].len() == 2
            && parts[2].len() == 2
            && parts[0].chars().all(|c| c.is_ascii_digit())
            && parts[1].chars().all(|c| c.is_ascii_digit())
            && parts[2].chars().all(|c| c.is_ascii_digit())
        {
            return Some(candidate.to_string());
        }
    }
    None
}

/// Builds a chronological timeline of L2 engineering logs, passive session observations,
/// and L3 ADR milestones around an optional anchor note/date/query.
pub fn build_timeline(
    vault_path: &Path,
    query: &TimelineQuery,
) -> Result<Vec<TimelineEntry>, Box<dyn std::error::Error>> {
    let canon_vault = if let Ok(c) = vault_path.canonicalize() {
        c
    } else {
        vault_path.to_path_buf()
    };

    let adapter = open_vault_adapter(&canon_vault);
    let files: Vec<PathBuf> = adapter.scan_files()?;

    let mut entries = Vec::new();

    for rel_path in files {
        let rel_str = rel_path.to_string_lossy().replace('\\', "/");

        let doc = match adapter.read_document(&rel_path) {
            Ok(d) => d,
            Err(_) => continue,
        };

        let is_relevant = doc.hierarchy == HierarchyLevel::L2Log
            || doc.hierarchy == HierarchyLevel::L3Evergreen
            || doc.hierarchy == HierarchyLevel::L0Ephemeral
            || rel_str.ends_with("-session-observations.md")
            || rel_str.starts_with("projects/")
            || rel_str.starts_with("10_Projects/")
            || rel_str.starts_with("decisions/")
            || rel_str.starts_with("20_Cards/")
            || rel_str.starts_with("logs/")
            || rel_str.starts_with("01_AI_Logs/");

        if !is_relevant {
            continue;
        }

        let kind = if rel_str.ends_with("-session-observations.md") {
            "observation".to_string()
        } else if rel_str.starts_with("projects/")
            || rel_str.starts_with("10_Projects/")
            || rel_str.starts_with("1_Projects/")
        {
            "project".to_string()
        } else if doc.hierarchy == HierarchyLevel::L3Evergreen
            || rel_str.starts_with("decisions/")
            || rel_str.starts_with("20_Cards/")
        {
            "decision".to_string()
        } else if doc.hierarchy == HierarchyLevel::L2Log
            || rel_str.starts_with("logs/")
            || rel_str.starts_with("01_AI_Logs/")
        {
            "log".to_string()
        } else if doc.hierarchy == HierarchyLevel::L0Ephemeral
            || rel_str.starts_with("00_Daily/")
            || rel_str.starts_with("daily/")
        {
            "ephemeral".to_string()
        } else {
            "log".to_string()
        };

        let date_from_fm = doc.frontmatter.get("date").and_then(|v| {
            if let Some(s) = v.as_str() {
                Some(s.to_string())
            } else {
                v.as_i64().map(|n| n.to_string())
            }
        });

        let date = date_from_fm
            .as_deref()
            .and_then(extract_iso_date)
            .or_else(|| {
                rel_path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .and_then(extract_iso_date)
            })
            .unwrap_or_else(|| {
                if doc.mtime > 0 {
                    chrono::DateTime::from_timestamp(doc.mtime as i64, 0)
                        .map(|dt| dt.format("%Y-%m-%d").to_string())
                        .unwrap_or_else(|| Local::now().format("%Y-%m-%d").to_string())
                } else {
                    Local::now().format("%Y-%m-%d").to_string()
                }
            });

        let timestamp = if doc.mtime > 0 {
            chrono::DateTime::from_timestamp(doc.mtime as i64, 0)
                .map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
                .unwrap_or_else(|| format!("{}T00:00:00Z", date))
        } else {
            format!("{}T00:00:00Z", date)
        };

        let id = rel_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(&doc.title)
            .to_string();

        let summary = doc
            .frontmatter
            .get("summary")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                for line in doc.body.lines() {
                    let t = line.trim();
                    if !t.is_empty() && !t.starts_with('#') {
                        return t.chars().take(120).collect();
                    }
                }
                String::new()
            });

        let project = doc
            .frontmatter
            .get("project")
            .or_else(|| doc.frontmatter.get("projects"))
            .and_then(|v| {
                if let Some(s) = v.as_str() {
                    Some(s.to_string())
                } else if let Some(arr) = v.as_array() {
                    arr.first().and_then(|x| x.as_str()).map(|s| s.to_string())
                } else {
                    None
                }
            })
            .or_else(|| {
                if rel_str.starts_with("projects/") || rel_str.starts_with("10_Projects/") {
                    rel_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_string())
                } else {
                    None
                }
            });

        if let Some(p_filter) = &query.project {
            let matches_project = project
                .as_deref()
                .is_some_and(|p| p.eq_ignore_ascii_case(p_filter))
                || rel_str.to_lowercase().contains(&p_filter.to_lowercase());
            if !matches_project {
                continue;
            }
        }

        let status = doc.lifecycle_status().to_string();
        entries.push(TimelineEntry {
            id,
            timestamp,
            date,
            kind,
            hierarchy: doc.hierarchy.to_string(),
            title: doc.title,
            relative_path: rel_str,
            summary,
            status,
            project,
        });
    }

    entries.sort_by(|a, b| {
        b.date
            .cmp(&a.date)
            .then_with(|| b.timestamp.cmp(&a.timestamp))
            .then_with(|| a.relative_path.cmp(&b.relative_path))
    });

    let window = query.window.max(1);

    if let Some(anchor) = &query.anchor {
        let anchor_lower = anchor.to_lowercase();
        let anchor_idx = entries.iter().position(|e| {
            e.relative_path.to_lowercase().contains(&anchor_lower)
                || e.title.to_lowercase().contains(&anchor_lower)
                || e.date == *anchor
                || e.id.to_lowercase().contains(&anchor_lower)
        });

        if let Some(idx) = anchor_idx {
            let half = window / 2;
            let start = idx.saturating_sub(half);
            let end = (start + window).min(entries.len());
            let start = end.saturating_sub(window);
            return Ok(entries[start..end].to_vec());
        }
    }

    entries.truncate(window);
    Ok(entries)
}

/// Parsed representation of a note locator `[@global:]<path>[:<start>-<end>][#<heading>]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedLocator {
    pub is_global: bool,
    pub path: String,
    pub line_range: Option<(u32, u32)>,
    pub heading: Option<String>,
}

/// Strips copied K-DSL wrappers (`!μ:L3|...|score:...|links:... § ...` or `#1 | ...`)
/// so small models can pass search output lines directly into `inspect_note`.
fn strip_kdsl_wrappers(raw: &str) -> &str {
    let mut s = raw.trim();
    if (s.starts_with('`') && s.ends_with('`') && s.len() >= 2)
        || (s.starts_with('"') && s.ends_with('"') && s.len() >= 2)
        || (s.starts_with('\'') && s.ends_with('\'') && s.len() >= 2)
    {
        s = s[1..s.len() - 1].trim();
    }

    // Strip leading `#<rank> | `
    if let Some(after_hash) = s.strip_prefix('#') {
        if let Some(pipe_idx) = after_hash.find('|') {
            let rank_part = after_hash[..pipe_idx].trim();
            if !rank_part.is_empty() && rank_part.chars().all(|c| c.is_ascii_digit()) {
                s = after_hash[pipe_idx + 1..].trim();
            }
        }
    }

    // Strip leading `!μ:<tier>|` or `!u:<tier>|`
    if s.starts_with("!μ:") || s.starts_with("!u:") {
        if let Some(pipe_idx) = s.find('|') {
            s = s[pipe_idx + 1..].trim();
        }
    }

    // Strip trailing ` § <snippet>`
    if let Some(sec_idx) = s.find(" § ") {
        s = s[..sec_idx].trim();
    }

    // Strip trailing K-DSL metadata fields (`|score:...`, `|rank:...`, `|links:...`, `|via:...`)
    let mut cut_at = s.len();
    for marker in ["|score:", "|rank:", "|links:", "|via:"] {
        if let Some(idx) = s.find(marker) {
            if idx < cut_at {
                cut_at = idx;
            }
        }
    }
    s[..cut_at].trim()
}

/// Parses a note path or locator string (`[@global:]<path>[:<start>-<end>][#<heading>]`).
///
/// Tolerates copied K-DSL search wire prefixes/suffixes and stems without `.md`.
pub fn parse_locator(raw: &str) -> Result<ParsedLocator, Box<dyn std::error::Error>> {
    let trimmed = strip_kdsl_wrappers(raw);
    if trimmed.is_empty() {
        return Err("relative_path cannot be empty".into());
    }

    let (is_global, rest) = if let Some(stripped) = trimmed.strip_prefix("@global:") {
        (true, stripped)
    } else {
        (false, trimmed)
    };

    if rest.is_empty() {
        return Err("relative_path cannot be empty".into());
    }

    let bytes = rest.as_bytes();
    let mut split_idx: Option<usize> = None;
    if bytes.len() >= 3 {
        for i in (0..=bytes.len() - 3).rev() {
            if bytes[i] == b'.'
                && bytes[i + 1].eq_ignore_ascii_case(&b'm')
                && bytes[i + 2].eq_ignore_ascii_case(&b'd')
            {
                let end = i + 3;
                match rest[end..].chars().next() {
                    None | Some(':') | Some('#') => {
                        split_idx = Some(end);
                        break;
                    }
                    Some(_) => {}
                }
            }
        }
    }

    // Also split :<start>-<end> or #<heading> on stems without an explicit .md extension
    if split_idx.is_none() {
        if let Some(colon_idx) = rest.rfind(':') {
            let after = &rest[colon_idx + 1..];
            let range_part = after.split('#').next().unwrap_or(after);
            if let Some((a, b)) = range_part.split_once('-') {
                if !a.is_empty()
                    && !b.is_empty()
                    && a.bytes().all(|c| c.is_ascii_digit())
                    && b.bytes().all(|c| c.is_ascii_digit())
                {
                    split_idx = Some(colon_idx);
                }
            }
        } else if let Some(hash_idx) = rest.find('#') {
            if hash_idx > 0 {
                split_idx = Some(hash_idx);
            }
        }
    }

    let (path, suffix) = match split_idx {
        Some(idx) => (&rest[..idx], &rest[idx..]),
        None => (rest, ""),
    };

    let (line_range, heading) = if suffix.is_empty() {
        (None, None)
    } else if let Some(after_colon) = suffix.strip_prefix(':') {
        let (range_part, hash_part) = match after_colon.find('#') {
            Some(pos) => (
                &after_colon[..pos],
                Some(after_colon[pos + 1..].to_string()),
            ),
            None => (after_colon, None),
        };
        let (start_s, end_s) = range_part
            .split_once('-')
            .ok_or("Invalid line range in locator: expected :<start>-<end>")?;
        let start: u32 = start_s
            .parse()
            .map_err(|_| "Invalid start line in locator")?;
        let end: u32 = end_s.parse().map_err(|_| "Invalid end line in locator")?;
        if start == 0 {
            return Err("Line numbers in locator must be >= 1".into());
        }
        if start > end {
            return Err("Invalid line range in locator: start > end".into());
        }
        (Some((start, end)), hash_part)
    } else if let Some(after_hash) = suffix.strip_prefix('#') {
        (None, Some(after_hash.to_string()))
    } else {
        (None, None)
    };

    Ok(ParsedLocator {
        is_global,
        path: path.to_string(),
        line_range,
        heading,
    })
}

/// Inspects a single markdown note safely inside `vault_path` (with symlink & path-traversal protection).
pub fn inspect_note_detail(
    vault_path: &Path,
    relative_path: &str,
) -> Result<NoteDetailReport, Box<dyn std::error::Error>> {
    inspect_note_detail_with_options(vault_path, relative_path, false)
}

struct SlicedNote {
    canon_vault: PathBuf,
    safe_rel: PathBuf,
    clean_rel_path: String,
    doc: k0maru_core::Document,
    body: String,
}

fn validate_relative_no_traversal(raw_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let p = Path::new(raw_path);
    if p.is_absolute() {
        return Err("Absolute paths are not allowed".into());
    }
    for comp in p.components() {
        match comp {
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err("Path traversal or root components are not allowed".into());
            }
            _ => {}
        }
    }
    Ok(())
}

fn resolve_vault_note_path(
    canon_vault: &Path,
    raw_path: &str,
    scope_label: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    validate_relative_no_traversal(raw_path)?;

    let clean_input = raw_path.replace('\\', "/");
    let clean_input = clean_input.trim_start_matches("./");
    let input_path = Path::new(clean_input);

    // Reject explicit non-markdown files (unless `<input>.md` actually exists)
    if let Some(ext) = input_path.extension().and_then(|e| e.to_str()) {
        if !ext.eq_ignore_ascii_case("md") {
            let with_md = canon_vault.join(format!("{}.md", clean_input));
            if !with_md.is_file() {
                return Err("Only markdown (.md) notes can be inspected".into());
            }
        }
    }

    // Build candidate relative paths: exact, +.md, and stripped leading vault directory name
    let mut candidates = Vec::new();
    let push_with_md = |vec: &mut Vec<String>, s: &str| {
        if s.is_empty() {
            return;
        }
        let has_md = Path::new(s)
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("md"));
        if has_md {
            vec.push(s.to_string());
        } else {
            vec.push(format!("{}.md", s));
        }
    };

    push_with_md(&mut candidates, clean_input);
    if let Some(vault_name) = canon_vault.file_name().and_then(|s| s.to_str()) {
        if let Some(after_vault) = clean_input
            .strip_prefix(vault_name)
            .and_then(|s| s.strip_prefix('/'))
        {
            push_with_md(&mut candidates, after_vault);
        }
    }

    for cand in &candidates {
        let target = canon_vault.join(cand);
        if target.exists() {
            let canon_note = target.canonicalize()?;
            if !canon_note.starts_with(canon_vault) || !canon_note.is_file() {
                return Err(format!(
                    "Target note does not reside within the {} directory or is not a file",
                    scope_label
                )
                .into());
            }
            return Ok(canon_note);
        }
    }

    // Fallback: resolve stem/title via LinkResolver across indexed vault markdown files
    let adapter = open_vault_adapter(canon_vault);
    let all_files = adapter.scan_files().unwrap_or_default();
    let mut docs_for_resolver: Vec<(i64, String, String)> = Vec::new();
    let mut rel_paths: Vec<String> = Vec::new();
    for (idx, rel) in all_files.iter().enumerate() {
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        let title = adapter
            .read_document(rel)
            .map(|d| d.title)
            .unwrap_or_else(|_| {
                rel.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string()
            });
        docs_for_resolver.push((idx as i64, rel_str.clone(), title));
        rel_paths.push(rel_str);
    }

    let resolver = LinkResolver::from_docs(docs_for_resolver);
    let lookup_target = clean_input
        .strip_suffix(".md")
        .or_else(|| clean_input.strip_suffix(".MD"))
        .unwrap_or(clean_input);
    if let Resolution::Doc(id) = resolver.resolve("", lookup_target) {
        if let Some(rel_str) = rel_paths.get(id as usize) {
            let canon_note = canon_vault.join(rel_str).canonicalize()?;
            if canon_note.starts_with(canon_vault) && canon_note.is_file() {
                return Ok(canon_note);
            }
        }
    }

    // Build nearest suggestions instead of returning a raw OS error
    let needle = lookup_target.to_lowercase();
    let mut suggestions: Vec<String> = rel_paths
        .iter()
        .filter(|p| {
            let pl = p.to_lowercase();
            pl.contains(&needle)
                || needle
                    .rsplit('/')
                    .next()
                    .is_some_and(|stem| !stem.is_empty() && pl.contains(stem))
        })
        .take(3)
        .cloned()
        .collect();
    if suggestions.is_empty() {
        suggestions = rel_paths.into_iter().take(3).collect();
    }

    if suggestions.is_empty() {
        Err(format!(
            "Note '{}' not found (paths are relative to {}: {}).",
            raw_path,
            scope_label,
            canon_vault.display()
        )
        .into())
    } else {
        Err(format!(
            "Note '{}' not found (paths are relative to {}: {}). Did you mean: {}?",
            raw_path,
            scope_label,
            canon_vault.display(),
            suggestions.join(", ")
        )
        .into())
    }
}

fn read_and_slice_note(
    vault_path: &Path,
    parsed: &ParsedLocator,
    full_body: bool,
    scope_label: &str,
) -> Result<SlicedNote, Box<dyn std::error::Error>> {
    validate_relative_no_traversal(&parsed.path)?;

    let canon_vault = vault_path.canonicalize()?;
    let canon_note = resolve_vault_note_path(&canon_vault, &parsed.path, scope_label)?;

    let safe_rel = canon_note.strip_prefix(&canon_vault)?.to_path_buf();
    let adapter = open_vault_adapter(&canon_vault);
    let doc = adapter.read_document(&safe_rel)?;

    let body = if !full_body {
        if let Some((start_line, end_line)) = parsed.line_range {
            let raw_file = fs::read_to_string(&canon_note)?;
            slice_file_lines(&raw_file, start_line, end_line)
        } else {
            doc.body.clone()
        }
    } else {
        doc.body.clone()
    };
    let clean_rel_path = safe_rel.to_string_lossy().replace('\\', "/");

    Ok(SlicedNote {
        canon_vault,
        safe_rel,
        clean_rel_path,
        doc,
        body,
    })
}

/// Inspects a markdown note or section locator (`path[:start-end][#heading]`) inside `vault_path`.
///
/// When `locator` specifies a `start-end` range and `full_body` is `false`, `report.body`
/// contains the 1-based inclusive file lines `start..=end` read directly from the raw file.
pub fn inspect_note_detail_with_options(
    vault_path: &Path,
    locator: &str,
    full_body: bool,
) -> Result<NoteDetailReport, Box<dyn std::error::Error>> {
    let parsed = parse_locator(locator)?;
    if parsed.is_global {
        return Err(
            "@global: locator requires a configured global vault and read-only inspection".into(),
        );
    }

    let sliced = read_and_slice_note(vault_path, &parsed, full_body, "vault")?;
    let status = sliced.doc.lifecycle_status().to_string();
    let superseded_by = sliced.doc.superseded_by();
    let supersedes = sliced.doc.supersedes();

    let cache_dir = sliced.canon_vault.join(".k0maru");
    let cache_path = cache_dir.join("cache.sqlite");
    let incoming_links = if let Ok(mut storage) = SqliteStorage::open(&cache_path) {
        let adapter = open_vault_adapter(&sliced.canon_vault);
        let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
        let _ = scanner.sync(false);

        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let _ = storage.record_access(&sliced.clean_rel_path, now_secs);

        collect_backlinks_from_storage(&storage, &sliced.safe_rel, &sliced.doc.title)
    } else {
        Vec::new()
    };

    let outgoing_links = sliced.doc.links.into_iter().map(|l| l.target).collect();

    Ok(NoteDetailReport {
        relative_path: sliced.clean_rel_path,
        title: sliced.doc.title,
        hierarchy: sliced.doc.hierarchy.to_string(),
        status,
        frontmatter: sliced.doc.frontmatter,
        superseded_by,
        supersedes,
        outgoing_links,
        incoming_links,
        tags: sliced.doc.tags,
        body: sliced.body,
    })
}

/// Strictly read-only note inspection for a global vault (`@global:<rel>[:s-e][#h]`).
///
/// Enforces canonical path traversal checks against `global_vault`, reads file lines
/// directly, pulls backlinks from [`crate::storage::ReadOnlyStorage`], and performs
/// ZERO cache syncs and ZERO `record_access` writes.
pub fn inspect_note_detail_ro(
    global_vault: &Path,
    locator: &str,
) -> Result<NoteDetailReport, Box<dyn std::error::Error>> {
    inspect_note_detail_ro_with_options(global_vault, locator, None, false)
}

/// Strictly read-only note inspection for a global vault with optional pre-opened
/// [`crate::storage::ReadOnlyStorage`] and `full_body` override.
pub fn inspect_note_detail_ro_with_options(
    global_vault: &Path,
    locator: &str,
    global_storage: Option<&k0maru_core::storage::ReadOnlyStorage>,
    full_body: bool,
) -> Result<NoteDetailReport, Box<dyn std::error::Error>> {
    let parsed = parse_locator(locator)?;
    let sliced = read_and_slice_note(global_vault, &parsed, full_body, "global vault")?;

    let status = sliced.doc.lifecycle_status().to_string();
    let superseded_by = sliced.doc.superseded_by();
    let supersedes = sliced.doc.supersedes();

    let owned_ro = if global_storage.is_none() {
        let cache_path = sliced.canon_vault.join(".k0maru").join("cache.sqlite");
        if cache_path.is_file() {
            k0maru_core::storage::ReadOnlyStorage::open(&cache_path).ok()
        } else {
            None
        }
    } else {
        None
    };

    let incoming_links = match global_storage.or(owned_ro.as_ref()) {
        Some(ro) => {
            collect_backlinks_from_storage(ro.storage(), &sliced.safe_rel, &sliced.doc.title)
        }
        None => Vec::new(),
    };

    let outgoing_links = sliced.doc.links.into_iter().map(|l| l.target).collect();

    Ok(NoteDetailReport {
        relative_path: sliced.clean_rel_path,
        title: sliced.doc.title,
        hierarchy: sliced.doc.hierarchy.to_string(),
        status,
        frontmatter: sliced.doc.frontmatter,
        superseded_by,
        supersedes,
        outgoing_links,
        incoming_links,
        tags: sliced.doc.tags,
        body: sliced.body,
    })
}

fn collect_backlinks_from_storage(
    storage: &SqliteStorage,
    safe_rel: &Path,
    doc_title: &str,
) -> Vec<String> {
    let mut links = Vec::new();
    if let Some(stem) = safe_rel.file_stem().and_then(|s| s.to_str()) {
        if let Ok(bls) = storage.get_backlinks(stem) {
            for bl in bls {
                links.push(bl.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    if !doc_title.is_empty() {
        if let Ok(bls) = storage.get_backlinks(doc_title) {
            for bl in bls {
                let formatted = bl.to_string_lossy().replace('\\', "/");
                if !links.contains(&formatted) {
                    links.push(formatted);
                }
            }
        }
    }
    links.sort();
    links.dedup();
    links
}

fn slice_file_lines(raw: &str, start_line: u32, end_line: u32) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    if lines.is_empty() || start_line == 0 {
        return String::new();
    }
    let start_idx = (start_line as usize).saturating_sub(1);
    if start_idx >= lines.len() {
        return String::new();
    }
    let end_idx = (end_line as usize).min(lines.len());
    if start_idx >= end_idx {
        return String::new();
    }
    lines[start_idx..end_idx].join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_iso_date_handles_multibyte_utf8_safely() {
        assert_eq!(
            extract_iso_date("2026-11-09-initial-bootstrap"),
            Some("2026-11-09".to_string())
        );
        // Multi-byte UTF-8 where byte index 10 lands inside a 3-byte CJK character
        assert_eq!(extract_iso_date("2026年11月09日-架构日志"), None);
        assert_eq!(extract_iso_date("中文标题测试笔记"), None);
    }
}
