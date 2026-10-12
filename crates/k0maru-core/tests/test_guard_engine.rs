use k0maru_core::guard::engine::GuardEngine;
use k0maru_core::guard::models::{GuardRule, RuleCondition, RuleSeverity};
use std::path::PathBuf;

#[test]
fn test_guard_engine_forbidden_violation_in_diff() {
    let diff = r#"
diff --git a/src/payment/settle.go b/src/payment/settle.go
index 1234567..89abcdef 100644
--- a/src/payment/settle.go
+++ b/src/payment/settle.go
@@ -10,3 +10,5 @@ func SettleLedger(ctx context.Context) error {
     // Start settlement
+    execWithoutLock(ctx)
+    return nil
 }
"#;

    let rules = vec![GuardRule {
        id: "ADR-SETTLE-001".to_string(),
        title: "Mandatory Distributed Lock".to_string(),
        source_note: "20_Cards/ADR-SETTLE-001.md".to_string(),
        file_pattern: Some("*.go".to_string()),
        condition: RuleCondition::Forbidden("execWithoutLock".to_string()),
        severity: RuleSeverity::Deny,
        remediation: "Acquire Redis lock before modifying ledger.".to_string(),
    }];

    let report = GuardEngine::check_diff(diff, &rules);

    assert_eq!(report.checked_files, 1);
    assert!(
        !report.is_passed,
        "Report must not pass when Deny rule is violated"
    );
    assert_eq!(report.violations.len(), 1);

    let v = &report.violations[0];
    assert_eq!(v.rule_id, "ADR-SETTLE-001");
    assert_eq!(v.file_path, "src/payment/settle.go");
    assert_eq!(v.line_number, Some(11));
    assert!(v
        .matched_content
        .as_deref()
        .unwrap()
        .contains("execWithoutLock"));
    assert_eq!(v.severity, RuleSeverity::Deny);
    assert_eq!(v.source_note, "20_Cards/ADR-SETTLE-001.md");
    assert_eq!(v.remediation, "Acquire Redis lock before modifying ledger.");
}

#[test]
fn test_guard_engine_must_contain_violation_in_diff() {
    let diff = r#"
diff --git a/src/payment/settle.go b/src/payment/settle.go
index 1234567..89abcdef 100644
--- a/src/payment/settle.go
+++ b/src/payment/settle.go
@@ -10,3 +10,5 @@ func SettleLedger(ctx context.Context) error {
+    doSomethingElse()
+    return nil
 }
"#;

    let rules = vec![GuardRule {
        id: "ADR-SETTLE-001".to_string(),
        title: "Mandatory Distributed Lock".to_string(),
        source_note: "20_Cards/ADR-SETTLE-001.md".to_string(),
        file_pattern: Some("*.go".to_string()),
        condition: RuleCondition::MustContain("redis.*SetNX|lock:".to_string()),
        severity: RuleSeverity::Deny,
        remediation: "Acquire Redis lock before modifying ledger.".to_string(),
    }];

    let report = GuardEngine::check_diff(diff, &rules);

    assert_eq!(report.checked_files, 1);
    assert!(!report.is_passed);
    assert_eq!(report.violations.len(), 1);
    assert_eq!(report.violations[0].rule_id, "ADR-SETTLE-001");
    assert_eq!(report.violations[0].file_path, "src/payment/settle.go");
}

#[test]
fn test_guard_engine_clean_compliant_diff_passes_under_10ms() {
    let diff = r#"
diff --git a/src/payment/settle.go b/src/payment/settle.go
index 1234567..89abcdef 100644
--- a/src/payment/settle.go
+++ b/src/payment/settle.go
@@ -10,3 +10,6 @@ func SettleLedger(ctx context.Context) error {
+    lock := redis.SetNX("lock:settle", 1, time.Second)
+    defer lock.Release()
+    return nil
 }
"#;

    let rules = vec![
        GuardRule {
            id: "ADR-SETTLE-001".to_string(),
            title: "Mandatory Distributed Lock".to_string(),
            source_note: "20_Cards/ADR-SETTLE-001.md".to_string(),
            file_pattern: Some("*.go".to_string()),
            condition: RuleCondition::MustContain("redis.*SetNX|lock:".to_string()),
            severity: RuleSeverity::Deny,
            remediation: "Acquire Redis lock before modifying ledger.".to_string(),
        },
        GuardRule {
            id: "ADR-SETTLE-002".to_string(),
            title: "No naked execution".to_string(),
            source_note: "20_Cards/ADR-SETTLE-002.md".to_string(),
            file_pattern: Some("*.go".to_string()),
            condition: RuleCondition::Forbidden("execWithoutLock".to_string()),
            severity: RuleSeverity::Deny,
            remediation: "Never execute without lock".to_string(),
        },
    ];

    let report = GuardEngine::check_diff(diff, &rules);

    assert!(report.is_passed);
    assert_eq!(report.violations.len(), 0);
    assert_eq!(report.checked_files, 1);
    assert!(
        report.duration_ms < 10,
        "GuardEngine check_diff took {}ms, expected < 10ms",
        report.duration_ms
    );
}

#[test]
fn test_guard_engine_check_files() {
    let files = vec![
        (
            PathBuf::from("src/payment/settle.go"),
            "package payment\n\nfunc Settle() {\n    execWithoutLock()\n}\n".to_string(),
        ),
        (
            PathBuf::from("src/model/user.go"),
            "package model\n\ntype User struct{}\n".to_string(),
        ),
    ];

    let rules = vec![GuardRule {
        id: "ADR-001".to_string(),
        title: "No unauthenticated settlement".to_string(),
        source_note: "20_Cards/ADR-001.md".to_string(),
        file_pattern: Some("**/settle.go".to_string()),
        condition: RuleCondition::Forbidden("execWithoutLock".to_string()),
        severity: RuleSeverity::Deny,
        remediation: "Use secure execution".to_string(),
    }];

    let report = GuardEngine::check_files(&files, &rules);

    assert_eq!(report.checked_files, 2);
    assert!(!report.is_passed);
    assert_eq!(report.violations.len(), 1);
    assert_eq!(report.violations[0].file_path, "src/payment/settle.go");
    assert_eq!(report.violations[0].line_number, Some(4));
}
