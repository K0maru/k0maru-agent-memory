use assert_cmd::Command;
use k0maru::mcp::McpServer;
use serde_json::json;
use std::fs;
use std::path::Path;
use std::process::Command as StdCommand;
use tempfile::tempdir;

fn init_git_repo(path: &Path) {
    let out = StdCommand::new("git")
        .args(["init"])
        .current_dir(path)
        .output()
        .expect("git init failed");
    assert!(out.status.success());

    let _ = StdCommand::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(path)
        .output();
    let _ = StdCommand::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(path)
        .output();
}

#[test]
fn test_guard_install_pre_commit_hook() {
    let dir = tempdir().unwrap();
    init_git_repo(dir.path());

    let mut cmd = Command::cargo_bin("k0maru").unwrap();
    cmd.current_dir(dir.path())
        .args(["guard", "install", "--path", dir.path().to_str().unwrap()])
        .assert()
        .success();

    let hook_path = dir.path().join(".git").join("hooks").join("pre-commit");
    assert!(hook_path.exists(), "pre-commit hook file must be created");

    let content = fs::read_to_string(&hook_path).unwrap();
    assert!(content.contains("k0maru guard check --diff"));

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::metadata(&hook_path).unwrap().permissions();
        assert_eq!(perms.mode() & 0o111, 0o111, "hook must be executable");
    }
}

#[test]
fn test_mcp_guard_check_flags_violation() {
    let vault_dir = tempdir().unwrap();
    let k0maru_dir = vault_dir.path().join(".k0maru");
    fs::create_dir_all(&k0maru_dir).unwrap();

    fs::write(
        k0maru_dir.join("guard.toml"),
        r#"
[[rules]]
id = "ADR-SETTLE-001"
title = "Mandatory Distributed Lock"
file_pattern = "*.go"
severity = "deny"
forbidden = "execWithoutLock"
remediation = "Acquire Redis lock before modifying ledger."
"#,
    )
    .unwrap();

    let mut server = McpServer::new(vault_dir.path());

    let dirty_diff = r#"
diff --git a/settle.go b/settle.go
new file mode 100644
--- /dev/null
+++ b/settle.go
@@ -0,0 +1,5 @@
+package settle
+func SettleLedger() {
+    execWithoutLock()
+}
"#;

    let res = server.handle_tool_call(
        json!(1),
        &json!({
            "name": "guard_check",
            "arguments": {
                "diff": dirty_diff
            }
        }),
    );

    let content_text = res["result"]["content"][0]["text"].as_str().unwrap();
    assert!(content_text.contains("ADR-SETTLE-001"));
    assert!(content_text.contains("execWithoutLock"));
    assert!(content_text.contains("\"is_passed\": false"));
}

#[test]
fn test_guard_cli_check_clean_and_deny_violations() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    init_git_repo(root);

    let k0maru_dir = root.join(".k0maru");
    fs::create_dir_all(&k0maru_dir).unwrap();
    fs::write(
        k0maru_dir.join("guard.toml"),
        r#"
[[rules]]
id = "ADR-001"
title = "No Raw SQL"
file_pattern = "*.rs"
severity = "deny"
forbidden = "execute_raw"
remediation = "Use query builder."
"#,
    )
    .unwrap();

    // Clean check passes (exit 0)
    let mut cmd_clean = Command::cargo_bin("k0maru").unwrap();
    cmd_clean
        .current_dir(root)
        .args(["guard", "check", "--vault", root.to_str().unwrap()])
        .assert()
        .success();

    // Stdin diff with violation fails (exit 1)
    let bad_diff = r#"
diff --git a/src/db.rs b/src/db.rs
--- a/src/db.rs
+++ b/src/db.rs
@@ -1,1 +1,2 @@
+    db.execute_raw("DROP TABLE users");
"#;

    let mut cmd_fail = Command::cargo_bin("k0maru").unwrap();
    cmd_fail
        .current_dir(root)
        .args([
            "guard",
            "check",
            "--vault",
            root.to_str().unwrap(),
            "--diff",
        ])
        .write_stdin(bad_diff)
        .assert()
        .failure()
        .code(1);
}

