mod common;

use k0maru::adapters::{GenericWikiAdapter, ObsidianAdapter};
use k0maru::core::models::HierarchyLevel;
use k0maru::core::traits::VaultAdapter;
use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn test_obsidian_scan_files_and_ignore_hidden() {
    let temp_vault = common::fixtures::mock_obsidian_vault();
    let root = temp_vault.path();

    // Create ignored directories and files
    fs::create_dir_all(root.join(".git")).unwrap();
    fs::write(root.join(".git/config"), "dummy git config").unwrap();
    fs::write(root.join(".git/HEAD"), "ref: refs/heads/main").unwrap();

    fs::create_dir_all(root.join(".obsidian")).unwrap();
    fs::write(root.join(".obsidian/workspace.json"), "{}").unwrap();
    fs::write(root.join(".obsidian/app.json"), "{}").unwrap();

    fs::create_dir_all(root.join(".trash")).unwrap();
    fs::write(root.join(".trash/deleted.md"), "# Deleted note").unwrap();

    fs::create_dir_all(root.join(".k0maru")).unwrap();
    fs::write(root.join(".k0maru/cache.sqlite"), "binary").unwrap();

    fs::create_dir_all(root.join(".scratch")).unwrap();
    fs::write(root.join(".scratch/temp.md"), "# Scratch note").unwrap();

    fs::write(root.join(".hidden.md"), "# Hidden root file").unwrap();

    let adapter = ObsidianAdapter::new(root);
    let files = adapter.scan_files().expect("Failed to scan files");

    // Must find exactly the 4 mock markdown files
    assert_eq!(files.len(), 4);

    let expected_files: Vec<PathBuf> = vec![
        PathBuf::from("00_Daily/2026-09-29.md"),
        PathBuf::from("01_AI_Logs/2026-09-29-eval.md"),
        PathBuf::from("10_Projects/k0maru-memory.md"),
        PathBuf::from("20_Cards/Architecture_Decisions.md"),
    ];

    assert_eq!(files, expected_files);

    // Verify none of the hidden or trash files leaked in
    for f in &files {
        let s = f.to_string_lossy();
        assert!(!s.starts_with('.'));
        assert!(!s.contains("/."));
        assert!(!s.contains(".trash"));
        assert!(!s.contains(".git"));
        assert!(!s.contains(".obsidian"));
        assert!(!s.contains(".k0maru"));
        assert!(!s.contains(".scratch"));
    }
}

#[test]
fn test_obsidian_read_document_all_fields() {
    let temp_vault = common::fixtures::mock_obsidian_vault();
    let root = temp_vault.path();
    let adapter = ObsidianAdapter::new(root);

    // 1. 00_Daily
    let daily_path = Path::new("00_Daily/2026-09-29.md");
    let daily_doc = adapter
        .read_document(daily_path)
        .expect("Failed to read daily doc");
    assert_eq!(daily_doc.path, daily_path);
    assert_eq!(daily_doc.title, "2026-09-29 Daily Log");
    assert_eq!(daily_doc.hierarchy, HierarchyLevel::L0Ephemeral);
    assert!(daily_doc.tags.contains(&"daily".to_string()));
    assert!(daily_doc.tags.contains(&"scratchpad".to_string()));
    assert_eq!(daily_doc.links.len(), 1);
    assert_eq!(daily_doc.links[0].target, "Architecture_Decisions");
    assert_eq!(daily_doc.links[0].alias, None);
    assert_eq!(daily_doc.content_hash.len(), 16);
    assert!(daily_doc.mtime > 0);
    assert!(daily_doc.body.contains("Quick scratch notes for today."));

    // 2. 01_AI_Logs
    let log_path = Path::new("01_AI_Logs/2026-09-29-eval.md");
    let log_doc = adapter
        .read_document(log_path)
        .expect("Failed to read log doc");
    assert_eq!(log_doc.path, log_path);
    assert_eq!(log_doc.title, "Evaluation Run 01");
    assert_eq!(log_doc.hierarchy, HierarchyLevel::L2Log);
    assert!(log_doc.tags.contains(&"ai".to_string()));
    assert!(log_doc.tags.contains(&"eval".to_string()));
    assert_eq!(log_doc.links.len(), 1);
    assert_eq!(log_doc.links[0].target, "Architecture_Decisions");
    assert_eq!(
        log_doc.links[0].alias,
        Some("Architecture Reference".to_string())
    );

    // 3. 10_Projects
    let project_path = Path::new("10_Projects/k0maru-memory.md");
    let project_doc = adapter
        .read_document(project_path)
        .expect("Failed to read project doc");
    assert_eq!(project_doc.path, project_path);
    assert_eq!(project_doc.title, "K0maru Agent Memory");
    assert_eq!(project_doc.hierarchy, HierarchyLevel::L3Evergreen);
    assert!(project_doc.tags.contains(&"project".to_string()));
    assert!(project_doc.tags.contains(&"rust".to_string()));
    assert_eq!(project_doc.links.len(), 1);
    assert_eq!(project_doc.links[0].target, "Architecture_Decisions");

    // 4. 20_Cards
    let card_path = Path::new("20_Cards/Architecture_Decisions.md");
    let card_doc = adapter
        .read_document(card_path)
        .expect("Failed to read card doc");
    assert_eq!(card_doc.path, card_path);
    assert_eq!(card_doc.title, "Architecture Decisions");
    assert_eq!(card_doc.hierarchy, HierarchyLevel::L3Evergreen);
    assert!(card_doc.tags.contains(&"architecture".to_string()));
    assert!(card_doc.tags.contains(&"core".to_string()));
    assert_eq!(card_doc.links.len(), 1);
    assert_eq!(card_doc.links[0].target, "k0maru-memory");
    assert_eq!(
        card_doc.links[0].alias,
        Some("K0maru Memory Project".to_string())
    );
}

