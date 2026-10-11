use std::fs;
use std::sync::Arc;

use k0maru_core::{
    extract_elements, parse_frontmatter, CacheStorage, HierarchyLevel, HybridSearchEngine,
    IncrementalScanner, MockEmbeddingEngine, Note, ObsidianAdapter, SearchMode, SqliteStorage,
    VaultAdapter, VectorSyncEngine,
};
use tempfile::tempdir;

#[test]
fn test_core_isolation_full_lifecycle() {
    let temp_vault = tempdir().expect("Failed to create temp vault dir");
    let vault_path = temp_vault.path();

    // 1. Setup mock vault files across hierarchy tiers
    let daily_dir = vault_path.join("00_Daily");
    let cards_dir = vault_path.join("20_Cards");
    fs::create_dir_all(&daily_dir).expect("create daily dir");
    fs::create_dir_all(&cards_dir).expect("create cards dir");

    let card_a = cards_dir.join("Architecture.md");
    let card_b = cards_dir.join("Modularity.md");
    let daily_note = daily_dir.join("2026-10-11.md");

    fs::write(
        &card_a,
        r#"---
title: "Core Architecture"
hierarchy: L3Evergreen
tags:
  - architecture
  - rust
---
# Core Architecture

This note explains the core library decoupling in [[Modularity]].
"#,
    )
    .expect("write card A");

    fs::write(
        &card_b,
        r#"---
title: "Modularity Principles"
hierarchy: L3Evergreen
tags:
  - modularity
---
# Modularity Principles

Deep modules provide simple interfaces for complex operations.
Backlink to [[Architecture]].
"#,
    )
    .expect("write card B");

    fs::write(
        &daily_note,
        r#"# 2026-10-11 Scratchpad

Investigated decoupling today. Refer to [[Architecture|Arch Design]].
#scratch #daily
"#,
    )
    .expect("write daily note");

    // 2. Initialize in-memory SQLite storage
    let mut storage = SqliteStorage::in_memory().expect("in-memory sqlite storage");
    storage.initialize().expect("initialize storage");

    // 3. Scan and sync using ObsidianAdapter and IncrementalScanner
    let adapter = ObsidianAdapter::new(vault_path);
    let files = adapter.scan_files().expect("scan files");
    assert_eq!(files.len(), 3);

    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    let dirty_sets = scanner
        .compute_dirty_sets(false)
        .expect("compute dirty sets");
    assert_eq!(dirty_sets.added.len(), 3);
    assert_eq!(dirty_sets.modified.len(), 0);
    assert_eq!(dirty_sets.deleted.len(), 0);

    let stats = scanner.sync(false).expect("scanner sync");
    assert_eq!(stats.added, 3);

    // 4. Vector sync using MockEmbeddingEngine
    let embedder = Arc::new(MockEmbeddingEngine::new(384));
    let vec_stats = VectorSyncEngine::sync(&mut storage, &*embedder).expect("vector sync");
    assert_eq!(vec_stats.embedded_count, 3);

    // 5. Query hybrid search
    let hybrid = HybridSearchEngine::new(&storage, Some(embedder.clone()));
    let results = hybrid
        .search("Architecture", SearchMode::Hybrid, 10)
        .expect("search");
    assert!(!results.is_empty());
    assert_eq!(results[0].title, "Core Architecture");

    // 6. Test domain models & type alias Note == Document
    let doc: Note = storage
        .get_document(card_a.strip_prefix(vault_path).unwrap())
        .expect("get document")
        .expect("document exists");
    assert_eq!(doc.hierarchy, HierarchyLevel::L3Evergreen);
    assert!(doc.links.iter().any(|l| l.target == "Modularity"));

    // 7. Test parser utilities
    let content = "Check [[TargetCard|Display]] and #cool/tag in text.";
    let (links, tags) = extract_elements(content);
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].target, "TargetCard");
    assert_eq!(links[0].alias.as_deref(), Some("Display"));
    assert_eq!(tags, vec!["#cool/tag".to_string()]);

    let (fm, body) = parse_frontmatter("---\nkey: val\n---\nbody text");
    assert_eq!(fm.get("key").unwrap().as_str(), Some("val"));
    assert_eq!(body, "body text");
}
