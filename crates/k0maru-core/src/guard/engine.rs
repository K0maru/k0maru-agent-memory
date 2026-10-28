use std::path::{Path, PathBuf};
use std::time::Instant;

use regex::Regex;

use crate::guard::models::{GuardReport, GuardRule, GuardViolation, RuleCondition};

/// Compiled rule with pre-compiled regex for ultra-fast repeated evaluation.
struct CompiledRule<'a> {
    rule: &'a GuardRule,
    regex: Option<Regex>,
    allow_substring_fallback: bool,
}

impl<'a> CompiledRule<'a> {
    fn new(rule: &'a GuardRule) -> Self {
        let raw_pattern = match &rule.condition {
            RuleCondition::MustContain(p) => p.as_str(),
            RuleCondition::Forbidden(p) => p.as_str(),
        };

        let is_bare_ident = !raw_pattern.is_empty()
            && raw_pattern
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
            && !raw_pattern.ends_with('_')
            && !raw_pattern.starts_with('_');

        if is_bare_ident {
            let re_str = format!(r"\b{}\b", raw_pattern);
            let regex = Regex::new(&re_str).ok();
            Self {
                rule,
                regex,
                allow_substring_fallback: false,
            }
        } else if raw_pattern.ends_with("()") && !raw_pattern.ends_with(r"\()") {
            let base = &raw_pattern[..raw_pattern.len() - 2];
            let escaped_base = regex::escape(base);
            let pat = if base
                .chars()
                .next()
                .map(|c| c.is_ascii_alphanumeric() || c == '_')
                .unwrap_or(false)
            {
                format!(r"\b{}\s*\(", escaped_base)
            } else {
                format!(r"{}\s*\(", escaped_base)
            };
            let regex = Regex::new(&pat).ok();
            Self {
                rule,
                regex,
                allow_substring_fallback: false,
            }
        } else {
            let has_char_class = raw_pattern.contains("[0-")
                || raw_pattern.contains("[1-")
                || raw_pattern.contains("[a-")
                || raw_pattern.contains("[A-")
                || raw_pattern.contains("[^")
                || raw_pattern.contains("]+")
                || raw_pattern.contains("]*")
                || raw_pattern.contains("]?")
                || raw_pattern.contains("]{");
            let has_regex_meta = raw_pattern.contains('\\')
                || raw_pattern.contains('|')
                || raw_pattern.contains('^')
                || raw_pattern.contains('$')
                || raw_pattern.contains(".*")
                || raw_pattern.contains(".+")
                || raw_pattern.contains(")?")
                || raw_pattern.contains('{')
                || raw_pattern.contains('}')
                || has_char_class
                || raw_pattern.contains('+')
                || raw_pattern.contains('?');

            if !has_regex_meta {
                let escaped = regex::escape(raw_pattern);
                let regex = Regex::new(&escaped).ok();
                Self {
                    rule,
                    regex,
                    allow_substring_fallback: true,
                }
            } else {
                let regex = Regex::new(raw_pattern).ok();
                Self {
                    rule,
                    regex,
                    allow_substring_fallback: false,
                }
            }
        }
    }

    fn is_match(&self, text: &str) -> bool {
        let pat = match &self.rule.condition {
            RuleCondition::MustContain(p) => p.as_str(),
            RuleCondition::Forbidden(p) => p.as_str(),
        };
        if let Some(re) = &self.regex {
            re.is_match(text) || (self.allow_substring_fallback && text.contains(pat))
        } else {
            text.contains(pat)
        }
    }
}

fn matches_single_pattern(pattern: &str, path_str: &str, path: &Path) -> bool {
    let file_name = path.file_name().and_then(|f| f.to_str()).unwrap_or("");

    // 1. If pattern does not contain '/', match against file_name or full path
    if !pattern.contains('/') {
        if let Ok(glob_pat) = glob::Pattern::new(pattern) {
            if glob_pat.matches(file_name) || glob_pat.matches(path_str) {
                return true;
            }
        }
    }

    // 2. Direct glob match
    if let Ok(glob_pat) = glob::Pattern::new(pattern) {
        if glob_pat.matches(path_str) {
            return true;
        }
    }

    // 3. Try with **/ prepended if not already starting with ** or /
    if !pattern.starts_with("**") && !pattern.starts_with('/') {
        let prefixed = format!("**/{}", pattern);
        if let Ok(glob_pat) = glob::Pattern::new(&prefixed) {
            if glob_pat.matches(path_str) {
                return true;
            }
        }
    }

    false
}

