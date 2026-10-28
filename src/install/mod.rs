//! FastMCP client installation and automated JSON configuration engine.
//!
//! Provides safe, atomic configuration injection for AI coding agent MCP clients
//! (Claude Code, Cursor, Antigravity/Gemini CLI, Windsurf, Cline) with dry-run preview,
//! selective targeting, and non-destructive JSON merging.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::ecosystem::McpClient;

/// Target MCP client(s) to install and configure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallTarget {
    All,
    Claude,
    Cursor,
    Gemini,
    Windsurf,
    Cline,
    Hermes,
    OpenClaw,
    Omp,
}

impl InstallTarget {
    /// Returns the slice or vector of clients corresponding to this target.
    pub fn clients(&self) -> Vec<McpClient> {
        match self {
            Self::All => McpClient::all().to_vec(),
            Self::Claude => vec![McpClient::Claude],
            Self::Cursor => vec![McpClient::Cursor],
            Self::Gemini => vec![McpClient::Gemini],
            Self::Windsurf => vec![McpClient::Windsurf],
            Self::Cline => vec![McpClient::Cline],
            Self::Hermes => vec![McpClient::Hermes],
            Self::OpenClaw => vec![McpClient::OpenClaw],
            Self::Omp => vec![McpClient::Omp],
        }
    }
}

impl std::str::FromStr for InstallTarget {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "all" => Ok(Self::All),
            "claude" | "claude-code" => Ok(Self::Claude),
            "cursor" => Ok(Self::Cursor),
            "gemini" | "antigravity" => Ok(Self::Gemini),
            "windsurf" => Ok(Self::Windsurf),
            "cline" | "roo" | "roo-cline" => Ok(Self::Cline),
            "hermes" | "hermes-agent" => Ok(Self::Hermes),
            "openclaw" | "claw" => Ok(Self::OpenClaw),
            "omp" | "oh-my-pi" => Ok(Self::Omp),
            other => Err(format!(
                "Unknown install target: '{}'. Supported targets: all, claude, cursor, gemini, windsurf, cline, hermes, openclaw, omp",
                other
            )),
        }
    }
}

impl std::fmt::Display for InstallTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::All => write!(f, "all"),
            Self::Claude => write!(f, "claude"),
            Self::Cursor => write!(f, "cursor"),
            Self::Gemini => write!(f, "gemini"),
            Self::Windsurf => write!(f, "windsurf"),
            Self::Cline => write!(f, "cline"),
            Self::Hermes => write!(f, "hermes"),
            Self::OpenClaw => write!(f, "openclaw"),
            Self::Omp => write!(f, "omp"),
        }
    }
}

/// Options controlling installation behavior.
#[derive(Debug, Clone)]
pub struct InstallOptions {
    /// Root directory of target vault to mount.
    pub vault_path: PathBuf,
    /// Target client(s) to configure.
    pub target: InstallTarget,
    /// If true, preview configuration without writing to disk.
    pub dry_run: bool,
    /// Optional home directory override for testing or sandboxed installation.
    pub home_override: Option<PathBuf>,
}

/// Per-client installation outcome and diagnostic result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientInstallOutcome {
    pub client: McpClient,
    pub config_path: PathBuf,
    pub created_file: bool,
    pub already_configured: bool,
    pub updated: bool,
    pub dry_run: bool,
    pub preview_json: Option<String>,
}

/// Summary report returned by `run_install`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallReport {
    pub vault_path: PathBuf,
    pub dry_run: bool,
    pub outcomes: Vec<ClientInstallOutcome>,
}

/// Injects or updates `"k0maru-memory"` server entry into existing JSON content.
///
/// Preserves other servers, custom top-level keys, and ensures `mcpServers` object exists.
pub fn inject_k0maru_mcp(
    existing_json_str: Option<&str>,
    vault_path: &Path,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    inject_k0maru_mcp_for_client(existing_json_str, vault_path, McpClient::Claude)
}