#[test]
fn test_obsidian_frontmatter_overrides_folder_heuristics() {
    let temp_vault = common::fixtures::mock_obsidian_vault();
    let root = temp_vault.path();
    let adapter = ObsidianAdapter::new(root);

    // Case 1: L3 card located in 00_Daily with hierarchy: L3
    let daily_override = root.join("00_Daily/daily_override.md");
    fs::write(
        &daily_override,
        r#"---
hierarchy: L3
---
# Permanent Thought in Daily
Important core invariant.
"#,
    )
    .unwrap();

    let doc = adapter
        .read_document(Path::new("00_Daily/daily_override.md"))
        .unwrap();
    assert_eq!(doc.hierarchy, HierarchyLevel::L3Evergreen);

    // Case 2: L0 log in 20_Cards with level: L0
    let card_override = root.join("20_Cards/transient_scratch.md");
    fs::write(
        &card_override,
        r#"---
level: L0
---
# Just a Scratchpad
Temporary ideas.
"#,
    )
    .unwrap();

    let doc = adapter
        .read_document(Path::new("20_Cards/transient_scratch.md"))
        .unwrap();
    assert_eq!(doc.hierarchy, HierarchyLevel::L0Ephemeral);

    // Case 3: L2 log declared by integer level: 2
    let int_override = root.join("20_Cards/int_override.md");
    fs::write(
        &int_override,
        r#"---
level: 2
---
# Int Level Log
Trace details.
"#,
    )
    .unwrap();

    let doc = adapter
        .read_document(Path::new("20_Cards/int_override.md"))
        .unwrap();
    assert_eq!(doc.hierarchy, HierarchyLevel::L2Log);

    // Case 4: L1 Resource declared by word "resource"
    let word_override = root.join("01_AI_Logs/cli_cheat.md");
    fs::write(
        &word_override,
        r#"---
hierarchy: resource
---
# CLI Cheatsheet
Commands list.
"#,
    )
    .unwrap();

    let doc = adapter
        .read_document(Path::new("01_AI_Logs/cli_cheat.md"))
        .unwrap();
    assert_eq!(doc.hierarchy, HierarchyLevel::L1Resource);
}

