use std::path::Path;

use crate::core::models::Document;
use crate::guard::models::{GuardRule, RuleCondition, RuleSeverity};

/// Extracts guard invariant rules declared in a document's frontmatter and markdown body.
pub fn extract_rules_from_document(doc: &Document) -> Vec<GuardRule> {
    let mut rules = Vec::new();
    let source_note = if !doc.path.as_os_str().is_empty() {
        doc.path.to_string_lossy().to_string()
    } else {
        doc.title.clone()
    };

    // 1. Extract rules from YAML frontmatter
    if let Some(frontmatter_map) = doc.frontmatter.as_object() {
        let invariants_val = frontmatter_map
            .get("invariants")
            .or_else(|| frontmatter_map.get("guard_rules"))
            .or_else(|| frontmatter_map.get("rules"));

        if let Some(invariants_array) = invariants_val.and_then(|v| v.as_array()) {
            for (idx, item) in invariants_array.iter().enumerate() {
                if let Some(obj) = item.as_object() {
                    let id = obj
                        .get("id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| format!("{}-{}", doc.title, idx + 1));

                    let title = obj
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&id)
                        .to_string();

                    let file_pattern = obj
                        .get("file_pattern")
                        .or_else(|| obj.get("files"))
                        .or_else(|| obj.get("scope"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    let severity = obj
                        .get("severity")
                        .and_then(|v| v.as_str())
                        .map(|s| match s.to_lowercase().as_str() {
                            "warn" => RuleSeverity::Warn,
                            _ => RuleSeverity::Deny,
                        })
                        .unwrap_or(RuleSeverity::Deny);

                    let remediation = obj
                        .get("remediation")
                        .or_else(|| obj.get("fix"))
                        .or_else(|| obj.get("description"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    let must_contain = obj.get("must_contain").and_then(|v| v.as_str());
                    let forbidden = obj.get("forbidden").and_then(|v| v.as_str());

                    if let Some(must) = must_contain {
                        rules.push(GuardRule {
                            id: id.clone(),
                            title: title.clone(),
                            source_note: source_note.clone(),
                            file_pattern: file_pattern.clone(),
                            condition: RuleCondition::MustContain(must.to_string()),
                            severity,
                            remediation: remediation.clone(),
                        });
                    }

                    if let Some(forbid) = forbidden {
                        rules.push(GuardRule {
                            id: id.clone(),
                            title: title.clone(),
                            source_note: source_note.clone(),
                            file_pattern: file_pattern.clone(),
                            condition: RuleCondition::Forbidden(forbid.to_string()),
                            severity,
                            remediation: remediation.clone(),
                        });
                    }
                }
            }
        }
    }

    // 2. Extract rules from markdown body sections (## Non-Negotiable Invariants or ## 核心工程铁律)
    let body_rules = extract_rules_from_body(&doc.body, &source_note);
    rules.extend(body_rules);

    rules
}

/// Extracts rules from markdown body heuristic sections.
fn extract_rules_from_body(body: &str, source_note: &str) -> Vec<GuardRule> {
    let mut rules = Vec::new();
    let lines: Vec<&str> = body.lines().collect();
    let mut in_invariant_section = false;
    let mut section_depth = 0;
    let mut in_code_block = false;
    let mut code_block_lang = String::new();
    let mut code_block_content = String::new();

    for line in lines {
        let trimmed = line.trim();

        // Check code blocks
        if trimmed.starts_with("```") {
            if in_code_block {
                in_code_block = false;
                if in_invariant_section
                    && (code_block_lang.contains("yaml") || code_block_lang.contains("yml"))
                {
                    if let Ok(yaml_val) =
                        serde_yaml::from_str::<serde_json::Value>(&code_block_content)
                    {
                        if let Some(arr) = yaml_val.as_array() {
                            for (idx, item) in arr.iter().enumerate() {
                                if let Some(obj) = item.as_object() {
                                    let id = obj
                                        .get("id")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string())
                                        .unwrap_or_else(|| format!("BODY-RULE-{}", idx + 1));
                                    let title = obj
                                        .get("title")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or(&id)
                                        .to_string();
                                    let file_pattern = obj
                                        .get("file_pattern")
                                        .or_else(|| obj.get("files"))
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string());
                                    let severity = obj
                                        .get("severity")
                                        .and_then(|v| v.as_str())
                                        .map(|s| match s.to_lowercase().as_str() {
                                            "warn" => RuleSeverity::Warn,
                                            _ => RuleSeverity::Deny,
                                        })
                                        .unwrap_or(RuleSeverity::Deny);
                                    let remediation = obj
                                        .get("remediation")
                                        .or_else(|| obj.get("fix"))
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    if let Some(must) =
                                        obj.get("must_contain").and_then(|v| v.as_str())
                                    {
                                        rules.push(GuardRule {
                                            id: id.clone(),
                                            title: title.clone(),
                                            source_note: source_note.to_string(),
                                            file_pattern: file_pattern.clone(),
                                            condition: RuleCondition::MustContain(must.to_string()),
                                            severity,
                                            remediation: remediation.clone(),
                                        });
                                    }
                                    if let Some(forbid) =
                                        obj.get("forbidden").and_then(|v| v.as_str())
                                    {
                                        rules.push(GuardRule {
                                            id: id.clone(),
                                            title: title.clone(),
                                            source_note: source_note.to_string(),
                                            file_pattern: file_pattern.clone(),
                                            condition: RuleCondition::Forbidden(forbid.to_string()),
                                            severity,
                                            remediation: remediation.clone(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
                code_block_content.clear();
                code_block_lang.clear();
            } else {
                in_code_block = true;
                code_block_lang = trimmed.trim_start_matches('`').trim().to_lowercase();
                code_block_content.clear();
            }
            continue;
        }

        if in_code_block {
            code_block_content.push_str(line);
            code_block_content.push('\n');
            continue;
        }

        // Check headings
        if trimmed.starts_with('#') {
            let heading_level = trimmed.chars().take_while(|&c| c == '#').count();
            let heading_text = trimmed[heading_level..].trim().to_lowercase();

            if heading_text.contains("non-negotiable invariants")
                || heading_text.contains("核心工程铁律")
                || heading_text.contains("architectural invariants")
                || heading_text.contains("架构铁律")
            {
                in_invariant_section = true;
                section_depth = heading_level;
                continue;
            } else if in_invariant_section && heading_level <= section_depth {
                in_invariant_section = false;
            }
        }

        if in_invariant_section && (trimmed.starts_with('-') || trimmed.starts_with('*')) {
            let item_text = trimmed[1..].trim();
            if let Some(extracted) = parse_markdown_list_rule(item_text, source_note) {
                rules.extend(extracted);
            }
        }
    }

    rules
}

/// Helper to parse a single bullet item in an invariant section.
/// Supports formats:
/// - `[ID] (severity) [file_pattern] forbidden: "pattern" | remediation: "..."`
/// - `**ID**: files: "pattern", forbidden: "pattern", severity: deny, remediation: "..."`
fn parse_markdown_list_rule(text: &str, source_note: &str) -> Option<Vec<GuardRule>> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut id = String::new();
    let mut title = String::new();
    let mut file_pattern = None;
    let mut severity = RuleSeverity::Deny;
    let mut must_contain = None;
    let mut forbidden = None;
    let mut remediation = String::new();

    // Check if starts with `[ID]`
    if let Some(rest) = trimmed.strip_prefix('[') {
        if let Some((id_part, mut after_id)) = rest.split_once(']') {
            id = id_part.trim().to_string();
            after_id = after_id.trim();

            // Check for optional `(severity)`
            if after_id.starts_with('(') {
                if let Some((sev_part, rest_sev)) = after_id[1..].split_once(')') {
                    if sev_part.trim().eq_ignore_ascii_case("warn") {
                        severity = RuleSeverity::Warn;
                    }
                    after_id = rest_sev.trim();
                }
            }

            // Check for optional `[file_pattern]`
            if after_id.starts_with('[') {
                if let Some((pat_part, rest_pat)) = after_id[1..].split_once(']') {
                    file_pattern = Some(pat_part.trim().to_string());
                    after_id = rest_pat.trim();
                }
            }

            // Parse key-values in after_id (separated by `|` or `,`)
            parse_key_value_tokens(
                after_id,
                &mut file_pattern,
                &mut severity,
                &mut must_contain,
                &mut forbidden,
                &mut remediation,
                &mut title,
            );
        }
    } else if let Some(rest) = trimmed.strip_prefix("**") {
        if let Some((id_part, after_id)) = rest.split_once("**") {
            let mut id_clean = id_part.trim();
            if id_clean.starts_with('[') && id_clean.ends_with(']') {
                id_clean = &id_clean[1..id_clean.len() - 1];
            }
            id = id_clean.trim().to_string();
            let after_clean = after_id.trim_start_matches(':').trim();

            parse_key_value_tokens(
                after_clean,
                &mut file_pattern,
                &mut severity,
                &mut must_contain,
                &mut forbidden,
                &mut remediation,
                &mut title,
            );
        }
    } else {
        // Fallback: check if line contains forbidden: or must_contain:
        if trimmed.contains("forbidden:") || trimmed.contains("must_contain:") {
            id = format!("RULE-{}", &crate::core::models::HierarchyLevel::L3Evergreen);
            parse_key_value_tokens(
                trimmed,
                &mut file_pattern,
                &mut severity,
                &mut must_contain,
                &mut forbidden,
                &mut remediation,
                &mut title,
            );
        }
    }

    if id.is_empty() || (must_contain.is_none() && forbidden.is_none()) {
        return None;
    }

    if title.is_empty() {
        title = id.clone();
    }

    let mut rules = Vec::new();
    if let Some(must) = must_contain {
        rules.push(GuardRule {
            id: id.clone(),
            title: title.clone(),
            source_note: source_note.to_string(),
            file_pattern: file_pattern.clone(),
            condition: RuleCondition::MustContain(must),
            severity,
            remediation: remediation.clone(),
        });
    }

    if let Some(forbid) = forbidden {
        rules.push(GuardRule {
            id: id.clone(),
            title: title.clone(),
            source_note: source_note.to_string(),
            file_pattern: file_pattern.clone(),
            condition: RuleCondition::Forbidden(forbid),
            severity,
            remediation: remediation.clone(),
        });
    }

    Some(rules)
}

/// Helper to parse key-value tokens like `files: "*.rs", forbidden: "pattern", remediation: "..."`
fn parse_key_value_tokens(
    text: &str,
    file_pattern: &mut Option<String>,
    severity: &mut RuleSeverity,
    must_contain: &mut Option<String>,
    forbidden: &mut Option<String>,
    remediation: &mut String,
    title: &mut String,
) {
    let mut parts: Vec<&str> = Vec::new();
    if text.contains('|') {
        parts = text.split('|').collect();
    } else {
        // Split by comma outside quotes
        let mut cur_start = 0;
        let mut in_quote = false;
        let mut quote_char = '"';
        for (i, c) in text.char_indices() {
            if (c == '"' || c == '\'') && (!in_quote || c == quote_char) {
                in_quote = !in_quote;
                quote_char = c;
            } else if c == ',' && !in_quote {
                parts.push(&text[cur_start..i]);
                cur_start = i + 1;
            }
        }
        parts.push(&text[cur_start..]);
    }

    for part in parts {
        let trimmed = part.trim();
        if let Some((k, v)) = trimmed.split_once(':') {
            let key = k.trim().to_lowercase();
            let val = clean_token_value(v.trim());

            match key.as_str() {
                "files" | "file" | "file_pattern" | "scope" => {
                    *file_pattern = Some(val);
                }
                "severity" | "level" => {
                    if val.eq_ignore_ascii_case("warn") {
                        *severity = RuleSeverity::Warn;
                    } else {
                        *severity = RuleSeverity::Deny;
                    }
                }
                "must_contain" | "must" | "require" => {
                    *must_contain = Some(val);
                }
                "forbidden" | "forbid" | "deny" => {
                    *forbidden = Some(val);
                }
                "remediation" | "fix" | "action" => {
                    *remediation = val;
                }
                "title" => {
                    *title = val;
                }
                _ => {}
            }
        }
    }
}

/// Cleans quotes or backticks from token value.
fn clean_token_value(raw: &str) -> String {
    let trimmed = raw.trim();
    let unquoted = if (trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2)
        || (trimmed.starts_with('\'') && trimmed.ends_with('\'') && trimmed.len() >= 2)
        || (trimmed.starts_with('`') && trimmed.ends_with('`') && trimmed.len() >= 2)
    {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    };
    unquoted.replace(r"\\", r"\")
}

/// Searches vault root for guard configuration files (`.k0maru/guard.toml`, `guard.yaml`, etc.)
/// and extracts declared rules.
pub fn extract_rules_from_vault(
    vault_root: &Path,
) -> Result<Vec<GuardRule>, Box<dyn std::error::Error>> {
    let candidates = [
        vault_root.join(".k0maru").join("guard.toml"),
        vault_root.join(".k0maru").join("guard.yaml"),
        vault_root.join(".k0maru").join("guard.yml"),
        vault_root.join("guard.toml"),
        vault_root.join("guard.yaml"),
        vault_root.join("guard.yml"),
    ];

    for candidate in &candidates {
        if candidate.exists() {
            return extract_rules_from_config(candidate);
        }
    }

    Ok(Vec::new())
}

/// Extracts guard rules from a standalone `.toml` or `.yaml` configuration file.
pub fn extract_rules_from_config(
    config_path: &Path,
) -> Result<Vec<GuardRule>, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(config_path)?;
    let source_note = config_path.to_string_lossy().to_string();
    let ext = config_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    let mut rules = Vec::new();

    if ext == "toml" {
        let toml_val: toml::Value = toml::from_str(&content)?;
        let rules_val = toml_val.get("rules").or_else(|| toml_val.get("invariants"));

        if let Some(arr) = rules_val.and_then(|v| v.as_array()) {
            for (idx, item) in arr.iter().enumerate() {
                if let Some(table) = item.as_table() {
                    let id = table
                        .get("id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| format!("CONF-{}", idx + 1));
                    let title = table
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&id)
                        .to_string();
                    let file_pattern = table
                        .get("file_pattern")
                        .or_else(|| table.get("files"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let severity = table
                        .get("severity")
                        .and_then(|v| v.as_str())
                        .map(|s| match s.to_lowercase().as_str() {
                            "warn" => RuleSeverity::Warn,
                            _ => RuleSeverity::Deny,
                        })
                        .unwrap_or(RuleSeverity::Deny);
                    let remediation = table
                        .get("remediation")
                        .or_else(|| table.get("fix"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    if let Some(must) = table.get("must_contain").and_then(|v| v.as_str()) {
                        rules.push(GuardRule {
                            id: id.clone(),
                            title: title.clone(),
                            source_note: source_note.clone(),
                            file_pattern: file_pattern.clone(),
                            condition: RuleCondition::MustContain(must.to_string()),
                            severity,
                            remediation: remediation.clone(),
                        });
                    }

                    if let Some(forbid) = table.get("forbidden").and_then(|v| v.as_str()) {
                        rules.push(GuardRule {
                            id: id.clone(),
                            title: title.clone(),
                            source_note: source_note.clone(),
                            file_pattern: file_pattern.clone(),
                            condition: RuleCondition::Forbidden(forbid.to_string()),
                            severity,
                            remediation: remediation.clone(),
                        });
                    }
                }
            }
        }
    } else {
        // YAML parser
        let yaml_val: serde_json::Value = serde_yaml::from_str(&content)?;
        let rules_val = yaml_val
            .get("rules")
            .or_else(|| yaml_val.get("invariants"))
            .or_else(|| {
                if yaml_val.is_array() {
                    Some(&yaml_val)
                } else {
                    None
                }
            });

        if let Some(arr) = rules_val.and_then(|v| v.as_array()) {
            for (idx, item) in arr.iter().enumerate() {
                if let Some(obj) = item.as_object() {
                    let id = obj
                        .get("id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| format!("CONF-{}", idx + 1));
                    let title = obj
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&id)
                        .to_string();
                    let file_pattern = obj
                        .get("file_pattern")
                        .or_else(|| obj.get("files"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let severity = obj
                        .get("severity")
                        .and_then(|v| v.as_str())
                        .map(|s| match s.to_lowercase().as_str() {
                            "warn" => RuleSeverity::Warn,
                            _ => RuleSeverity::Deny,
                        })
                        .unwrap_or(RuleSeverity::Deny);
                    let remediation = obj
                        .get("remediation")
                        .or_else(|| obj.get("fix"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    if let Some(must) = obj.get("must_contain").and_then(|v| v.as_str()) {
                        rules.push(GuardRule {
                            id: id.clone(),
                            title: title.clone(),
                            source_note: source_note.clone(),
                            file_pattern: file_pattern.clone(),
                            condition: RuleCondition::MustContain(must.to_string()),
                            severity,
                            remediation: remediation.clone(),
                        });
                    }

                    if let Some(forbid) = obj.get("forbidden").and_then(|v| v.as_str()) {
                        rules.push(GuardRule {
                            id: id.clone(),
                            title: title.clone(),
                            source_note: source_note.clone(),
                            file_pattern: file_pattern.clone(),
                            condition: RuleCondition::Forbidden(forbid.to_string()),
                            severity,
                            remediation: remediation.clone(),
                        });
                    }
                }
            }
        }
    }

    Ok(rules)
}
