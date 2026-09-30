use std::fs;
use tempfile::TempDir;

use common::fixtures::{mock_karpathy_wiki, mock_obsidian_vault};
use k0maru::adapters::{GenericWikiAdapter, ObsidianAdapter};
use k0maru::loadout::{estimate_tokens, LoadoutBuilder};
use k0maru::scanner::IncrementalScanner;
use k0maru::storage::SqliteStorage;

mod common;

/// Creates a dedicated test vault with full rich data:
/// - 10_Projects/k0maru-memory.md: links 6 L3 cards, 4 L2 logs, has callout vision
/// - 10_Projects/trading-bot.md: has blockquote vision, links 1 card
/// - 20_Cards/ with cards having `## 📌 核心概念` or quote
/// - 01_AI_Logs/ with 4 logs
fn create_rich_mock_vault() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temporary directory for rich vault");
    let root = temp_dir.path();

    let projects_dir = root.join("10_Projects");
    let cards_dir = root.join("20_Cards");
    let logs_dir = root.join("01_AI_Logs");

    fs::create_dir_all(&projects_dir).unwrap();
    fs::create_dir_all(&cards_dir).unwrap();
    fs::create_dir_all(&logs_dir).unwrap();

    // Create 6 L3 cards
    for i in 1..=6 {
        let card_content = format!(
            r#"---
title: Rule Card {i}
hierarchy: L3Evergreen
tags:
  - rule
---
# Rule Card {i}

## 📌 核心概念
> Core concept for invariant rule card {i}: always verify before commit.

## Detailed Description
Additional context and background that shouldn't bloat the loadout.
"#
        );
        fs::write(cards_dir.join(format!("rule-card-{i}.md")), card_content).unwrap();
    }

    // Create 1 card without `## 📌 核心概念` but with initial blockquote
    let special_card = r#"---
title: Special Invariant
hierarchy: L3Evergreen
---
# Special Invariant

> Single source of truth in files, disposable cache in SQLite.

Body text here.
"#;
    fs::write(cards_dir.join("special-invariant.md"), special_card).unwrap();

    // Create 4 L2 logs
    for i in 1..=4 {
        let log_content = format!(
            r#"---
title: AI Log Run 0{i}
hierarchy: L2Log
---
# AI Log Run 0{i}
Executed task {i} with 0 errors.
"#
        );
        fs::write(
            logs_dir.join(format!("2026-09-30-run-0{i}.md")),
            log_content,
        )
        .unwrap();
    }

    // Create Project 1: k0maru-memory
    let project_1 = r#"---
title: K0maru Agent Memory
status: active
tags:
  - project
  - memory
---
# K0maru Agent Memory

> [!NOTE]
> High-performance zero-daemon agent memory hub mounting Obsidian and Karpathy Wiki.

## Linked Rules
- [[20_Cards/rule-card-1]]
- [[20_Cards/rule-card-2]]
- [[20_Cards/rule-card-3]]
- [[20_Cards/rule-card-4]]
- [[20_Cards/rule-card-5]]
- [[20_Cards/rule-card-6]]
- [[20_Cards/special-invariant]]

## Execution Traces
- [[01_AI_Logs/2026-09-30-run-01]]
- [[01_AI_Logs/2026-09-30-run-02]]
- [[01_AI_Logs/2026-09-30-run-03]]
- [[01_AI_Logs/2026-09-30-run-04]]
"#;
    fs::write(projects_dir.join("k0maru-memory.md"), project_1).unwrap();

    // Create Project 2: trading-bot
    let project_2 = r#"---
title: Quantitative Trading Engine
status: planning
---
# Quantitative Trading Engine

> Low latency market-making bot targeting OKX and Binance.

## Rules
- [[20_Cards/rule-card-1]]
"#;
    fs::write(projects_dir.join("trading-bot.md"), project_2).unwrap();

    temp_dir
}