/// Splits a pattern on top-level commas, ignoring commas nested inside `{...}`.
fn split_top_level_commas(pattern: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut current = String::new();

    for ch in pattern.chars() {
        match ch {
            '{' => {
                depth += 1;
                current.push(ch);
            }
            '}' => {
                depth = depth.saturating_sub(1);
                current.push(ch);
            }
            ',' if depth == 0 => {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    parts.push(trimmed.to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    let trimmed = current.trim();
    if !trimmed.is_empty() {
        parts.push(trimmed.to_string());
    }

    parts
}

/// Recursively expands `{a,b}` alternatives inside a single glob pattern.
fn expand_brace_expression(pattern: &str, out: &mut Vec<String>) {
    if let Some(open) = pattern.find('{') {
        if let Some(close_rel) = pattern[open..].find('}') {
            let close = open + close_rel;
            let prefix = &pattern[..open];
            let suffix = &pattern[close + 1..];
            for alternative in pattern[open + 1..close].split(',') {
                let expanded = format!("{}{}{}", prefix, alternative.trim(), suffix);
                expand_brace_expression(&expanded, out);
            }
            return;
        }
    }
    out.push(pattern.to_string());
}

/// Expands brace expressions (e.g. `src/**/*.{rs,ts,py}`) and splits on
/// top-level commas into individual glob patterns.
fn expand_brace_patterns(pattern: &str) -> Vec<String> {
    let mut expanded = Vec::new();
    for part in split_top_level_commas(pattern) {
        expand_brace_expression(&part, &mut expanded);
    }
    expanded
}

/// Matches a file path against an optional glob pattern.
fn matches_file_pattern(pattern_opt: Option<&str>, path: &Path) -> bool {
    let path_str = path.to_string_lossy().replace('\\', "/");

    let pattern_has_k0maru = pattern_opt.map(|p| p.contains(".k0maru")).unwrap_or(false);

    let is_markdown = path_str.ends_with(".md");
    let is_under_src_or_lib = path_str.starts_with("src/")
        || path_str.contains("/src/")
        || path_str.starts_with("lib/")
        || path_str.contains("/lib/");
    let is_vault_dir = path_str.starts_with("10_Projects/")
        || path_str.starts_with("20_Cards/")
        || path_str.starts_with("01_AI_Logs/")
        || path_str.starts_with("00_Daily/")
        || path_str.starts_with(".k0maru/wiki/")
        || path_str.contains("/10_Projects/")
        || path_str.contains("/20_Cards/")
        || path_str.contains("/01_AI_Logs/")
        || path_str.contains("/00_Daily/")
        || path_str.contains("/.k0maru/wiki/");

    let is_exempt_markdown = is_markdown && !is_under_src_or_lib && is_vault_dir;

    let is_k0maru_rules = path_str == ".k0maru/rules"
        || path_str.starts_with(".k0maru/rules/")
        || path_str.ends_with("/.k0maru/rules")
        || path_str.contains("/.k0maru/rules/");

    let is_guard_or_gitignore = path_str == ".k0maru/guard.toml"
        || path_str.ends_with("/.k0maru/guard.toml")
        || path_str == ".k0maru/.gitignore"
        || path_str.ends_with("/.k0maru/.gitignore");

    let is_cache_sqlite = path_str.ends_with("cache.sqlite")
        || path_str.ends_with("cache.sqlite-wal")
        || path_str.ends_with("cache.sqlite-shm");

    let is_internal_wiki_or_cache =
        is_exempt_markdown || is_guard_or_gitignore || is_k0maru_rules || is_cache_sqlite;

    if is_internal_wiki_or_cache && !pattern_has_k0maru {
        return false;
    }

    let pattern = match pattern_opt {
        Some(p) if !p.trim().is_empty() => p.trim(),
        _ => return true,
    };

    if pattern == "*" || pattern == "**/*" {
        return true;
    }

    for sub_pat in expand_brace_patterns(pattern) {
        let sub = sub_pat.trim();
        if sub.is_empty() {
            continue;
        }
        if sub == "*" || sub == "**/*" {
            return true;
        }
        if matches_single_pattern(sub, &path_str, path) {
            return true;
        }
    }

    false
}

/// Strips comments from code lines to avoid flagging commentary.
fn strip_code_comment(line: &str, file_path: &Path) -> String {
    let file_name = file_path.file_name().and_then(|f| f.to_str()).unwrap_or("");
    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();

    if ext == "md" || ext == "txt" || ext == "rst" {
        return line.to_string();
    }

    let is_hash_comment_lang = matches!(
        ext.as_str(),
        "py" | "sh"
            | "bash"
            | "zsh"
            | "rb"
            | "pl"
            | "toml"
            | "yaml"
            | "yml"
            | "conf"
            | "env"
            | "r"
            | "mk"
    ) || file_name == "Makefile"
        || file_name == "Dockerfile";

    let mut out = String::with_capacity(line.len());
    let mut in_quote: Option<char> = None;
    let mut escaped = false;
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if escaped {
            escaped = false;
            out.push(c);
            i += 1;
            continue;
        }

        if c == '\\' {
            escaped = true;
            out.push(c);
            i += 1;
            continue;
        }

        if let Some(q) = in_quote {
            if c == q {
                in_quote = None;
            }
            out.push(c);
            i += 1;
            continue;
        }

        if c == '"' || c == '\'' || c == '`' {
            in_quote = Some(c);
            out.push(c);
            i += 1;
            continue;
        }

        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
            let mut j = i + 2;
            let mut closed = false;
            while j + 1 < chars.len() {
                if chars[j] == '*' && chars[j + 1] == '/' {
                    closed = true;
                    i = j + 2;
                    break;
                }
                j += 1;
            }
            if !closed {
                break;
            }
            out.push(' ');
            continue;
        }

        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '/' && !is_hash_comment_lang {
            break;
        }

        if c == '#' && is_hash_comment_lang && (i == 0 || chars[i - 1].is_whitespace()) {
            break;
        }

        out.push(c);
        i += 1;
    }

    out
}

/// Cleans raw diff path: unquotes and strips a/ or b/ prefixes.
fn clean_diff_path(raw: &str) -> Option<PathBuf> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "/dev/null" {
        return None;
    }

    let unquoted = if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    };

    let stripped = if let Some(s) = unquoted.strip_prefix("b/") {
        s
    } else if let Some(s) = unquoted.strip_prefix("a/") {
        s
    } else {
        unquoted
    };

    if stripped.is_empty() || stripped == "/dev/null" {
        None
    } else {
        Some(PathBuf::from(stripped))
    }
}

