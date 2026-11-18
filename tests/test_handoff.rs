use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Barrier};
use std::time::{Duration, Instant};

use chrono::{TimeDelta, Utc};
use k0maru::{
    format_handoff_list_markdown, format_handoff_markdown, parse_duration_secs,
    ClaimHandoffRequest, CreateHandoffRequest, HandoffError, HandoffFileRef, HandoffRecord,
    HandoffState, HandoffStatus, HandoffStore, HANDOFF_SCHEMA_VERSION, MAX_HANDOFF_BYTES,
};
use tempfile::TempDir;

const RACE_ENV_FLAG: &str = "K0MARU_TEST_HANDOFF_RACE_HELPER";
const GIT_ENV_FLAG: &str = "K0MARU_TEST_HANDOFF_GIT_HELPER";
const RACE_ENV_VAULT: &str = "K0MARU_TEST_HANDOFF_VAULT";
const RACE_ENV_ID: &str = "K0MARU_TEST_HANDOFF_ID";
const RACE_ENV_AGENT: &str = "K0MARU_TEST_HANDOFF_AGENT";
const RACE_ENV_GO_FILE: &str = "K0MARU_TEST_HANDOFF_GO_FILE";
const RACE_ENV_READY_FILE: &str = "K0MARU_TEST_HANDOFF_READY_FILE";

fn base_create_req(title: &str) -> CreateHandoffRequest {
    CreateHandoffRequest {
        from_agent: "agent-alpha".to_string(),
        to_agent: None,
        project: Some("payment-gateway".to_string()),
        title: title.to_string(),
        goal: "Ensure idempotent settlement across retries".to_string(),
        state: HandoffState::InProgress,
        done: vec!["Added ledger schema".to_string()],
        next_steps: vec!["Wire retry queue".to_string()],
        open_questions: vec!["Confirm dead-letter TTL".to_string()],
        refs: vec!["10_Projects/payment-gateway.md:12-30#Settlement".to_string()],
        files: vec![HandoffFileRef {
            path: "src/settlement/retry.rs".to_string(),
            lines: Some("10-45".to_string()),
        }],
        branch: None,
        commit: None,
        expires_at: None,
        repo_path: None,
    }
}

