use std::fs;
use tempfile::TempDir;

/// Creates an isolated temporary mock Obsidian vault following the SecondBrain structure:
/// - 00_Daily/
/// - 01_AI_Logs/
/// - 10_Projects/
/// - 20_Cards/
///
/// Each directory contains sample Markdown files with YAML frontmatter and WikiLinks.
pub fn mock_obsidian_vault() -> TempDir {
    let temp_dir =
        TempDir::new().expect("Failed to create temporary directory for mock Obsidian vault");
    let root = temp_dir.path();

    // Create folders
    let daily_dir = root.join("00_Daily");
    let logs_dir = root.join("01_AI_Logs");
    let projects_dir = root.join("10_Projects");
    let cards_dir = root.join("20_Cards");

    fs::create_dir_all(&daily_dir).expect("Failed to create 00_Daily directory");
    fs::create_dir_all(&logs_dir).expect("Failed to create 01_AI_Logs directory");
    fs::create_dir_all(&projects_dir).expect("Failed to create 10_Projects directory");
    fs::create_dir_all(&cards_dir).expect("Failed to create 20_Cards directory");

    // 00_Daily note (L0 Ephemeral)
    let daily_note = r#"---
date: 2026-09-29
tags:
  - daily
  - scratchpad
---
# 2026-09-29 Daily Log

Quick scratch notes for today. Checked [[Architecture_Decisions]].
Working on sprint tasks.
"#;
    fs::write(daily_dir.join("2026-09-29.md"), daily_note).expect("Failed to write daily note");

    // 01_AI_Logs note (L2 Log)
    let log_note = r#"---
title: Evaluation Run 01
date: 2026-09-29
tags:
  - ai
  - eval
---
# Evaluation Run 01

Benchmarked FTS5 performance against [[Architecture_Decisions|Architecture Reference]].
Results: latency < 2ms, zero daemon overhead.
"#;
    fs::write(logs_dir.join("2026-09-29-eval.md"), log_note).expect("Failed to write AI log note");

    // 10_Projects note (L1/Project Resource)
    let project_note = r#"---
title: K0maru Agent Memory
status: active
tags:
  - project
  - rust
---
# K0maru Agent Memory

Cleanroom memory hub mounting Obsidian and Karpathy Wiki.
See [[Architecture_Decisions]] for architectural invariants.
"#;
    fs::write(projects_dir.join("k0maru-memory.md"), project_note)
        .expect("Failed to write project note");

    // 20_Cards note (L3 Evergreen)
    let card_note = r#"---
title: Architecture Decisions
status: evergreen
tags:
  - architecture
  - core
---
# Architecture Decisions

Core invariants:
1. Truth in files, performance in cache.
2. Zero proprietary formats.

Active project reference: [[k0maru-memory|K0maru Memory Project]].
"#;
    fs::write(cards_dir.join("Architecture_Decisions.md"), card_note)
        .expect("Failed to write card note");

    temp_dir
}

/// Creates an isolated temporary mock Karpathy-style flat LLM-Wiki:
/// Contains flat markdown files with relative WikiLinks and frontmatter.
pub fn mock_karpathy_wiki() -> TempDir {
    let temp_dir =
        TempDir::new().expect("Failed to create temporary directory for mock Karpathy wiki");
    let root = temp_dir.path();

    let index_note = r#"---
title: LLM Wiki Index
tags:
  - index
  - wiki
---
# LLM Wiki Index

Welcome to the LLM-Wiki.
- [[rust-memory]]
- [[sqlite-fts5|SQLite FTS5 Search]]
"#;
    fs::write(root.join("index.md"), index_note).expect("Failed to write index note");

    let rust_note = r#"---
title: Rust Memory Architecture
tags:
  - rust
  - architecture
---
# Rust Memory Architecture

Overview of memory systems in Rust.
References [[index]] and [[sqlite-fts5]].
"#;
    fs::write(root.join("rust-memory.md"), rust_note).expect("Failed to write rust memory note");

    let sqlite_note = r#"---
title: SQLite FTS5 Notes
tags:
  - sqlite
  - search
---
# SQLite FTS5 Notes

Fast full-text search notes. Backlink to [[index]].
"#;
    fs::write(root.join("sqlite-fts5.md"), sqlite_note).expect("Failed to write sqlite note");

    temp_dir
}
