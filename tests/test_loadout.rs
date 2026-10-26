mod common;

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

use assert_cmd::Command;
use k0maru::adapters::ObsidianAdapter;
use k0maru::git::{FileChangeStatus, ModifiedFile};
use k0maru::loadout::{estimate_tokens, LoadoutBuilder};
use k0maru::scanner::IncrementalScanner;
use k0maru::storage::SqliteStorage;

fn create_rich_mock_vault() -> TempDir {
    let temp_dir = TempDir::new().expect("tempdir");
    let root = temp_dir.path();

    let projects_dir = root.join("10_Projects");
    let cards_dir = root.join("20_Cards");
    let logs_dir = root.join("01_AI_Logs");

    fs::create_dir_all(&projects_dir).unwrap();
    fs::create_dir_all(&cards_dir).unwrap();
    fs::create_dir_all(&logs_dir).unwrap();

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
"#
        );
        fs::write(cards_dir.join(format!("rule-card-{i}.md")), card_content).unwrap();
    }

    let project_1 = r#"---
title: K0maru Agent Memory
status: active
tags:
  - project
  - memory
---
# K0maru Agent Memory

> Cleanroom zero-daemon agent memory hub mounting Obsidian and Karpathy Wiki.

## Linked Rules
- [[20_Cards/rule-card-1]]
- [[20_Cards/rule-card-2]]
- [[20_Cards/rule-card-3]]
- [[20_Cards/rule-card-4]]
- [[20_Cards/rule-card-5]]
- [[20_Cards/rule-card-6]]
"#;
    fs::write(projects_dir.join("k0maru-memory.md"), project_1).unwrap();

    let project_2 = r#"---
title: Quantitative Trading Engine
status: planning
---
# Quantitative Trading Engine

> Low latency market-making bot targeting OKX and Binance.
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
    assert!(estimate_tokens(&loadout.markdown) < 300);

    // Substring / fuzzy match
    let res_sub = builder.build("trading").unwrap();
    assert!(res_sub.is_some());
    assert_eq!(res_sub.unwrap().project_name, "trading-bot");

    // Case-insensitive match
    let res_ci = builder.build("K0MARU").unwrap();
    assert!(res_ci.is_some());
    assert_eq!(res_ci.unwrap().project_name, "k0maru-memory");
}

#[test]
fn test_project_resolver_prevents_log_clutter_regression() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    let projects_dir = root.join("10_Projects");
    let logs_dir = root.join("01_AI_Logs");
    fs::create_dir_all(&projects_dir).unwrap();
    fs::create_dir_all(&logs_dir).unwrap();

    let project_content = r#"---
title: OKX资金费率套利与跨生态对冲系统
status: active
slug: okx-funding-arbitrage
aliases:
  - funding-arbitrage
tags:
  - topic/funding-arbitrage
---
# OKX资金费率套利与跨生态对冲系统

> 跨交易所与去中心化资金费率套利与自动调仓引擎。
"#;
    fs::write(
        projects_dir.join("OKX资金费率套利与跨生态对冲系统.md"),
        project_content,
    )
    .unwrap();

    // 20 logs with identical names
    for i in 1..=20 {
        let log_content = format!(
            r#"---
title: 2026-10-{:02}-okx-funding-arbitrage-phase-5-live-tui-monitor
hierarchy: L2Log
---
# 2026-10-{:02} Noise Log
"#,
            i, i
        );
        fs::write(
            logs_dir.join(format!(
                "2026-10-{:02}-okx-funding-arbitrage-phase-5-live-tui-monitor.md",
                i
            )),
            log_content,
        )
        .unwrap();
    }

    let adapter = ObsidianAdapter::new(root);
    let mut storage = SqliteStorage::in_memory().unwrap();
    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    scanner.sync(false).unwrap();

    let builder = LoadoutBuilder::new(&adapter, &storage);

    // Resolving by slug should resolve to project, not a truncated log!
    let found = builder.find_project("okx-funding-arbitrage").unwrap();
    assert_eq!(
        found,
        Some(PathBuf::from(
            "10_Projects/OKX资金费率套利与跨生态对冲系统.md"
        )),
        "Expected okx-funding-arbitrage to resolve to Chinese project file"
    );

    let loadout = builder.build("okx-funding-arbitrage").unwrap().unwrap();
    assert_eq!(loadout.project_name, "OKX资金费率套利与跨生态对冲系统");
}

