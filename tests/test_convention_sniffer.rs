use std::fs;
use std::path::PathBuf;

use chrono::NaiveDate;
use k0maru::convention::{
    generate_filename, slugify, synthesize_markdown, ConventionSniffer, FlushRequest, NamingStyle,
    NoteCategory, VaultConvention,
};
use tempfile::tempdir;

#[test]
fn test_slugify_and_filename_generation() {
    // Basic slugification
    assert_eq!(
        slugify("Migrate Cache from SQLite to SQLite-Vec"),
        "migrate-cache-from-sqlite-to-sqlite-vec"
    );
    assert_eq!(
        slugify("  Special   Characters & Symbols! #123  "),
        "special-characters-symbols-123"
    );
    assert_eq!(slugify("---multiple---dashes---"), "multiple-dashes");
    assert_eq!(slugify(""), "untitled");
    assert_eq!(slugify("   "), "untitled");

    // SlugOnly
    let slug_file = generate_filename("Hello World", &NamingStyle::SlugOnly, None);
    assert_eq!(slug_file, "hello-world.md");

    // DateSlug with explicit NaiveDate
    let date = NaiveDate::from_ymd_opt(2026, 10, 4).unwrap();
    let date_file = generate_filename("New Hybrid Engine", &NamingStyle::DateSlug, Some(date));
    assert_eq!(date_file, "2026-10-04-new-hybrid-engine.md");

    // DateSlug default to today (matches YYYY-MM-DD-*.md)
    let today_file = generate_filename("Today Task", &NamingStyle::DateSlug, None);
    assert!(
        today_file.ends_with("-today-task.md"),
        "Filename '{}' should end with -today-task.md",
        today_file
    );
    assert_eq!(today_file.len(), 10 + 1 + "today-task.md".len());

    // TimestampSlug
    let ts_file = generate_filename("Timestamped Note", &NamingStyle::TimestampSlug, Some(date));
    assert_eq!(ts_file, "202610040000-timestamped-note.md");

    // TitleCase
    let title_file = generate_filename(
        "Architecture / ADR: Deep Dive",
        &NamingStyle::TitleCase,
        None,
    );
    assert_eq!(title_file, "Architecture  ADR Deep Dive.md");
}

#[test]
fn test_flat_vault_karpathy_wiki_convention() {
    let temp = tempdir().unwrap();
    let vault_root = temp.path();

    // Create a flat Karpathy LLM-Wiki structure (no folders, slug-only files)
    fs::write(
        vault_root.join("index.md"),
        "# Index\n\nWelcome to the knowledge base.\n",
    )
    .unwrap();
    fs::write(
        vault_root.join("neural-networks.md"),
        "# Neural Networks\n\nFoundational models.\n",
    )
    .unwrap();
    fs::write(
        vault_root.join("vector-search.md"),
        "# Vector Search\n\nApproximate nearest neighbors.\n",
    )
    .unwrap();

    let convention = ConventionSniffer::sniff(vault_root);

    assert_eq!(convention.vault_root, vault_root.canonicalize().unwrap());
    assert!(!convention.has_explicit_rules);
    assert!(convention.rule_source.is_none());
    assert_eq!(convention.naming_style, NamingStyle::SlugOnly);

    // Categories in a flat vault route to vault root ("")
    assert_eq!(convention.target_subdir_for("decision"), PathBuf::from(""));
    assert_eq!(convention.target_subdir_for("log"), PathBuf::from(""));
    assert_eq!(convention.target_subdir_for("concept"), PathBuf::from(""));

    // Verify filename for new note in flat vault
    let filename = generate_filename("Flash Attention", &convention.naming_style, None);
    assert_eq!(filename, "flash-attention.md");
}

#[test]
fn test_structured_vault_topology_routing() {
    let temp = tempdir().unwrap();
    let vault_root = temp.path();

    // Create standard structured folders
    let decisions_dir = vault_root.join("decisions");
    let logs_dir = vault_root.join("logs");
    let concepts_dir = vault_root.join("concepts");
    fs::create_dir_all(&decisions_dir).unwrap();
    fs::create_dir_all(&logs_dir).unwrap();
    fs::create_dir_all(&concepts_dir).unwrap();

    // Add existing notes with DateSlug pattern and YAML frontmatter
    fs::write(
        decisions_dir.join("2026-09-01-sqlite-storage.md"),
        "---\ntitle: \"SQLite Storage\"\ndate: 2026-09-01\n---\n\n# SQLite Storage\n",
    )
    .unwrap();
    fs::write(
        logs_dir.join("2026-09-02-session-alpha.md"),
        "---\ntitle: \"Session Alpha\"\ndate: 2026-09-02\n---\n\n# Session Alpha\n",
    )
    .unwrap();

    let convention = ConventionSniffer::sniff(vault_root);

    assert!(!convention.has_explicit_rules);
    assert_eq!(
        convention.target_subdir_for("decision"),
        PathBuf::from("decisions")
    );
    assert_eq!(convention.target_subdir_for("log"), PathBuf::from("logs"));
    assert_eq!(
        convention.target_subdir_for("concept"),
        PathBuf::from("concepts")
    );
    assert_eq!(convention.naming_style, NamingStyle::DateSlug);
    assert!(convention.has_frontmatter);

    // Unknown category should fall back to vault root
    assert_eq!(
        convention.target_subdir_for("unknown_random"),
        PathBuf::from("")
    );
}

