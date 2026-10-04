use std::fs;

use assert_cmd::Command;
use k0maru::convention::{FlushEngine, FlushRequest};
use k0maru::core::traits::CacheStorage;
use k0maru::storage::SqliteStorage;
use tempfile::tempdir;

#[test]
fn test_flush_dry_run() {
    let temp = tempdir().unwrap();
    let vault_root = temp.path();

    let engine = FlushEngine::new(vault_root).unwrap();

    let request = FlushRequest {
        title: "Dry Run Architecture Note".to_string(),
        summary: Some("Validating zero-disk write preview".to_string()),
        content: "Detailed technical evaluation without disk modification.".to_string(),
        category: "decision".to_string(),
        tags: vec!["arch".to_string(), "preview".to_string()],
        related_notes: vec!["Roadmap 2026".to_string()],
        dry_run: true,
    };

    let result = engine.flush(request).unwrap();

    assert!(result.dry_run, "Result must indicate dry_run");
    assert!(
        !result.file_path.exists(),
        "Dry-run must not create any file on disk"
    );
    assert!(
        result.content_preview.contains("Dry Run Architecture Note"),
        "Preview must contain note title"
    );
    assert!(
        result
            .content_preview
            .contains("Validating zero-disk write preview"),
        "Preview must contain summary"
    );
    assert!(
        result.content_preview.contains("[[Roadmap 2026]]"),
        "Preview must contain WikiLinks"
    );
}

#[test]
fn test_flush_live_write() {
    let temp = tempdir().unwrap();
    let vault_root = temp.path();

    // Create a subfolder convention structure
    fs::create_dir_all(vault_root.join("logs")).unwrap();

    let engine = FlushEngine::new(vault_root).unwrap();

    let request = FlushRequest {
        title: "Live Execution Milestone".to_string(),
        summary: Some("Successfully completed sprint ticket".to_string()),
        content: "Implemented flush engine with anti-collision and auto-sync.".to_string(),
        category: "log".to_string(),
        tags: vec!["sprint".to_string(), "milestone".to_string()],
        related_notes: vec!["Ticket 30".to_string()],
        dry_run: false,
    };

    let result = engine.flush(request).unwrap();

    assert!(!result.dry_run, "Dry run flag must be false");
    assert!(result.created, "Created flag must be true");
    assert!(
        result.file_path.exists(),
        "Target file must exist on disk after flush"
    );

    let on_disk_content = fs::read_to_string(&result.file_path).unwrap();
    assert_eq!(
        on_disk_content, result.content_preview,
        "File on disk must match content preview exactly"
    );
    assert!(
        on_disk_content.starts_with("---"),
        "Content must have YAML frontmatter"
    );
    assert!(
        on_disk_content.contains("Live Execution Milestone"),
        "Content must include note title"
    );
    assert!(
        on_disk_content.contains("[[Ticket 30]]"),
        "Content must include formatted WikiLinks"
    );
}

#[test]
fn test_flush_collision_avoidance() {
    let temp = tempdir().unwrap();
    let vault_root = temp.path();

    let engine = FlushEngine::new(vault_root).unwrap();

    let request1 = FlushRequest {
        title: "Database Migration Decision".to_string(),
        summary: Some("Initial decision".to_string()),
        content: "Version 1: chose SQLite-Vec for local vector storage.".to_string(),
        category: "decision".to_string(),
        tags: vec!["db".to_string()],
        related_notes: vec![],
        dry_run: false,
    };

    let res1 = engine.flush(request1.clone()).unwrap();
    assert!(res1.created, "First flush should create the file");
    let file1_path = res1.file_path.clone();
    assert!(file1_path.exists());

    // Flush identical content: should deduplicate without creating a duplicate file
    let res_dup = engine.flush(request1).unwrap();
    assert!(
        !res_dup.created,
        "Identical content flush must indicate created: false"
    );
    assert_eq!(res_dup.file_path, file1_path);

    // Flush modified content with same title: must create `<stem>-v2.md`
    let request2 = FlushRequest {
        title: "Database Migration Decision".to_string(),
        summary: Some("Updated decision".to_string()),
        content: "Version 2: updated memory limits and cache page size.".to_string(),
        category: "decision".to_string(),
        tags: vec!["db".to_string(), "tuning".to_string()],
        related_notes: vec![],
        dry_run: false,
    };

    let res2 = engine.flush(request2).unwrap();
    assert!(res2.created, "Second version should create a new file");
    assert_ne!(res2.file_path, file1_path);
    assert!(
        res2.file_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .contains("-v2.md"),
        "Second file should have -v2.md suffix: {}",
        res2.file_path.display()
    );

    // Both files must exist independently with their original contents
    let content1 = fs::read_to_string(&file1_path).unwrap();
    let content2 = fs::read_to_string(&res2.file_path).unwrap();
    assert!(content1.contains("Version 1: chose SQLite-Vec"));
    assert!(content2.contains("Version 2: updated memory limits"));

    // Flush modified content again: must create `<stem>-v3.md`
    let request3 = FlushRequest {
        title: "Database Migration Decision".to_string(),
        summary: Some("Third update".to_string()),
        content: "Version 3: added WAL mode auto-checkpoint.".to_string(),
        category: "decision".to_string(),
        tags: vec!["db".to_string()],
        related_notes: vec![],
        dry_run: false,
    };

    let res3 = engine.flush(request3).unwrap();
    assert!(res3.created);
    assert!(
        res3.file_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .contains("-v3.md"),
        "Third file should have -v3.md suffix: {}",
        res3.file_path.display()
    );
}