/// Injects or updates `"k0maru-memory"` server entry into existing JSON content for a specific client.
///
/// Preserves other servers, custom top-level keys, and ensures `mcpServers` object exists.
pub fn inject_k0maru_mcp_for_client(
    existing_json_str: Option<&str>,
    vault_path: &Path,
    client: McpClient,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let mut root: serde_json::Value = match existing_json_str {
        Some(s) if !s.trim().is_empty() => serde_json::from_str(s)?,
        _ => serde_json::json!({ "mcpServers": {} }),
    };

    let obj = root
        .as_object_mut()
        .ok_or("Configuration root must be a JSON object")?;

    if !obj.contains_key("mcpServers") {
        obj.insert("mcpServers".to_string(), serde_json::json!({}));
    }

    let servers = obj
        .get_mut("mcpServers")
        .and_then(|v| v.as_object_mut())
        .ok_or("Field 'mcpServers' must be a JSON object")?;

    let vault_str = vault_path.to_string_lossy().to_string();
    let args = if client == McpClient::Omp {
        vec!["mcp", "--vault", vault_str.as_str(), "--sidecar"]
    } else {
        vec!["mcp", "--vault", vault_str.as_str()]
    };
    let server_entry = serde_json::json!({
        "command": "k0maru",
        "args": args
    });

    servers.insert("k0maru-memory".to_string(), server_entry);

    let formatted = serde_json::to_string_pretty(&root)?;
    Ok(formatted)
}

/// Checks whether the existing JSON already mounts `k0maru-memory` targeting `target_vault`.
fn check_already_configured(
    existing_content: Option<&str>,
    target_vault: &Path,
    client: McpClient,
) -> bool {
    let content = match existing_content {
        Some(c) if !c.trim().is_empty() => c,
        _ => return false,
    };

    let val = match serde_json::from_str::<serde_json::Value>(content) {
        Ok(v) => v,
        Err(_) => return false,
    };

    let server_entry = val
        .get("mcpServers")
        .and_then(|s| s.get("k0maru-memory").or_else(|| s.get("k0maru")))
        .or_else(|| val.get("k0maru-memory").or_else(|| val.get("k0maru")));

    let entry = match server_entry {
        Some(e) => e,
        None => return false,
    };

    let args = match entry.get("args").and_then(|a| a.as_array()) {
        Some(a) => a,
        None => return false,
    };
    if client == McpClient::Omp {
        let has_sidecar = args.iter().any(|a| a.as_str() == Some("--sidecar"));
        if !has_sidecar {
            return false;
        }
    }

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if let Some(arg_str) = arg.as_str() {
            let configured_vault = if arg_str == "--vault" || arg_str == "-v" {
                iter.next().and_then(|v| v.as_str()).map(PathBuf::from)
            } else {
                arg_str.strip_prefix("--vault=").map(PathBuf::from)
            };

            if let Some(v_path) = configured_vault {
                if v_path == target_vault {
                    return true;
                }
                if let (Ok(c1), Ok(c2)) = (
                    std::fs::canonicalize(&v_path),
                    std::fs::canonicalize(target_vault),
                ) {
                    if c1 == c2 {
                        return true;
                    }
                }
            }
        }
    }

    false
}

/// Ensures Antigravity / Gemini CLI has the sensitivity instructions.md in its MCP directory.
fn ensure_gemini_instructions(config_path: &Path) {
    if let Some(parent) = config_path.parent() {
        let mcp_dir = parent.join("mcp").join("k0maru-memory");
        if std::fs::create_dir_all(&mcp_dir).is_ok() {
            let instructions_path = mcp_dir.join("instructions.md");
            let instructions_content = r#"# K0maru Memory Hub Tool Guidelines & Sensitivity Invariants

`k0maru-memory` mounts the user's permanent local knowledge base (SecondBrain / LLM-Wiki).
It stores research reports, architectural decisions (ADRs), technical feasibility studies, infrastructure configs, project notes, postmortems, and domain knowledge.

## Critical Invariant: Proactive Memory Recall
You MUST trigger `recall_memory` in `k0maru-memory` under the following circumstances:
1. **Information & Documentation Requests**:
   Whenever the user asks for information, documentation, notes, or background on ANY topic, technology, platform, project, or keyword (for example: "给我xxx相关信息", "告诉我xxx", "查一下xxx", "xxx怎么配置", "关于xxx的调研"):
   - **DO NOT** assume the knowledge does not exist.
   - **DO NOT** answer solely from pre-training without checking `recall_memory`.
   - **DO NOT** call unrelated tools or external web search before checking `recall_memory`.
   - **ALWAYS** call `recall_memory(query: "<topic_keyword>")` first.
2. **Context & Setup Lookups**:
   - Whenever asked about previous experiments (e.g. Colab, GPU setups, benchmarks, A100 evaluations, models, Docker, Redis).
   - Whenever asked how something was configured, deployed, or previously solved.
3. **Session Bootstrap**:
   - When starting work on a specific project or repo, invoke `get_project_loadout(project_name: "<name>")` to fetch architectural invariants (<300 tokens).
4. **Knowledge Crystallization**:
   - After solving a complex bug, making a non-trivial architectural choice, or discovering an important workaround, invoke `flush_session` or `distill_session_skill` to persist the solution.
"#;
            let _ = std::fs::write(instructions_path, instructions_content);
        }
    }
}

