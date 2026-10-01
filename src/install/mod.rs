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
            other => Err(format!(
                "Unknown install target: '{}'. Supported targets: all, claude, cursor, gemini, windsurf, cline",
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
    let server_entry = serde_json::json!({
        "command": "k0maru",
        "args": ["mcp", "--vault", vault_str]
    });

    servers.insert("k0maru-memory".to_string(), server_entry);

    let formatted = serde_json::to_string_pretty(&root)?;
    Ok(formatted)
}

/// Checks whether the existing JSON already mounts `k0maru-memory` targeting `target_vault`.
fn check_already_configured(existing_content: Option<&str>, target_vault: &Path) -> bool {
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

        let already_configured = check_already_configured(existing_content.as_deref(), &vault_path);

        let merged_json = inject_k0maru_mcp(existing_content.as_deref(), &vault_path)?;

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