fn init_git_repo(dir: &Path) {
    let run = |args: &[&str]| {
        let out = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .output()
            .expect("git command should execute");
        assert!(
            out.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
    };

    run(&["init", "-b", "feat/settlement"]);
    run(&["config", "user.name", "Test Bot"]);
    run(&["config", "user.email", "test@example.invalid"]);
    fs::write(dir.join("README.md"), "# Payment Gateway\n").unwrap();
    run(&["add", "README.md"]);
    run(&["commit", "-m", "initial commit"]);
}

#[test]
fn subprocess_claim_helper() {
    if std::env::var_os(GIT_ENV_FLAG).is_some() {
        let vault = std::env::var(RACE_ENV_VAULT).expect("vault env");
        let store = HandoffStore::new(vault);
        let rec = store
            .create(base_create_req("Git env hygiene"))
            .expect("create with bogus GIT_DIR should succeed");
        assert_eq!(rec.branch.as_deref(), Some("feat/settlement"));
        assert!(rec.commit.as_ref().is_some_and(|c| c.len() == 40));
        std::process::exit(0);
    }

    if std::env::var_os(RACE_ENV_FLAG).is_none() {
        return;
    }

    let vault = std::env::var(RACE_ENV_VAULT).expect("vault env");
    let id = std::env::var(RACE_ENV_ID).expect("id env");
    let agent = std::env::var(RACE_ENV_AGENT).expect("agent env");
    let go_file = std::env::var(RACE_ENV_GO_FILE).expect("go file env");
    let ready_file = std::env::var(RACE_ENV_READY_FILE).expect("ready file env");

    fs::write(&ready_file, "ready").expect("write ready marker");

    let deadline = Instant::now() + Duration::from_secs(10);
    while !Path::new(&go_file).exists() {
        if Instant::now() >= deadline {
            eprintln!("timed out waiting for go signal");
            std::process::exit(1);
        }
        std::thread::sleep(Duration::from_millis(2));
    }

    let store = HandoffStore::new(vault);
    match store.claim(ClaimHandoffRequest {
        agent,
        id: Some(id),
        project: None,
        latest: false,
    }) {
        Ok(rec) => {
            println!("WON:{}", rec.id);
            std::process::exit(0);
        }
        Err(err @ HandoffError::AlreadyClaimedOrMissing(_)) => {
            std::process::exit(err.exit_code());
        }
        Err(err) => {
            eprintln!("unexpected error: {}", err);
            std::process::exit(1);
        }
    }
}

#[test]
fn test_create_auto_fills_git_and_validates_id_format() {
    let dir = TempDir::new().unwrap();
    init_git_repo(dir.path());

    // Verify inherited bogus GIT_DIR does not hijack read-only git auto-detection.
    // Note: keep env clean inside a child/or test with explicit repo_path without mutating global env.
    let store = HandoffStore::new(dir.path());
    let rec = store
        .create(base_create_req("Implement ledger retry"))
        .expect("create should succeed");

    assert_eq!(rec.schema, HANDOFF_SCHEMA_VERSION);
    assert_eq!(rec.from_agent, "agent-alpha");
    assert_eq!(rec.project.as_deref(), Some("payment-gateway"));
    assert_eq!(rec.state, HandoffState::InProgress);
    assert_eq!(rec.branch.as_deref(), Some("feat/settlement"));
    assert!(
        rec.commit.as_ref().is_some_and(|c| c.len() == 40),
        "expected full commit sha, got {:?}",
        rec.commit
    );
    assert!(rec.claimed_by.is_none());
    assert!(rec.claimed_at.is_none());

    // ID must match YYYYMMDDTHHMMSSZ-<8 lowercase hex>
    let parts: Vec<&str> = rec.id.split('-').collect();
    assert_eq!(parts.len(), 2, "unexpected id format: {}", rec.id);
    assert_eq!(parts[0].len(), 16);
    assert!(parts[0].ends_with('Z'));
    assert_eq!(parts[1].len(), 8);
    assert!(parts[1].chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')));

    chrono::DateTime::parse_from_rfc3339(&rec.created_at).expect("valid RFC3339 created_at");

    let pending_file = store.pending_dir().join(format!("{}.json", rec.id));
    assert!(pending_file.exists());

    // Ensure no leftover temp files remain in .k0maru/handoffs/
    let leftover_temps: Vec<_> = fs::read_dir(store.handoffs_dir())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with(".tmp-"))
        .collect();
    assert!(leftover_temps.is_empty());

    // Verify inherited bogus GIT_DIR/GIT_WORK_TREE/GIT_INDEX_FILE never hijack create().
    let exe = std::env::current_exe().expect("current test binary");
    let out = Command::new(&exe)
        .args(["--exact", "subprocess_claim_helper"])
        .env(GIT_ENV_FLAG, "1")
        .env(RACE_ENV_VAULT, dir.path())
        .env("GIT_DIR", "/nonexistent/bogus/.git")
        .env("GIT_WORK_TREE", "/nonexistent/bogus")
        .env("GIT_INDEX_FILE", "/nonexistent/bogus/index")
        .output()
        .expect("spawn git env hygiene helper");
    assert!(
        out.status.success(),
        "bogus GIT_DIR helper failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn test_validation_rejects_empty_fields_traversal_and_oversized_payload() {
    let dir = TempDir::new().unwrap();
    let store = HandoffStore::new(dir.path());

    let mut empty_title = base_create_req("   ");
    empty_title.title = "   ".to_string();
    assert!(matches!(
        store.create(empty_title),
        Err(HandoffError::InvalidInput(_))
    ));

    let mut empty_agent = base_create_req("Valid title");
    empty_agent.from_agent = "".to_string();
    assert!(matches!(
        store.create(empty_agent),
        Err(HandoffError::InvalidInput(_))
    ));

    let bad_refs = [
        "../secret.md",
        "notes/../../secret.md:1-10#Heading",
        "/etc/passwd",
        "/abs/note.md:3-9#H",
        "notes\\..\\secret.md",
        "C:\\vault\\note.md",
        "",
    ];
    for bad in bad_refs {
        let mut req = base_create_req("Bad ref");
        req.refs = vec![bad.to_string()];
        assert!(
            matches!(store.create(req), Err(HandoffError::InvalidInput(_))),
            "expected InvalidInput for ref {:?}",
            bad
        );
    }

    // Valid ref carrying `:start-end#Heading` with punctuation in heading must succeed.
    let mut good_ref = base_create_req("Good ref");
    good_ref.refs = vec!["notes/a.md:10-25#../not-a-path".to_string()];
    assert!(store.create(good_ref).is_ok());

    for bad_file in ["../Cargo.toml", "src/../../etc/passwd", "/tmp/x.rs"] {
        let mut req = base_create_req("Bad file");
        req.files = vec![HandoffFileRef {
            path: bad_file.to_string(),
            lines: None,
        }];
        assert!(
            matches!(store.create(req), Err(HandoffError::InvalidInput(_))),
            "expected InvalidInput for file {:?}",
            bad_file
        );
    }

    let mut huge = base_create_req("Too large");
    huge.goal = "a".repeat(MAX_HANDOFF_BYTES + 1);
    assert!(matches!(
        store.create(huge),
        Err(HandoffError::PayloadTooLarge { .. })
    ));

    let mut bad_expiry = base_create_req("Bad expiry");
    bad_expiry.expires_at = Some("not-a-timestamp".to_string());
    assert!(matches!(
        store.create(bad_expiry),
        Err(HandoffError::InvalidInput(_))
    ));
}

#[test]
fn test_list_show_and_non_claiming_pending_hint() {
    let dir = TempDir::new().unwrap();
    let store = HandoffStore::new(dir.path());

    // Non-existent store returns empty / None without creating directories.
    assert!(store.list(None, true).unwrap().is_empty());
    assert!(store.latest_pending_hint(None).unwrap().is_none());
    assert!(!store.handoffs_dir().exists());

    let mut r1 = base_create_req("Older task");
    r1.project = Some("payment-gateway".to_string());
    let rec1 = store.create(r1).unwrap();

    let mut r2 = base_create_req("Other project task");
    r2.project = Some("distributed-settlement".to_string());
    let rec2 = store.create(r2).unwrap();

    let mut r3 = base_create_req("Newest gateway task");
    r3.project = Some("payment-gateway".to_string());
    r3.from_agent = "agent-beta".to_string();
    let rec3 = store.create(r3).unwrap();

    // latest_pending_hint must match project and NOT claim.
    let hint = store
        .latest_pending_hint(Some("PAYMENT-GATEWAY"))
        .unwrap()
        .expect("hint should be present");
    assert_eq!(
        hint,
        format!(
            "## Pending Handoff\n- {}: Newest gateway task (from agent-beta)",
            rec3.id
        )
    );
    assert!(store
        .pending_dir()
        .join(format!("{}.json", rec3.id))
        .exists());
    assert!(!store
        .claimed_dir()
        .join(format!("{}.json", rec3.id))
        .exists());

    // list(project_filter, false) returns newest first.
    let pg_pending = store.list(Some("payment-gateway"), false).unwrap();
    assert_eq!(pg_pending.len(), 2);
    assert_eq!(pg_pending[0].record.id, rec3.id);
    assert_eq!(pg_pending[1].record.id, rec1.id);

    // show by ID and show latest
    let shown_by_id = store.show(Some(&rec2.id), None).unwrap();
    assert_eq!(shown_by_id.status, HandoffStatus::Pending);
    assert_eq!(shown_by_id.record.title, "Other project task");

    let shown_latest = store.show(None, Some("payment-gateway")).unwrap();
    assert_eq!(shown_latest.record.id, rec3.id);

    // Claim rec3 and verify list(false) vs list(true)
    let claimed = store
        .claim(ClaimHandoffRequest {
            agent: "agent-gamma".to_string(),
            id: None,
            project: Some("payment-gateway".to_string()),
            latest: true,
        })
        .unwrap();
    assert_eq!(claimed.id, rec3.id);
    assert_eq!(claimed.claimed_by.as_deref(), Some("agent-gamma"));
    assert!(claimed.claimed_at.is_some());

    let pending_after = store.list(Some("payment-gateway"), false).unwrap();
    assert_eq!(pending_after.len(), 1);
    assert_eq!(pending_after[0].record.id, rec1.id);

    let all_after = store.list(Some("payment-gateway"), true).unwrap();
    assert_eq!(all_after.len(), 2);
    assert_eq!(all_after[0].status, HandoffStatus::Claimed);
    assert_eq!(all_after[1].status, HandoffStatus::Pending);
}

#[test]
fn test_expiry_and_to_agent_filtering() {
    let dir = TempDir::new().unwrap();
    let store = HandoffStore::new(dir.path());

    // Expired handoff
    let mut expired_req = base_create_req("Expired work");
    expired_req.expires_at = Some("2020-01-01T00:00:00Z".to_string());
    let expired_rec = store.create(expired_req).unwrap();

    // Targeted handoff
    let mut targeted_req = base_create_req("For argon only");
    targeted_req.to_agent = Some("Argon".to_string());
    targeted_req.expires_at = Some("2099-01-01T00:00:00Z".to_string());
    let targeted_rec = store.create(targeted_req).unwrap();

    // Broadcast handoff
    let broadcast_rec = store.create(base_create_req("Open work")).unwrap();

    // Expired handoff cannot be claimed by ID
    let err_exp = store
        .claim(ClaimHandoffRequest {
            agent: "argon".to_string(),
            id: Some(expired_rec.id.clone()),
            project: None,
            latest: false,
        })
        .unwrap_err();
    assert!(matches!(err_exp, HandoffError::Expired(_)));
    assert_eq!(err_exp.exit_code(), 3);

    // Mismatched agent cannot claim targeted handoff by ID
    let err_mismatch = store
        .claim(ClaimHandoffRequest {
            agent: "hydrogen".to_string(),
            id: Some(targeted_rec.id.clone()),
            project: None,
            latest: false,
        })
        .unwrap_err();
    assert!(matches!(
        err_mismatch,
        HandoffError::TargetAgentMismatch { .. }
    ));
    assert_eq!(err_mismatch.exit_code(), 3);
    assert!(store
        .pending_dir()
        .join(format!("{}.json", targeted_rec.id))
        .exists());

    // Claim latest as hydrogen skips expired and targeted-to-Argon, claiming broadcast_rec
    let h_claimed = store
        .claim(ClaimHandoffRequest {
            agent: "hydrogen".to_string(),
            id: None,
            project: None,
            latest: true,
        })
        .unwrap();
    assert_eq!(h_claimed.id, broadcast_rec.id);

    // Case-insensitive to_agent match succeeds for "ARGON"
    let a_claimed = store
        .claim(ClaimHandoffRequest {
            agent: "ARGON".to_string(),
            id: Some(targeted_rec.id.clone()),
            project: None,
            latest: false,
        })
        .unwrap();
    assert_eq!(a_claimed.id, targeted_rec.id);
    assert_eq!(a_claimed.claimed_by.as_deref(), Some("ARGON"));

    // Only expired_rec remains in pending/, so hint is None and list(false) is empty
    assert!(store.latest_pending_hint(None).unwrap().is_none());
    assert!(store.list(None, false).unwrap().is_empty());

    // list(true) still exposes expired_rec with Expired status
    let all = store.list(None, true).unwrap();
    assert!(all
        .iter()
        .any(|e| e.record.id == expired_rec.id && e.status == HandoffStatus::Expired));
}

#[test]
fn test_prune_removes_old_claimed_and_expired_never_unexpired_pending() {
    let dir = TempDir::new().unwrap();
    let store = HandoffStore::new(dir.path());
    let now = Utc::now();

    // 1. Unexpired pending (no expiry)
    let pending_no_exp = store.create(base_create_req("Keep forever")).unwrap();

    // 2. Unexpired pending (future expiry)
    let mut future_req = base_create_req("Keep until future");
    future_req.expires_at = Some((now + TimeDelta::days(30)).to_rfc3339());
    let pending_future = store.create(future_req).unwrap();

    // 3. Old expired pending (expired 20 days ago)
    let mut old_exp_req = base_create_req("Old expired");
    old_exp_req.expires_at = Some((now - TimeDelta::days(20)).to_rfc3339());
    let old_expired = store.create(old_exp_req).unwrap();

    // 4. Recent expired pending (expired 1 day ago)
    let mut recent_exp_req = base_create_req("Recent expired");
    recent_exp_req.expires_at = Some((now - TimeDelta::days(1)).to_rfc3339());
    let recent_expired = store.create(recent_exp_req).unwrap();

    // 5. Old claimed & recent claimed
    let old_claimed = store.create(base_create_req("Old claimed")).unwrap();
    store
        .claim(ClaimHandoffRequest {
            agent: "worker-1".to_string(),
            id: Some(old_claimed.id.clone()),
            ..Default::default()
        })
        .unwrap();
    // Backdate claimed_at to 20 days ago
    let old_claimed_path = store.claimed_dir().join(format!("{}.json", old_claimed.id));
    let mut old_claimed_rec: HandoffRecord =
        serde_json::from_slice(&fs::read(&old_claimed_path).unwrap()).unwrap();
    old_claimed_rec.claimed_at = Some((now - TimeDelta::days(20)).to_rfc3339());
    fs::write(
        &old_claimed_path,
        serde_json::to_vec_pretty(&old_claimed_rec).unwrap(),
    )
    .unwrap();

    let recent_claimed = store.create(base_create_req("Recent claimed")).unwrap();
    store
        .claim(ClaimHandoffRequest {
            agent: "worker-2".to_string(),
            id: Some(recent_claimed.id.clone()),
            ..Default::default()
        })
        .unwrap();

    let older_than_14d = parse_duration_secs("14d").unwrap();
    assert_eq!(older_than_14d, 14 * 86_400);

    let res = store.prune_with_now(older_than_14d, now).unwrap();
    assert_eq!(res.deleted_expired, 1);
    assert_eq!(res.deleted_claimed, 1);
    assert_eq!(res.kept_claimed, 1);
    assert_eq!(res.kept_pending, 3);
    assert!(res.deleted_ids.contains(&old_expired.id));
    assert!(res.deleted_ids.contains(&old_claimed.id));

    assert!(store
        .pending_dir()
        .join(format!("{}.json", pending_no_exp.id))
        .exists());
    assert!(store
        .pending_dir()
        .join(format!("{}.json", pending_future.id))
        .exists());
    assert!(store
        .pending_dir()
        .join(format!("{}.json", recent_expired.id))
        .exists());
    assert!(store
        .claimed_dir()
        .join(format!("{}.json", recent_claimed.id))
        .exists());
    assert!(!store
        .pending_dir()
        .join(format!("{}.json", old_expired.id))
        .exists());
    assert!(!old_claimed_path.exists());

    // Even prune(0) must NEVER delete unexpired pending handoffs.
    let now_after = Utc::now() + TimeDelta::seconds(1);
    let res_zero = store.prune_with_now(0, now_after).unwrap();
    assert_eq!(res_zero.deleted_expired, 1);
    assert_eq!(res_zero.deleted_claimed, 1);
    assert_eq!(res_zero.kept_pending, 2);
    assert!(store
        .pending_dir()
        .join(format!("{}.json", pending_no_exp.id))
        .exists());
    assert!(store
        .pending_dir()
        .join(format!("{}.json", pending_future.id))
        .exists());
}

#[test]
fn test_crash_recovery_renders_claimed_unknown_claimer() {
    let dir = TempDir::new().unwrap();
    let store = HandoffStore::new(dir.path());
    let rec = store.create(base_create_req("Crash window test")).unwrap();

    // Simulate a crash right after fs::rename(pending -> claimed) before metadata rewrite.
    let pending_path = store.pending_dir().join(format!("{}.json", rec.id));
    let claimed_path = store.claimed_dir().join(format!("{}.json", rec.id));
    fs::rename(&pending_path, &claimed_path).unwrap();

    let entry = store.show(Some(&rec.id), None).unwrap();
    assert_eq!(entry.status, HandoffStatus::Claimed);
    assert!(entry.record.claimed_by.is_none());

    let md = format_handoff_markdown(&entry);
    assert!(
        md.contains("claimed (claimer unknown)"),
        "expected 'claimed (claimer unknown)' in:\n{}",
        md
    );

    let list = store.list(None, true).unwrap();
    let list_md = format_handoff_list_markdown(&list);
    assert!(
        list_md.contains("claimed (claimer unknown)"),
        "expected 'claimed (claimer unknown)' in list:\n{}",
        list_md
    );

    // Subsequent claim attempt must fail with AlreadyClaimedOrMissing.
    let err = store
        .claim(ClaimHandoffRequest {
            agent: "late-agent".to_string(),
            id: Some(rec.id.clone()),
            ..Default::default()
        })
        .unwrap_err();
    assert!(matches!(err, HandoffError::AlreadyClaimedOrMissing(_)));
}

#[test]
fn test_concurrent_claim_race_threads_and_subprocesses() {
    // 1) Pure 8-thread race on a single handoff ID.
    let dir_threads = TempDir::new().unwrap();
    let store_threads = HandoffStore::new(dir_threads.path());
    let rec_a = store_threads
        .create(base_create_req("8-thread race target"))
        .unwrap();

    let thread_barrier = Arc::new(Barrier::new(8));
    let mut handles = Vec::with_capacity(8);
    for idx in 0..8 {
        let vault = dir_threads.path().to_path_buf();
        let id = rec_a.id.clone();
        let barrier = Arc::clone(&thread_barrier);
        handles.push(std::thread::spawn(move || {
            let store = HandoffStore::new(vault);
            barrier.wait();
            store.claim(ClaimHandoffRequest {
                agent: format!("thread-{}", idx),
                id: Some(id),
                ..Default::default()
            })
        }));
    }

    let mut winners = Vec::new();
    let mut losers = 0usize;
    for h in handles {
        match h.join().unwrap() {
            Ok(claimed) => winners.push(claimed),
            Err(HandoffError::AlreadyClaimedOrMissing(_)) => losers += 1,
            Err(other) => panic!("unexpected race error: {}", other),
        }
    }
    assert_eq!(winners.len(), 1, "expected exactly 1 thread winner");
    assert_eq!(losers, 7, "expected 7 thread losers");

    // 2) Combined 8-thread + 4-subprocess race on the same handoff ID.
    let dir_combo = TempDir::new().unwrap();
    let store_combo = HandoffStore::new(dir_combo.path());
    let rec_b = store_combo
        .create(base_create_req("Thread + subprocess race target"))
        .unwrap();

    let exe = std::env::current_exe().expect("current test binary");
    let sync_dir = TempDir::new().unwrap();
    let go_file = sync_dir.path().join("go");

    let num_procs = 4usize;
    let mut children = Vec::with_capacity(num_procs);
    let mut ready_files = Vec::with_capacity(num_procs);
    for p in 0..num_procs {
        let ready = sync_dir.path().join(format!("ready-{}", p));
        let child = Command::new(&exe)
            .args(["--exact", "subprocess_claim_helper"])
            .env(RACE_ENV_FLAG, "1")
            .env(RACE_ENV_VAULT, dir_combo.path())
            .env(RACE_ENV_ID, &rec_b.id)
            .env(RACE_ENV_AGENT, format!("proc-{}", p))
            .env(RACE_ENV_GO_FILE, &go_file)
            .env(RACE_ENV_READY_FILE, &ready)
            .spawn()
            .expect("spawn subprocess racer");
        children.push(child);
        ready_files.push(ready);
    }

    // Wait for all subprocesses to initialize before releasing the race.
    let ready_deadline = Instant::now() + Duration::from_secs(10);
    for rf in &ready_files {
        while !rf.exists() {
            assert!(
                Instant::now() < ready_deadline,
                "subprocess did not become ready in time: {}",
                rf.display()
            );
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    let num_threads = 8usize;
    let combo_barrier = Arc::new(Barrier::new(num_threads + 1));
    let mut combo_handles = Vec::with_capacity(num_threads);
    for t in 0..num_threads {
        let vault = dir_combo.path().to_path_buf();
        let id = rec_b.id.clone();
        let barrier = Arc::clone(&combo_barrier);
        combo_handles.push(std::thread::spawn(move || {
            let store = HandoffStore::new(vault);
            barrier.wait();
            store.claim(ClaimHandoffRequest {
                agent: format!("combo-thread-{}", t),
                id: Some(id),
                ..Default::default()
            })
        }));
    }

    // Release both the 8 threads and all 4 subprocesses simultaneously.
    fs::write(&go_file, "go").unwrap();
    combo_barrier.wait();

    let mut total_winners = 0usize;
    let mut winning_agents = Vec::new();
    for h in combo_handles {
        match h.join().unwrap() {
            Ok(rec) => {
                total_winners += 1;
                winning_agents.push(rec.claimed_by.unwrap());
            }
            Err(HandoffError::AlreadyClaimedOrMissing(_)) => {}
            Err(other) => panic!("unexpected thread race error: {}", other),
        }
    }

    for (idx, child) in children.into_iter().enumerate() {
        let output = child.wait_with_output().expect("wait for child");
        match output.status.code() {
            Some(0) => {
                total_winners += 1;
                winning_agents.push(format!("proc-{}", idx));
            }
            Some(3) => {}
            other => panic!(
                "subprocess {} exited with {:?}; stderr={}",
                idx,
                other,
                String::from_utf8_lossy(&output.stderr)
            ),
        }
    }

    assert_eq!(
        total_winners, 1,
        "expected exactly 1 winner across 8 threads + {} subprocesses, got {:?}",
        num_procs, winning_agents
    );
    assert!(!store_combo
        .pending_dir()
        .join(format!("{}.json", rec_b.id))
        .exists());
    let persisted = store_combo.show(Some(&rec_b.id), None).unwrap();
    assert_eq!(persisted.status, HandoffStatus::Claimed);
    assert_eq!(persisted.record.claimed_by, Some(winning_agents[0].clone()));

    // 3) 8 threads racing `--latest` across 3 pending handoffs: 3 distinct winners, 5 losers.
    let dir_latest = TempDir::new().unwrap();
    let store_latest = HandoffStore::new(dir_latest.path());
    let mut created_ids = HashSet::new();
    for i in 0..3 {
        let rec = store_latest
            .create(base_create_req(&format!("Batch item {}", i)))
            .unwrap();
        created_ids.insert(rec.id);
    }

    let latest_barrier = Arc::new(Barrier::new(8));
    let mut latest_handles = Vec::with_capacity(8);
    for t in 0..8 {
        let vault = dir_latest.path().to_path_buf();
        let barrier = Arc::clone(&latest_barrier);
        latest_handles.push(std::thread::spawn(move || {
            let store = HandoffStore::new(vault);
            barrier.wait();
            store.claim(ClaimHandoffRequest {
                agent: format!("latest-thread-{}", t),
                id: None,
                project: None,
                latest: true,
            })
        }));
    }

    let mut claimed_ids = HashSet::new();
    let mut latest_losers = 0usize;
    for h in latest_handles {
        match h.join().unwrap() {
            Ok(rec) => {
                assert!(
                    claimed_ids.insert(rec.id.clone()),
                    "duplicate claim on {}",
                    rec.id
                );
            }
            Err(HandoffError::AlreadyClaimedOrMissing(_)) => latest_losers += 1,
            Err(other) => panic!("unexpected --latest race error: {}", other),
        }
    }
    assert_eq!(claimed_ids, created_ids);
    assert_eq!(latest_losers, 5);
}