/// Runs the installation and configuration process according to the provided `options`.
pub fn run_install(
    options: InstallOptions,
) -> Result<InstallReport, Box<dyn std::error::Error + Send + Sync>> {
    let vault_path = if let Ok(canon) = std::fs::canonicalize(&options.vault_path) {
        canon
    } else if options.vault_path.is_absolute() {
        options.vault_path.clone()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(&options.vault_path))
            .unwrap_or_else(|_| options.vault_path.clone())
    };

    let clients = options.target.clients();
    let mut outcomes = Vec::new();

    for client in clients {
        let config_path = client
            .config_path(options.home_override.as_deref())
            .ok_or_else(|| format!("Could not resolve config path for {}", client.name()))?;

        let file_exists = config_path.exists();
        let created_file = !file_exists;

        let existing_content = if file_exists {
            Some(std::fs::read_to_string(&config_path)?)
        } else {
            None
        };

        let already_configured =
            check_already_configured(existing_content.as_deref(), &vault_path, client);

        let merged_json =
            inject_k0maru_mcp_for_client(existing_content.as_deref(), &vault_path, client)?;

        let (updated, preview_json) = if options.dry_run {
            (false, Some(merged_json))
        } else {
            if let Some(parent) = config_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let content_with_newline = if merged_json.ends_with('\n') {
                merged_json
            } else {
                format!("{}\n", merged_json)
            };

            let tmp_path = config_path.with_extension(format!("tmp.{}", std::process::id()));
            std::fs::write(&tmp_path, &content_with_newline)?;
            if std::fs::rename(&tmp_path, &config_path).is_err() {
                std::fs::write(&config_path, &content_with_newline)?;
                let _ = std::fs::remove_file(&tmp_path);
            }
            if client == McpClient::Gemini {
                ensure_gemini_instructions(&config_path);
            }
            (true, None)
        };

        outcomes.push(ClientInstallOutcome {
            client,
            config_path,
            created_file,
            already_configured,
            updated,
            dry_run: options.dry_run,
            preview_json,
        });
    }

    Ok(InstallReport {
        vault_path,
        dry_run: options.dry_run,
        outcomes,
    })
}

/// Formats an `InstallReport` for human-friendly terminal display.
pub fn format_install_report(report: &InstallReport) -> String {
    let mut out = String::new();
    out.push_str("🔧 K0maru MCP Client Configurator\n");
    out.push_str("==================================================\n");
    out.push_str(&format!(
        "📁 Target Vault: {}\n",
        report.vault_path.display()
    ));
    if report.dry_run {
        out.push_str("🔍 Mode: DRY RUN (no files modified)\n");
    } else {
        out.push_str("🚀 Mode: LIVE INSTALL\n");
    }
    out.push_str("--------------------------------------------------\n");

    for outcome in &report.outcomes {
        let status_icon = if report.dry_run { "🔍" } else { "✓" };

        let action_str = if report.dry_run {
            if outcome.created_file {
                "Would create file and inject k0maru-memory"
            } else if outcome.already_configured {
                "Already configured with this vault (would refresh entry)"
            } else {
                "Would update config and inject k0maru-memory"
            }
        } else if outcome.created_file {
            "Created config file and injected k0maru-memory"
        } else if outcome.already_configured {
            "Refreshed k0maru-memory configuration (already configured)"
        } else {
            "Injected k0maru-memory into existing configuration"
        };

        out.push_str(&format!(
            "{} {} ({})\n   → {}\n",
            status_icon,
            outcome.client.name(),
            outcome.config_path.display(),
            action_str
        ));

        if let Some(ref preview) = outcome.preview_json {
            out.push_str("   [Preview]:\n");
            let lines: Vec<&str> = preview.lines().collect();
            for line in lines.iter().take(15) {
                out.push_str(&format!("     {}\n", line));
            }
            if lines.len() > 15 {
                out.push_str("     ...\n");
            }
        }
    }

    out.push_str("--------------------------------------------------\n");
    if report.dry_run {
        out.push_str("💡 Run without --dry-run to apply these changes.\n");
    } else {
        out.push_str("🎉 Installation complete! Restart your AI agents to load k0maru-memory.\n");
        out.push_str("💡 Verify with: k0maru doctor\n");
    }
    out
}

