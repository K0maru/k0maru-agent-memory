use std::path::Path;

use crate::core::models::Document;
use crate::guard::models::{GuardRule, RuleCondition, RuleSeverity};

/// Helper to parse a single JSON-compatible rule object map into one or more GuardRules.
pub fn parse_rule_object(
    obj: &serde_json::Map<String, serde_json::Value>,
    default_id: String,
    source_note: &str,
) -> Vec<GuardRule> {
    let id = obj
        .get("id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or(default_id);

    let title = obj
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or(&id)
        .to_string();

    let extract_pattern = |v: &serde_json::Value| -> Option<String> {
        if let Some(s) = v.as_str() {
            Some(s.to_string())
        } else if let Some(arr) = v.as_array() {
            let strs: Vec<&str> = arr.iter().filter_map(|x| x.as_str()).collect();
            if !strs.is_empty() {
                Some(strs.join(","))
            } else {
                None
            }
        } else {
            None
        }
    };

    let mut file_pattern = obj
        .get("file_pattern")
        .or_else(|| obj.get("files"))
        .or_else(|| obj.get("scope"))
        .or_else(|| obj.get("paths"))
        .and_then(extract_pattern);

    let nested_condition = obj.get("condition").and_then(|v| v.as_object());
    let has_condition =
        nested_condition.is_some() || obj.get("condition").and_then(|v| v.as_str()).is_some();
    let has_forbid_or_must = obj.get("forbidden").is_some()
        || obj.get("forbid").is_some()
        || obj.get("must_contain").is_some()
        || nested_condition
            .map(|map| {
                map.contains_key("forbidden")
                    || map.contains_key("forbid")
                    || map.contains_key("must_contain")
            })
            .unwrap_or(false);

    if file_pattern.is_none() && !has_condition && has_forbid_or_must {
        if let Some(pat) = obj.get("pattern").and_then(|v| v.as_str()) {
            file_pattern = Some(pat.to_string());
        }
    }

    let severity = obj
        .get("severity")
        .or_else(|| obj.get("level"))
        .and_then(|v| v.as_str())
        .map(|s| match s.to_lowercase().as_str() {
            "warn" | "warning" | "info" | "notice" => RuleSeverity::Warn,
            _ => RuleSeverity::Deny,
        })
        .unwrap_or(RuleSeverity::Deny);

    let remediation = obj
        .get("remediation")
        .or_else(|| obj.get("fix"))
        .or_else(|| obj.get("fix_hint"))
        .or_else(|| obj.get("description"))
        .or_else(|| obj.get("action"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Collects one rule per declared value, suffixing the id when a single
    // declaration lists multiple patterns (arrays or nested condition maps).
    let push_values = |rules: &mut Vec<GuardRule>, values: Vec<String>, is_forbidden: bool| {
        let multiple = values.len() > 1;
        for (idx, value) in values.into_iter().enumerate() {
            let rule_id = if multiple {
                format!("{}-{}", id, idx + 1)
            } else {
                id.clone()
            };
            let condition = if is_forbidden {
                RuleCondition::Forbidden(value)
            } else {
                RuleCondition::MustContain(value)
            };
            rules.push(GuardRule {
                id: rule_id,
                title: title.clone(),
                source_note: source_note.to_string(),
                file_pattern: file_pattern.clone(),
                condition,
                severity,
                remediation: remediation.clone(),
            });
        }
    };

    let mut rules = Vec::new();

    if let Some(values) = obj.get("must_contain").map(value_to_patterns) {
        push_values(&mut rules, values, false);
    }

    if let Some(values) = obj
        .get("forbidden")
        .or_else(|| obj.get("forbid"))
        .map(value_to_patterns)
    {
        push_values(&mut rules, values, true);
    }

    if let (Some(cond), Some(pat)) = (
        obj.get("condition").and_then(|v| v.as_str()),
        obj.get("pattern").and_then(|v| v.as_str()),
    ) {
        let cond_lower = cond.to_lowercase();
        match cond_lower.as_str() {
            "forbidden" | "forbid" | "deny" => {
                push_values(&mut rules, vec![pat.to_string()], true);
            }
            "must_contain" | "must" | "require" => {
                push_values(&mut rules, vec![pat.to_string()], false);
            }
            _ => {}
        }
    }

    // Nested condition map, e.g.
    //   condition:
    //     forbidden: [redis_setnx]
    //     must_contain: [acquire_fenced_lease]
    if let Some(cond_map) = nested_condition {
        if let Some(values) = cond_map.get("must_contain").map(value_to_patterns) {
            push_values(&mut rules, values, false);
        }
        if let Some(values) = cond_map
            .get("forbidden")
            .or_else(|| cond_map.get("forbid"))
            .map(value_to_patterns)
        {
            push_values(&mut rules, values, true);
        }
    }

    rules
}

/// Normalizes a rule value into a list of patterns, accepting either a single
/// string or an array of strings.
fn value_to_patterns(value: &serde_json::Value) -> Vec<String> {
    match value {
        serde_json::Value::String(s) => vec![s.to_string()],
        serde_json::Value::Array(items) => items
            .iter()
            .filter_map(|item| item.as_str().map(|s| s.to_string()))
            .collect(),
        _ => Vec::new(),
    }
}

/// Extracts guard invariant rules declared in a document's frontmatter and markdown body.
pub fn extract_rules_from_document(doc: &Document) -> Vec<GuardRule> {
    if !doc.is_active() {
        return Vec::new();
    }

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
                    let default_id = format!("{}-{}", doc.title, idx + 1);
                    rules.extend(parse_rule_object(obj, default_id, &source_note));
                }
            }
        }
    }

    // 2. Extract rules from markdown body sections (## Non-Negotiable Invariants or ## 核心工程铁律)
    let body_rules = extract_rules_from_body(&doc.body, &source_note);
    rules.extend(body_rules);
    for rule in &mut rules {
        if rule.remediation.is_empty() {
            rule.remediation = match &rule.condition {
                RuleCondition::MustContain(m) => {
                    format!("Ensure changes include required pattern `{}`.", m)
                }
                RuleCondition::Forbidden(f) => {
                    format!("Remove or replace forbidden pattern `{}`.", f)
                }
            };
        }
    }
    let supersedes = doc.supersedes();
    if !supersedes.is_empty() {
        let provenance = format!(" (Supersedes {})", supersedes.join(", "));
        for rule in &mut rules {
            if !rule.remediation.contains("Supersedes ") {
                rule.remediation.push_str(&provenance);
            }
        }
    }

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
                                    let default_id = format!("BODY-RULE-{}", idx + 1);
                                    rules.extend(parse_rule_object(obj, default_id, source_note));
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
            if let Some(extracted) = parse_markdown_list_rule(trimmed, source_note) {
                rules.extend(extracted);
            }
        }
    }

    rules
}