/// Core Sentinel Engine for Living ADR invariant enforcement and diff auditing.
pub struct GuardEngine;

fn parse_hunk_counts(line: &str) -> Option<(usize, usize, usize)> {
    let trimmed = line.trim();
    if !trimmed.starts_with("@@") {
        return None;
    }
    let rest = trimmed.strip_prefix("@@")?.trim_start();
    let end_idx = rest.find("@@")?;
    let header_content = rest[..end_idx].trim();
    let (minus_part, plus_part) = header_content.split_once('+')?;
    let minus_clean = minus_part.trim().strip_prefix('-')?;

    let old_count = if let Some((_start, count)) = minus_clean.split_once(',') {
        count.trim().parse::<usize>().unwrap_or(1)
    } else {
        1
    };

    let (new_start, new_count) = if let Some((start, count)) = plus_part.trim().split_once(',') {
        let s = start.trim().parse::<usize>().unwrap_or(1);
        let c = count.trim().parse::<usize>().unwrap_or(1);
        (s, c)
    } else {
        let s = plus_part.trim().parse::<usize>().unwrap_or(1);
        (s, 1)
    };

    Some((new_start, old_count, new_count))
}

impl GuardEngine {
    /// Audits a unified git diff against active invariant rules.
    ///
    /// Evaluates added lines (`+`) in diff hunks, records exact line numbers and diff loci,
    /// and completes within < 10ms for typical changesets.
    pub fn check_diff(diff_text: &str, rules: &[GuardRule]) -> GuardReport {
        Self::check_diff_with_reader(diff_text, rules, |path, pred| {
            if path.exists() {
                std::fs::read_to_string(path)
                    .map(|c| pred(&c))
                    .unwrap_or(false)
            } else {
                false
            }
        })
    }

