mod common;

use k0maru::adapters::{GenericWikiAdapter, ObsidianAdapter};
use k0maru::core::traits::CacheStorage;
use k0maru::scanner::IncrementalScanner;
use k0maru::SqliteStorage;
use std::fs;
use std::path::PathBuf;

#[test]
fn test_incremental_scanner_obsidian_lifecycle() {
    let temp_vault = common::fixtures::mock_obsidian_vault();
    let root = temp_vault.path();

    let adapter = ObsidianAdapter::new(root);
    let mut storage = SqliteStorage::in_memory().expect("Failed to initialize in-memory SQLite");

    // 1. Initial sync: empty cache -> all documents added
    {
        let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
        let stats = scanner.sync(false).expect("Initial sync failed");

        assert_eq!(stats.added, 4);
        assert_eq!(stats.modified, 0);
        assert_eq!(stats.deleted, 0);
        assert_eq!(stats.unchanged, 0);
        assert_eq!(stats.total_processed(), 4);
    }

    // Verify initial indexing worked via FTS search and links
    let search_results = storage
        .search_fts("Architecture Decisions", 5)
        .expect("FTS search failed");
    assert!(!search_results.is_empty());
    assert_eq!(search_results[0].title, "Architecture Decisions");

    // 2. Unmodified second sync: zero modifications, completed instantaneously
    {
        let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
        let stats = scanner.sync(false).expect("Second sync failed");

        assert_eq!(stats.added, 0);
        assert_eq!(stats.modified, 0);
        assert_eq!(stats.deleted, 0);
        assert_eq!(stats.unchanged, 4);
        assert_eq!(stats.total_processed(), 4);
        // Requirement: finishes in <5ms for unmodified vault
        // Allow slight headroom in unoptimized debug build while enforcing sub-20ms
        assert!(
            stats.duration_ms < 20,
            "Unmodified sync took too long: {}ms",
            stats.duration_ms
        );
    }

    // 3. Single file modification: only modified document re-indexed
    let card_path = root.join("20_Cards/Architecture_Decisions.md");
    let updated_content = r#"---
title: Architecture Decisions
status: evergreen
tags:
  - architecture
  - core
  - quantum
---
# Architecture Decisions

Core invariants:
1. Truth in files, performance in cache.
2. Zero proprietary formats.
3. SuperpositionStateMarker for instant FTS retrieval.

Active project reference: [[k0maru-memory|K0maru Memory Project]].
"#;
    fs::write(&card_path, updated_content).expect("Failed to update card file");

    {
        let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
        let stats = scanner.sync(false).expect("Update sync failed");

        assert_eq!(stats.added, 0);
        assert_eq!(stats.modified, 1);
        assert_eq!(stats.deleted, 0);
        assert_eq!(stats.unchanged, 3);
        assert_eq!(stats.total_processed(), 4);
    }

    // Verify updated content is immediately searchable via FTS5
    let updated_results = storage
        .search_fts("SuperpositionStateMarker", 5)
        .expect("FTS search for updated term failed");
    assert_eq!(updated_results.len(), 1);
    assert_eq!(updated_results[0].title, "Architecture Decisions");

    // 4. Single file deletion: document and links cascade-cleaned
    let daily_path = root.join("00_Daily/2026-09-29.md");
    fs::remove_file(&daily_path).expect("Failed to delete daily note file");

    {
        let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
        let stats = scanner.sync(false).expect("Deletion sync failed");

        assert_eq!(stats.added, 0);
        assert_eq!(stats.modified, 0);
        assert_eq!(stats.deleted, 1);
        assert_eq!(stats.unchanged, 3);
        assert_eq!(stats.total_processed(), 4);
    }

    // Verify deleted document is no longer in SQLite cache
    let deleted_search = storage
        .search_fts("scratchpad", 5)
        .expect("FTS search failed");
    assert!(deleted_search.is_empty());

    let outgoing = storage
        .get_outgoing_links(&PathBuf::from("00_Daily/2026-09-29.md"))
        .expect("Outgoing links check failed");
    assert!(outgoing.is_empty());

    // 5. Force full re-sync: all remaining 3 documents marked as modified
    {
        let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
        let stats = scanner.sync(true).expect("Force full sync failed");

        assert_eq!(stats.added, 0);
        assert_eq!(stats.modified, 3);
        assert_eq!(stats.deleted, 0);
        assert_eq!(stats.unchanged, 0);
        assert_eq!(stats.total_processed(), 3);
    }
}

