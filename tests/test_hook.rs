mod common;

use std::fs;

use common::fixtures::mock_obsidian_vault;
use k0maru::hook::{
    build_timeline, inspect_note_detail, run_hook, HookEvent, HookOptions, TimelineQuery,
};

#[test]
fn test_run_hook_session_start() {
    let vault = mock_obsidian_vault();
    let options = HookOptions {
        event: HookEvent::SessionStart,
        vault_path: vault.path().to_path_buf(),
        repo_path: Some(vault.path().to_path_buf()),
        project: Some("k0maru-memory".into()),
        tool_name: None,
        summary: None,
        stdin_payload: None,
    };

    let report = run_hook(&options).expect("SessionStart hook should succeed");
    assert_eq!(report.event, HookEvent::SessionStart);
    assert!(report.handled);
    assert!(report.guard_passed);
    assert!(report.output_markdown.is_some());

    let md = report.output_markdown.unwrap();
    assert!(md.contains("k0maru-memory"));
    let tokens = k0maru::loadout::estimate_tokens(&md);
    assert!(
        tokens <= 300,
        "Loadout token budget should be <= 300 tokens, got {}",
        tokens
    );
}

#[test]
fn test_run_hook_post_tool_failure() {
    let vault = mock_obsidian_vault();

    // 1. Empty payload returns handled: false
    let empty_options = HookOptions {
        event: HookEvent::PostToolFailure,
        vault_path: vault.path().to_path_buf(),
        repo_path: None,
        project: None,
        tool_name: Some("cargo".into()),
        summary: None,
        stdin_payload: Some("   ".into()),
    };
    let empty_report = run_hook(&empty_options).expect("Hook should run");
    assert!(!empty_report.handled);

    // 2. Real tool failure appends to session observations
    let options = HookOptions {
        event: HookEvent::PostToolFailure,
        vault_path: vault.path().to_path_buf(),
        repo_path: None,
        project: None,
        tool_name: Some("cargo-build".into()),
        summary: Some("error[E0432]: unresolved import `foo::bar`".into()),
        stdin_payload: None,
    };
    let report = run_hook(&options).expect("Hook should run");
    assert!(report.handled);
    assert!(report.observation_path.is_some());

    let obs_path = report.observation_path.unwrap();
    assert!(obs_path.exists());
    let content = fs::read_to_string(&obs_path).expect("Should read observation file");
    assert!(content.contains("### ["));
    assert!(content.contains("Tool Failure (cargo-build)"));
    assert!(content.contains("error[E0432]: unresolved import `foo::bar`"));
    assert!(content.contains("hierarchy: L2Log"));
}

#[test]
fn test_run_hook_stop_prevents_loop_on_stop_hook_active() {
    let vault = mock_obsidian_vault();
    let options = HookOptions {
        event: HookEvent::Stop,
        vault_path: vault.path().to_path_buf(),
        repo_path: Some(vault.path().to_path_buf()),
        project: None,
        tool_name: None,
        summary: None,
        stdin_payload: Some("{\"stop_hook_active\": true}".into()),
    };

    let report = run_hook(&options).expect("Hook should run");
    assert!(!report.handled);
    assert_eq!(report.skipped_reason.as_deref(), Some("stop_hook_active"));
    assert!(report.observation_path.is_none());
}

#[test]
fn test_run_hook_session_end_with_summary() {
    let vault = mock_obsidian_vault();
    let options = HookOptions {
        event: HookEvent::SessionEnd,
        vault_path: vault.path().to_path_buf(),
        repo_path: Some(vault.path().to_path_buf()),
        project: None,
        tool_name: None,
        summary: Some("Completed feature implementation and tests".into()),
        stdin_payload: None,
    };

    let report = run_hook(&options).expect("SessionEnd hook should succeed");
    assert!(report.handled);
    assert!(report.observation_path.is_some());

    let obs_path = report.observation_path.unwrap();
    let content = fs::read_to_string(&obs_path).unwrap();
    assert!(content.contains("Session Handoff Summary"));
    assert!(content.contains("Completed feature implementation and tests"));
}

#[test]
fn test_build_timeline() {
    let vault = mock_obsidian_vault();
    let entries =
        build_timeline(vault.path(), &TimelineQuery::default()).expect("Timeline should build");
    assert!(!entries.is_empty(), "Should discover notes in mock vault");

    // Sorted chronologically descending
    for i in 1..entries.len() {
        assert!(
            entries[i - 1].date >= entries[i].date,
            "Timeline entries must be descending by date"
        );
    }

    // Anchor search
    let query_with_anchor = TimelineQuery {
        anchor: Some("eval".into()),
        window: 3,
        project: None,
    };
    let anchored_entries = build_timeline(vault.path(), &query_with_anchor)
        .expect("Timeline with anchor should build");
    assert!(!anchored_entries.is_empty());
    assert!(anchored_entries.len() <= 3);

    // Project filtering
    let query_with_project = TimelineQuery {
        anchor: None,
        window: 10,
        project: Some("k0maru-memory".into()),
    };
    let project_entries = build_timeline(vault.path(), &query_with_project)
        .expect("Timeline with project filter should build");
    for e in &project_entries {
        let matches = e
            .project
            .as_deref()
            .is_some_and(|p| p.eq_ignore_ascii_case("k0maru-memory"))
            || e.relative_path.to_lowercase().contains("k0maru-memory");
        assert!(matches, "Filtered entry should match project: {:?}", e);
    }
}

#[test]
fn test_inspect_note_detail_success_and_security() {
    let vault = mock_obsidian_vault();

    // 1. Valid note inspection
    let report = inspect_note_detail(vault.path(), "10_Projects/k0maru-memory.md")
        .expect("Should inspect valid note");
    assert_eq!(report.relative_path, "10_Projects/k0maru-memory.md");
    assert!(!report.title.is_empty());
    assert_eq!(report.status, "active");
    assert!(!report.body.is_empty());

    // 2. Reject empty path
    assert!(inspect_note_detail(vault.path(), "   ").is_err());

    // 3. Reject absolute path
    assert!(inspect_note_detail(vault.path(), "/etc/passwd").is_err());

    // 4. Reject parent directory traversal
    assert!(inspect_note_detail(vault.path(), "../Cargo.toml").is_err());
    assert!(inspect_note_detail(vault.path(), "10_Projects/../../Cargo.toml").is_err());

    // 5. Reject non-markdown file
    assert!(inspect_note_detail(vault.path(), "Cargo.toml").is_err());
}
