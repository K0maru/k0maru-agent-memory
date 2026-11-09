//! Zero-LLM Passive Lifecycle Hooks & Progressive Disclosure (`timeline_memory` / `inspect_note`).
//!
//! Provides `<5ms` pure-Rust session capture (`SessionStart`, `PostToolFailure`, `Stop`, `SessionEnd`)
//! and chronological timeline / note inspection APIs shared across CLI, MCP, and WebUI.

use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

use chrono::Local;
use serde::{Deserialize, Serialize};

use crate::adapters::{GenericWikiAdapter, ObsidianAdapter};
use crate::config::K0maruConfig;
use crate::core::models::HierarchyLevel;
use crate::guard::GuardEngine;
use crate::loadout::{estimate_tokens, LoadoutBuilder};
use crate::scanner::IncrementalScanner;
use crate::storage::SqliteStorage;
use crate::{CacheStorage, VaultAdapter};

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

/// Determines whether a vault root contains structured Johnny.Decimal / LLM-Wiki folders.
fn is_structured_vault(vault_path: &Path) -> bool {
    vault_path.join(".obsidian").exists()
        || vault_path.join("10_Projects").is_dir()
        || vault_path.join("20_Cards").is_dir()
        || vault_path.join("projects").is_dir()
        || vault_path.join("decisions").is_dir()
        || vault_path.join("logs").is_dir()
        || vault_path.join("cards").is_dir()
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

/// Refreshes SQLite full-text search and backlink indexes incrementally.
fn sync_vault_cache(vault_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let cache_dir = vault_path.join(".k0maru");
    let cache_path = cache_dir.join("cache.sqlite");
    if let Ok(mut storage) = SqliteStorage::open(&cache_path) {
        if is_structured_vault(vault_path) {
            let adapter = ObsidianAdapter::new(vault_path);
            let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
            let _ = scanner.sync(false);
        } else {
            let adapter = GenericWikiAdapter::new(vault_path);
            let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
            let _ = scanner.sync(false);
        }
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

/// Determines the directory for observation logs inside the vault.
fn resolve_logs_dir(vault_path: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if vault_path.join("logs").is_dir() {
        Ok(vault_path.join("logs"))
    } else if vault_path.join("01_AI_Logs").is_dir() {
        Ok(vault_path.join("01_AI_Logs"))
    } else if vault_path.join("10_Projects").is_dir() || vault_path.join("20_Cards").is_dir() {
        let d = vault_path.join("01_AI_Logs");
        fs::create_dir_all(&d)?;
        Ok(d)
    } else {
        let d = vault_path.join("logs");
        fs::create_dir_all(&d)?;
        Ok(d)
    }
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

            let is_obs = is_structured_vault(&options.vault_path);
            let cache_dir = options.vault_path.join(".k0maru");
            let cache_path = cache_dir.join("cache.sqlite");
            let mut storage = match SqliteStorage::open(&cache_path) {
                Ok(s) => s,
                Err(_) => SqliteStorage::in_memory()?,
            };

            if is_obs {
                let adapter = ObsidianAdapter::new(&options.vault_path);
                let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
                let _ = scanner.sync(false);
            } else {
                let adapter = GenericWikiAdapter::new(&options.vault_path);
                let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
                let _ = scanner.sync(false);
            }

            let loadout_res = if !modified_files.is_empty() {
                if is_obs {
                    let adapter = ObsidianAdapter::new(&options.vault_path);
                    let builder = LoadoutBuilder::new(&adapter, &storage);
                    builder.build_with_diff(&project_query, &modified_files)?
                } else {
                    let adapter = GenericWikiAdapter::new(&options.vault_path);
                    let builder = LoadoutBuilder::new(&adapter, &storage);
                    builder.build_with_diff(&project_query, &modified_files)?
                }
            } else if is_obs {
                let adapter = ObsidianAdapter::new(&options.vault_path);
                let builder = LoadoutBuilder::new(&adapter, &storage);
                builder.build(&project_query)?
            } else {
                let adapter = GenericWikiAdapter::new(&options.vault_path);
                let builder = LoadoutBuilder::new(&adapter, &storage);
                builder.build(&project_query)?
            };

            let mut markdown = loadout_res.map(|r| r.markdown).unwrap_or_else(|| {
                format!(
                    "# {}\n*No specific project context pack found.*",
                    project_query
                )
            });

            if let Some((date_or_file, summary_line)) =
                find_recent_session_summary(&options.vault_path)
            {
                let continuity = format!(
                    "\n\n## Recent Session Continuity\n- Last session ({}): {}\n",
                    date_or_file, summary_line
                );
                let candidate = format!("{}{}", markdown, continuity);
                let budget = config.loadout_token_budget.min(300);
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

            let rules = crate::guard::resolve_guard_rules_with_repo(
                &options.vault_path,
                options.project.as_deref(),
                Some(&repo_root),
            )?;

            let diff_text = crate::guard::get_git_diff_with_mode(&repo_root, None, false)?;
            let guard_report = GuardEngine::check_diff_with_reader(
                &diff_text,
                &rules,
                crate::guard::repo_content_reader(&repo_root),
            );

            if !guard_report.is_passed {
                let formatted = crate::guard::format_guard_report(&guard_report);
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

    let is_obs = is_structured_vault(&canon_vault);
    let files: Vec<PathBuf> = if is_obs {
        let adapter = ObsidianAdapter::new(&canon_vault);
        adapter.scan_files()?
    } else {
        let adapter = GenericWikiAdapter::new(&canon_vault);
        adapter.scan_files()?
    };

    let mut entries = Vec::new();

    for rel_path in files {
        let rel_str = rel_path.to_string_lossy().replace('\\', "/");

        let doc = if is_obs {
            let adapter = ObsidianAdapter::new(&canon_vault);
            match adapter.read_document(&rel_path) {
                Ok(d) => d,
                Err(_) => continue,
            }
        } else {
            let adapter = GenericWikiAdapter::new(&canon_vault);
            match adapter.read_document(&rel_path) {
                Ok(d) => d,
                Err(_) => continue,
            }
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

/// Inspects a single markdown note safely inside `vault_path` (with symlink & path-traversal protection).
pub fn inspect_note_detail(
    vault_path: &Path,
    relative_path: &str,
) -> Result<NoteDetailReport, Box<dyn std::error::Error>> {
    let trimmed = relative_path.trim();
    if trimmed.is_empty() {
        return Err("relative_path cannot be empty".into());
    }

    let p = Path::new(trimmed);
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

    if !p
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
    {
        return Err("Only markdown (.md) notes can be inspected".into());
    }

    let canon_vault = vault_path.canonicalize()?;
    let target = canon_vault.join(p);
    let canon_note = target.canonicalize()?;

    if !canon_note.starts_with(&canon_vault) || !canon_note.is_file() {
        return Err(
            "Target note does not reside within the vault directory or is not a file".into(),
        );
    }

    let safe_rel = canon_note.strip_prefix(&canon_vault)?;
    let is_obs = is_structured_vault(&canon_vault);

    let doc = if is_obs {
        let adapter = ObsidianAdapter::new(&canon_vault);
        adapter.read_document(safe_rel)?
    } else {
        let adapter = GenericWikiAdapter::new(&canon_vault);
        adapter.read_document(safe_rel)?
    };

    let cache_dir = canon_vault.join(".k0maru");
    let cache_path = cache_dir.join("cache.sqlite");
    let incoming_links = if let Ok(mut storage) = SqliteStorage::open(&cache_path) {
        if is_obs {
            let adapter = ObsidianAdapter::new(&canon_vault);
            let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
            let _ = scanner.sync(false);
        } else {
            let adapter = GenericWikiAdapter::new(&canon_vault);
            let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
            let _ = scanner.sync(false);
        }

        let mut links = Vec::new();
        if let Some(stem) = safe_rel.file_stem().and_then(|s| s.to_str()) {
            if let Ok(bls) = storage.get_backlinks(stem) {
                for bl in bls {
                    links.push(bl.to_string_lossy().replace('\\', "/"));
                }
            }
        }
        if !doc.title.is_empty() {
            if let Ok(bls) = storage.get_backlinks(&doc.title) {
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
    } else {
        Vec::new()
    };

    let status = doc.lifecycle_status().to_string();
    let superseded_by = doc.superseded_by();
    let supersedes = doc.supersedes();
    let outgoing_links = doc.links.into_iter().map(|l| l.target).collect();

    Ok(NoteDetailReport {
        relative_path: safe_rel.to_string_lossy().replace('\\', "/"),
        title: doc.title,
        hierarchy: doc.hierarchy.to_string(),
        status,
        frontmatter: doc.frontmatter,
        superseded_by,
        supersedes,
        outgoing_links,
        incoming_links,
        tags: doc.tags,
        body: doc.body,
    })
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
