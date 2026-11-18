//! Persistent user & cockpit configuration (`~/.config/k0maru/config.toml`).
//!
//! Supports zero-network TOML serialization/deserialization and hot-swappable
//! vault path tracking (`vault_path` + `recent_vaults`).

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Persistent configuration stored at `~/.config/k0maru/config.toml`
/// (or overridden via `K0MARU_CONFIG_PATH`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct K0maruConfig {
    /// Default active markdown vault / LLM-Wiki path when running outside a repo vault.
    pub vault_path: Option<PathBuf>,
    /// Recently opened/switched vaults (MRU order, max 10).
    pub recent_vaults: Vec<PathBuf>,
    /// Preferred local ONNX embedding model alias (`bge-small` or `bge-m3`).
    pub embedding_model: String,
    /// Default retrieval mode (`hybrid`, `fts`, or `vector`).
    pub default_search_mode: String,
    /// Maximum token budget for `k0maru loadout` / `get_project_loadout`.
    pub loadout_token_budget: usize,
    /// Whether `k0maru guard check` treats warnings as strict failures by default.
    pub guard_strict: bool,
    /// Whether passive lifecycle hooks (`PostToolUseFailure`, `Stop`, `SessionEnd`) record observations.
    pub passive_capture: bool,
    /// Default port for `k0maru ui`.
    pub ui_port: u16,
}

impl Default for K0maruConfig {
    fn default() -> Self {
        Self {
            vault_path: None,
            recent_vaults: Vec::new(),
            embedding_model: "bge-small".to_string(),
            default_search_mode: "hybrid".to_string(),
            loadout_token_budget: 300,
            guard_strict: false,
            passive_capture: true,
            ui_port: 3721,
        }
    }
}