#[test]
fn test_loadout_builder_exact_and_fuzzy_match() {
    let fixture = create_rich_mock_vault();
    let adapter = ObsidianAdapter::new(fixture.path());
    let mut storage = SqliteStorage::in_memory().unwrap();
    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    scanner.sync(false).unwrap();

    let builder = LoadoutBuilder::new(&adapter, &storage);

    // Exact match
    let res = builder.build("k0maru-memory").unwrap();
    assert!(res.is_some(), "Exact match should find project");
    let loadout = res.unwrap();
    assert_eq!(loadout.project_name, "k0maru-memory");
    assert!(
        loadout.scope.contains("zero-daemon agent memory hub"),
        "Scope was: {}",
        loadout.scope
    );

    // Substring / fuzzy match
    let res_sub = builder.build("trading").unwrap();
    assert!(
        res_sub.is_some(),
        "Substring 'trading' should match trading-bot"
    );
    assert_eq!(res_sub.unwrap().project_name, "trading-bot");

    // Case-insensitive match
    let res_ci = builder.build("K0MARU").unwrap();
    assert!(res_ci.is_some(), "Case-insensitive match should work");
    assert_eq!(res_ci.unwrap().project_name, "k0maru-memory");
}

#[test]
fn test_loadout_builder_card_concept_and_l3_limit() {
    let fixture = create_rich_mock_vault();
    let adapter = ObsidianAdapter::new(fixture.path());
    let mut storage = SqliteStorage::in_memory().unwrap();
    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    scanner.sync(false).unwrap();

    let builder = LoadoutBuilder::new(&adapter, &storage);
    let loadout = builder
        .build("k0maru-memory")
        .unwrap()
        .expect("should find k0maru-memory");

    // Project links 7 cards, must be capped at 5
    assert_eq!(
        loadout.l3_cards.len(),
        5,
        "L3 cards must be capped at 5 maximum"
    );

    // Concept extraction from `## 📌 核心概念`
    let card1 = &loadout.l3_cards[0];
    assert_eq!(card1.name, "rule-card-1");
    assert_eq!(
        card1.concept.as_deref(),
        Some("Core concept for invariant rule card 1: always verify before commit.")
    );

    // Output markdown contains the card references and concepts
    assert!(loadout.markdown.contains("[[20_Cards/rule-card-1]]"));
    assert!(loadout.markdown.contains("always verify before commit"));
}

#[test]
fn test_loadout_builder_l2_logs_limit_3() {
    let fixture = create_rich_mock_vault();
    let adapter = ObsidianAdapter::new(fixture.path());
    let mut storage = SqliteStorage::in_memory().unwrap();
    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    scanner.sync(false).unwrap();

    let builder = LoadoutBuilder::new(&adapter, &storage);
    let loadout = builder
        .build("k0maru-memory")
        .unwrap()
        .expect("should find k0maru-memory");

    // Project links 4 logs, must be capped at 3
    assert_eq!(
        loadout.l2_logs.len(),
        3,
        "L2 logs must be capped at 3 maximum"
    );
    assert_eq!(loadout.l2_logs[0].name, "2026-09-30-run-01");
    assert!(loadout
        .markdown
        .contains("[[01_AI_Logs/2026-09-30-run-01]]"));
}

#[test]
fn test_loadout_builder_token_length_budget() {
    let fixture = create_rich_mock_vault();
    let adapter = ObsidianAdapter::new(fixture.path());
    let mut storage = SqliteStorage::in_memory().unwrap();
    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    scanner.sync(false).unwrap();

    let builder = LoadoutBuilder::new(&adapter, &storage);
    let loadout = builder
        .build("k0maru-memory")
        .unwrap()
        .expect("should find project");

    // Must be strictly < 300 tokens
    let tokens = estimate_tokens(&loadout.markdown);
    assert!(
        tokens < 300,
        "Markdown exceeds token budget! Estimated tokens: {}, text len: {}",
        tokens,
        loadout.markdown.len()
    );
    assert!(
        loadout.markdown.len() < 1500,
        "Markdown character length too large: {}",
        loadout.markdown.len()
    );
    assert!(
        loadout.estimated_tokens < 300,
        "Reported estimated_tokens exceeds 300: {}",
        loadout.estimated_tokens
    );
}

#[test]
fn test_loadout_builder_list_projects() {
    let fixture = create_rich_mock_vault();
    let adapter = ObsidianAdapter::new(fixture.path());
    let mut storage = SqliteStorage::in_memory().unwrap();
    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    scanner.sync(false).unwrap();

    let builder = LoadoutBuilder::new(&adapter, &storage);
    let projects = builder.list_projects().unwrap();

    assert_eq!(projects.len(), 2);
    let names: Vec<&str> = projects.iter().map(|p| p.name.as_str()).collect();
    assert!(names.contains(&"k0maru-memory"));
    assert!(names.contains(&"trading-bot"));
}