#[test]
fn test_explicit_rules_agents_md() {
    let temp = tempdir().unwrap();
    let vault_root = temp.path();

    // Write AGENTS.md specifying custom subdirectories
    let agents_md = r#"
# Memory Vault Organization Rules

Please adhere to the following directory layout:
- decisions: docs/adr
- logs: my-journal
- concepts: wiki/cards
"#;
    fs::write(vault_root.join("AGENTS.md"), agents_md).unwrap();

    let convention = ConventionSniffer::sniff(vault_root);

    assert!(convention.has_explicit_rules);
    assert_eq!(
        convention.rule_source,
        Some(vault_root.canonicalize().unwrap().join("AGENTS.md"))
    );
    assert_eq!(
        convention.target_subdir_for("decision"),
        PathBuf::from("docs/adr")
    );
    assert_eq!(
        convention.target_subdir_for("log"),
        PathBuf::from("my-journal")
    );
    assert_eq!(
        convention.target_subdir_for("concept"),
        PathBuf::from("wiki/cards")
    );
}

#[test]
fn test_template_detection_and_substitution() {
    let temp = tempdir().unwrap();
    let vault_root = temp.path();

    let tmpl_dir = vault_root.join("templates");
    fs::create_dir_all(&tmpl_dir).unwrap();

    let template_content = r#"---
title: "{{title}}"
date: {{date}}
category: {{category}}
tags: [{{tags}}]
---

# {{title}}

> **Summary**: {{summary}}

## Context
{{content}}

## References
{{related}}
"#;
    fs::write(tmpl_dir.join("decision.md"), template_content).unwrap();

    let convention = ConventionSniffer::sniff(vault_root);

    assert!(convention.template.is_some());
    let tmpl = convention.template.as_ref().unwrap();
    assert!(tmpl.contains("{{title}}"));

    let req = FlushRequest {
        title: "Migrate Cache to SQLite-Vec".to_string(),
        summary: Some("Benchmark evaluation and schema choices".to_string()),
        content: "We replaced sqlite-vec virtual table with unified storage.".to_string(),
        category: "decision".to_string(),
        tags: vec!["rust".to_string(), "vectors".to_string()],
        related_notes: vec![
            "Hybrid Search Architecture".to_string(),
            "Vector Sync Engine".to_string(),
        ],
        dry_run: false,
    };

    let synthesized = synthesize_markdown(&req, &convention);

    assert!(synthesized.contains("title: \"Migrate Cache to SQLite-Vec\""));
    assert!(synthesized.contains("# Migrate Cache to SQLite-Vec"));
    assert!(synthesized.contains("> **Summary**: Benchmark evaluation and schema choices"));
    assert!(synthesized.contains("We replaced sqlite-vec virtual table with unified storage."));
    assert!(synthesized.contains("rust, vectors"));
    assert!(synthesized.contains("[[Hybrid Search Architecture]], [[Vector Sync Engine]]"));
}

#[test]
fn test_standard_synthesizer_without_template() {
    let convention = VaultConvention {
        vault_root: PathBuf::from("/tmp/mock-vault"),
        has_explicit_rules: false,
        rule_source: None,
        target_subdirs: std::collections::HashMap::new(),
        naming_style: NamingStyle::DateSlug,
        has_frontmatter: true,
        template: None,
    };

    let req = FlushRequest {
        title: "Fix Token Count Estimator".to_string(),
        summary: Some("Updated token estimation ratio to 3.5 chars/token".to_string()),
        content: "Detailed findings from benchmarking against tiktoken.".to_string(),
        category: "log".to_string(),
        tags: vec!["performance".to_string(), "#tokens".to_string()],
        related_notes: vec!["Token Estimator Spec".to_string()],
        dry_run: true,
    };

    let md = synthesize_markdown(&req, &convention);

    // Verify Frontmatter
    assert!(md.starts_with("---\n"));
    assert!(md.contains("title: \"Fix Token Count Estimator\"\n"));
    assert!(md.contains("category: log\n"));
    assert!(md.contains("  - performance\n"));
    assert!(md.contains("  - tokens\n"));

    // Verify Header & Summary
    assert!(md.contains("# Fix Token Count Estimator\n"));
    assert!(md.contains("> **Summary**: Updated token estimation ratio to 3.5 chars/token\n"));

    // Verify Section
    assert!(md.contains("## Context & Decisions\n\nDetailed findings from benchmarking"));

    // Verify WikiLinks
    assert!(md.contains("## Related Notes\n- [[Token Estimator Spec]]\n"));
}

#[test]
fn test_path_traversal_defense() {
    let temp = tempdir().unwrap();
    let vault_root = temp.path();

    // AGENTS.md with malicious directory escaping vault root
    let evil_agents_md = r#"
decisions: ../../etc/passwd
logs: /root/private
concepts: safe-concept
"#;
    fs::write(vault_root.join("AGENTS.md"), evil_agents_md).unwrap();

    let convention = ConventionSniffer::sniff(vault_root);

    // Malicious directives must be rejected and fallback to root ("")
    assert_eq!(convention.target_subdir_for("decision"), PathBuf::from(""));
    assert_eq!(convention.target_subdir_for("log"), PathBuf::from(""));
    // Safe directive should be accepted
    assert_eq!(
        convention.target_subdir_for("concept"),
        PathBuf::from("safe-concept")
    );
}

#[test]
fn test_note_category_enum_and_loose_matching() {
    assert_eq!(NoteCategory::Decision.as_str(), "decision");
    assert_eq!(NoteCategory::Log.as_str(), "log");
    assert_eq!(NoteCategory::Concept.as_str(), "concept");
    assert_eq!(NoteCategory::Generic.as_str(), "generic");

    assert_eq!(NoteCategory::from_str_loose("ADR"), NoteCategory::Decision);
    assert_eq!(NoteCategory::from_str_loose("Journal"), NoteCategory::Log);
    assert_eq!(NoteCategory::from_str_loose("cards"), NoteCategory::Concept);
    assert_eq!(
        NoteCategory::from_str_loose("random_tag"),
        NoteCategory::Generic
    );
}
