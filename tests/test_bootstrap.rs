use std::fs;
use std::path::Path;
use std::process::Command as StdCommand;
use tempfile::tempdir;

use k0maru::adapters::ObsidianAdapter;
use k0maru::bootstrap::{run_bootstrap, BootstrapOptions};
use k0maru::guard::{resolve_guard_rules, GuardEngine};
use k0maru::install::{install_claude_project_hooks, install_repo_hooks};
use k0maru::loadout::LoadoutBuilder;
use k0maru::storage::SqliteStorage;

fn init_git_repo(path: &Path) {
    let _ = StdCommand::new("git")
        .args(["init"])
        .current_dir(path)
        .output();
    let _ = StdCommand::new("git")
        .args(["config", "user.name", "K0maru Tester"])
        .current_dir(path)
        .output();
    let _ = StdCommand::new("git")
        .args(["config", "user.email", "tester@k0maru.dev"])
        .current_dir(path)
        .output();
}

#[test]
fn test_bootstrap_zero_vault_repo() {
    let repo_dir = tempdir().unwrap();
    let repo_path = repo_dir.path();

    // Create typical project markers
    fs::write(
        repo_path.join("README.md"),
        "# Acme Payment Service\n\nHigh-throughput transactional payment processor.\nHandles webhooks and idempotency.",
    )
    .unwrap();

    fs::write(
        repo_path.join("Cargo.toml"),
        r#"[package]
name = "acme-payment"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();

    fs::write(
        repo_path.join("AGENTS.md"),
        "# Agent Constitution\n- Never bypass transactional lock checks.\n- Ensure all errors are typed.",
    )
    .unwrap();

    let options = BootstrapOptions {
        repo_path: repo_path.to_path_buf(),
        vault_path: None,
        project_name: None,
        install_clients: false,
        home_override: None,
    };

    let report = run_bootstrap(options).expect("run_bootstrap failed");

    assert_eq!(report.project_slug, "acme-payment");
    assert_eq!(report.created_files.len(), 3);
    assert_eq!(report.indexed_docs, 3);

    // Verify directory topology and files
    let default_vault = report.repo_path.join(".k0maru").join("wiki");
    assert_eq!(report.vault_path, default_vault);

    let project_file = default_vault.join("10_Projects").join("acme-payment.md");
    assert!(
        project_file.exists(),
        "10_Projects/acme-payment.md must exist"
    );

    let adr_file = default_vault
        .join("20_Cards")
        .join("adr-001-repository-guardrails.md");
    assert!(
        adr_file.exists(),
        "20_Cards/adr-001-repository-guardrails.md must exist"
    );

    let logs_dir = default_vault.join("01_AI_Logs");
    let log_files: Vec<_> = fs::read_dir(&logs_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .ends_with("-initial-bootstrap.md")
        })
        .collect();
    assert_eq!(log_files.len(), 1, "Initial bootstrap log must be created");

    // Verify cache.sqlite exists and contains indexed documents
    let cache_db = default_vault.join(".k0maru").join("cache.sqlite");
    assert!(cache_db.exists(), "cache.sqlite must exist");

    let storage = SqliteStorage::open(&cache_db).expect("open cache.sqlite");
    assert_eq!(storage.document_count().unwrap(), 3);

    // Check project note content
    let project_note = fs::read_to_string(&project_file).unwrap();
    assert!(project_note.contains("title: \"acme-payment\""));
    assert!(project_note.contains("[[20_Cards/adr-001-repository-guardrails]]"));
    assert!(project_note.contains("High-throughput transactional payment processor"));
}

