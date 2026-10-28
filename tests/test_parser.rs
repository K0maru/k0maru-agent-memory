mod common;

use std::fs;
use std::path::PathBuf;

use chrono::NaiveDate;
use k0maru::adapters::{GenericWikiAdapter, ObsidianAdapter};
use k0maru::convention::{generate_filename, slugify, NamingStyle};
use k0maru::core::models::HierarchyLevel;
use k0maru::core::traits::VaultAdapter;
use k0maru::parser::{extract_elements, extract_tags, extract_wikilinks, parse_frontmatter};
use k0maru::scanner::IncrementalScanner;
use k0maru::storage::SqliteStorage;
use serde_json::json;

#[test]
fn test_frontmatter_yaml_parsing_and_metadata() {
    let content = r#"---
title: "Deep Module Design"
hierarchy: L3Evergreen
tags:
  - architecture
  - clean-code
verified: true
counter: 42
---
# Deep Module Design

This is the document body with some content.
"#;

    let (frontmatter, body) = parse_frontmatter(content);
    assert_eq!(frontmatter["title"], "Deep Module Design");
    assert_eq!(frontmatter["hierarchy"], "L3Evergreen");
    assert_eq!(frontmatter["tags"][0], "architecture");
    assert_eq!(frontmatter["tags"][1], "clean-code");
    assert_eq!(frontmatter["verified"], true);
    assert_eq!(frontmatter["counter"], 42);

    assert!(body.starts_with("# Deep Module Design"));
    assert!(body.contains("This is the document body with some content."));

    // Empty frontmatter test
    let (empty_fm, empty_body) = parse_frontmatter("# Just Title\nNo frontmatter");
    assert_eq!(empty_fm, json!({}));
    assert_eq!(empty_body, "# Just Title\nNo frontmatter");
}

#[test]
fn test_wikilinks_extraction_and_alias_resolution() {
    let body = r#"
# Notes
Linking to [[Architecture_Decisions]] and with alias [[k0maru-memory|K0maru Memory Hub]].
Anchor link: [[Project#Milestones]].
"#;

    let links = extract_wikilinks(body);
    assert_eq!(links.len(), 3);
    assert_eq!(links[0].target, "Architecture_Decisions");
    assert_eq!(links[0].alias, None);

    assert_eq!(links[1].target, "k0maru-memory");
    assert_eq!(links[1].alias, Some("K0maru Memory Hub".to_string()));

    assert_eq!(links[2].target, "Project#Milestones");

    let tags = extract_tags(body);
    assert!(tags.is_empty());

    let (found_links, found_tags) = extract_elements(body);
    assert_eq!(found_links.len(), 3);
    assert_eq!(found_tags.len(), 0);
}

#[test]
fn test_obsidian_adapter_scan_and_hidden_filtering() {
    let temp_vault = common::fixtures::mock_obsidian_vault();
    let root = temp_vault.path();

    // Create ignored directories
    fs::create_dir_all(root.join(".git")).unwrap();
    fs::write(root.join(".git/config"), "dummy").unwrap();
    fs::create_dir_all(root.join(".trash")).unwrap();
    fs::write(root.join(".trash/deleted.md"), "# Deleted").unwrap();
    fs::create_dir_all(root.join(".k0maru")).unwrap();
    fs::write(root.join(".k0maru/cache.sqlite"), "bin").unwrap();

    let adapter = ObsidianAdapter::new(root);
    let files = adapter.scan_files().expect("scan files");

    // Must find exactly the 4 mock markdown files
    assert_eq!(files.len(), 4);
    assert!(files.contains(&PathBuf::from("00_Daily/2026-09-29.md")));
    assert!(files.contains(&PathBuf::from("10_Projects/k0maru-memory.md")));
    assert!(files.contains(&PathBuf::from("20_Cards/Architecture_Decisions.md")));

    for f in &files {
        let s = f.to_string_lossy();
        assert!(!s.contains(".git"));
        assert!(!s.contains(".trash"));
        assert!(!s.contains(".k0maru"));
    }
}

#[test]
fn test_generic_wiki_adapter_conformance() {
    let temp_wiki = common::fixtures::mock_karpathy_wiki();
    let root = temp_wiki.path();

    let adapter = GenericWikiAdapter::new(root);
    let files = adapter.scan_files().expect("scan files");
    assert_eq!(files.len(), 3);

    for rel_path in &files {
        let doc = adapter.read_document(rel_path).expect("read doc");
        assert_eq!(doc.hierarchy, HierarchyLevel::L3Evergreen);
        assert!(!doc.title.is_empty());
    }
}

#[test]
fn test_incremental_scanner_change_detection() {
    let temp_vault = common::fixtures::mock_obsidian_vault();
    let root = temp_vault.path();

    let adapter = ObsidianAdapter::new(root);
    let mut storage = SqliteStorage::in_memory().expect("in-memory db");

    // 1. Initial sync
    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    let stats1 = scanner.sync(false).expect("initial sync");
    assert_eq!(stats1.added, 4);
    assert_eq!(stats1.modified, 0);

    // 2. Second sync unmodified
    let stats2 = scanner.sync(false).expect("second sync");
    assert_eq!(stats2.added, 0);
    assert_eq!(stats2.unchanged, 4);

    // 3. Modify single file
    let card = root.join("20_Cards/Architecture_Decisions.md");
    let current_content = fs::read_to_string(&card).unwrap();
    fs::write(&card, format!("{}\nUpdated line here.", current_content)).unwrap();

    let stats3 = scanner.sync(false).expect("third sync");
    assert_eq!(stats3.modified, 1);
    assert_eq!(stats3.unchanged, 3);
}

#[test]
fn test_convention_sniffer_slugify_and_filename() {
    assert_eq!(
        slugify("Migrate Cache from SQLite to SQLite-Vec"),
        "migrate-cache-from-sqlite-to-sqlite-vec"
    );
    assert_eq!(
        slugify("  Special   Characters & Symbols! #123  "),
        "special-characters-symbols-123"
    );

    let date = NaiveDate::from_ymd_opt(2026, 10, 4).unwrap();
    let date_file = generate_filename("New Hybrid Engine", &NamingStyle::DateSlug, Some(date));
    assert_eq!(date_file, "2026-10-04-new-hybrid-engine.md");

    let slug_file = generate_filename("Hello World", &NamingStyle::SlugOnly, None);
    assert_eq!(slug_file, "hello-world.md");
}

#[test]
fn test_frontmatter_invariants_extraction() {
    let content = r#"---
title: "Living ADR Guard"
hierarchy: L3Evergreen
invariants:
  - id: ADR-001
    pattern: "*.rs"
    forbidden: "unwrap\\(\\)"
    severity: deny
    remediation: "Use ? operator instead"
---
# Living ADR Guard
Content body.
"#;

    let (frontmatter, _) = parse_frontmatter(content);
    assert!(frontmatter.get("invariants").is_some());
    let invs = frontmatter["invariants"].as_array().unwrap();
    assert_eq!(invs.len(), 1);
    assert_eq!(invs[0]["id"], "ADR-001");
    assert_eq!(invs[0]["severity"], "deny");
}
