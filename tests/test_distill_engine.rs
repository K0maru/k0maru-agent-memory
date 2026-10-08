use std::fs;
use tempfile::tempdir;

use k0maru::core::CacheStorage;
use k0maru::distill::{DistillEngine, DistillOptions};
use k0maru::offload::OffloadEngine;
use k0maru::storage::SqliteStorage;

#[test]
fn test_distill_text_safe_flush_to_vault() {
    let vault_tmp = tempdir().unwrap();
    let vault_root = vault_tmp.path();
    let refs_tmp = tempdir().unwrap();
    let refs_dir = refs_tmp.path();

    // Create a skills target folder
    fs::create_dir_all(vault_root.join("skills")).unwrap();

    let engine = DistillEngine::new(vault_root, refs_dir);

    let trace = r#"
$ cargo build --release
error[E0432]: unresolved import `k0maru::distill`
 --> src/lib.rs:9:9
  |
9 | pub mod distill;
  |         ^^^^^^^ no `distill` in the root

error: could not compile `k0maru` (lib) due to 1 previous error
$ touch src/distill/mod.rs
$ cargo build --release
    Finished `release` profile [optimized] target(s) in 0.12s
"#;

    let opts = DistillOptions {
        title: Some("Resolve Missing Distill Module Error".to_string()),
        context_hint: Some("Creating missing module during release build".to_string()),
        category: Some("skill".to_string()),
        tags: vec!["topic/rust".to_string()],
        related_notes: vec!["Modular Architecture".to_string()],
        dry_run: false,
        ..Default::default()
    };

    let result = engine.distill_text(trace, opts).unwrap();

    assert_eq!(result.skill.title, "Resolve Missing Distill Module Error");
    assert!(result.flush_result.created);
    assert!(!result.flush_result.dry_run);
    assert!(result.flush_result.file_path.exists());
    assert!(result
        .flush_result
        .relative_path
        .to_string_lossy()
        .starts_with("skills/"));

    let content = fs::read_to_string(&result.flush_result.file_path).unwrap();
    assert!(content.contains("# Resolve Missing Distill Module Error"));
    assert!(content.contains("## 🎯 触发上下文与应用场景 (Trigger Context)"));
    assert!(content.contains("## 🔍 故障根因与错误特征 (Root Cause & Signatures)"));
    assert!(content.contains("## 🛠️ 修复策略与执行命令 (Remediation & Commands)"));
    assert!(content.contains("## 🛡️ 防范规约与常青法则 (Prevention Rules & Best Practices)"));
    assert!(content.contains("[[Modular Architecture]]"));
}

#[test]
fn test_distill_dry_run_does_not_write_to_disk() {
    let vault_tmp = tempdir().unwrap();
    let vault_root = vault_tmp.path();
    let refs_tmp = tempdir().unwrap();
    let refs_dir = refs_tmp.path();

    fs::create_dir_all(vault_root.join("skills")).unwrap();

    let engine = DistillEngine::new(vault_root, refs_dir);

    let trace = "command failed: exit status 1";
    let opts = DistillOptions {
        title: Some("Dry Run Preview Test".to_string()),
        context_hint: None,
        category: Some("skill".to_string()),
        tags: vec![],
        related_notes: vec![],
        dry_run: true,
        ..Default::default()
    };

    let result = engine.distill_text(trace, opts).unwrap();

    assert!(result.flush_result.dry_run);
    assert!(!result.flush_result.file_path.exists());
    assert!(result.preview_markdown.contains("# Dry Run Preview Test"));
}

