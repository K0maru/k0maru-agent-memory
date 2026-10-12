use k0maru_core::core::models::{Document, HierarchyLevel};
use k0maru_core::guard::extractor::{extract_rules_from_document, extract_rules_from_vault};
use k0maru_core::guard::models::{
    GuardReport, GuardRule, GuardViolation, RuleCondition, RuleSeverity,
};
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn test_guard_models_serialization() {
    let rule = GuardRule {
        id: "ADR-001".to_string(),
        title: "Test Invariant".to_string(),
        source_note: "20_Cards/ADR-001.md".to_string(),
        file_pattern: Some("*.rs".to_string()),
        condition: RuleCondition::Forbidden("unwrap\\(\\)".to_string()),
        severity: RuleSeverity::Deny,
        remediation: "Use ? instead of unwrap()".to_string(),
    };

    let serialized = serde_json::to_string(&rule).expect("Failed to serialize GuardRule");
    let deserialized: GuardRule =
        serde_json::from_str(&serialized).expect("Failed to deserialize GuardRule");
    assert_eq!(rule, deserialized);

    let violation = GuardViolation {
        rule_id: "ADR-001".to_string(),
        rule_title: "Test Invariant".to_string(),
        file_path: "src/main.rs".to_string(),
        line_number: Some(42),
        matched_content: Some("let x = foo.unwrap();".to_string()),
        severity: RuleSeverity::Deny,
        source_note: "20_Cards/ADR-001.md".to_string(),
        remediation: "Use ? instead of unwrap()".to_string(),
    };

    let report = GuardReport {
        checked_files: 1,
        checked_diff_hunks: 1,
        violations: vec![violation],
        is_passed: false,
        duration_ms: 5,
    };

    let report_json = serde_json::to_string(&report).expect("Failed to serialize GuardReport");
    assert!(report_json.contains("ADR-001"));
    assert!(report_json.contains("unwrap"));
}

#[test]
fn test_extract_rules_from_frontmatter() {
    let doc = Document {
        path: PathBuf::from("20_Cards/ADR-SETTLE-001.md"),
        title: "Mandatory Distributed Lock".to_string(),
        hierarchy: HierarchyLevel::L3Evergreen,
        frontmatter: json!({
            "invariants": [
                {
                    "id": "ADR-SETTLE-001",
                    "title": "Mandatory Distributed Lock",
                    "files": "*.go",
                    "must_contain": "redis.*SetNX|lock:",
                    "forbidden": "execWithoutLock",
                    "severity": "deny",
                    "remediation": "Acquire Redis lock before modifying ledger."
                },
                {
                    "id": "ADR-SETTLE-002",
                    "title": "No panic in settlement",
                    "file_pattern": "settle/**/*.go",
                    "forbidden": "panic\\(",
                    "severity": "warn",
                    "remediation": "Return error instead of panic"
                }
            ]
        }),
        links: vec![],
        tags: vec![],
        content_hash: "hash1".to_string(),
        mtime: 1000,
        body: "# Settlement Architecture\n\nDetails here.".to_string(),
    };

    let rules = extract_rules_from_document(&doc);
    assert_eq!(rules.len(), 3); // 1 must_contain + 1 forbidden from first, 1 forbidden from second

    let r0 = &rules[0];
    assert_eq!(r0.id, "ADR-SETTLE-001");
    assert_eq!(r0.source_note, "20_Cards/ADR-SETTLE-001.md");
    assert_eq!(r0.file_pattern.as_deref(), Some("*.go"));
    assert_eq!(r0.severity, RuleSeverity::Deny);
    assert!(matches!(&r0.condition, RuleCondition::MustContain(c) if c == "redis.*SetNX|lock:"));

    let r1 = &rules[1];
    assert_eq!(r1.id, "ADR-SETTLE-001");
    assert!(matches!(&r1.condition, RuleCondition::Forbidden(c) if c == "execWithoutLock"));

    let r2 = &rules[2];
    assert_eq!(r2.id, "ADR-SETTLE-002");
    assert_eq!(r2.file_pattern.as_deref(), Some("settle/**/*.go"));
    assert_eq!(r2.severity, RuleSeverity::Warn);
    assert!(matches!(&r2.condition, RuleCondition::Forbidden(c) if c == "panic\\("));
}