#[test]
fn test_obsidian_write_card_and_non_destructive_collision() {
    let temp_vault = common::fixtures::mock_obsidian_vault();
    let root = temp_vault.path();
    let adapter = ObsidianAdapter::new(root);

    // 1. Write an L3 Evergreen card
    let path_l3 = adapter
        .write_card(
            "Cleanroom Design",
            "# Cleanroom Design\nInvariants here.",
            HierarchyLevel::L3Evergreen,
        )
        .expect("Failed to write L3 card");
    assert_eq!(path_l3, PathBuf::from("20_Cards/Cleanroom Design.md"));
    assert!(root.join(&path_l3).exists());

    // Verify it parses cleanly
    let doc_l3 = adapter.read_document(&path_l3).unwrap();
    assert_eq!(doc_l3.title, "Cleanroom Design");
    assert_eq!(doc_l3.hierarchy, HierarchyLevel::L3Evergreen);
    assert!(doc_l3.body.contains("Invariants here."));

    // 2. Write an L2 Log card
    let path_l2 = adapter
        .write_card(
            "Benchmark Trace",
            "# Benchmark Trace\nLatency: 1ms",
            HierarchyLevel::L2Log,
        )
        .expect("Failed to write L2 log");
    assert_eq!(path_l2, PathBuf::from("01_AI_Logs/Benchmark Trace.md"));
    assert!(root.join(&path_l2).exists());

    // 3. Write an L0 Ephemeral daily card
    let path_l0 = adapter
        .write_card(
            "Quick Scratch",
            "# Quick Scratch",
            HierarchyLevel::L0Ephemeral,
        )
        .expect("Failed to write L0 card");
    assert_eq!(path_l0, PathBuf::from("00_Daily/Quick Scratch.md"));
    assert!(root.join(&path_l0).exists());

    // 4. Non-destructive Collision test: write another card with identical title
    let path_collision = adapter
        .write_card(
            "Cleanroom Design",
            "# Cleanroom Design Part 2\nUpdated invariants.",
            HierarchyLevel::L3Evergreen,
        )
        .expect("Failed to write colliding card");

    assert_eq!(
        path_collision,
        PathBuf::from("20_Cards/Cleanroom Design_1.md")
    );
    assert!(root.join(&path_collision).exists());

    // Verify the original card was NOT overwritten or mutated
    let original_doc = adapter.read_document(&path_l3).unwrap();
    assert!(original_doc.body.contains("Invariants here."));
    assert!(!original_doc.body.contains("Updated invariants."));

    let new_doc = adapter.read_document(&path_collision).unwrap();
    assert!(new_doc.body.contains("Updated invariants."));

    // Verify scan_files now includes both
    let files = adapter.scan_files().unwrap();
    assert!(files.contains(&path_l3));
    assert!(files.contains(&path_collision));
}

#[test]
fn test_generic_wiki_adapter_scan_and_read() {
    let temp_wiki = common::fixtures::mock_karpathy_wiki();
    let root = temp_wiki.path();

    // Create ignored directories
    fs::create_dir_all(root.join(".git")).unwrap();
    fs::write(root.join(".git/HEAD"), "ref: refs/heads/main").unwrap();
    fs::create_dir_all(root.join(".trash")).unwrap();
    fs::write(root.join(".trash/old.md"), "# Old").unwrap();

    let adapter = GenericWikiAdapter::new(root);
    let files = adapter.scan_files().expect("Failed to scan generic wiki");

    let expected_files = vec![
        PathBuf::from("index.md"),
        PathBuf::from("rust-memory.md"),
        PathBuf::from("sqlite-fts5.md"),
    ];
    assert_eq!(files, expected_files);

    // Read index.md
    let index_doc = adapter
        .read_document(Path::new("index.md"))
        .expect("Failed to read index");
    assert_eq!(index_doc.path, PathBuf::from("index.md"));
    assert_eq!(index_doc.title, "LLM Wiki Index");
    assert_eq!(index_doc.hierarchy, HierarchyLevel::L3Evergreen);
    assert!(index_doc.tags.contains(&"index".to_string()));
    assert!(index_doc.tags.contains(&"wiki".to_string()));
    assert_eq!(index_doc.links.len(), 2);
    assert_eq!(index_doc.links[0].target, "rust-memory");
    assert_eq!(index_doc.links[1].target, "sqlite-fts5");
    assert_eq!(
        index_doc.links[1].alias,
        Some("SQLite FTS5 Search".to_string())
    );

    // Read rust-memory.md
    let rust_doc = adapter
        .read_document(Path::new("rust-memory.md"))
        .expect("Failed to read rust-memory");
    assert_eq!(rust_doc.title, "Rust Memory Architecture");
    assert_eq!(rust_doc.hierarchy, HierarchyLevel::L3Evergreen);
    assert_eq!(rust_doc.links.len(), 2);
}