/// Strips JSONC syntax (`//` line comments, `/* ... */` block comments and
/// trailing commas) outside of string literals so `.claude/settings.local.json`
/// files written by hand still parse.
fn strip_jsonc_comments(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;

    while let Some(c) = chars.next() {
        if in_line_comment {
            if c == '\n' {
                in_line_comment = false;
                out.push(c);
            }
            continue;
        }
        if in_block_comment {
            if c == '*' && chars.peek() == Some(&'/') {
                chars.next();
                in_block_comment = false;
            }
            continue;
        }
        if in_string {
            out.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }

        match c {
            '"' => {
                in_string = true;
                out.push(c);
            }
            '/' => match chars.peek() {
                Some('/') => {
                    chars.next();
                    in_line_comment = true;
                }
                Some('*') => {
                    chars.next();
                    in_block_comment = true;
                }
                _ => out.push(c),
            },
            _ => out.push(c),
        }
    }

    strip_trailing_commas(&out)
}

/// Removes commas that directly precede a closing `}` or `]`.
fn strip_trailing_commas(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut in_string = false;
    let mut escaped = false;
    let mut idx = 0;

    while idx < chars.len() {
        let c = chars[idx];
        if in_string {
            out.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            idx += 1;
            continue;
        }
        if c == '"' {
            in_string = true;
            out.push(c);
            idx += 1;
            continue;
        }
        if c == ',' {
            let mut next = idx + 1;
            while next < chars.len() && chars[next].is_whitespace() {
                next += 1;
            }
            if next < chars.len() && (chars[next] == '}' || chars[next] == ']') {
                idx += 1;
                continue;
            }
        }
        out.push(c);
        idx += 1;
    }

    out
}

/// Renders the `--vault` value used inside `.claude/settings.local.json`.
///
/// A vault inside the project is referenced with a project-relative path
/// (e.g. `.k0maru/wiki`) because Claude Code runs hooks from the project root;
/// this keeps the committed settings portable across machines and directory
/// moves. A vault outside the project keeps its absolute path.
fn portable_claude_vault_arg(repo_path: &Path, vault_path: &Path) -> String {
    let canonical_repo = crate::guard::canonicalize_best_effort(repo_path);
    let canonical_vault = crate::guard::canonicalize_best_effort(vault_path);

    let relative = canonical_vault
        .strip_prefix(&canonical_repo)
        .ok()
        .or_else(|| vault_path.strip_prefix(repo_path).ok());

    relative
        .filter(|relative| !relative.as_os_str().is_empty())
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|| vault_path.to_string_lossy().to_string())
}

/// Registers (or upgrades) a k0maru command inside a Claude Code hook event.
///
/// Accepts every schema shape Claude Code has shipped (bare string, object
/// with `command`, object with a nested `hooks` array). An existing k0maru
/// entry is rewritten in place to the current command, which makes repeated
/// installs idempotent and upgrades stale flags/paths.
fn upsert_claude_hook_command(
    hooks_obj: &mut serde_json::Map<String, serde_json::Value>,
    event: &str,
    command_marker: &str,
    command: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if !hooks_obj.contains_key(event) || !hooks_obj[event].is_array() {
        hooks_obj.insert(event.to_string(), serde_json::json!([]));
    }

    let entries = hooks_obj
        .get_mut(event)
        .and_then(|value| value.as_array_mut())
        .ok_or_else(|| format!("Field 'hooks.{}' must be a JSON array", event))?;

    let command_value = || serde_json::Value::String(command.to_string());
    let mut configured = false;

    for entry in entries.iter_mut() {
        if let Some(nested) = entry
            .get_mut("hooks")
            .and_then(|value| value.as_array_mut())
        {
            for hook in nested.iter_mut() {
                let matches = hook
                    .get("command")
                    .and_then(|value| value.as_str())
                    .map(|value| value.contains(command_marker))
                    .unwrap_or(false);
                if matches {
                    if let Some(obj) = hook.as_object_mut() {
                        obj.insert("command".to_string(), command_value());
                    }
                    configured = true;
                }
            }
        }

        if entry
            .get("command")
            .and_then(|value| value.as_str())
            .map(|value| value.contains(command_marker))
            .unwrap_or(false)
        {
            if let Some(obj) = entry.as_object_mut() {
                obj.insert("command".to_string(), command_value());
            }
            configured = true;
        }

        let bare_match = entry
            .as_str()
            .map(|value| value.contains(command_marker))
            .unwrap_or(false);
        if bare_match {
            *entry = command_value();
            configured = true;
        }
    }

    if !configured {
        // Claude Code's official hook schema groups commands under a `hooks`
        // array on a matcher entry.
        entries.push(serde_json::json!({
            "matcher": "",
            "hooks": [
                {
                    "type": "command",
                    "command": command
                }
            ]
        }));
    }

    Ok(())
}