#[test]
fn test_change_aware_loadout_with_git_diff() {
    let fixture = create_rich_mock_vault();
    let adapter = ObsidianAdapter::new(fixture.path());
    let mut storage = SqliteStorage::in_memory().unwrap();
    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    scanner.sync(false).unwrap();

    let builder = LoadoutBuilder::new(&adapter, &storage);

    let modified = vec![ModifiedFile::new(
        "src/storage/sqlite.rs",
        FileChangeStatus::Modified,
    )];

    let loadout = builder
        .build_with_diff("k0maru-memory", &modified)
        .unwrap()
        .expect("Project loadout should be found");

    assert_eq!(loadout.project_name, "k0maru-memory");
    assert!(loadout.markdown.contains("[Diff-Aware]"));
    assert!(loadout.markdown.contains("Active Changes"));
    assert!(estimate_tokens(&loadout.markdown) < 300);
}

#[test]
fn test_loadout_strict_token_budget_under_long_paths_and_massive_diff() {
    let fixture = create_rich_mock_vault();
    let adapter = ObsidianAdapter::new(fixture.path());
    let mut storage = SqliteStorage::in_memory().unwrap();
    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    scanner.sync(false).unwrap();

    let builder = LoadoutBuilder::new(&adapter, &storage);

    // Adversarial: 50 modified files with 200+ character path names
    let mut massive_files = Vec::new();
    for i in 0..50 {
        let long_path = format!(
            "crates/enterprise/deeply/nested/subsystem/component_number_{i}/very_long_module_name_for_stress_testing_loadout_token_limits_strictly_under_budget.rs"
        );
        massive_files.push(ModifiedFile::new(&long_path, FileChangeStatus::Modified));
    }

    let loadout = builder
        .build_with_diff("k0maru-memory", &massive_files)
        .unwrap()
        .unwrap();

    let token_count = estimate_tokens(&loadout.markdown);
    assert!(
        token_count < 300,
        "Token budget MUST be <300, got: {token_count}"
    );
}

#[test]
fn test_loadout_cli_markdown_and_json() {
    let fixture = create_rich_mock_vault();
    let root = fixture.path().to_str().unwrap();

    let mut cmd_sync = Command::cargo_bin("k0maru").unwrap();
    cmd_sync.arg("sync").arg("--vault").arg(root);
    cmd_sync.assert().success();

    let mut cmd_md = Command::cargo_bin("k0maru").unwrap();
    cmd_md
        .arg("loadout")
        .arg("k0maru-memory")
        .arg("--vault")
        .arg(root);
    let assert_md = cmd_md.assert().success();
    let stdout_md = String::from_utf8(assert_md.get_output().stdout.clone()).unwrap();
    assert!(stdout_md.contains("k0maru-memory"));

    let mut cmd_json = Command::cargo_bin("k0maru").unwrap();
    cmd_json
        .arg("loadout")
        .arg("k0maru-memory")
        .arg("--vault")
        .arg(root)
        .arg("--json");
    let assert_json = cmd_json.assert().success();
    let stdout_json = String::from_utf8(assert_json.get_output().stdout.clone()).unwrap();
    let json_val: serde_json::Value = serde_json::from_str(&stdout_json).unwrap();
    assert_eq!(json_val["project_name"], "k0maru-memory");
    assert!(json_val["estimated_tokens"].as_u64().unwrap() < 300);
}