#[test]
fn test_guard_flat_llm_wiki_and_decisions_folder() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    // Flat root LLM-Wiki note declaring a Living ADR invariant.
    fs::write(
        root.join("adr-001.md"),
        r#"---
title: ADR 001 Flat Invariant
invariants:
  - id: ADR-FLAT-001
    title: Flat wiki invariant
    file_pattern: "*.rs"
    forbidden: "flat_forbidden_call"
    remediation: "Use the safe API."
---
# ADR 001

## Non-Negotiable Invariants
"#,
    )
    .unwrap();

    // Engineering decisions folder without any SecondBrain 20_Cards tree.
    let decisions_dir = root.join("decisions");
    fs::create_dir_all(&decisions_dir).unwrap();
    fs::write(
        decisions_dir.join("adr-002.md"),
        r#"---
title: ADR 002 Decisions Folder
invariants:
  - id: ADR-DECISIONS-002
    title: Nested decisions invariant
    file_pattern: "*.go"
    forbidden: "decisions_forbidden_call"
    remediation: "Route through the ledger service."
---
# ADR 002

## Non-Negotiable Invariants
"#,
    )
    .unwrap();

    assert!(
        !root.join("20_Cards").exists(),
        "fixture must not contain a SecondBrain cards folder"
    );

    let rules = k0maru::resolve_guard_rules(root, None).expect("resolve guard rules");
    let ids: Vec<&str> = rules.iter().map(|rule| rule.id.as_str()).collect();

    assert!(
        ids.contains(&"ADR-FLAT-001"),
        "flat root ADR must contribute rules, got: {ids:?}"
    );
    assert!(
        ids.contains(&"ADR-DECISIONS-002"),
        "decisions/ folder ADR must contribute rules, got: {ids:?}"
    );
}

#[test]
fn test_guard_project_filter_exact_slug_isolation() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("20_Cards")).unwrap();

    let write_card = |name: &str, id: &str, project: &str, pattern: &str| {
        fs::write(
            root.join("20_Cards").join(format!("{name}.md")),
            format!(
                "---\ntitle: {name}\nproject: {project}\ninvariants:\n  - id: {id}\n    title: {id}\n    file_pattern: \"*.rs\"\n    forbidden: \"{pattern}\"\n    remediation: \"Remedy for {id}\"\n---\n# {name}\n"
            ),
        )
        .unwrap();
    };

    write_card("alpha-card", "ALPHA-001", "alpha", "alpha_bad");
    write_card(
        "beta-legacy-card",
        "BETA-LEGACY-001",
        "payment-beta-legacy",
        "beta_legacy_bad",
    );

    let alpha = k0maru::guard::resolve_guard_rules_with_repo(root, Some("alpha"), None).unwrap();
    let alpha_ids: Vec<&str> = alpha.iter().map(|rule| rule.id.as_str()).collect();
    assert!(alpha_ids.contains(&"ALPHA-001"), "got: {alpha_ids:?}");
    assert!(
        !alpha_ids.contains(&"BETA-LEGACY-001"),
        "got: {alpha_ids:?}"
    );

    // Exact slug equality: `beta` must never inherit `payment-beta-legacy`.
    let beta = k0maru::guard::resolve_guard_rules_with_repo(root, Some("beta"), None).unwrap();
    let beta_ids: Vec<&str> = beta.iter().map(|rule| rule.id.as_str()).collect();
    assert!(!beta_ids.contains(&"ALPHA-001"), "got: {beta_ids:?}");
    assert!(!beta_ids.contains(&"BETA-LEGACY-001"), "got: {beta_ids:?}");

    let legacy =
        k0maru::guard::resolve_guard_rules_with_repo(root, Some("payment-beta-legacy"), None)
            .unwrap();
    assert!(legacy.iter().any(|rule| rule.id == "BETA-LEGACY-001"));
    assert!(!legacy.iter().any(|rule| rule.id == "ALPHA-001"));
}

