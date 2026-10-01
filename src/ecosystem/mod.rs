//! MCP Client Ecosystem detection and configuration inspection.
//!
//! Provides discovery, path resolution, and configuration status inspection
//! across AI coding agents (Claude Code, Cursor, Antigravity/Gemini CLI, Windsurf, Cline).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Supported AI agent MCP clients in the ecosystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpClient {
    Claude,
    Cursor,
    Gemini,
    Windsurf,
    Cline,
}

impl McpClient {
    /// Human-friendly display name of the MCP client.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Claude => "Claude Code",
            Self::Cursor => "Cursor",
            Self::Gemini => "Antigravity / Gemini CLI",
            Self::Windsurf => "Windsurf",
            Self::Cline => "Cline / Roo Code",
        }
    }

    /// Slice of all supported MCP clients.
    pub fn all() -> &'static [Self] {
        &[
            Self::Claude,
            Self::Cursor,
            Self::Gemini,
            Self::Windsurf,
            Self::Cline,
        ]
    }

    /// Resolves the default configuration file path for the client given an optional home directory.
    ///
    /// If `home_override` is `None`, uses `dirs::home_dir()`.
    pub fn config_path(&self, home_override: Option<&Path>) -> Option<PathBuf> {
        let home = match home_override {
            Some(h) => h.to_path_buf(),
            None => dirs::home_dir()?,
        };

        match self {
            Self::Claude => Some(home.join(".claude.json")),
            Self::Cursor => Some(home.join(".cursor").join("mcp.json")),
            Self::Gemini => {
                let config_json = home.join(".gemini").join("config").join("mcp_config.json");
                let antigravity_json = home
                    .join(".gemini")
                    .join("antigravity-cli")
                    .join("mcp_config.json");

                // If antigravity-cli config exists and primary config doesn't, pick antigravity-cli
                if antigravity_json.exists() && !config_json.exists() {
                    Some(antigravity_json)
                } else {
                    Some(config_json)
                }
            }
            Self::Windsurf => Some(
                home.join(".codeium")
                    .join("windsurf")
                    .join("mcp_config.json"),
            ),
            Self::Cline => {
                // Check possible locations across macOS, Linux, and Windows:
                let macos_dir = home
                    .join("Library")
                    .join("Application Support")
                    .join("Code")
                    .join("User")
                    .join("globalStorage");
                let linux_dir = home
                    .join(".config")
                    .join("Code")
                    .join("User")
                    .join("globalStorage");
                let win_dir = home
                    .join("AppData")
                    .join("Roaming")
                    .join("Code")
                    .join("User")
                    .join("globalStorage");

                // Check existing directories or config files in precedence
                let candidates = [
                    macos_dir
                        .join("saoudrizwan.claude-dev")
                        .join("settings")
                        .join("cline_mcp_settings.json"),
                    macos_dir
                        .join("rooveterinaryinc.roo-cline")
                        .join("settings")
                        .join("cline_mcp_settings.json"),
                    linux_dir
                        .join("saoudrizwan.claude-dev")
                        .join("settings")
                        .join("cline_mcp_settings.json"),
                    linux_dir
                        .join("rooveterinaryinc.roo-cline")
                        .join("settings")
                        .join("cline_mcp_settings.json"),
                    win_dir
                        .join("saoudrizwan.claude-dev")
                        .join("settings")
                        .join("cline_mcp_settings.json"),
                    win_dir
                        .join("rooveterinaryinc.roo-cline")
                        .join("settings")
                        .join("cline_mcp_settings.json"),
                ];

                for candidate in candidates {
                    if candidate.exists() {
                        return Some(candidate);
                    }
                }

                // If none exist, choose default for current OS
                #[cfg(target_os = "macos")]
                {
                    Some(
                        macos_dir
                            .join("saoudrizwan.claude-dev")
                            .join("settings")
                            .join("cline_mcp_settings.json"),
                    )
                }
                #[cfg(target_os = "windows")]
                {
                    Some(
                        win_dir
                            .join("saoudrizwan.claude-dev")
                            .join("settings")
                            .join("cline_mcp_settings.json"),
                    )
                }
                #[cfg(not(any(target_os = "macos", target_os = "windows")))]
                {
                    Some(
                        linux_dir
                            .join("saoudrizwan.claude-dev")
                            .join("settings")
                            .join("cline_mcp_settings.json"),
                    )
                }
            }
        }
    }

    /// Resolves default configuration path using system home directory.
    pub fn default_config_path(&self) -> Option<PathBuf> {
        self.config_path(None)
    }
}