#[test]
fn test_loadout_heterogeneous_wiki_folders() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    let projects_dir = root.join("projects");
    let cards_dir = root.join("cards");
    let logs_dir = root.join("logs");
    fs::create_dir_all(&projects_dir).unwrap();
    fs::create_dir_all(&cards_dir).unwrap();
    fs::create_dir_all(&logs_dir).unwrap();

    fs::write(
        projects_dir.join("wiki-memory.md"),
        r#"---
title: Heterogeneous Wiki Memory
status: active
---
# Heterogeneous Wiki Memory

> Flat LLM-Wiki project mounted from engineering folders.

## Linked Rules
- [[cards/rule-card-1]]
- [[logs/2026-10-01-run]]
"#,
    )
    .unwrap();

    fs::write(
        cards_dir.join("rule-card-1.md"),
        r#"---
title: Rule Card 1
---
# Rule Card 1

## 📌 核心概念
> Always verify heterogeneous topology resolution before commit.
"#,
    )
    .unwrap();

    fs::write(
        logs_dir.join("2026-10-01-run.md"),
        r#"---
title: 2026-10-01 heterogeneous topology run
hierarchy: L2Log
---
# 2026-10-01 Run
"#,
    )
    .unwrap();

    let adapter = ObsidianAdapter::new(root);
    let mut storage = SqliteStorage::in_memory().unwrap();
    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    scanner.sync(false).unwrap();

    let builder = LoadoutBuilder::new(&adapter, &storage);

    assert_eq!(
        builder.find_project("wiki-memory").unwrap(),
        Some(PathBuf::from("projects/wiki-memory.md")),
        "projects/ folder must be discovered as a project root"
    );

    let loadout = builder
        .build("wiki-memory")
        .unwrap()
        .expect("heterogeneous wiki loadout should build");

    assert_eq!(loadout.project_name, "wiki-memory");
    assert!(
        loadout
            .l3_cards
            .iter()
            .any(|card| card.path == "cards/rule-card-1.md"),
        "cards/ note must resolve as an L3 card, got: {:?}",
        loadout.l3_cards
    );
    assert!(
        loadout
            .l2_logs
            .iter()
            .any(|log| log.path == "logs/2026-10-01-run.md"),
        "logs/ note must resolve as an L2 log, got: {:?}",
        loadout.l2_logs
    );
    assert!(estimate_tokens(&loadout.markdown) < 300);
}

