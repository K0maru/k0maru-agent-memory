use serde::{Deserialize, Serialize};

/// Severity level of a guard invariant rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RuleSeverity {
    #[default]
    #[serde(alias = "deny", alias = "Deny", alias = "DENY")]
    Deny,
    #[serde(alias = "warn", alias = "Warn", alias = "WARN")]
    Warn,
}

impl RuleSeverity {
    pub fn is_deny(&self) -> bool {
        matches!(self, Self::Deny)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Deny => "deny",
            Self::Warn => "warn",
        }
    }
}

impl std::fmt::Display for RuleSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Invariant condition evaluated against changed files and diffs.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleCondition {
    /// Added/modified lines must match this regex/substring
    MustContain(String),
    /// Added/modified lines must not match this forbidden regex/substring
    Forbidden(String),
}

/// An architectural invariant rule extracted from an ADR, project document, or vault config.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardRule {
    pub id: String,
    pub title: String,
    pub source_note: String,
    pub file_pattern: Option<String>,
    pub condition: RuleCondition,
    pub severity: RuleSeverity,
    pub remediation: String,
}

/// An observed violation of a guard invariant rule during diff or file audit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardViolation {
    pub rule_id: String,
    pub rule_title: String,
    pub file_path: String,
    pub line_number: Option<usize>,
    pub matched_content: Option<String>,
    pub severity: RuleSeverity,
    pub source_note: String,
    pub remediation: String,
}

/// Comprehensive report resulting from a guard audit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardReport {
    pub checked_files: usize,
    pub checked_diff_hunks: usize,
    pub violations: Vec<GuardViolation>,
    pub is_passed: bool,
    pub duration_ms: u128,
}

impl GuardReport {
    pub fn new(
        checked_files: usize,
        checked_diff_hunks: usize,
        violations: Vec<GuardViolation>,
        duration_ms: u128,
    ) -> Self {
        let is_passed = !violations.iter().any(|v| v.severity.is_deny());
        Self {
            checked_files,
            checked_diff_hunks,
            violations,
            is_passed,
            duration_ms,
        }
    }
}