/// Inspection status of a specific MCP client configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientConfigStatus {
    pub client: McpClient,
    pub config_path: PathBuf,
    pub installed: bool,
    pub configured: bool,
    pub vault_path: Option<PathBuf>,
}

/// Alias for spec compatibility.
pub type ClientConfigInfo = ClientConfigStatus;

/// Inspects the configuration and installation status of a given MCP client.
pub fn inspect_client(client: McpClient, home_override: Option<&Path>) -> ClientConfigStatus {
    let home = home_override
        .map(|p| p.to_path_buf())
        .or_else(dirs::home_dir);

    let config_path = client
        .config_path(home_override)
        .unwrap_or_else(|| PathBuf::from(client.name()));

    let home_ref = home.as_deref();

    let installed = check_installed(client, &config_path, home_ref);

    let mut configured = false;
    let mut vault_path = None;

    if config_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                // Look for k0maru-memory or k0maru in mcpServers or top-level
                let server_entry = json
                    .get("mcpServers")
                    .and_then(|s| s.get("k0maru-memory").or_else(|| s.get("k0maru")))
                    .or_else(|| json.get("k0maru-memory").or_else(|| json.get("k0maru")));

                if let Some(entry) = server_entry {
                    configured = true;
                    // Extract vault path from args
                    if let Some(args) = entry.get("args").and_then(|a| a.as_array()) {
                        let mut iter = args.iter();
                        while let Some(arg) = iter.next() {
                            if let Some(arg_str) = arg.as_str() {
                                if arg_str == "--vault" || arg_str == "-v" {
                                    if let Some(next_arg) = iter.next().and_then(|v| v.as_str()) {
                                        vault_path = Some(PathBuf::from(next_arg));
                                        break;
                                    }
                                } else if let Some(val) = arg_str.strip_prefix("--vault=") {
                                    vault_path = Some(PathBuf::from(val));
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    ClientConfigStatus {
        client,
        config_path,
        installed,
        configured,
        vault_path,
    }
}

/// Helper to check if a client application or configuration is installed.
fn check_installed(client: McpClient, config_path: &Path, home: Option<&Path>) -> bool {
    if config_path.exists() {
        return true;
    }

    let home = match home {
        Some(h) => h,
        None => return false,
    };

    match client {
        McpClient::Claude => home.join(".claude.json").exists() || home.join(".claude").exists(),
        McpClient::Cursor => home.join(".cursor").exists(),
        McpClient::Gemini => home.join(".gemini").exists(),
        McpClient::Windsurf => home.join(".codeium").exists(),
        McpClient::Cline => {
            // Check if any VS Code globalStorage cline extension folder exists
            let macos_dir = home
                .join("Library")
                .join("Application Support")
                .join("Code")
                .join("User")
                .join("globalStorage");
            let linux_dir = home
                .join(".config")
                .join("Code")
                .join("User")
                .join("globalStorage");
            let win_dir = home
                .join("AppData")
                .join("Roaming")
                .join("Code")
                .join("User")
                .join("globalStorage");

            [macos_dir, linux_dir, win_dir].iter().any(|dir| {
                dir.join("saoudrizwan.claude-dev").exists()
                    || dir.join("rooveterinaryinc.roo-cline").exists()
            })
        }
    }
}

/// Inspects the configuration and installation status of all supported MCP clients.
pub fn inspect_all_clients(home_override: Option<&Path>) -> Vec<ClientConfigStatus> {
    McpClient::all()
        .iter()
        .map(|client| inspect_client(*client, home_override))
        .collect()
}
