use std::path::{Path, PathBuf};
use std::time::Instant;

use regex::Regex;

use crate::guard::models::{GuardReport, GuardRule, GuardViolation, RuleCondition};

/// Compiled rule with pre-compiled regex for ultra-fast repeated evaluation.
struct CompiledRule<'a> {
    rule: &'a GuardRule,
    regex: Option<Regex>,
}

impl<'a> CompiledRule<'a> {
    fn new(rule: &'a GuardRule) -> Self {
        let pattern = match &rule.condition {
            RuleCondition::MustContain(p) => p.as_str(),
            RuleCondition::Forbidden(p) => p.as_str(),
        };
        let regex = Regex::new(pattern).ok();
        Self { rule, regex }
    }

    fn is_match(&self, text: &str) -> bool {
        if let Some(re) = &self.regex {
            re.is_match(text)
        } else {
            let pat = match &self.rule.condition {
                RuleCondition::MustContain(p) => p.as_str(),
                RuleCondition::Forbidden(p) => p.as_str(),
            };
            text.contains(pat)
        }
    }
}

/// Matches a file path against an optional glob pattern.
fn matches_file_pattern(pattern_opt: Option<&str>, path: &Path) -> bool {
    let pattern = match pattern_opt {
        Some(p) if !p.trim().is_empty() => p.trim(),
        _ => return true,
    };

    if pattern == "*" || pattern == "**/*" {
        return true;
    }

    let path_str = path.to_string_lossy();
    let file_name = path.file_name().and_then(|f| f.to_str()).unwrap_or("");

    // 1. If pattern does not contain '/', match against file_name or full path
    if !pattern.contains('/') {
        if let Ok(glob_pat) = glob::Pattern::new(pattern) {
            if glob_pat.matches(file_name) || glob_pat.matches(&path_str) {
                return true;
            }
        }
    }

    // 2. Direct glob match
    if let Ok(glob_pat) = glob::Pattern::new(pattern) {
        if glob_pat.matches(&path_str) {
            return true;
        }
    }

    // 3. Try with **/ prepended if not already starting with ** or /
    if !pattern.starts_with("**") && !pattern.starts_with('/') {
        let prefixed = format!("**/{}", pattern);
        if let Ok(glob_pat) = glob::Pattern::new(&prefixed) {
            if glob_pat.matches(&path_str) {
                return true;
            }
        }
    }

    false
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

impl GuardEngine {
    /// Audits a unified git diff against active invariant rules.
    ///
    /// Evaluates added lines (`+`) in diff hunks, records exact line numbers and diff loci,
    /// and completes within < 10ms for typical changesets.
    pub fn check_diff(diff_text: &str, rules: &[GuardRule]) -> GuardReport {
        let start = Instant::now();
        let compiled_rules: Vec<CompiledRule> = rules.iter().map(CompiledRule::new).collect();

        let mut violations = Vec::new();
        let mut checked_files = 0;
        let mut checked_diff_hunks = 0;

        let mut current_file: Option<PathBuf> = None;
        let mut current_line: usize = 0;
        let mut in_hunk = false;
        let mut added_lines: Vec<(usize, String)> = Vec::new();

        let finalize_file = |file: &Option<PathBuf>,
                             added: &[(usize, String)],
                             violations: &mut Vec<GuardViolation>,
                             checked_files: &mut usize| {
            if let Some(file_path) = file {
                *checked_files += 1;
                let joined_added = added
                    .iter()
                    .map(|(_, s)| s.as_str())
                    .collect::<Vec<_>>()
                    .join("\n");

                for cr in &compiled_rules {
                    if !matches_file_pattern(cr.rule.file_pattern.as_deref(), file_path) {
                        continue;
                    }

                    match &cr.rule.condition {
                        RuleCondition::Forbidden(_) => {
                            for (line_no, line_content) in added {
                                if cr.is_match(line_content) {
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
                            if !added.is_empty() && !cr.is_match(&joined_added) {
                                violations.push(GuardViolation {
                                    rule_id: cr.rule.id.clone(),
                                    rule_title: cr.rule.title.clone(),
                                    file_path: file_path.to_string_lossy().to_string(),
                                    line_number: added.first().map(|(l, _)| *l),
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

        for raw_line in diff_text.lines() {
            let line = raw_line.trim_end();

            // Detect new file header: `diff --git a/file b/file`
            if let Some(rest) = line.strip_prefix("diff --git ") {
                finalize_file(
                    &current_file,
                    &added_lines,
                    &mut violations,
                    &mut checked_files,
                );
                added_lines.clear();
                in_hunk = false;

                // Extract b path
                if let Some((_a, b)) = rest.split_once(' ') {
                    current_file = clean_diff_path(b);
                } else {
                    current_file = None;
                }
                continue;
            }

            // Target path `+++ b/file`
            if let Some(rest) = line.strip_prefix("+++ ") {
                if let Some(cleaned) = clean_diff_path(rest) {
                    current_file = Some(cleaned);
                }
                continue;
            }

            // Hunk header: `@@ -old_start,old_count +new_start,new_count @@`
            if line.starts_with("@@") {
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
                continue;
            }

            if in_hunk {
                if let Some(rest) = line.strip_prefix('+') {
                    // Added line
                    added_lines.push((current_line, rest.to_string()));
                    current_line += 1;
                } else if line.starts_with('-') {
                    // Deleted line - do not increment new file line counter
                } else if line.starts_with(' ') {
                    // Context line
                    current_line += 1;
                }
            }
        }

        finalize_file(
            &current_file,
            &added_lines,
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
            for cr in &compiled_rules {
                if !matches_file_pattern(cr.rule.file_pattern.as_deref(), path) {
                    continue;
                }

                match &cr.rule.condition {
                    RuleCondition::Forbidden(_) => {
                        for (idx, line) in content.lines().enumerate() {
                            let line_no = idx + 1;
                            if cr.is_match(line) {
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
