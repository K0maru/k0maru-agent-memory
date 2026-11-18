use std::fs;
use std::path::Path;
use std::process::Command as StdCommand;

use assert_cmd::Command;
use k0maru::git::sync_git_pre_scan;
use k0maru::storage::SqliteStorage;
use tempfile::tempdir;

fn git(dir: &Path, args: &[&str]) -> String {
    let out = StdCommand::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .expect("git invocation");
    assert!(
        out.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn init_repo(dir: &Path) {
    git(dir, &["init", "-b", "main"]);
    git(dir, &["config", "user.name", "Test Bot"]);
    git(dir, &["config", "user.email", "test@example.com"]);
}

fn commit_with_time(dir: &Path, msg: &str, unix_ts: &str) {
    let out = StdCommand::new("git")
        .arg("-C")
        .arg(dir)
        .args(["commit", "-m", msg])
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env("GIT_AUTHOR_DATE", format!("{unix_ts} +0000"))
        .env("GIT_COMMITTER_DATE", format!("{unix_ts} +0000"))
        .output()
        .expect("git commit");
    assert!(
        out.status.success(),
        "git commit failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn test_sync_git_ignores_inherited_git_dir_and_ingests_subdir_times() {
    let tmp = tempdir().unwrap();
    let repo = tmp.path().join("repo");
    let bogus_git_dir = tmp.path().join("bogus.git");
    fs::create_dir_all(&repo).unwrap();
    fs::create_dir_all(&bogus_git_dir).unwrap();
    git(&bogus_git_dir, &["init", "--bare"]);

    init_repo(&repo);
    let vault = repo.join("vault");
    fs::create_dir_all(vault.join("01_AI_Logs")).unwrap();
    fs::write(
        vault.join("01_AI_Logs/2026-09-01-settlement.md"),
        "---\ntitle: Settlement Log\nhierarchy: L2Log\n---\n# Settlement Log\nInitial.\n",
    )
    .unwrap();
    git(&repo, &["add", "-A"]);
    commit_with_time(&repo, "commit 1", "1780000000");

    fs::write(
        vault.join("01_AI_Logs/2026-09-05-gateway.md"),
        "---\ntitle: Gateway Log\nhierarchy: L2Log\n---\n# Gateway Log\nSecond.\n",
    )
    .unwrap();
    git(&repo, &["add", "-A"]);
    commit_with_time(&repo, "commit 2", "1780500000");

    // Set bogus inherited GIT_DIR/GIT_WORK_TREE/GIT_INDEX_FILE just like a foreign git hook!
    let assert = Command::cargo_bin("k0maru")
        .unwrap()
        .env("GIT_DIR", &bogus_git_dir)
        .env("GIT_WORK_TREE", &bogus_git_dir)
        .env("GIT_INDEX_FILE", bogus_git_dir.join("index"))
        .args([
            "sync",
            "--vault",
            vault.to_str().unwrap(),
            "--git",
            "--json",
        ])
        .assert()
        .success();

    let stdout_json: serde_json::Value =
        serde_json::from_slice(&assert.get_output().stdout).unwrap();
    assert_eq!(stdout_json["added"], 2);

    let storage = SqliteStorage::open_read_only(vault.join(".k0maru/cache.sqlite")).unwrap();
    let times = storage
        .git_times(&[
            "01_AI_Logs/2026-09-01-settlement.md",
            "01_AI_Logs/2026-09-05-gateway.md",
        ])
        .unwrap();
    assert_eq!(
        times.get("01_AI_Logs/2026-09-01-settlement.md"),
        Some(&1780000000)
    );
    assert_eq!(
        times.get("01_AI_Logs/2026-09-05-gateway.md"),
        Some(&1780500000)
    );

    let head = git(&repo, &["rev-parse", "HEAD"]);
    assert_eq!(
        storage.meta_get("git_time_head").unwrap().as_deref(),
        Some(head.as_str())
    );
}

#[test]
fn test_sync_git_mid_merge_warns_and_falls_back_to_plain_sync() {
    let tmp = tempdir().unwrap();
    let vault = tmp.path().join("vault");
    fs::create_dir_all(&vault).unwrap();
    init_repo(&vault);

    fs::write(vault.join("Card.md"), "# Card\nBody.\n").unwrap();
    git(&vault, &["add", "-A"]);
    commit_with_time(&vault, "base", "1780000000");

    // Simulate an in-progress merge via MERGE_HEAD.
    let head = git(&vault, &["rev-parse", "HEAD"]);
    let merge_head = vault.join(".git/MERGE_HEAD");
    fs::write(&merge_head, format!("{head}\n")).unwrap();

    let output = Command::cargo_bin("k0maru")
        .unwrap()
        .args(["sync", "--vault", vault.to_str().unwrap(), "--git"])
        .assert()
        .success()
        .get_output()
        .clone();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("MERGE_HEAD") || stderr.contains("preflight failed"),
        "expected warning on stderr, got: {stderr}"
    );

    // Normal sync still indexed Card.md even though preflight fell back.
    let storage = SqliteStorage::open_read_only(vault.join(".k0maru/cache.sqlite")).unwrap();
    assert_eq!(storage.document_count().unwrap(), 1);
    assert!(storage.meta_get("git_time_head").unwrap().is_none());
}

#[test]
fn test_sync_git_pull_refusals_and_ff_only_without_push_commit_or_stash() {
    let tmp = tempdir().unwrap();
    let bare_remote = tmp.path().join("remote.git");
    fs::create_dir_all(&bare_remote).unwrap();
    git(&bare_remote, &["init", "--bare", "-b", "main"]);

    // Seed remote from clone_a.
    let clone_a = tmp.path().join("clone_a");
    StdCommand::new("git")
        .args([
            "clone",
            bare_remote.to_str().unwrap(),
            clone_a.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    git(&clone_a, &["config", "user.name", "Test Bot"]);
    git(&clone_a, &["config", "user.email", "test@example.com"]);
    fs::write(clone_a.join("Alpha.md"), "# Alpha\nBase.\n").unwrap();
    git(&clone_a, &["add", "-A"]);
    commit_with_time(&clone_a, "alpha", "1780000000");
    git(&clone_a, &["push", "-u", "origin", "main"]);

    // Clone into local vault (clone_b).
    let vault = tmp.path().join("vault");
    StdCommand::new("git")
        .args([
            "clone",
            bare_remote.to_str().unwrap(),
            vault.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    git(&vault, &["config", "user.name", "Test Bot"]);
    git(&vault, &["config", "user.email", "test@example.com"]);

    // Advance remote via clone_a.
    fs::write(clone_a.join("Beta.md"), "# Beta\nUpstream commit.\n").unwrap();
    git(&clone_a, &["add", "-A"]);
    commit_with_time(&clone_a, "beta", "1781000000");
    git(&clone_a, &["push", "origin", "main"]);

    let cache_path = vault.join(".k0maru/cache.sqlite");
    let mut storage = SqliteStorage::open(&cache_path).unwrap();

    // 1. Dirty tracked file -> --git-pull refuses; reflog unchanged.
    let reflog_before = git(&vault, &["reflog"]);
    fs::write(vault.join("Alpha.md"), "# Alpha\nDirty edit.\n").unwrap();
    let dirty_outcome = sync_git_pre_scan(&vault, &mut storage, true);
    assert!(dirty_outcome.preflight_ok);
    assert!(!dirty_outcome.pulled);
    assert_eq!(git(&vault, &["reflog"]), reflog_before);

    // Restore clean working tree.
    git(&vault, &["checkout", "--", "Alpha.md"]);

    // 2. Detached HEAD -> --git-pull refuses; reflog unchanged after checkout.
    git(&vault, &["checkout", "--detach"]);
    let detached_reflog = git(&vault, &["reflog"]);
    let detached_outcome = sync_git_pre_scan(&vault, &mut storage, true);
    assert!(!detached_outcome.pulled);
    assert_eq!(git(&vault, &["reflog"]), detached_reflog);

    // Switch back to main.
    git(&vault, &["checkout", "main"]);

    // 3. No upstream branch -> --git-pull refuses.
    git(&vault, &["checkout", "-b", "no-upstream"]);
    let no_up_reflog = git(&vault, &["reflog"]);
    let no_up_outcome = sync_git_pre_scan(&vault, &mut storage, true);
    assert!(!no_up_outcome.pulled);
    assert_eq!(git(&vault, &["reflog"]), no_up_reflog);

    // Switch back to main, clean + tracking upstream -> fast-forward pull succeeds!
    git(&vault, &["checkout", "main"]);
    let remote_head = git(&clone_a, &["rev-parse", "HEAD"]);
    Command::cargo_bin("k0maru")
        .unwrap()
        .args([
            "sync",
            "--vault",
            vault.to_str().unwrap(),
            "--git",
            "--git-pull",
            "--json",
        ])
        .assert()
        .success();

    assert_eq!(git(&vault, &["rev-parse", "HEAD"]), remote_head);
    assert!(vault.join("Beta.md").exists());

    // Verify Beta.md was synced and its commit timestamp was recorded.
    let ro = SqliteStorage::open_read_only(&cache_path).unwrap();
    assert_eq!(ro.document_count().unwrap(), 2);
    let times = ro.git_times(&["Beta.md"]).unwrap();
    assert_eq!(times.get("Beta.md"), Some(&1781000000));
    // Ensure nothing was pushed or created on remote.
    assert_eq!(git(&bare_remote, &["rev-parse", "main"]), remote_head);

    // 4. Subsequent sync on unchanged HEAD skips git log backfill.
    drop(ro);
    let mut rw = SqliteStorage::open(&cache_path).unwrap();
    let unchanged_outcome = sync_git_pre_scan(&vault, &mut rw, false);
    assert!(unchanged_outcome.preflight_ok);
    assert!(!unchanged_outcome.git_times_updated);

    // 5. Diverged local + remote branch: `--git-pull` refuses non-ff without merge/rebase/stash/push.
    fs::write(clone_a.join("Gamma.md"), "# Gamma\nRemote branch.\n").unwrap();
    git(&clone_a, &["add", "-A"]);
    commit_with_time(&clone_a, "gamma remote", "1782000000");
    git(&clone_a, &["push", "origin", "main"]);

    fs::write(vault.join("Local.md"), "# Local\nLocal branch.\n").unwrap();
    git(&vault, &["add", "-A"]);
    commit_with_time(&vault, "local diverge", "1782000100");
    let diverged_reflog = git(&vault, &["reflog"]);

    Command::cargo_bin("k0maru")
        .unwrap()
        .args([
            "sync",
            "--vault",
            vault.to_str().unwrap(),
            "--git",
            "--git-pull",
        ])
        .assert()
        .success();

    assert_eq!(git(&vault, &["reflog"]), diverged_reflog);
    assert!(!vault.join("Gamma.md").exists());
}

#[test]
fn test_sync_git_non_git_vault_warns_and_succeeds() {
    let tmp = tempdir().unwrap();
    let vault = tmp.path();
    fs::write(
        vault.join("Standalone.md"),
        "# Standalone\nNot a git repo.\n",
    )
    .unwrap();

    let output = Command::cargo_bin("k0maru")
        .unwrap()
        .args(["sync", "--vault", vault.to_str().unwrap(), "--git"])
        .assert()
        .success()
        .get_output()
        .clone();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("preflight failed"));

    let ro = SqliteStorage::open_read_only(vault.join(".k0maru/cache.sqlite")).unwrap();
    assert_eq!(ro.document_count().unwrap(), 1);
}