/// Bracket-balanced helper that requires `s.starts_with('[')` and returns
/// the content inside balanced outer brackets and the remainder of the string.
fn take_bracketed(s: &str) -> Option<(&str, &str)> {
    if !s.starts_with('[') {
        return None;
    }
    let mut depth = 0;
    for (idx, c) in s.char_indices() {
        if c == '[' {
            depth += 1;
        } else if c == ']' {
            depth -= 1;
            if depth == 0 {
                return Some((&s[1..idx], &s[idx + 1..]));
            }
        }
    }
    None
}

/// Helper to parse a single bullet item in an invariant section.
/// Supports formats:
/// - `[ID] (severity) [file_pattern] forbidden: "pattern" | remediation: "..."`
/// - `**ID**: files: "pattern", forbidden: "pattern", severity: deny, remediation: "..."`
pub fn parse_markdown_list_rule(text: &str, source_note: &str) -> Option<Vec<GuardRule>> {
    let t = text.trim();
    let trimmed = if let Some(r) = t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")) {
        r.trim()
    } else if t.starts_with('-') && !t.starts_with("--") {
        t[1..].trim()
    } else {
        t
    };
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
    if let Some((id_part, mut after_id)) = take_bracketed(trimmed) {
        id = id_part.trim().to_string();
        after_id = after_id.trim();

        // Check for optional `(severity)`
        if after_id.starts_with('(') {
            if let Some((sev_part, rest_sev)) = after_id[1..].split_once(')') {
                let s = sev_part.trim().to_lowercase();
                if s == "warn" || s == "warning" || s == "info" || s == "notice" {
                    severity = RuleSeverity::Warn;
                }
                after_id = rest_sev.trim();
            }
        }

        // Check for optional `[file_pattern]`
        if after_id.starts_with('[') {
            if let Some((pat_part, rest_pat)) = take_bracketed(after_id) {
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
        if must_contain.is_none() && forbidden.is_none() {
            parse_natural_language_tokens(
                after_id,
                &mut file_pattern,
                &mut must_contain,
                &mut forbidden,
                &mut remediation,
            );
        }
    } else if let Some(rest) = trimmed.strip_prefix("**") {
        if let Some((id_part, after_id)) = rest.split_once("**") {
            let mut id_clean = id_part.trim();
            if id_clean.starts_with('[') && id_clean.ends_with(']') {
                id_clean = &id_clean[1..id_clean.len() - 1];
            }
            id = id_clean.trim().to_string();
            let mut after_clean = after_id.trim_start_matches(':').trim();

            // Check for optional `(severity)`
            if after_clean.starts_with('(') {
                if let Some((sev_part, rest_sev)) = after_clean[1..].split_once(')') {
                    let s = sev_part.trim().to_lowercase();
                    if s == "warn" || s == "warning" || s == "info" || s == "notice" {
                        severity = RuleSeverity::Warn;
                    }
                    after_clean = rest_sev.trim_start_matches(':').trim();
                }
            }

            // Check for optional `[file_pattern]`
            if after_clean.starts_with('[') {
                if let Some((pat_part, rest_pat)) = take_bracketed(after_clean) {
                    file_pattern = Some(pat_part.trim().to_string());
                    after_clean = rest_pat.trim_start_matches(':').trim();
                }
            }

            parse_key_value_tokens(
                after_clean,
                &mut file_pattern,
                &mut severity,
                &mut must_contain,
                &mut forbidden,
                &mut remediation,
                &mut title,
            );
            if must_contain.is_none() && forbidden.is_none() {
                parse_natural_language_tokens(
                    after_clean,
                    &mut file_pattern,
                    &mut must_contain,
                    &mut forbidden,
                    &mut remediation,
                );
            }
        }
    } else {
        // Fallback: check if line contains forbidden: or must_contain:
        let trimmed_lower = trimmed.to_ascii_lowercase();
        if trimmed_lower.contains("forbidden:")
            || trimmed_lower.contains("forbid:")
            || trimmed_lower.contains("deny:")
            || trimmed_lower.contains("must_contain:")
            || trimmed_lower.contains("mustcontain:")
            || trimmed_lower.contains("must-contain:")
            || trimmed_lower.contains("must:")
            || trimmed_lower.contains("require:")
            || trimmed_lower.starts_with("forbid ")
            || trimmed_lower.starts_with("forbidden ")
            || trimmed_lower.starts_with("deny ")
            || trimmed_lower.starts_with("require ")
            || trimmed_lower.starts_with("must contain ")
        {
            let stem = Path::new(source_note)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("anon");
            let sum: u32 = trimmed
                .bytes()
                .fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
            id = format!("RULE-{}-{:04x}", stem, sum & 0xffff);
            parse_key_value_tokens(
                trimmed,
                &mut file_pattern,
                &mut severity,
                &mut must_contain,
                &mut forbidden,
                &mut remediation,
                &mut title,
            );
            if must_contain.is_none() && forbidden.is_none() {
                parse_natural_language_tokens(
                    trimmed,
                    &mut file_pattern,
                    &mut must_contain,
                    &mut forbidden,
                    &mut remediation,
                );
            }
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
/// Checks if a string starts with a valid rule key identifier followed by a colon.
fn starts_with_rule_key(s: &str) -> bool {
    let trimmed = s.trim_start();
    let ident_end = trimmed
        .find(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '-')
        .unwrap_or(0);
    if ident_end == 0 {
        return false;
    }
    let ident = &trimmed[..ident_end];
    let after_ident = trimmed[ident_end..].trim_start();
    if !after_ident.starts_with(':') {
        return false;
    }
    let key_clean = ident.to_lowercase();
    matches!(
        key_clean.as_str(),
        "files"
            | "file"
            | "file_pattern"
            | "scope"
            | "paths"
            | "severity"
            | "level"
            | "must_contain"
            | "mustcontain"
            | "must-contain"
            | "must"
            | "require"
            | "forbidden"
            | "forbid"
            | "deny"
            | "remediation"
            | "fix"
            | "fix_hint"
            | "description"
            | "action"
            | "title"
            | "id"
            | "condition"
            | "pattern"
    )
}

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
    let mut cur_start = 0;
    let mut in_quote: Option<char> = None;
    let mut escaped = false;

    for (i, c) in text.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if c == '\\' {
            escaped = true;
            continue;
        }

        if let Some(q) = in_quote {
            if c == q {
                in_quote = None;
            }
        } else if c == '"' || c == '\'' || c == '`' {
            in_quote = Some(c);
        } else if (c == '|' || c == ',') && starts_with_rule_key(&text[i + c.len_utf8()..]) {
            parts.push(&text[cur_start..i]);
            cur_start = i + c.len_utf8();
        }
    }
    parts.push(&text[cur_start..]);

    for part in parts {
        let trimmed = part.trim();
        if let Some((k, v)) = trimmed.split_once(':') {
            let key = k.trim().to_lowercase();
            let val = clean_token_value(v.trim());

            match key.as_str() {
                "files" | "file" | "file_pattern" | "scope" | "paths" => {
                    *file_pattern = Some(val);
                }
                "severity" | "level" => {
                    let v_lower = val.to_lowercase();
                    if v_lower == "warn"
                        || v_lower == "warning"
                        || v_lower == "info"
                        || v_lower == "notice"
                    {
                        *severity = RuleSeverity::Warn;
                    } else {
                        *severity = RuleSeverity::Deny;
                    }
                }
                "must_contain" | "mustcontain" | "must-contain" | "must" | "require" => {
                    *must_contain = Some(val);
                }
                "forbidden" | "forbid" | "deny" => {
                    *forbidden = Some(val);
                }
                "remediation" | "fix" | "fix_hint" | "description" | "action" => {
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

/// Fallback parser for natural-language invariant bullets without colon after the verb, e.g.:
/// `- [ADR-010] Forbid `legacy_lock()` in `src/**/*.rs` — Remediation: Use distributed_lock().`
/// `- [ADR-011] Require `acquire_fenced_lease()` in `src/**/*.rs` — Remediation: Use fenced leases.`
fn parse_natural_language_tokens(
    text: &str,
    file_pattern: &mut Option<String>,
    must_contain: &mut Option<String>,
    forbidden: &mut Option<String>,
    remediation: &mut String,
) {
    let mut main_clause = text.trim();
    for sep in ["—", " -- ", " | ", "; "] {
        if let Some((before, after)) = main_clause.split_once(sep) {
            let after_trim = after.trim();
            let after_lower = after_trim.to_ascii_lowercase();
            for prefix in ["remediation:", "remediation ", "fix:", "fix_hint:"] {
                if after_lower.starts_with(prefix) {
                    let rem = after_trim[prefix.len()..].trim();
                    if !rem.is_empty() && remediation.is_empty() {
                        *remediation = clean_token_value(rem);
                    }
                    main_clause = before.trim();
                    break;
                }
            }
        }
    }

    let lower = main_clause.to_ascii_lowercase();
    let (is_forbid, rest) = if let Some(r) = lower.strip_prefix("forbidden ") {
        (true, &main_clause[main_clause.len() - r.len()..])
    } else if let Some(r) = lower.strip_prefix("forbid ") {
        (true, &main_clause[main_clause.len() - r.len()..])
    } else if let Some(r) = lower.strip_prefix("deny ") {
        (true, &main_clause[main_clause.len() - r.len()..])
    } else if let Some(r) = lower.strip_prefix("must contain ") {
        (false, &main_clause[main_clause.len() - r.len()..])
    } else if let Some(r) = lower.strip_prefix("must_contain ") {
        (false, &main_clause[main_clause.len() - r.len()..])
    } else if let Some(r) = lower.strip_prefix("require ") {
        (false, &main_clause[main_clause.len() - r.len()..])
    } else {
        return;
    };

    let rest = rest.trim();
    let (pat, after_pat) = take_quoted_or_token(rest);
    if pat.is_empty() {
        return;
    }
    if is_forbid {
        *forbidden = Some(pat);
    } else {
        *must_contain = Some(pat);
    }

    let after_trim = after_pat.trim();
    let after_lower = after_trim.to_ascii_lowercase();
    if let Some(in_rest) = after_lower.strip_prefix("in ") {
        let orig_in_rest = &after_trim[after_trim.len() - in_rest.len()..].trim();
        let (glob_val, _) = take_quoted_or_token(orig_in_rest);
        if !glob_val.is_empty() && file_pattern.is_none() {
            *file_pattern = Some(glob_val.trim_end_matches('.').to_string());
        }
    }
}

fn take_quoted_or_token(s: &str) -> (String, &str) {
    let trimmed = s.trim_start();
    if let Some(first) = trimmed.chars().next() {
        if first == '`' || first == '"' || first == '\'' {
            let after_open = &trimmed[first.len_utf8()..];
            if let Some(end_idx) = after_open.find(first) {
                let val = &after_open[..end_idx];
                let remainder = &after_open[end_idx + first.len_utf8()..];
                return (val.to_string(), remainder);
            }
        }
    }
    let end_idx = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
    (clean_token_value(&trimmed[..end_idx]), &trimmed[end_idx..])
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
                    let default_id = format!("CONF-{}", idx + 1);
                    if let Ok(serde_json::Value::Object(map)) = serde_json::to_value(table) {
                        rules.extend(parse_rule_object(&map, default_id, &source_note));
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
                    let default_id = format!("CONF-{}", idx + 1);
                    rules.extend(parse_rule_object(obj, default_id, &source_note));
                }
            }
        }
    }

    Ok(rules)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::models::{Document, HierarchyLevel};
    use serde_json::json;
    use std::path::PathBuf;

    #[test]
    fn test_superseded_document_emits_no_guard_rules() {
        let doc = Document {
            path: PathBuf::from("ADR-001.md"),
            title: "ADR 001 Old".to_string(),
            hierarchy: HierarchyLevel::L3Evergreen,
            frontmatter: json!({
                "status": "superseded",
                "rules": [
                    {
                        "id": "RULE-OLD",
                        "title": "Old Invariant",
                        "forbidden": "legacy_api()",
                        "severity": "deny"
                    }
                ]
            }),
            links: Vec::new(),
            tags: vec!["status/superseded".to_string()],
            content_hash: "hash".to_string(),
            mtime: 0,
            body: "## Non-Negotiable Invariants\n- [RULE-BODY] (deny) [*.rs] forbidden: \"old_call()\" | remediation: \"do not call\"".to_string(),
        };

        let rules = extract_rules_from_document(&doc);
        assert!(rules.is_empty());
    }

    #[test]
    fn test_superseding_document_attaches_provenance_to_remediation() {
        let doc = Document {
            path: PathBuf::from("ADR-002.md"),
            title: "ADR 002 New".to_string(),
            hierarchy: HierarchyLevel::L3Evergreen,
            frontmatter: json!({
                "status": "active",
                "supersedes": ["ADR-001.md"],
                "rules": [
                    {
                        "id": "RULE-NEW",
                        "title": "New Invariant",
                        "forbidden": "new_forbidden()",
                        "severity": "deny",
                        "remediation": "Use updated API"
                    }
                ]
            }),
            links: Vec::new(),
            tags: Vec::new(),
            content_hash: "hash2".to_string(),
            mtime: 0,
            body: String::new(),
        };

        let rules = extract_rules_from_document(&doc);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].id, "RULE-NEW");
        assert_eq!(
            rules[0].remediation,
            "Use updated API (Supersedes ADR-001.md)"
        );
    }
}