/// Creates or safely merges `<repo_path>/.claude/settings.local.json`, registering a
/// non-destructive `hooks.SessionStart` command entry that automatically briefs Claude Code
/// with the sub-300-token project loadout at session initiation, plus a `hooks.Stop`
/// guard check that blocks the turn on architectural invariant violations.
pub fn install_claude_project_hooks(
    repo_path: &Path,
    vault_path: &Path,
    project_slug: &str,
) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let claude_dir = repo_path.join(".claude");
    std::fs::create_dir_all(&claude_dir)?;

    let config_path = claude_dir.join("settings.local.json");
    let mut root: serde_json::Value = if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)?;
        if content.trim().is_empty() {
            serde_json::json!({})
        } else {
            match serde_json::from_str::<serde_json::Value>(&strip_jsonc_comments(&content)) {
                Ok(value) if value.is_object() => value,
                _ => return Err(
                    "Existing .claude/settings.local.json is malformed JSON; refusing to overwrite"
                        .into(),
                ),
            }
        }
    } else {
        serde_json::json!({})
    };

    let root_obj = root
        .as_object_mut()
        .ok_or("Root settings must be a JSON object")?;

    if !root_obj.contains_key("hooks") || !root_obj["hooks"].is_object() {
        root_obj.insert("hooks".to_string(), serde_json::json!({}));
    }

    let hooks_obj = root_obj
        .get_mut("hooks")
        .and_then(|v| v.as_object_mut())
        .ok_or("Field 'hooks' must be a JSON object")?;

    let vault_arg = portable_claude_vault_arg(repo_path, vault_path);
    // Claude Code hooks run with the GUI shell PATH on macOS, which often omits
    // `~/.local/bin` / `~/.cargo/bin`; export both so `k0maru` always resolves.
    let loadout_cmd = format!(
        "export PATH=\"$HOME/.local/bin:$HOME/.cargo/bin:$PATH\"; k0maru loadout \"{}\" --diff --vault \"{}\"",
        project_slug, vault_arg
    );
    let stop_cmd = format!(
        "export PATH=\"$HOME/.local/bin:$HOME/.cargo/bin:$PATH\"; k0maru guard check --diff --claude-stop-hook --vault \"{}\"",
        vault_arg
    );

    upsert_claude_hook_command(hooks_obj, "SessionStart", "k0maru loadout", &loadout_cmd)?;
    upsert_claude_hook_command(
        hooks_obj,
        "Stop",
        "guard check --diff --claude-stop-hook",
        &stop_cmd,
    )?;

    let formatted = serde_json::to_string_pretty(&root)?;
    let content_with_newline = if formatted.ends_with('\n') {
        formatted
    } else {
        format!("{}\n", formatted)
    };

    let tmp_path = config_path.with_extension(format!("tmp.{}", std::process::id()));
    std::fs::write(&tmp_path, &content_with_newline)?;
    if std::fs::rename(&tmp_path, &config_path).is_err() {
        std::fs::write(&config_path, &content_with_newline)?;
        let _ = std::fs::remove_file(&tmp_path);
    }

    Ok(config_path)
}

/// Installs proactive repository hooks: Git pre-commit guard (if `.git` is present)
/// and Claude Code `.claude/settings.local.json` SessionStart loadout briefing hook.
pub fn install_repo_hooks(
    repo_path: &Path,
    vault_path: &Path,
    project_slug: &str,
) -> (Option<PathBuf>, Option<PathBuf>) {
    let guard_hook = if repo_path.join(".git").exists() {
        crate::guard::run_guard_install(crate::guard::GuardInstallArgs {
            hook: "pre-commit".to_string(),
            path: Some(repo_path.to_path_buf()),
            vault: Some(vault_path.to_path_buf()),
        })
        .ok()
    } else {
        None
    };

    let claude_hook = install_claude_project_hooks(repo_path, vault_path, project_slug).ok();

    (guard_hook, claude_hook)
}