    /// Audits a unified git diff against active invariant rules with a custom file content predicate.
    pub fn check_diff_with_reader<F>(
        diff_text: &str,
        rules: &[GuardRule],
        mut file_satisfies: F,
    ) -> GuardReport
    where
        F: FnMut(&Path, &dyn Fn(&str) -> bool) -> bool,
    {
        let start = Instant::now();
        let compiled_rules: Vec<CompiledRule> = rules.iter().map(CompiledRule::new).collect();

        let mut violations = Vec::new();
        let mut checked_files = 0;
        let mut checked_diff_hunks = 0;

        let mut current_file: Option<PathBuf> = None;
        let mut current_line: usize = 0;
        let mut in_hunk = false;
        let mut is_deleted_file = false;
        let mut added_lines: Vec<(usize, String)> = Vec::new();
        let mut deleted_lines: Vec<(usize, String)> = Vec::new();

        let mut finalize_file = |file: &Option<PathBuf>,
                                 added: &[(usize, String)],
                                 deleted: &[(usize, String)],
                                 is_deleted: bool,
                                 violations: &mut Vec<GuardViolation>,
                                 checked_files: &mut usize| {
            if let Some(file_path) = file {
                *checked_files += 1;
                let file_norm = file_path.to_string_lossy().replace('\\', "/");
                let joined_added = added
                    .iter()
                    .map(|(_, s)| s.as_str())
                    .collect::<Vec<_>>()
                    .join("\n");
                let joined_deleted = deleted
                    .iter()
                    .map(|(_, s)| s.as_str())
                    .collect::<Vec<_>>()
                    .join("\n");

                for cr in &compiled_rules {
                    let src_norm = cr.rule.source_note.replace('\\', "/");
                    if !src_norm.is_empty()
                        && (file_norm == src_norm
                            || file_norm.ends_with(&format!("/{}", src_norm))
                            || src_norm.ends_with(&format!("/{}", file_norm)))
                    {
                        continue;
                    }

                    if !matches_file_pattern(cr.rule.file_pattern.as_deref(), file_path) {
                        continue;
                    }

                    match &cr.rule.condition {
                        RuleCondition::Forbidden(_) => {
                            let pat = match &cr.rule.condition {
                                RuleCondition::MustContain(p) => p.as_str(),
                                RuleCondition::Forbidden(p) => p.as_str(),
                            };
                            let pat_upper = pat.to_ascii_uppercase();
                            let id_upper = cr.rule.id.to_ascii_uppercase();
                            let is_comment_or_conflict_rule = pat.contains("//")
                                || pat.contains("<<<<<<<")
                                || pat.contains("=======")
                                || pat_upper.contains("PRIVATE KEY")
                                || pat_upper.contains("BEGIN ")
                                || pat_upper.contains("SECRET")
                                || pat_upper.contains("AKIA")
                                || pat_upper.starts_with("GHP_")
                                || pat_upper.starts_with("SK-")
                                || id_upper.contains("SECRET")
                                || id_upper.contains("TOKEN")
                                || id_upper.contains("KEY");

                            for (line_no, line_content) in added {
                                let target_text = if is_comment_or_conflict_rule {
                                    line_content.clone()
                                } else {
                                    let code_only = strip_code_comment(line_content, file_path);
                                    if code_only.trim().is_empty() {
                                        continue;
                                    }
                                    code_only
                                };

                                if cr.is_match(&target_text) {
                                    violations.push(GuardViolation {
                                        rule_id: cr.rule.id.clone(),
                                        rule_title: cr.rule.title.clone(),
                                        file_path: file_path.to_string_lossy().to_string(),
                                        line_number: Some(*line_no),
                                        matched_content: Some(line_content.clone()),
                                        severity: cr.rule.severity,
                                        source_note: cr.rule.source_note.clone(),
                                        remediation: cr.rule.remediation.clone(),
                                    });
                                }
                            }
                        }
                        RuleCondition::MustContain(_) => {
                            if is_deleted {
                                continue;
                            }
                            let deleted_required =
                                !deleted.is_empty() && cr.is_match(&joined_deleted);
                            let added_has_it = !added.is_empty() && cr.is_match(&joined_added);
                            let existing_has_it =
                                file_satisfies(file_path, &|content| cr.is_match(content));

                            if (deleted_required && !added_has_it)
                                || (!added.is_empty() && !added_has_it && !existing_has_it)
                            {
                                let line_num = if deleted_required {
                                    deleted.first().map(|(l, _)| *l)
                                } else {
                                    added.first().map(|(l, _)| *l)
                                };
                                violations.push(GuardViolation {
                                    rule_id: cr.rule.id.clone(),
                                    rule_title: cr.rule.title.clone(),
                                    file_path: file_path.to_string_lossy().to_string(),
                                    line_number: line_num,
                                    matched_content: None,
                                    severity: cr.rule.severity,
                                    source_note: cr.rule.source_note.clone(),
                                    remediation: cr.rule.remediation.clone(),
                                });
                            }
                        }
                    }
                }
            }
        };

        let mut hunk_old_remaining: usize = 0;
        let mut hunk_new_remaining: usize = 0;

        for raw_line in diff_text.lines() {
            let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);

            if in_hunk && (hunk_old_remaining > 0 || hunk_new_remaining > 0) {
                if let Some(rest) = line.strip_prefix('+') {
                    added_lines.push((current_line, rest.trim_end().to_string()));
                    current_line += 1;
                    hunk_new_remaining = hunk_new_remaining.saturating_sub(1);
                } else if let Some(rest) = line.strip_prefix('-') {
                    deleted_lines.push((current_line, rest.trim_end().to_string()));
                    hunk_old_remaining = hunk_old_remaining.saturating_sub(1);
                } else if line.starts_with(' ') {
                    current_line += 1;
                    hunk_old_remaining = hunk_old_remaining.saturating_sub(1);
                    hunk_new_remaining = hunk_new_remaining.saturating_sub(1);
                } else if line.is_empty() && hunk_old_remaining > 0 && hunk_new_remaining > 0 {
                    current_line += 1;
                    hunk_old_remaining -= 1;
                    hunk_new_remaining -= 1;
                } else if line.starts_with('\\') {
                    // \ No newline at end of file
                } else {
                    in_hunk = false;
                }

                if hunk_old_remaining == 0 && hunk_new_remaining == 0 {
                    in_hunk = false;
                }

                if in_hunk {
                    continue;
                }
            }

            // Outside active hunk
            if let Some(rest) = line.strip_prefix("diff --git ") {
                finalize_file(
                    &current_file,
                    &added_lines,
                    &deleted_lines,
                    is_deleted_file,
                    &mut violations,
                    &mut checked_files,
                );
                added_lines.clear();
                deleted_lines.clear();
                in_hunk = false;
                is_deleted_file = false;

                if let Some((_a, b)) = rest.split_once(' ') {
                    current_file = clean_diff_path(b);
                } else {
                    current_file = None;
                }
                continue;
            }

            if line == "+++ /dev/null" || line.starts_with("deleted file mode") {
                is_deleted_file = true;
            }

            if let Some(rest) = line.strip_prefix("+++ ") {
                if let Some(cleaned) = clean_diff_path(rest) {
                    current_file = Some(cleaned);
                }
                continue;
            }

            if line.starts_with("@@") {
                if let Some((new_start, old_cnt, new_cnt)) = parse_hunk_counts(line) {
                    current_line = new_start;
                    hunk_old_remaining = old_cnt;
                    hunk_new_remaining = new_cnt;
                    in_hunk = hunk_old_remaining > 0 || hunk_new_remaining > 0;
                    checked_diff_hunks += 1;
                } else {
                    in_hunk = true;
                    checked_diff_hunks += 1;
                    if let Some(plus_idx) = line.find('+') {
                        let after_plus = &line[plus_idx + 1..];
                        let digits: String = after_plus
                            .chars()
                            .take_while(|c| c.is_ascii_digit())
                            .collect();
                        if let Ok(num) = digits.parse::<usize>() {
                            current_line = num;
                        }
                    }
                    hunk_old_remaining = 0;
                    hunk_new_remaining = 0;
                }
                continue;
            }
        }