#[test]
fn test_guard_untracked_files_are_audited() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    init_git_repo(root);

    let k0maru_dir = root.join(".k0maru");
    fs::create_dir_all(&k0maru_dir).unwrap();
    fs::write(
        k0maru_dir.join("guard.toml"),
        r#"
[[rules]]
id = "ADR-UNTRACKED-001"
title = "No raw SQL"
file_pattern = "*.rs"
severity = "deny"
forbidden = "execute_raw"
remediation = "Use the query builder."
"#,
    )
    .unwrap();

    // Never `git add`ed: only an untracked-file audit can catch this.
    fs::write(
        root.join("new_module.rs"),
        "fn leak() {\n    db.execute_raw(\"select 1\");\n}\n",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("k0maru").unwrap();
    cmd.current_dir(root)
        .args(["guard", "check", "--vault", root.to_str().unwrap()])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn test_guard_diff_covers_staged_changes_hidden_by_worktree_revert() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    init_git_repo(root);

    fs::write(root.join("db.rs"), "fn ok() {}\n").unwrap();
    for args in [vec!["add", "db.rs"], vec!["commit", "-m", "init"]] {
        let out = StdCommand::new("git")
            .args(&args)
            .current_dir(root)
            .output()
            .expect("git command failed");
        assert!(out.status.success(), "git {:?} failed", args);
    }

    // Stage a violation, then restore the clean working tree so only the index
    // carries it. `git diff HEAD` alone cannot see this.
    fs::write(root.join("db.rs"), "fn leak() { db.execute_raw(\"x\"); }\n").unwrap();
    let staged = StdCommand::new("git")
        .args(["add", "db.rs"])
        .current_dir(root)
        .output()
        .expect("git add failed");
    assert!(staged.status.success());
    fs::write(root.join("db.rs"), "fn ok() {}\n").unwrap();

    let diff = k0maru::guard::get_git_diff(root, None).expect("git diff");
    assert!(
        diff.contains("execute_raw"),
        "staged-only violation must appear in the audited diff, got: {diff}"
    );
}

#[test]
fn test_guard_install_hook_is_fail_closed_and_vault_bound() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    init_git_repo(root);

    let vault = root.join(".k0maru").join("wiki");
    fs::create_dir_all(&vault).unwrap();

    let mut cmd = Command::cargo_bin("k0maru").unwrap();
    cmd.current_dir(root)
        .args(["guard", "install", "--path", root.to_str().unwrap()])
        .assert()
        .success();

    let content = fs::read_to_string(root.join(".git").join("hooks").join("pre-commit")).unwrap();
    assert!(
        content.contains("|| exit 1"),
        "hook must abort the commit when the audit fails"
    );
    assert!(
        content.contains("K0MARU_GUARD_OPTIONAL"),
        "hook must fail closed when k0maru is unavailable"
    );
    assert!(
        content.contains("--vault \""),
        "hook must bind an explicit vault, got:\n{content}"
    );
    assert!(
        content.contains(".k0maru/wiki\""),
        "hook must default to the repo-local .k0maru/wiki vault, got:\n{content}"
    );
}

#[test]
fn test_guard_same_commit_supersession_cannot_disable_invariants() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    init_git_repo(root);

    let vault = root.join(".k0maru").join("wiki");
    let decisions = vault.join("decisions");
    fs::create_dir_all(&decisions).unwrap();

    fs::write(
        decisions.join("adr-001.md"),
        r#"---
title: ADR 001 Lock Invariant
status: active
invariants:
  - id: ADR-S1-001
    title: Mandatory distributed lock
    file_pattern: "*.rs"
    forbidden: "exec_without_lock"
    remediation: "Acquire the Redis lock first."
---
# ADR 001
"#,
    )
    .unwrap();

    for args in [vec!["add", "."], vec!["commit", "-m", "adr"]] {
        let out = StdCommand::new("git")
            .args(&args)
            .current_dir(root)
            .output()
            .expect("git command failed");
        assert!(out.status.success(), "git {:?} failed", args);
    }

    // Same changeset: retire the ADR and ship the violation it forbids.
    fs::write(
        decisions.join("adr-001.md"),
        r#"---
title: ADR 001 Lock Invariant
status: superseded
superseded_by: "[[adr-002]]"
invariants:
  - id: ADR-S1-001
    title: Mandatory distributed lock
    file_pattern: "*.rs"
    forbidden: "exec_without_lock"
    remediation: "Acquire the Redis lock first."
---
# ADR 001
"#,
    )
    .unwrap();
    fs::write(
        decisions.join("adr-002.md"),
        "---\ntitle: ADR 002\nstatus: active\n---\n# ADR 002\n",
    )
    .unwrap();

    let repo_root = k0maru::guard::resolve_repo_root(root);
    let rules =
        k0maru::guard::resolve_guard_rules_with_repo(&vault, None, Some(&repo_root)).unwrap();
    assert!(
        rules.iter().any(|rule| rule.id == "ADR-S1-001"),
        "an ADR retired in the same commit must still be enforced, got: {:?}",
        rules
            .iter()
            .map(|rule| rule.id.as_str())
            .collect::<Vec<_>>()
    );
}