#[test]
fn test_distill_node_from_offload_refs() {
    let vault_tmp = tempdir().unwrap();
    let vault_root = vault_tmp.path();
    let refs_tmp = tempdir().unwrap();
    let refs_dir = refs_tmp.path();

    fs::create_dir_all(vault_root.join("skills")).unwrap();

    // 1. Offload a long trace using OffloadEngine
    let offload_engine = OffloadEngine::new(refs_dir).with_threshold(5);
    let long_log = (1..=20)
        .map(|i| format!("Step {} executing build task...", i))
        .collect::<Vec<_>>()
        .join("\n")
        + "\nerror: fatal compilation abort\n$ cargo fix\nFinished profile";

    let offload_res = offload_engine.offload_text(&long_log).unwrap();
    assert!(offload_res.truncated);
    let node_id = offload_res.node_id;

    // 2. Distill from node reference
    let distill_engine = DistillEngine::new(vault_root, refs_dir);
    let opts = DistillOptions {
        title: Some("Recover from Build Abort".to_string()),
        context_hint: Some("Offload trace recovery".to_string()),
        category: Some("skill".to_string()),
        tags: vec!["topic/build".to_string()],
        related_notes: vec![],
        dry_run: false,
        ..Default::default()
    };

    let result = distill_engine.distill_node(&node_id, opts).unwrap();
    assert!(result.flush_result.created);
    assert!(result.flush_result.file_path.exists());

    let content = fs::read_to_string(&result.flush_result.file_path).unwrap();
    assert!(content.contains("fatal compilation abort"));
}

#[test]
fn test_distill_file_from_disk() {
    let vault_tmp = tempdir().unwrap();
    let vault_root = vault_tmp.path();
    let refs_tmp = tempdir().unwrap();
    let refs_dir = refs_tmp.path();

    fs::create_dir_all(vault_root.join("skills")).unwrap();

    let trace_file = refs_tmp.path().join("trace_sample.log");
    fs::write(
        &trace_file,
        "Traceback (most recent call last):\n  File \"run.py\", line 10\nKeyError: 'ENV'",
    )
    .unwrap();

    let engine = DistillEngine::new(vault_root, refs_dir);
    let opts = DistillOptions {
        title: None,
        context_hint: Some("Missing ENV config".to_string()),
        category: None,
        tags: vec![],
        related_notes: vec![],
        dry_run: false,
        ..Default::default()
    };

    let result = engine.distill_file(&trace_file, opts).unwrap();
    assert!(result.flush_result.created);
    assert!(result.flush_result.file_path.exists());
    assert!(result.skill.root_cause.contains("KeyError"));
}

#[test]
fn test_distill_instant_cache_synchronization() {
    let vault_tmp = tempdir().unwrap();
    let vault_root = vault_tmp.path();
    let refs_tmp = tempdir().unwrap();
    let refs_dir = refs_tmp.path();

    fs::create_dir_all(vault_root.join("skills")).unwrap();

    // Initialize sqlite cache schema before test
    let cache_dir = vault_root.join(".k0maru");
    fs::create_dir_all(&cache_dir).unwrap();
    let cache_db = cache_dir.join("cache.sqlite");
    {
        let storage = SqliteStorage::open(&cache_db).unwrap();
        let _ = storage.search_fts("Resolve Port", 5);
    }

    let engine = DistillEngine::new(vault_root, refs_dir);
    let trace = "error: failed to bind address 127.0.0.1:8080. Address already in use.\n$ kill -9 12345\n$ k0maru ui\nBound server successfully.";

    let opts = DistillOptions {
        title: Some("Resolve Port 8080 Conflict".to_string()),
        context_hint: Some("Port occupation during server startup".to_string()),
        category: Some("skill".to_string()),
        tags: vec!["topic/network".to_string()],
        related_notes: vec![],
        dry_run: false,
        ..Default::default()
    };

    let result = engine.distill_text(trace, opts).unwrap();
    assert!(result.flush_result.created);

    // Verify cache.sqlite was updated and can be searched via FTS5 immediately
    let cache_db = vault_root.join(".k0maru").join("cache.sqlite");
    assert!(cache_db.exists());

    let storage = SqliteStorage::open(&cache_db).unwrap();
    let search_hits = storage.search_fts("Resolve Port 8080 Conflict", 5).unwrap();
    assert!(!search_hits.is_empty());
    assert_eq!(search_hits[0].title, "Resolve Port 8080 Conflict");
}