#[test]
fn test_bootstrap_enables_instant_loadout_and_guard() {
    let repo_dir = tempdir().unwrap();
    let repo_path = repo_dir.path();

    fs::write(
        repo_path.join("Cargo.toml"),
        r#"[package]
name = "guard-sentinel-test"
version = "0.1.0"
"#,
    )
    .unwrap();
    fs::write(repo_path.join("README.md"), "# Guard Sentinel Test Service").unwrap();

    let options = BootstrapOptions {
        repo_path: repo_path.to_path_buf(),
        vault_path: None,
        project_name: None,
        install_clients: false,
        home_override: None,
    };

    let report = run_bootstrap(options).expect("bootstrap should succeed");

    // 1. Guard check verification: invariants extracted from 20_Cards/adr-001-repository-guardrails.md
    let rules = resolve_guard_rules(&report.vault_path, None).expect("resolve rules");
    assert!(
        rules.iter().any(|r| r.id == "no-hardcoded-private-keys"),
        "Must contain no-hardcoded-private-keys rule"
    );
    assert!(
        rules.iter().any(|r| r.id == "no-conflict-markers"),
        "Must contain no-conflict-markers rule"
    );

    // 2. Simulated diff with private key must be rejected
    let bad_key_diff = r#"
diff --git a/src/secret.rs b/src/secret.rs
--- a/src/secret.rs
+++ b/src/secret.rs
@@ -1,2 +1,4 @@
+// Compromised credentials
+-----BEGIN RSA PRIVATE KEY-----
+MIIEowIBAAKCAQEA0...
"#;
    let report_bad_key = GuardEngine::check_diff(bad_key_diff, &rules);
    assert!(
        !report_bad_key.is_passed,
        "Private key diff must fail guard"
    );
    assert_eq!(report_bad_key.violations.len(), 1);
    assert_eq!(
        report_bad_key.violations[0].rule_id,
        "no-hardcoded-private-keys"
    );

    // 3. Simulated diff with conflict marker must be rejected
    let bad_conflict_diff = r#"
diff --git a/src/main.rs b/src/main.rs
--- a/src/main.rs
+++ b/src/main.rs
@@ -10,3 +10,5 @@
+<<<<<<< HEAD
+fn unresolved_conflict() {}
"#;
    let report_conflict = GuardEngine::check_diff(bad_conflict_diff, &rules);
    assert!(
        !report_conflict.is_passed,
        "Conflict marker diff must fail guard"
    );
    assert_eq!(report_conflict.violations.len(), 1);
    assert_eq!(report_conflict.violations[0].rule_id, "no-conflict-markers");

    // 4. Safe diff must pass cleanly
    let clean_diff = r#"
diff --git a/src/main.rs b/src/main.rs
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,3 @@
-fn main() {}
+fn main() { println!("Clean commit"); }
"#;
    let report_clean = GuardEngine::check_diff(clean_diff, &rules);
    assert!(report_clean.is_passed, "Clean diff must pass guard");

    // 5. Context Loadout verification: instant retrieval from indexed SQLite
    let adapter = ObsidianAdapter::new(&report.vault_path);
    let cache_db = report.vault_path.join(".k0maru").join("cache.sqlite");
    let storage = SqliteStorage::open(&cache_db).expect("open cache");
    let builder = LoadoutBuilder::new(&adapter, &storage);

    let loadout = builder
        .build(&report.project_slug)
        .expect("build loadout for slug")
        .expect("project loadout found");
    assert_eq!(loadout.project_name, "guard-sentinel-test");
    assert!(
        !loadout.l3_cards.is_empty(),
        "Must link ADR-001 architectural guardrails"
    );
}

#[test]
fn test_install_claude_project_hooks() {
    let repo_dir = tempdir().unwrap();
    let repo_path = repo_dir.path();
    let vault_path = repo_path.join(".k0maru").join("wiki");

    let hook_path = install_claude_project_hooks(repo_path, &vault_path, "sample-service")
        .expect("install hook");
    assert!(hook_path.exists());

    let content = fs::read_to_string(&hook_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();

    let session_start = json
        .get("hooks")
        .and_then(|h| h.get("SessionStart"))
        .and_then(|s| s.as_array())
        .expect("hooks.SessionStart array");

    assert_eq!(session_start.len(), 1);
    let cmd = session_start[0]
        .get("command")
        .and_then(|c| c.as_str())
        .unwrap();
    assert!(cmd.contains("k0maru loadout sample-service"));
    assert!(cmd.contains("--vault"));

    // Verify idempotency: running twice does not duplicate entry
    let hook_path_again =
        install_claude_project_hooks(repo_path, &vault_path, "sample-service").unwrap();
    assert_eq!(hook_path_again, hook_path);

    let content_again = fs::read_to_string(&hook_path).unwrap();
    let json_again: serde_json::Value = serde_json::from_str(&content_again).unwrap();
    let session_start_again = json_again["hooks"]["SessionStart"].as_array().unwrap();
    assert_eq!(
        session_start_again.len(),
        1,
        "Idempotency: entry must not be duplicated"
    );
}

#[test]
fn test_install_repo_hooks_with_git() {
    let repo_dir = tempdir().unwrap();
    let repo_path = repo_dir.path();
    init_git_repo(repo_path);

    let vault_path = repo_path.join(".k0maru").join("wiki");
    let (guard_hook, claude_hook) = install_repo_hooks(repo_path, &vault_path, "git-test-repo");

    assert!(
        guard_hook.is_some(),
        "Git pre-commit hook should be installed"
    );
    let gh = guard_hook.unwrap();
    assert!(gh.exists());
    let gh_content = fs::read_to_string(&gh).unwrap();
    assert!(gh_content.contains("k0maru guard check --diff"));

    assert!(claude_hook.is_some(), "Claude hook should be installed");
    let ch = claude_hook.unwrap();
    assert!(ch.exists());
}