#[test]
fn test_extract_rules_from_markdown_body_heuristic() {
    let doc = Document {
        path: PathBuf::from("10_Projects/TradingEngine.md"),
        title: "Trading Engine".to_string(),
        hierarchy: HierarchyLevel::L1Resource,
        frontmatter: json!({}),
        links: vec![],
        tags: vec![],
        content_hash: "hash2".to_string(),
        mtime: 1000,
        body: r#"
# Trading Engine System

Overview of the trading engine.

## Non-Negotiable Invariants
- [ADR-RISK-01] (deny) [*.rs] forbidden: "naked_position" | remediation: "Ensure stop-loss or hedge is active"
- [ADR-AUDIT-02] (warn) [crates/order/**/*.rs] must_contain: "audit_trail" | remediation: "Log order mutation to audit trail"

## 核心工程铁律
- **ADR-PERF-03**: files: "src/engine/*.rs", forbidden: "sleep\\(", severity: deny, remediation: "Do not sleep on hot path"

## Other Sections
- Regular notes
"#
        .to_string(),
    };

    let rules = extract_rules_from_document(&doc);
    assert_eq!(rules.len(), 3);

    assert_eq!(rules[0].id, "ADR-RISK-01");
    assert_eq!(rules[0].severity, RuleSeverity::Deny);
    assert_eq!(rules[0].file_pattern.as_deref(), Some("*.rs"));
    assert!(matches!(&rules[0].condition, RuleCondition::Forbidden(s) if s == "naked_position"));
    assert_eq!(rules[0].remediation, "Ensure stop-loss or hedge is active");

    assert_eq!(rules[1].id, "ADR-AUDIT-02");
    assert_eq!(rules[1].severity, RuleSeverity::Warn);
    assert_eq!(
        rules[1].file_pattern.as_deref(),
        Some("crates/order/**/*.rs")
    );
    assert!(matches!(&rules[1].condition, RuleCondition::MustContain(s) if s == "audit_trail"));

    assert_eq!(rules[2].id, "ADR-PERF-03");
    assert_eq!(rules[2].file_pattern.as_deref(), Some("src/engine/*.rs"));
    assert_eq!(rules[2].severity, RuleSeverity::Deny);
    assert!(matches!(&rules[2].condition, RuleCondition::Forbidden(s) if s == "sleep\\("));
}

#[test]
fn test_extract_rules_from_vault_config_fallback() {
    let dir = tempdir().unwrap();
    let k0maru_dir = dir.path().join(".k0maru");
    fs::create_dir_all(&k0maru_dir).unwrap();

    let guard_toml = r#"
[[rules]]
id = "CONF-001"
title = "No hardcoded secrets"
file_pattern = "*.rs"
severity = "deny"
forbidden = "SECRET_KEY\\s*="
remediation = "Use environment variables or vault secrets"

[[rules]]
id = "CONF-002"
title = "Copyright header required"
file_pattern = "*.go"
severity = "warn"
must_contain = "Copyright \\(c\\)"
remediation = "Add copyright header"
"#;
    fs::write(k0maru_dir.join("guard.toml"), guard_toml).unwrap();

    let rules = extract_rules_from_vault(dir.path()).unwrap();
    assert_eq!(rules.len(), 2);
    assert_eq!(rules[0].id, "CONF-001");
    assert_eq!(rules[0].severity, RuleSeverity::Deny);
    assert!(matches!(&rules[0].condition, RuleCondition::Forbidden(s) if s == "SECRET_KEY\\s*="));

    assert_eq!(rules[1].id, "CONF-002");
    assert_eq!(rules[1].severity, RuleSeverity::Warn);
    assert!(
        matches!(&rules[1].condition, RuleCondition::MustContain(s) if s == "Copyright \\(c\\)")
    );
}
