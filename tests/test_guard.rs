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
    assert!(
        content.contains("guard check --diff --staged"),
        "hook must audit only staged changes, got:\n{content}"
    );
    assert!(
        content.contains("# --- k0maru guard sentinel ---"),
        "hook must carry the sentinel marker, got:\n{content}"
    );
}

#[test]
fn test_guard_install_replaces_echo_only_mention_with_real_sentinel() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    init_git_repo(root);

    let hooks_dir = root.join(".git").join("hooks");
    fs::create_dir_all(&hooks_dir).unwrap();
    let hook_path = hooks_dir.join("pre-commit");
    fs::write(
        &hook_path,
        "#!/bin/sh\n# echo k0maru guard check --diff for manual audits\necho \"run k0maru guard check --diff\"\nexit 0\n",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("k0maru").unwrap();
    cmd.current_dir(root)
        .args(["guard", "install", "--path", root.to_str().unwrap()])
        .assert()
        .success();

    let content = fs::read_to_string(&hook_path).unwrap();
    assert!(
        content.contains("# --- k0maru guard sentinel ---"),
        "a textual mention inside an echo must not suppress installation, got:\n{content}"
    );
    assert!(
        content.contains("guard check --diff --staged"),
        "the real sentinel must be installed, got:\n{content}"
    );
    assert!(
        content.contains("for manual audits"),
        "existing hook logic must be preserved, got:\n{content}"
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

    // Same changeset: retire the ADR and ship the violation it forbids. The
    // `superseded_by` target does not exist, so this is not a legitimate
    // supersession and the rule must still be enforced.
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

    let repo_root = k0maru::guard::resolve_repo_root(root);
    let rules =
        k0maru::guard::resolve_guard_rules_with_repo(&vault, None, Some(&repo_root)).unwrap();
    assert!(
        rules.iter().any(|rule| rule.id == "ADR-S1-001"),
        "an ADR retired in the same commit without an active replacement must still be enforced, got: {:?}",
        rules
            .iter()
            .map(|rule| rule.id.as_str())
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_guard_legitimate_same_commit_supersession_is_honored() {
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
  - id: ADR-S2-001
    title: Mandatory distributed lock
    file_pattern: "*.rs"
    forbidden: "exec_without_lock_legacy"
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

    for args in [vec!["add", "."], vec!["commit", "-m", "adr"]] {
        let out = StdCommand::new("git")
            .args(&args)
            .current_dir(root)
            .output()
            .expect("git command failed");
        assert!(out.status.success(), "git {:?} failed", args);
    }

    // Legitimate retirement: adr-002 is active and exists in the working tree.
    fs::write(
        decisions.join("adr-001.md"),
        r#"---
title: ADR 001 Lock Invariant
status: superseded
superseded_by: "[[adr-002]]"
invariants:
  - id: ADR-S2-001
    title: Mandatory distributed lock
    file_pattern: "*.rs"
    forbidden: "exec_without_lock_legacy"
    remediation: "Acquire the Redis lock first."
---
# ADR 001
"#,
    )
    .unwrap();

    let repo_root = k0maru::guard::resolve_repo_root(root);
    let rules =
        k0maru::guard::resolve_guard_rules_with_repo(&vault, None, Some(&repo_root)).unwrap();
    assert!(
        !rules.iter().any(|rule| rule.id == "ADR-S2-001"),
        "a legitimate supersession must not resurrect the retired HEAD rule, got: {:?}",
        rules
            .iter()
            .map(|rule| rule.id.as_str())
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_guard_same_commit_rule_deletion_is_resurrected() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    init_git_repo(root);

    let vault = root.join(".k0maru").join("wiki");
    fs::create_dir_all(vault.join("20_Cards")).unwrap();
    let card = vault.join("20_Cards").join("adr-050.md");

    fs::write(
        &card,
        r#"---
title: ADR 050 SQL Guard
status: active
invariants:
  - id: ADR-S3-050
    title: No raw SQL
    file_pattern: "*.rs"
    forbidden: "execute_raw_s3"
    remediation: "Use the query builder."
---
# ADR 050
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

    // Delete just the rule (document stays active) in the same changeset.
    fs::write(
        &card,
        "---\ntitle: ADR 050 SQL Guard\nstatus: active\n---\n# ADR 050\n",
    )
    .unwrap();

    let repo_root = k0maru::guard::resolve_repo_root(root);
    let rules =
        k0maru::guard::resolve_guard_rules_with_repo(&vault, None, Some(&repo_root)).unwrap();
    assert!(
        rules.iter().any(|rule| rule.id == "ADR-S3-050"),
        "rules deleted by the same commit must still be enforced, got: {:?}",
        rules
            .iter()
            .map(|rule| rule.id.as_str())
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_guard_staged_only_ignores_untracked_and_unstaged_files() {
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

    // Untracked scratch file and an unstaged edit, both containing a violation.
    fs::write(
        root.join("scratch.rs"),
        "fn leak() { db.execute_raw(\"x\"); }\n",
    )
    .unwrap();
    fs::write(root.join("db.rs"), "fn leak() { db.execute_raw(\"y\"); }\n").unwrap();

    let staged = k0maru::guard::get_git_diff_with_mode(root, None, true).unwrap();
    assert!(
        !staged.contains("execute_raw"),
        "staged-only mode must ignore untracked and unstaged changes, got: {staged}"
    );

    let full = k0maru::guard::get_git_diff(root, None).unwrap();
    assert!(
        full.contains("execute_raw"),
        "full mode must audit untracked and unstaged changes, got: {full}"
    );

    // A staged violation is still audited (and detected) in staged-only mode.
    let add = StdCommand::new("git")
        .args(["add", "scratch.rs"])
        .current_dir(root)
        .output()
        .expect("git add failed");
    assert!(add.status.success());
    let staged = k0maru::guard::get_git_diff_with_mode(root, None, true).unwrap();
    assert!(
        staged.contains("execute_raw"),
        "staged-only mode must audit staged changes, got: {staged}"
    );
}

#[test]
fn test_guard_check_staged_ignores_untracked_scratch_and_unstaged_edits() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    init_git_repo(root);

    fs::create_dir_all(root.join(".k0maru")).unwrap();
    fs::write(
        root.join(".k0maru").join("guard.toml"),
        r#"
[[rules]]
id = "ADR-STAGED-001"
title = "No raw SQL"
file_pattern = "*.rs"
severity = "deny"
forbidden = "execute_raw"
remediation = "Use the query builder."
"#,
    )
    .unwrap();
    fs::write(root.join("README.md"), "# repo\n").unwrap();

    for args in [vec!["add", "."], vec!["commit", "-m", "init"]] {
        let out = StdCommand::new("git")
            .args(&args)
            .current_dir(root)
            .output()
            .expect("git command failed");
        assert!(out.status.success(), "git {:?} failed", args);
    }

    // Untracked scratch violation plus an unrelated staged change.
    fs::create_dir_all(root.join("scratch")).unwrap();
    fs::write(
        root.join("scratch").join("notes.rs"),
        "fn leak() { db.execute_raw(\"x\"); }\n",
    )
    .unwrap();
    fs::write(root.join("README.md"), "# repo\n\nunrelated edit\n").unwrap();
    let add = StdCommand::new("git")
        .args(["add", "README.md"])
        .current_dir(root)
        .output()
        .expect("git add failed");
    assert!(add.status.success());

    // Pre-commit mode: only the staged, clean change is audited.
    let mut cmd = Command::cargo_bin("k0maru").unwrap();
    cmd.current_dir(root)
        .args([
            "guard",
            "check",
            "--vault",
            root.to_str().unwrap(),
            "--diff",
            "--staged",
        ])
        .assert()
        .success();

    // Full mode still audits untracked files.
    let mut cmd = Command::cargo_bin("k0maru").unwrap();
    cmd.current_dir(root)
        .args([
            "guard",
            "check",
            "--vault",
            root.to_str().unwrap(),
            "--diff",
        ])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn test_guard_repo_root_project_isolation_no_cross_project_leak() {
    let dir = tempdir().unwrap();
    let vault = dir.path().join("vault");
    fs::create_dir_all(vault.join("20_Cards")).unwrap();

    fs::write(
        vault.join("20_Cards").join("alpha-card.md"),
        "---\ntitle: Alpha\nproject: alpha\ninvariants:\n  - id: ALPHA-ISO-001\n    title: Alpha only\n    file_pattern: \"*.rs\"\n    forbidden: \"alpha_iso_bad\"\n    remediation: \"Alpha remedy\"\n---\n# Alpha\n",
    )
    .unwrap();
    fs::write(
        vault.join("20_Cards").join("adr-001-repository-guardrails.md"),
        "---\ntitle: Global\ninvariants:\n  - id: GLOBAL-ISO-001\n    title: Global\n    file_pattern: \"*.rs\"\n    forbidden: \"global_iso_bad\"\n    remediation: \"Global remedy\"\n---\n# Global\n",
    )
    .unwrap();

    for repo_name in ["gamma", "beta"] {
        let repo = dir.path().join(repo_name);
        fs::create_dir_all(&repo).unwrap();
        let rules = k0maru::guard::resolve_guard_rules_with_repo(&vault, None, Some(&repo))
            .expect("resolve guard rules");
        let ids: Vec<&str> = rules.iter().map(|rule| rule.id.as_str()).collect();
        assert!(
            !ids.contains(&"ALPHA-ISO-001"),
            "project alpha rules must never leak into {repo_name}, got: {ids:?}"
        );
        assert!(
            ids.contains(&"GLOBAL-ISO-001"),
            "vault-global rules must still apply in {repo_name}, got: {ids:?}"
        );
    }

    // Explicit --project still selects the project card.
    let alpha = k0maru::guard::resolve_guard_rules_with_repo(&vault, Some("alpha"), None).unwrap();
    assert!(alpha.iter().any(|rule| rule.id == "ALPHA-ISO-001"));
}

fn commit_all(path: &Path, message: &str, extra_path: Option<&std::ffi::OsStr>) {
    let out = StdCommand::new("git")
        .args(["add", "-A"])
        .current_dir(path)
        .output()
        .expect("git add failed");
    assert!(out.status.success(), "git add failed");

    let mut cmd = StdCommand::new("git");
    cmd.args(["commit", "--no-verify", "-m", message])
        .current_dir(path);
    if let Some(extra) = extra_path {
        let current = std::env::var_os("PATH").unwrap_or_default();
        let mut joined = std::ffi::OsString::from(extra);
        joined.push(":");
        joined.push(current);
        cmd.env("PATH", joined);
    }
    let out = cmd.output().expect("git commit failed");
    assert!(
        out.status.success(),
        "git commit failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Writes a Living ADR card that forbids `execute_raw` in Rust files.
fn write_stop_hook_vault(vault: &Path) {
    let cards = vault.join("20_Cards");
    fs::create_dir_all(&cards).unwrap();
    fs::write(
        cards.join("adr-900-no-raw-sql.md"),
        "---\ntitle: ADR 900 No Raw SQL\ninvariants:\n  - id: STOP-001\n    title: No raw SQL execution\n    file_pattern: \"*.rs\"\n    forbidden: \"execute_raw\"\n    remediation: \"Use the typed query builder.\"\n---\n# ADR 900\n",
    )
    .unwrap();
}

#[test]
fn test_guard_claude_stop_hook_exits_2_with_stderr_report() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    init_git_repo(root);

    let vault = root.join(".k0maru").join("wiki");
    write_stop_hook_vault(&vault);

    fs::write(root.join("db.rs"), "fn ok() {}\n").unwrap();
    commit_all(root, "chore: baseline", None);

    fs::write(root.join("db.rs"), "fn leak() { db.execute_raw(\"x\"); }\n").unwrap();

    let assert = Command::cargo_bin("k0maru")
        .unwrap()
        .current_dir(root)
        .args([
            "guard",
            "check",
            "--diff",
            "--claude-stop-hook",
            "--vault",
            vault.to_str().unwrap(),
        ])
        .assert()
        .code(2);

    let output = assert.get_output();
    assert!(
        output.stdout.is_empty(),
        "a Stop hook must not write to stdout, got: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("STOP-001"),
        "stderr must carry the violation so Claude Code can relay it, got: {stderr}"
    );
    assert!(
        stderr.contains("[AGENT_SELF_HEAL_PROTOCOL]"),
        "stderr must carry the self-heal protocol, got: {stderr}"
    );
}

#[test]
fn test_guard_claude_stop_hook_is_silent_when_clean() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    init_git_repo(root);

    let vault = root.join(".k0maru").join("wiki");
    write_stop_hook_vault(&vault);

    fs::write(root.join("db.rs"), "fn ok() {}\n").unwrap();

    let assert = Command::cargo_bin("k0maru")
        .unwrap()
        .current_dir(root)
        .args([
            "guard",
            "check",
            "--diff",
            "--claude-stop-hook",
            "--vault",
            vault.to_str().unwrap(),
        ])
        .assert()
        .success();

    let output = assert.get_output();
    assert!(output.stdout.is_empty(), "clean Stop hook must stay silent");
    assert!(output.stderr.is_empty(), "clean Stop hook must stay silent");
}

#[test]
fn test_guard_claude_stop_hook_honors_stop_hook_active() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    init_git_repo(root);

    let vault = root.join(".k0maru").join("wiki");
    write_stop_hook_vault(&vault);

    fs::write(root.join("db.rs"), "fn ok() {}\n").unwrap();
    commit_all(root, "chore: baseline", None);
    fs::write(root.join("db.rs"), "fn leak() { db.execute_raw(\"x\"); }\n").unwrap();

    // A continuation already triggered by this hook must never loop again.
    let assert = Command::cargo_bin("k0maru")
        .unwrap()
        .current_dir(root)
        .args([
            "guard",
            "check",
            "--diff",
            "--claude-stop-hook",
            "--vault",
            vault.to_str().unwrap(),
        ])
        .write_stdin("{\"stop_hook_active\": true}")
        .assert()
        .success();

    let output = assert.get_output();
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
}

#[test]
fn test_guard_claude_stop_hook_ignores_uninitialized_vault() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    init_git_repo(root);

    let missing_vault = root.join(".k0maru").join("wiki");

    let assert = Command::cargo_bin("k0maru")
        .unwrap()
        .current_dir(root)
        .args([
            "guard",
            "check",
            "--diff",
            "--claude-stop-hook",
            "--vault",
            missing_vault.to_str().unwrap(),
        ])
        .assert()
        .success();

    let output = assert.get_output();
    assert!(output.stdout.is_empty());
}

#[test]
fn test_guard_check_prints_self_heal_protocol_on_both_streams() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    init_git_repo(root);

    let vault = root.join(".k0maru").join("wiki");
    write_stop_hook_vault(&vault);

    fs::write(root.join("db.rs"), "fn ok() {}\n").unwrap();
    commit_all(root, "chore: baseline", None);
    fs::write(root.join("db.rs"), "fn leak() { db.execute_raw(\"x\"); }\n").unwrap();

    let assert = Command::cargo_bin("k0maru")
        .unwrap()
        .current_dir(root)
        .args([
            "guard",
            "check",
            "--diff",
            "--vault",
            vault.to_str().unwrap(),
        ])
        .assert()
        .code(1);

    let output = assert.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stdout.contains("[AGENT_SELF_HEAL_PROTOCOL]"),
        "stdout must carry the protocol, got: {stdout}"
    );
    assert!(
        stderr.contains("[AGENT_SELF_HEAL_PROTOCOL]"),
        "stderr must carry the protocol, got: {stderr}"
    );
    assert!(
        stderr.contains("adr-900-no-raw-sql"),
        "the protocol must name the superseded source note, got: {stderr}"
    );
}

#[test]
fn test_guard_install_writes_portable_pre_commit_and_post_commit_hooks() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    init_git_repo(root);

    let vault = root.join(".k0maru").join("wiki");
    fs::create_dir_all(&vault).unwrap();

    Command::cargo_bin("k0maru")
        .unwrap()
        .current_dir(root)
        .args(["guard", "install", "--path", root.to_str().unwrap()])
        .assert()
        .success();

    let pre = fs::read_to_string(root.join(".git").join("hooks").join("pre-commit")).unwrap();
    let post = fs::read_to_string(root.join(".git").join("hooks").join("post-commit")).unwrap();

    let canonical_root = root.canonicalize().unwrap();
    assert!(
        pre.contains("--vault \"$REPO_ROOT/.k0maru/wiki\""),
        "pre-commit must bind the vault relative to the repo root, got:\n{pre}"
    );
    assert!(
        !pre.contains(&format!("--vault \"{}", canonical_root.display())),
        "pre-commit must not bake in a machine-specific vault path, got:\n{pre}"
    );

    assert!(
        post.contains("# --- k0maru post-commit sentinel ---"),
        "post-commit hook must carry its sentinel marker, got:\n{post}"
    );
    assert!(
        post.contains("VAULT_PATH=\"$REPO_ROOT/.k0maru/wiki\""),
        "post-commit must resolve a portable vault path, got:\n{post}"
    );
    assert!(
        post.contains("rebase-merge") && post.contains("rebase-apply"),
        "post-commit must skip rebase/merge replay commits, got:\n{post}"
    );
    assert!(
        post.contains("git show --stat --oneline HEAD"),
        "post-commit must flush the commit stat as a log note, got:\n{post}"
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::metadata(root.join(".git").join("hooks").join("post-commit"))
            .unwrap()
            .permissions();
        assert_eq!(
            perms.mode() & 0o111,
            0o111,
            "post-commit must be executable"
        );
    }
}

#[test]
fn test_guard_post_commit_hook_flushes_conventional_commit_log() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    init_git_repo(root);

    let vault = root.join(".k0maru").join("wiki");
    fs::create_dir_all(&vault).unwrap();
    fs::write(root.join("ledger.rs"), "fn ok() {}\n").unwrap();
    commit_all(root, "chore: baseline", None);

    Command::cargo_bin("k0maru")
        .unwrap()
        .current_dir(root)
        .args(["guard", "install", "--path", root.to_str().unwrap()])
        .assert()
        .success();

    let bin = Path::new(env!("CARGO_BIN_EXE_k0maru"));
    let bin_dir = bin.parent().unwrap().as_os_str();

    fs::write(root.join("ledger.rs"), "fn ok() { /* ledger */ }\n").unwrap();
    commit_all(root, "feat(ledger): add idempotent transfer", Some(bin_dir));

    let mut logs: Vec<std::path::PathBuf> = Vec::new();
    for entry in fs::read_dir(&vault).unwrap().flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            logs.push(path);
        }
    }
    assert!(
        !logs.is_empty(),
        "a conventional commit must be captured as a log note"
    );
    let captured: String = logs
        .iter()
        .map(|path| fs::read_to_string(path).unwrap_or_default())
        .collect();
    assert!(
        captured.contains("add idempotent transfer"),
        "the log note must record the commit subject, got: {captured}"
    );
}