        finalize_file(
            &current_file,
            &added_lines,
            &deleted_lines,
            is_deleted_file,
            &mut violations,
            &mut checked_files,
        );

        let duration_ms = start.elapsed().as_millis();
        GuardReport::new(checked_files, checked_diff_hunks, violations, duration_ms)
    }

    /// Audits static file contents against active invariant rules.
    pub fn check_files(files: &[(PathBuf, String)], rules: &[GuardRule]) -> GuardReport {
        let start = Instant::now();
        let compiled_rules: Vec<CompiledRule> = rules.iter().map(CompiledRule::new).collect();

        let mut violations = Vec::new();
        let checked_files = files.len();
        let checked_diff_hunks = files.len();

        for (path, content) in files {
            let file_norm = path.to_string_lossy().replace('\\', "/");

            for cr in &compiled_rules {
                let src_norm = cr.rule.source_note.replace('\\', "/");
                if !src_norm.is_empty()
                    && (file_norm == src_norm
                        || file_norm.ends_with(&format!("/{}", src_norm))
                        || src_norm.ends_with(&format!("/{}", file_norm)))
                {
                    continue;
                }

                if !matches_file_pattern(cr.rule.file_pattern.as_deref(), path) {
                    continue;
                }

                match &cr.rule.condition {
                    RuleCondition::Forbidden(_) => {
                        let pat = match &cr.rule.condition {
                            RuleCondition::MustContain(p) => p.as_str(),
                            RuleCondition::Forbidden(p) => p.as_str(),
                        };
                        let pat_upper = pat.to_ascii_uppercase();
                        let id_upper = cr.rule.id.to_ascii_uppercase();
                        let is_comment_or_conflict_rule = pat.contains("//")
                            || pat.contains("<<<<<<<")
                            || pat.contains("=======")
                            || pat_upper.contains("PRIVATE KEY")
                            || pat_upper.contains("BEGIN ")
                            || pat_upper.contains("SECRET")
                            || pat_upper.contains("AKIA")
                            || pat_upper.starts_with("GHP_")
                            || pat_upper.starts_with("SK-")
                            || id_upper.contains("SECRET")
                            || id_upper.contains("TOKEN")
                            || id_upper.contains("KEY");

                        for (idx, line) in content.lines().enumerate() {
                            let line_no = idx + 1;
                            let target_text = if is_comment_or_conflict_rule {
                                line.to_string()
                            } else {
                                let code_only = strip_code_comment(line, path);
                                if code_only.trim().is_empty() {
                                    continue;
                                }
                                code_only
                            };

                            if cr.is_match(&target_text) {
                                violations.push(GuardViolation {
                                    rule_id: cr.rule.id.clone(),
                                    rule_title: cr.rule.title.clone(),
                                    file_path: path.to_string_lossy().to_string(),
                                    line_number: Some(line_no),
                                    matched_content: Some(line.to_string()),
                                    severity: cr.rule.severity,
                                    source_note: cr.rule.source_note.clone(),
                                    remediation: cr.rule.remediation.clone(),
                                });
                            }
                        }
                    }
                    RuleCondition::MustContain(_) => {
                        if !cr.is_match(content) {
                            violations.push(GuardViolation {
                                rule_id: cr.rule.id.clone(),
                                rule_title: cr.rule.title.clone(),
                                file_path: path.to_string_lossy().to_string(),
                                line_number: None,
                                matched_content: None,
                                severity: cr.rule.severity,
                                source_note: cr.rule.source_note.clone(),
                                remediation: cr.rule.remediation.clone(),
                            });
                        }
                    }
                }
            }
        }

        let duration_ms = start.elapsed().as_millis();
        GuardReport::new(checked_files, checked_diff_hunks, violations, duration_ms)
    }
}