#[test]
fn test_loadout_surfaces_guard_rules_from_active_adr_card() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    let projects_dir = root.join("10_Projects");
    let cards_dir = root.join("20_Cards");
    fs::create_dir_all(&projects_dir).unwrap();
    fs::create_dir_all(&cards_dir).unwrap();

    fs::write(
        projects_dir.join("redis-cache.md"),
        r#"---
title: Redis Cache
status: active
---
# Redis Cache

> Scoped Redis cache invalidation engine.

## Architecture Rules
- [[20_Cards/adr-001-scoped-redis-setnx]]
"#,
    )
    .unwrap();

    fs::write(
        cards_dir.join("adr-001-scoped-redis-setnx.md"),
        r#"---
title: ADR-001 Scoped Redis SETNX
status: active
invariants:
  - id: no-unscoped-setnx
    title: No Unscoped SETNX
    forbidden: "redis_setnx_unscoped"
    file_pattern: "src/**/*.rs"
    severity: "deny"
    remediation: "Use redis_set_nx_scoped with an explicit namespace."
---
# ADR-001 Scoped Redis SETNX

> **Core Concept**: All Redis locks MUST be namespaced.
"#,
    )
    .unwrap();

    let adapter = ObsidianAdapter::new(root);
    let mut storage = SqliteStorage::in_memory().unwrap();
    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    scanner.sync(false).unwrap();

    let builder = LoadoutBuilder::new(&adapter, &storage);

    let loadout = builder
        .build("redis-cache")
        .unwrap()
        .expect("loadout should resolve the redis-cache project");

    assert!(
        loadout
            .l3_cards
            .iter()
            .any(|card| card.path.contains("adr-001-scoped-redis-setnx")),
        "Active ADR card must be linked into the loadout: {:?}",
        loadout.l3_cards
    );
    assert!(
        loadout.markdown.contains("redis_setnx_unscoped"),
        "Guard rule must surface in the compact loadout markdown:\n{}",
        loadout.markdown
    );
    assert!(
        loadout.markdown.contains("[Guard:"),
        "Guard rule must be rendered with its compact tag:\n{}",
        loadout.markdown
    );
    assert!(
        estimate_tokens(&loadout.markdown) < 300,
        "Loadout with guard rule MUST stay under 300 tokens, got {}",
        estimate_tokens(&loadout.markdown)
    );

    let modified = vec![ModifiedFile::new(
        "src/redis/lock.rs",
        FileChangeStatus::Modified,
    )];
    let diff_loadout = builder
        .build_with_diff("redis-cache", &modified)
        .unwrap()
        .expect("diff-aware loadout should resolve the redis-cache project");

    assert!(
        diff_loadout.markdown.contains("redis_setnx_unscoped"),
        "Guard rule must survive diff-aware loadout packing:\n{}",
        diff_loadout.markdown
    );
    assert!(
        diff_loadout.markdown.contains("[Guard:"),
        "Guard rule must be rendered with its compact tag in diff mode:\n{}",
        diff_loadout.markdown
    );
    assert!(
        estimate_tokens(&diff_loadout.markdown) < 300,
        "Diff-aware loadout with guard rule MUST stay under 300 tokens, got {}",
        estimate_tokens(&diff_loadout.markdown)
    );
}

#[test]
fn test_loadout_forwards_superseded_card_to_replacement() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    let projects_dir = root.join("10_Projects");
    let cards_dir = root.join("20_Cards");
    fs::create_dir_all(&projects_dir).unwrap();
    fs::create_dir_all(&cards_dir).unwrap();

    fs::write(
        projects_dir.join("payment-settlement.md"),
        r#"---
title: Payment Settlement
status: active
---
# Payment Settlement

## Architecture Rules
- [[20_Cards/adr-010-legacy-setnx-lock]]
"#,
    )
    .unwrap();

    fs::write(
        cards_dir.join("adr-010-legacy-setnx-lock.md"),
        r#"---
title: ADR-010 Legacy Lock
status: superseded
superseded_by: "[[20_Cards/adr-042-fenced-lease-lock]]"
---
> [SUPERSEDED] Superseded by [[20_Cards/adr-042-fenced-lease-lock]]

# ADR-010 Legacy Lock

> **Core Concept**: Legacy unscoped lock.
"#,
    )
    .unwrap();

    fs::write(
        cards_dir.join("adr-042-fenced-lease-lock.md"),
        r#"---
title: ADR-042 Fenced Lease Lock
status: active
supersedes:
  - "[[20_Cards/adr-010-legacy-setnx-lock]]"
---
# ADR-042 Fenced Lease Lock

> **Core Concept**: All handlers MUST use fenced lease locks.
"#,
    )
    .unwrap();

    let adapter = ObsidianAdapter::new(root);
    let mut storage = SqliteStorage::in_memory().unwrap();
    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    scanner.sync(false).unwrap();

    let builder = LoadoutBuilder::new(&adapter, &storage);
    let loadout = builder
        .build("payment-settlement")
        .unwrap()
        .expect("loadout should resolve");

    assert!(
        !loadout
            .l3_cards
            .iter()
            .any(|card| card.path.contains("adr-010")),
        "Superseded ADR-010 must not appear in loadout: {:?}",
        loadout.l3_cards
    );
    assert!(
        loadout
            .l3_cards
            .iter()
            .any(|card| card.path.contains("adr-042-fenced-lease-lock")),
        "Loadout must automatically forward superseded link to ADR-042: {:?}",
        loadout.l3_cards
    );
}