impl K0maruConfig {
    /// Resolves the active configuration file path (`K0MARU_CONFIG_PATH` or `~/.config/k0maru/config.toml`).
    pub fn config_file_path() -> PathBuf {
        if let Ok(custom) = std::env::var("K0MARU_CONFIG_PATH") {
            let trimmed = custom.trim();
            if !trimmed.is_empty() {
                return PathBuf::from(trimmed);
            }
        }
        if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
            return home.join(".config").join("k0maru").join("config.toml");
        }
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("k0maru")
            .join("config.toml")
    }

    /// Returns true if `K0MARU_CONFIG_PATH` is explicitly set in the environment.
    pub fn has_explicit_env_path() -> bool {
        std::env::var("K0MARU_CONFIG_PATH")
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false)
    }

    /// Loads configuration from `Self::config_file_path()`, returning `Self::default()` if absent.
    pub fn load() -> Self {
        let path = Self::config_file_path();
        let mut cfg = Self::load_from(&path).unwrap_or_default();
        if !Self::has_explicit_env_path() {
            cfg.recent_vaults.retain(|p| {
                let s = p.to_string_lossy();
                !s.contains("/var/folders/") && !s.contains("\\Temp\\") && p.is_dir()
            });
            if let Some(vp) = &cfg.vault_path {
                let s = vp.to_string_lossy();
                if s.contains("/var/folders/") || s.contains("\\Temp\\") || !vp.is_dir() {
                    cfg.vault_path = None;
                }
            }
        }
        cfg
    }

    /// Loads configuration from a specific file path.
    pub fn load_from(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        if !path.is_file() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path)?;
        Ok(Self::from_toml_str(&raw)?)
    }

    /// Saves configuration to `Self::config_file_path()`.
    pub fn save(&self) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let path = Self::config_file_path();
        if !Self::has_explicit_env_path() {
            if let Some(vp) = &self.vault_path {
                let s = vp.to_string_lossy();
                if s.contains("/var/folders/") || s.contains("\\Temp\\") {
                    return Ok(path);
                }
            }
        }
        self.save_to(&path)?;
        Ok(path)
    }

    /// Saves configuration to a specific file path.
    pub fn save_to(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, self.to_toml_string())?;
        Ok(())
    }

    /// Records a newly activated vault path and updates the MRU `recent_vaults` list (capped at 10).
    pub fn record_vault(&mut self, vault: &Path) {
        let normalized = vault.canonicalize().unwrap_or_else(|_| vault.to_path_buf());
        self.vault_path = Some(normalized.clone());
        self.recent_vaults.retain(|p| p != &normalized);
        self.recent_vaults.insert(0, normalized);
        if self.recent_vaults.len() > 10 {
            self.recent_vaults.truncate(10);
        }
    }

    /// Serializes configuration to deterministic TOML text.
    pub fn to_toml_string(&self) -> String {
        let mut lines = Vec::new();
        lines.push("# K0maru Agent Memory Hub Configuration".to_string());
        if let Some(vp) = &self.vault_path {
            lines.push(format!(
                "vault_path = {}",
                escape_toml_str(&vp.to_string_lossy())
            ));
        }
        let recent_items: Vec<String> = self
            .recent_vaults
            .iter()
            .map(|p| escape_toml_str(&p.to_string_lossy()))
            .collect();
        lines.push(format!("recent_vaults = [{}]", recent_items.join(", ")));
        lines.push(format!(
            "embedding_model = {}",
            escape_toml_str(&self.embedding_model)
        ));
        lines.push(format!(
            "default_search_mode = {}",
            escape_toml_str(&self.default_search_mode)
        ));
        lines.push(format!(
            "loadout_token_budget = {}",
            self.loadout_token_budget
        ));
        lines.push(format!("guard_strict = {}", self.guard_strict));
        lines.push(format!("passive_capture = {}", self.passive_capture));
        lines.push(format!("ui_port = {}", self.ui_port));
        lines.push(String::new());
        lines.join("\n")
    }

    /// Parses configuration from TOML text.
    pub fn from_toml_str(input: &str) -> Result<Self, String> {
        let mut cfg = Self::default();
        for raw_line in input.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key_raw, val_raw)) = line.split_once('=') else {
                continue;
            };
            let key = key_raw.trim();
            let val = val_raw.trim();
            match key {
                "vault_path" => {
                    let s = unquote_toml_str(val);
                    cfg.vault_path = if s.is_empty() {
                        None
                    } else {
                        Some(PathBuf::from(s))
                    };
                }
                "recent_vaults" => {
                    cfg.recent_vaults = parse_toml_string_array(val)
                        .into_iter()
                        .filter(|s| !s.is_empty())
                        .map(PathBuf::from)
                        .collect();
                }
                "embedding_model" => {
                    let s = unquote_toml_str(val);
                    if !s.is_empty() {
                        cfg.embedding_model = s;
                    }
                }
                "default_search_mode" => {
                    let s = unquote_toml_str(val);
                    if !s.is_empty() {
                        cfg.default_search_mode = s;
                    }
                }
                "loadout_token_budget" => {
                    if let Ok(n) = val.parse::<usize>() {
                        cfg.loadout_token_budget = n.clamp(50, 4096);
                    }
                }
                "guard_strict" => {
                    if let Ok(b) = val.parse::<bool>() {
                        cfg.guard_strict = b;
                    }
                }
                "passive_capture" => {
                    if let Ok(b) = val.parse::<bool>() {
                        cfg.passive_capture = b;
                    }
                }
                "ui_port" => {
                    if let Ok(p) = val.parse::<u16>() {
                        if p > 0 {
                            cfg.ui_port = p;
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(cfg)
    }
}

fn escape_toml_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn unquote_toml_str(val: &str) -> String {
    let trimmed = val.trim();
    if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
        let inner = &trimmed[1..trimmed.len() - 1];
        let mut out = String::with_capacity(inner.len());
        let mut chars = inner.chars();
        while let Some(ch) = chars.next() {
            if ch == '\\' {
                match chars.next() {
                    Some('\\') => out.push('\\'),
                    Some('"') => out.push('"'),
                    Some('n') => out.push('\n'),
                    Some('r') => out.push('\r'),
                    Some('t') => out.push('\t'),
                    Some(other) => {
                        out.push('\\');
                        out.push(other);
                    }
                    None => out.push('\\'),
                }
            } else {
                out.push(ch);
            }
        }
        out
    } else if trimmed.len() >= 2 && trimmed.starts_with('\'') && trimmed.ends_with('\'') {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    }
}

fn parse_toml_string_array(val: &str) -> Vec<String> {
    let trimmed = val.trim();
    let inner = trimmed
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .unwrap_or(trimmed)
        .trim();
    if inner.is_empty() {
        return Vec::new();
    }
    let mut items = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut escaped = false;
    for ch in inner.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' && in_quotes {
            current.push(ch);
            escaped = true;
            continue;
        }
        if ch == '"' {
            in_quotes = !in_quotes;
            current.push(ch);
            continue;
        }
        if ch == ',' && !in_quotes {
            items.push(unquote_toml_str(&current));
            current.clear();
        } else {
            current.push(ch);
        }
    }
    if !current.trim().is_empty() {
        items.push(unquote_toml_str(&current));
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_k0maru_config_toml_roundtrip_and_mru() {
        let tmp = tempdir().unwrap();
        let cfg_path = tmp.path().join("config.toml");
        let vault_a = tmp.path().join("wiki-alpha");
        let vault_b = tmp.path().join("wiki-beta");
        fs::create_dir_all(&vault_a).unwrap();
        fs::create_dir_all(&vault_b).unwrap();

        let mut cfg = K0maruConfig::default();
        cfg.record_vault(&vault_a);
        cfg.record_vault(&vault_b);
        cfg.embedding_model = "bge-m3".to_string();
        cfg.default_search_mode = "fts".to_string();
        cfg.loadout_token_budget = 250;
        cfg.guard_strict = true;
        cfg.passive_capture = false;
        cfg.ui_port = 4120;

        cfg.save_to(&cfg_path).unwrap();
        let loaded = K0maruConfig::load_from(&cfg_path).unwrap();
        assert_eq!(loaded, cfg);
        assert_eq!(loaded.recent_vaults.len(), 2);
        assert_eq!(
            loaded.vault_path.as_ref().unwrap(),
            &vault_b.canonicalize().unwrap()
        );
    }
}