#[test]
fn test_generic_wiki_frontmatter_override_and_configurable_default() {
    let temp_wiki = common::fixtures::mock_karpathy_wiki();
    let root = temp_wiki.path();

    // Configure default to L1Resource
    let adapter = GenericWikiAdapter::new(root).with_default_hierarchy(HierarchyLevel::L1Resource);
    assert_eq!(adapter.default_hierarchy(), HierarchyLevel::L1Resource);

    // File without hierarchy frontmatter uses configured default (L1Resource)
    let index_doc = adapter.read_document(Path::new("index.md")).unwrap();
    assert_eq!(index_doc.hierarchy, HierarchyLevel::L1Resource);

    // File with explicit hierarchy frontmatter overrides default
    let custom_file = root.join("custom_evergreen.md");
    fs::write(
        &custom_file,
        r#"---
hierarchy: L3Evergreen
tags:
  - test
---
# Custom Evergreen Note
High quality knowledge.
"#,
    )
    .unwrap();

    let custom_doc = adapter
        .read_document(Path::new("custom_evergreen.md"))
        .unwrap();
    assert_eq!(custom_doc.hierarchy, HierarchyLevel::L3Evergreen);
}

#[test]
fn test_generic_wiki_write_card_and_collision() {
    let temp_wiki = common::fixtures::mock_karpathy_wiki();
    let root = temp_wiki.path();
    let adapter = GenericWikiAdapter::new(root);

    // Write flat card in root
    let path = adapter
        .write_card(
            "new-concept",
            "# New Concept\nFlat wiki card.",
            HierarchyLevel::L3Evergreen,
        )
        .expect("Failed to write card in flat wiki");

    assert_eq!(path, PathBuf::from("new-concept.md"));
    assert!(root.join(&path).exists());

    let doc = adapter.read_document(&path).unwrap();
    assert_eq!(doc.title, "New Concept");
    assert_eq!(doc.hierarchy, HierarchyLevel::L3Evergreen);

    // Collision check
    let collision_path = adapter
        .write_card(
            "new-concept",
            "# New Concept Collision\nSecond version.",
            HierarchyLevel::L3Evergreen,
        )
        .expect("Failed to write collision card in flat wiki");

    assert_eq!(collision_path, PathBuf::from("new-concept_1.md"));
    assert!(root.join(&collision_path).exists());

    // Verify first card unchanged
    let original = adapter.read_document(&path).unwrap();
    assert!(original.body.contains("Flat wiki card."));
}

#[test]
fn test_adapter_edge_cases_and_error_handling() {
    let temp_vault = common::fixtures::mock_obsidian_vault();
    let root = temp_vault.path();
    let adapter = ObsidianAdapter::new(root);

    // 1. Reading non-existent document returns Err
    let non_existent = Path::new("20_Cards/does_not_exist.md");
    assert!(adapter.read_document(non_existent).is_err());

    // 2. Reading document with absolute path inside vault root works
    let abs_path = root.join("20_Cards/Architecture_Decisions.md");
    let doc_abs = adapter.read_document(&abs_path).unwrap();
    assert_eq!(doc_abs.title, "Architecture Decisions");
    assert_eq!(
        doc_abs.path,
        PathBuf::from("20_Cards/Architecture_Decisions.md")
    );

    // 3. Document without H1 and without frontmatter title falls back to file stem
    let no_h1_path = root.join("20_Cards/raw_stem_test.md");
    fs::write(
        &no_h1_path,
        r#"---
status: draft
---
No H1 heading here, just plain text.
"#,
    )
    .unwrap();

    let doc_no_h1 = adapter
        .read_document(Path::new("20_Cards/raw_stem_test.md"))
        .unwrap();
    assert_eq!(doc_no_h1.title, "raw_stem_test");

    // 4. Document with inline tag and frontmatter tags combined
    let mixed_tags_path = root.join("20_Cards/mixed_tags.md");
    fs::write(
        &mixed_tags_path,
        r#"---
tags:
  - rust
  - systems
---
# Mixed Tags Note
Check out #perf/zero-cost and #rust inline tags.
"#,
    )
    .unwrap();

    let mixed_doc = adapter
        .read_document(Path::new("20_Cards/mixed_tags.md"))
        .unwrap();
    assert!(mixed_doc.tags.contains(&"rust".to_string()));
    assert!(mixed_doc.tags.contains(&"systems".to_string()));
    assert!(mixed_doc.tags.contains(&"#perf/zero-cost".to_string()));

    // 5. Title with sanitization required
    let safe_path = adapter
        .write_card(
            "Weird: Title / With * Bad ? Chars",
            "# Weird Title",
            HierarchyLevel::L3Evergreen,
        )
        .unwrap();
    assert_eq!(
        safe_path,
        PathBuf::from("20_Cards/Weird_ Title _ With _ Bad _ Chars.md")
    );
    assert!(root.join(&safe_path).exists());
}