#[test]
fn test_loadout_builder_nonexistent_returns_none() {
    let fixture = create_rich_mock_vault();
    let adapter = ObsidianAdapter::new(fixture.path());
    let storage = SqliteStorage::in_memory().unwrap();

    let builder = LoadoutBuilder::new(&adapter, &storage);
    let res = builder.build("non-existent-xyz").unwrap();
    assert!(res.is_none());
}

#[test]
fn test_loadout_cli_markdown_and_json() {
    use assert_cmd::Command;
    use predicates::prelude::*;

    let fixture = create_rich_mock_vault();
    let vault_path = fixture.path().to_str().unwrap();

    // 1. Markdown stdout
    let mut cmd = Command::cargo_bin("k0maru").unwrap();
    cmd.args(["loadout", "k0maru", "--vault", vault_path])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "# 🎒 Agent Task Loadout: k0maru-memory",
        ))
        .stdout(predicate::str::contains("Project Scope"))
        .stdout(predicate::str::contains("[[20_Cards/rule-card-1]]"))
        .stdout(predicate::str::contains("[[01_AI_Logs/2026-09-30-run-01]]"));

    // 2. JSON stdout
    let mut cmd_json = Command::cargo_bin("k0maru").unwrap();
    let output = cmd_json
        .args(["loadout", "k0maru", "--vault", vault_path, "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_val: serde_json::Value =
        serde_json::from_slice(&output).expect("Output should be valid JSON");
    assert_eq!(json_val["project_name"], "k0maru-memory");
    assert_eq!(json_val["l3_cards"].as_array().unwrap().len(), 5);
    assert_eq!(json_val["l2_logs"].as_array().unwrap().len(), 3);
    assert!(json_val["estimated_tokens"].as_u64().unwrap() < 300);

    // 3. List command
    let mut cmd_list = Command::cargo_bin("k0maru").unwrap();
    cmd_list
        .args(["loadout", "--vault", vault_path, "--list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("k0maru-memory"))
        .stdout(predicate::str::contains("trading-bot"));

    // 4. List command with --json
    let mut cmd_list_json = Command::cargo_bin("k0maru").unwrap();
    let list_output = cmd_list_json
        .args(["loadout", "--vault", vault_path, "--list", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let list_json: serde_json::Value =
        serde_json::from_slice(&list_output).expect("List output should be valid JSON");
    assert!(list_json.as_array().unwrap().len() >= 2);

    // 5. Nonexistent query exits with error
    let mut cmd_missing = Command::cargo_bin("k0maru").unwrap();
    cmd_missing
        .args(["loadout", "nonexistent", "--vault", vault_path])
        .assert()
        .failure();

    // 6. Copy flag runs cleanly without crashing
    let mut cmd_copy = Command::cargo_bin("k0maru").unwrap();
    cmd_copy
        .args(["loadout", "k0maru", "--vault", vault_path, "--copy"])
        .assert()
        .success();
}

#[test]
fn test_loadout_builder_with_mock_obsidian_vault() {
    let fixture = mock_obsidian_vault();
    let adapter = ObsidianAdapter::new(fixture.path());
    let mut storage = SqliteStorage::in_memory().unwrap();
    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    scanner.sync(false).unwrap();

    let builder = LoadoutBuilder::new(&adapter, &storage);
    let res = builder.build("k0maru-memory").unwrap();
    assert!(res.is_some());
    let loadout = res.unwrap();

    assert_eq!(loadout.project_name, "k0maru-memory");
    assert!(loadout.scope.contains("Cleanroom memory hub"));
    assert_eq!(loadout.l3_cards.len(), 1);
    assert_eq!(loadout.l3_cards[0].name, "Architecture_Decisions");
    assert!(
        loadout.l3_cards[0]
            .concept
            .as_deref()
            .unwrap_or("")
            .contains("Truth in files"),
        "Card concept was: {:?}",
        loadout.l3_cards[0].concept
    );
    assert!(loadout.estimated_tokens < 300);
}

#[test]
fn test_loadout_builder_with_mock_karpathy_wiki() {
    let fixture = mock_karpathy_wiki();
    let adapter = GenericWikiAdapter::new(fixture.path());
    let mut storage = SqliteStorage::in_memory().unwrap();
    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    scanner.sync(false).unwrap();

    let builder = LoadoutBuilder::new(&adapter, &storage);
    let projects = builder.list_projects().unwrap();
    // In mock_karpathy_wiki, rust-memory.md has tags: [rust, architecture] and no 10_Projects dir.
    // find_project can match rust-memory
    let match_res = builder.find_project("rust-memory").unwrap();
    assert!(match_res.is_some());
    let _ = projects;
}