#[test]
fn test_incremental_scanner_file_addition_and_dirty_sets() {
    let temp_vault = common::fixtures::mock_obsidian_vault();
    let root = temp_vault.path();

    let adapter = ObsidianAdapter::new(root);
    let mut storage = SqliteStorage::in_memory().expect("Failed to initialize in-memory SQLite");

    // Initial sync
    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    let initial_stats = scanner.sync(false).expect("Initial sync failed");
    assert_eq!(initial_stats.added, 4);

    // Add a new note into 10_Projects
    let new_project_file = root.join("10_Projects/new-agent.md");
    let content = r#"---
title: New Agent Project
tags:
  - agent
---
# New Agent Project
New agent memory hub implementation.
"#;
    fs::write(&new_project_file, content).expect("Failed to write new project file");

    // Check compute_dirty_sets inspection before running sync
    let dirty = scanner
        .compute_dirty_sets(false)
        .expect("Failed to compute dirty sets");
    assert_eq!(dirty.added, vec![PathBuf::from("10_Projects/new-agent.md")]);
    assert_eq!(dirty.modified, Vec::<PathBuf>::new());
    assert_eq!(dirty.deleted, Vec::<PathBuf>::new());
    assert_eq!(dirty.unchanged.len(), 4);

    // Sync now
    let sync_stats = scanner.sync(false).expect("Incremental sync failed");
    assert_eq!(sync_stats.added, 1);
    assert_eq!(sync_stats.modified, 0);
    assert_eq!(sync_stats.deleted, 0);
    assert_eq!(sync_stats.unchanged, 4);

    let search = storage
        .search_fts("New Agent Project", 5)
        .expect("Search failed");
    assert_eq!(search.len(), 1);
}

#[test]
fn test_incremental_scanner_with_generic_wiki_adapter() {
    let temp_wiki = common::fixtures::mock_karpathy_wiki();
    let root = temp_wiki.path();

    let adapter = GenericWikiAdapter::new(root);
    let mut storage = SqliteStorage::in_memory().expect("Failed to initialize in-memory SQLite");

    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);

    // Initial sync: 3 notes (index.md, rust-memory.md, sqlite-fts5.md)
    let stats = scanner.sync(false).expect("Initial sync failed");
    assert_eq!(stats.added, 3);
    assert_eq!(stats.modified, 0);
    assert_eq!(stats.deleted, 0);
    assert_eq!(stats.unchanged, 0);

    // Second sync: 3 unchanged
    let stats2 = scanner.sync(false).expect("Second sync failed");
    assert_eq!(stats2.unchanged, 3);

    // Modify index.md
    let index_file = root.join("index.md");
    let updated_index = r#"---
title: LLM Wiki Index
tags:
  - index
  - wiki
---
# LLM Wiki Index

Updated index content with BrandNewUniqueWikiTerm.
- [[rust-memory]]
"#;
    fs::write(&index_file, updated_index).expect("Failed to update index file");

    let stats3 = scanner.sync(false).expect("Update sync failed");
    assert_eq!(stats3.added, 0);
    assert_eq!(stats3.modified, 1);
    assert_eq!(stats3.unchanged, 2);

    let search = storage
        .search_fts("BrandNewUniqueWikiTerm", 5)
        .expect("FTS search failed");
    assert_eq!(search.len(), 1);
}