#[test]
fn test_flush_auto_sync() {
    let temp = tempdir().unwrap();
    let vault_root = temp.path();

    // Prepare .k0maru/cache.sqlite
    let k0maru_dir = vault_root.join(".k0maru");
    fs::create_dir_all(&k0maru_dir).unwrap();
    let cache_path = k0maru_dir.join("cache.sqlite");

    // Initialize sqlite schema
    {
        let storage = SqliteStorage::open(&cache_path).unwrap();
        // ensure schema is written
        let empty_search = storage.search_fts("QuantumComputingZeta", 5).unwrap();
        assert!(empty_search.is_empty());
    }

    let engine = FlushEngine::new(vault_root).unwrap();

    let request = FlushRequest {
        title: "Quantum Computing Benchmark".to_string(),
        summary: Some("Simulating qubits in rust".to_string()),
        content: "QuantumComputingZeta experimental test notes with high fidelity.".to_string(),
        category: "concept".to_string(),
        tags: vec!["quantum".to_string()],
        related_notes: vec![],
        dry_run: false,
    };

    let result = engine.flush(request).unwrap();
    assert!(result.created);

    // Verify that SQLite FTS index was updated automatically
    let storage = SqliteStorage::open(&cache_path).unwrap();
    let hits = storage.search_fts("QuantumComputingZeta", 5).unwrap();
    assert_eq!(hits.len(), 1, "Should find flushed note in FTS5 cache");
    assert!(hits[0].title.contains("Quantum Computing Benchmark"));
}

#[test]
fn test_cli_flush_smoke() {
    let temp = tempdir().unwrap();
    let vault_root = temp.path();

    // 1. Help message smoke
    let mut help_cmd = Command::cargo_bin("k0maru").expect("binary k0maru should exist");
    help_cmd
        .arg("flush")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("--title"))
        .stdout(predicates::str::contains("--category"))
        .stdout(predicates::str::contains("--dry-run"))
        .stdout(predicates::str::contains("--json"));

    // 2. CLI Dry Run
    let mut dry_cmd = Command::cargo_bin("k0maru").expect("binary k0maru should exist");
    dry_cmd
        .arg("flush")
        .arg("--vault")
        .arg(vault_root)
        .arg("--title")
        .arg("CLI Smoke Note")
        .arg("--content")
        .arg("Testing CLI invocation in dry run mode")
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicates::str::contains("[Dry Run] Note preview"));

    // Verify no file was created
    let count = fs::read_dir(vault_root).unwrap().count();
    assert_eq!(count, 0, "No file should be created under dry run");

    // 3. CLI Live Write
    let mut write_cmd = Command::cargo_bin("k0maru").expect("binary k0maru should exist");
    write_cmd
        .arg("flush")
        .arg("--vault")
        .arg(vault_root)
        .arg("--title")
        .arg("CLI Smoke Note")
        .arg("--content")
        .arg("Testing CLI invocation in live write mode")
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "✓ Crystallized note successfully",
        ));

    // Verify file created
    let count_after = fs::read_dir(vault_root).unwrap().count();
    assert_eq!(count_after, 1, "One file should be created");

    // 4. CLI JSON output on identical note
    let mut json_cmd = Command::cargo_bin("k0maru").expect("binary k0maru should exist");
    let assert_json = json_cmd
        .arg("flush")
        .arg("--vault")
        .arg(vault_root)
        .arg("--title")
        .arg("CLI Smoke Note")
        .arg("--content")
        .arg("Testing CLI invocation in live write mode")
        .arg("--json")
        .assert()
        .success();

    let json_output = String::from_utf8_lossy(&assert_json.get_output().stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&json_output).expect("Output should be valid JSON");
    assert_eq!(parsed["created"], false);
    assert_eq!(parsed["dry_run"], false);
}
