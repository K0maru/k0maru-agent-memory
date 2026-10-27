use std::fs;
use std::path::Path;
use std::str::FromStr;
use tempfile::tempdir;

use assert_cmd::Command;
use k0maru::adapters::GenericWikiAdapter;
use k0maru::convention::{FlushEngine, FlushRequest};
use k0maru::distill::{DistillEngine, DistillOptions, DistillTarget, DistilledSkill};
use k0maru::guard::resolve_guard_rules;
use k0maru::loadout::LoadoutBuilder;
use k0maru::storage::SqliteStorage;
use k0maru::RuleCondition;
use predicates::prelude::*;
#[test]
fn test_distill_text_safe_flush_to_vault() {
    let vault_tmp = tempdir().unwrap();
    let vault_root = vault_tmp.path();
    let refs_tmp = tempdir().unwrap();
    let refs_dir = refs_tmp.path();

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

    let content = fs::read_to_string(&result.flush_result.file_path).unwrap();
    assert!(content.contains("# Resolve Missing Distill Module Error"));
    assert!(content.contains("## 🎯 触发上下文与应用场景"));
}

#[test]
fn test_distilled_skill_to_hermes_markdown_and_target() {
    assert_eq!(
        DistillTarget::from_str("hermes").unwrap(),
        DistillTarget::Hermes
    );
    assert_eq!(
        DistillTarget::from_str("default").unwrap(),
        DistillTarget::Default
    );

    let skill = DistilledSkill {
        title: "Fix SQLite Vec Linking Error".to_string(),
        trigger_context: "Building on macOS Apple Silicon".to_string(),
        root_cause: "Missing C compiler flags for sqlite-vec dynamic extension".to_string(),
        remediation: "export SQLITE_VEC_STATIC=1\ncargo build".to_string(),
        prevention_rules: vec!["Always verify static linking on aarch64".to_string()],
        tags: vec!["#type/skill".to_string(), "#topic/rust".to_string()],
        related_notes: vec!["[[SQLite Architecture]]".to_string()],
    };

    let md = skill.to_hermes_markdown();
    assert!(md.contains("name: fix-sqlite-vec-linking-error\n"));
    assert!(md.contains("trigger: Building on macOS Apple Silicon\n"));
    assert!(md.contains("## 🎯 触发上下文与应用场景"));
}

#[test]
fn test_distill_target_omp_creates_skill_md() {
    assert_eq!(DistillTarget::from_str("omp").unwrap(), DistillTarget::Omp);
    assert_eq!(
        DistillTarget::from_str("oh-my-pi").unwrap(),
        DistillTarget::Omp
    );
    assert_eq!(DistillTarget::Omp.to_string(), "omp");

    let vault_tmp = tempdir().unwrap();
    let home_tmp = tempdir().unwrap();
    let refs_tmp = tempdir().unwrap();

    let engine = DistillEngine::new(vault_tmp.path(), refs_tmp.path());
    let opts = DistillOptions {
        title: Some("OMP Skill Export".to_string()),
        context_hint: Some("Exporting a distilled skill into omp".to_string()),
        target: DistillTarget::Omp,
        home_override: Some(home_tmp.path().to_path_buf()),
        dry_run: false,
        ..Default::default()
    };

    let result = engine
        .distill_text(
            "error[E0432]: unresolved import `k0maru::distill`\n$ touch src/distill/mod.rs\n",
            opts,
        )
        .unwrap();

    let expected = home_tmp
        .path()
        .join(".omp")
        .join("agent")
        .join("skills")
        .join("omp-skill-export")
        .join("SKILL.md");

    assert_eq!(result.export_path.as_deref(), Some(expected.as_path()));
    assert!(expected.exists());

    let content = fs::read_to_string(&expected).unwrap();
    assert!(content.starts_with("---\n"));
    assert!(content.contains("name: omp-skill-export\n"));
    assert!(content.contains("description: "));
    assert!(content.contains("# OMP Skill Export"));
}

#[test]
fn test_flush_engine_dry_run_and_live_write() {
    let temp = tempdir().unwrap();
    let vault_root = temp.path();
    let engine = FlushEngine::new(vault_root).unwrap();

    // 1. Dry run
    let req_dry = FlushRequest {
        title: "Dry Run Architecture Note".to_string(),
        summary: Some("Validating zero-disk write preview".to_string()),
        content: "Detailed technical evaluation without disk modification.".to_string(),
        category: "decision".to_string(),
        tags: vec!["arch".to_string()],
        related_notes: vec!["Roadmap 2026".to_string()],
        dry_run: true,
        supersedes: vec![],
    };
    let res_dry = engine.flush(req_dry).unwrap();
    assert!(res_dry.dry_run);
    assert!(!res_dry.file_path.exists());
    assert!(res_dry
        .content_preview
        .contains("Dry Run Architecture Note"));

    // 2. Live write
    let req_live = FlushRequest {
        title: "Live Note".to_string(),
        summary: Some("Summary".to_string()),
        content: "Live content.".to_string(),
        category: "card".to_string(),
        tags: vec!["live".to_string()],
        related_notes: vec![],
        dry_run: false,
        supersedes: vec![],
    };
    let res_live = engine.flush(req_live).unwrap();
    assert!(!res_live.dry_run);
    assert!(res_live.file_path.exists());
}

#[test]
fn test_flush_supersedes_missing_target_leaves_no_orphan_note() {
    let temp = tempdir().unwrap();
    let vault_root = temp.path();
    let engine = FlushEngine::new(vault_root).unwrap();

    let request = FlushRequest {
        title: "Orphan Guard".to_string(),
        summary: Some("Should never hit disk".to_string()),
        content: "Body that must not be written.".to_string(),
        category: "card".to_string(),
        tags: vec![],
        related_notes: vec![],
        dry_run: false,
        supersedes: vec!["does-not-exist".to_string()],
    };

    let err = engine.flush(request).unwrap_err().to_string();
    assert!(
        err.contains("not found or ambiguous"),
        "unexpected error: {err}"
    );

    let mut markdown_files = Vec::new();
    for entry in walkdir::WalkDir::new(vault_root) {
        let entry = entry.unwrap();
        if entry.file_type().is_file()
            && entry.path().extension().and_then(|s| s.to_str()) == Some("md")
        {
            markdown_files.push(entry.path().to_path_buf());
        }
    }
    assert!(
        markdown_files.is_empty(),
        "a failed supersedes validation must not leave an orphan note: {markdown_files:?}"
    );
}

#[test]
fn test_flush_supersedes_is_idempotent_on_identical_note() {
    let temp = tempdir().unwrap();
    let vault_root = temp.path();
    let engine = FlushEngine::new(vault_root).unwrap();

    let old = engine
        .flush(FlushRequest {
            title: "ADR-700".to_string(),
            summary: Some("Old".to_string()),
            content: "Old body".to_string(),
            category: "card".to_string(),
            tags: vec![],
            related_notes: vec![],
            dry_run: false,
            supersedes: vec![],
        })
        .unwrap();

    let make_request = || FlushRequest {
        title: "ADR-701".to_string(),
        summary: Some("New".to_string()),
        content: "New body".to_string(),
        category: "card".to_string(),
        tags: vec![],
        related_notes: vec![],
        dry_run: false,
        supersedes: vec!["ADR-700".to_string()],
    };

    let first = engine.flush(make_request()).unwrap();
    assert!(first.created);
    assert_eq!(first.superseded_files, vec![old.relative_path.clone()]);

    // Re-flushing the exact same note must still (idempotently) report the
    // supersession instead of silently skipping it.
    let second = engine.flush(make_request()).unwrap();
    assert!(!second.created, "identical note must not be re-created");
    assert_eq!(
        second.superseded_files,
        vec![old.relative_path.clone()],
        "an identical re-flush must still report superseded files"
    );

    let old_content = fs::read_to_string(&old.file_path).unwrap();
    assert!(old_content.contains("status: superseded"));
    assert_eq!(
        old_content.matches("> [SUPERSEDED]").count(),
        1,
        "supersession callout must not be duplicated"
    );
}

#[test]
fn test_mark_note_superseded_handles_bom_and_crlf_frontmatter() {
    let temp = tempdir().unwrap();
    let vault_root = temp.path();
    let target = vault_root.join("adr-300.md");
    fs::write(
        &target,
        "\u{feff}---\r\ntitle: Old ADR\r\nstatus: active\r\n---\r\n\r\n# Old ADR\r\n",
    )
    .unwrap();

    let superseded = k0maru::convention::flush::mark_note_superseded(
        vault_root,
        "adr-300",
        Path::new("20_Cards/adr-301.md"),
    )
    .unwrap()
    .expect("target should resolve");

    assert_eq!(superseded, std::path::PathBuf::from("adr-300.md"));
    let content = fs::read_to_string(&target).unwrap();
    assert!(
        content.contains("status: superseded"),
        "BOM + CRLF frontmatter must be rewritten, got: {content:?}"
    );
    assert!(content.contains("superseded_by: \"[[20_Cards/adr-301]]\""));
    assert!(content.contains("> [SUPERSEDED] Superseded by [[20_Cards/adr-301]]"));
}

#[test]
fn test_cli_distill_smoke() {
    let temp_vault = tempdir().unwrap();
    let vault_root = temp_vault.path().to_str().unwrap();

    let mut cmd = Command::cargo_bin("k0maru").unwrap();
    cmd.args([
        "distill",
        "--vault",
        vault_root,
        "--title",
        "CLI Distill Smoke Test",
        "--dry-run",
    ])
    .write_stdin("error[E0308]: mismatched types\n")
    .assert()
    .success()
    .stdout(predicate::str::contains("CLI Distill Smoke Test"));
}

#[test]
fn test_flush_supersedes_marks_old_note_and_updates_guard_and_loadout() {
    let temp = tempdir().unwrap();
    let vault_root = temp.path();

    // Pre-initialize .k0maru/cache.sqlite so FlushEngine auto-syncs cache on flush
    let cache_dir = vault_root.join(".k0maru");
    fs::create_dir_all(&cache_dir).unwrap();
    let cache_path = cache_dir.join("cache.sqlite");
    {
        let _init_storage = SqliteStorage::open(&cache_path).unwrap();
    }

    let engine = FlushEngine::new(vault_root).unwrap();

    // 1. Flush old ADR-010 with a guard rule Forbidden("legacy_bus")
    let req_old = FlushRequest {
        title: "ADR-010".to_string(),
        summary: Some("Old bus architecture".to_string()),
        content: "## Non-Negotiable Invariants\n\n- [RULE-BUS] forbidden: \"legacy_bus\" | remediation: \"Do not use legacy bus\"".to_string(),
        category: "card".to_string(),
        tags: vec!["adr".to_string()],
        related_notes: vec![],
        dry_run: false,
        supersedes: vec![],
    };
    let res_old = engine.flush(req_old).unwrap();
    assert!(res_old.file_path.exists());
    assert!(res_old.superseded_files.is_empty());

    // Verify ADR-010 rule is initially active
    let rules_initial = resolve_guard_rules(vault_root, None).unwrap();
    assert!(
        rules_initial.iter().any(|r| {
            if let RuleCondition::Forbidden(s) = &r.condition {
                s == "legacy_bus"
            } else {
                false
            }
        }),
        "ADR-010 rule should be active initially"
    );

    // 2. Flush new ADR-042 with --supersedes ADR-010 and new guard rule Forbidden("deprecated_v1_api")
    let req_new = FlushRequest {
        title: "ADR-042".to_string(),
        summary: Some("New API architecture".to_string()),
        content: "## Non-Negotiable Invariants\n\n- [RULE-API] forbidden: \"deprecated_v1_api\" | remediation: \"Migrate to v2 API\"".to_string(),
        category: "card".to_string(),
        tags: vec!["adr".to_string()],
        related_notes: vec![],
        dry_run: false,
        supersedes: vec!["ADR-010".to_string()],
    };
    let res_new = engine.flush(req_new).unwrap();
    assert!(res_new.file_path.exists());
    assert_eq!(res_new.superseded_files.len(), 1);
    assert_eq!(res_new.superseded_files[0], res_old.relative_path);

    // 3. Verify ADR-010 content was updated on disk with status: superseded, superseded_by, and callout
    let old_content = fs::read_to_string(&res_old.file_path).unwrap();
    assert!(
        old_content.contains("status: superseded"),
        "ADR-010 frontmatter must have status: superseded"
    );
    assert!(
        old_content.contains("superseded_by:"),
        "ADR-010 frontmatter must have superseded_by"
    );
    assert!(
        old_content.contains("> [SUPERSEDED] Superseded by"),
        "ADR-010 body must have superseded callout"
    );

    // 4. Verify Guard rules: ADR-010 rule retired, ADR-042 rule active with provenance
    let rules_after = resolve_guard_rules(vault_root, None).unwrap();
    assert!(
        !rules_after.iter().any(|r| {
            if let RuleCondition::Forbidden(s) = &r.condition {
                s == "legacy_bus"
            } else {
                false
            }
        }),
        "ADR-010 rule should be retired"
    );

    let new_rule = rules_after
        .iter()
        .find(|r| {
            if let RuleCondition::Forbidden(s) = &r.condition {
                s == "deprecated_v1_api"
            } else {
                false
            }
        })
        .expect("ADR-042 rule should be active");
    assert!(
        new_rule.remediation.contains("Supersedes") || new_rule.remediation.contains("ADR-010"),
        "ADR-042 rule remediation should mention supersession provenance: {}",
        new_rule.remediation
    );

    // 5. Create project note referencing both ADR-010 and ADR-042
    let old_wikilink = res_old
        .relative_path
        .with_extension("")
        .to_string_lossy()
        .replace('\\', "/");
    let new_wikilink = res_new
        .relative_path
        .with_extension("")
        .to_string_lossy()
        .replace('\\', "/");
    let projects_dir = vault_root.join("10_Projects");
    fs::create_dir_all(&projects_dir).unwrap();
    let project_file = projects_dir.join("CoreService.md");
    fs::write(
        &project_file,
        format!(
            "---\ntitle: CoreService\nstatus: active\n---\n\n# CoreService\n\n> Platform Core Invariants.\n\n## References\n- [[{}]]\n- [[{}]]\n",
            old_wikilink, new_wikilink
        ),
    )
    .unwrap();

    // Sync cache with project note
    let adapter = GenericWikiAdapter::new(vault_root);
    let mut storage = SqliteStorage::open(&cache_path).unwrap();
    let mut scanner = k0maru::scanner::IncrementalScanner::new(&adapter, &mut storage);
    scanner.sync(false).unwrap();

    // 6. Build loadout and verify: ADR-010 is excluded, ADR-042 is included
    let builder = LoadoutBuilder::new(&adapter, &storage);
    let loadout = builder
        .build("CoreService")
        .unwrap()
        .expect("Loadout should resolve CoreService");

    let card_paths: Vec<&str> = loadout.l3_cards.iter().map(|c| c.path.as_str()).collect();
    assert!(
        card_paths
            .iter()
            .any(|p| p.contains("adr-042") || p.contains("ADR-042")),
        "Loadout must include active ADR-042: {:?}",
        card_paths
    );
    assert!(
        !card_paths
            .iter()
            .any(|p| p.contains("adr-010") || p.contains("ADR-010")),
        "Loadout must exclude superseded ADR-010: {:?}",
        card_paths
    );
}

#[test]
fn test_flush_unescapes_shell_newlines_in_invariant_content() {
    let temp = tempdir().unwrap();
    let vault_root = temp.path();
    let engine = FlushEngine::new(vault_root).unwrap();

    // Bash double quotes keep `\n` as two characters unless `$'...'` is used;
    // both spellings must produce the same multi-line invariant block.
    let content = "## Non-Negotiable Invariants\\n- [LOCK-011] (deny) [src/**/*.rs] forbidden: \"legacy_lock\" | remediation: \"Use the distributed lock\"";
    assert!(content.contains("\\n") && !content.contains('\n'));

    let result = engine
        .flush(FlushRequest {
            title: "Lock Rewrite".to_string(),
            summary: Some("Escaped newline normalization".to_string()),
            content: content.to_string(),
            category: "card".to_string(),
            tags: vec![],
            related_notes: vec![],
            dry_run: false,
            supersedes: vec![],
        })
        .unwrap();

    let written = fs::read_to_string(&result.file_path).unwrap();
    assert!(
        written.contains("## Non-Negotiable Invariants\n- [LOCK-011]"),
        "escaped newlines must be expanded into a real invariant block, got:\n{written}"
    );
    assert!(
        !written.contains("\\n- [LOCK-011]"),
        "no literal backslash-n may survive, got:\n{written}"
    );
    assert!(
        written.contains("category: decision"),
        "a card carrying invariants must be categorized as a decision, got:\n{written}"
    );
}

#[test]
fn test_flush_card_alias_maps_to_concept_without_invariants() {
    let temp = tempdir().unwrap();
    let vault_root = temp.path();
    fs::create_dir_all(vault_root.join("notes")).unwrap();
    let engine = FlushEngine::new(vault_root).unwrap();

    let result = engine
        .flush(FlushRequest {
            title: "Plain Concept Card".to_string(),
            summary: None,
            content: "A plain concept without invariants.".to_string(),
            category: "card".to_string(),
            tags: vec![],
            related_notes: vec![],
            dry_run: false,
            supersedes: vec![],
        })
        .unwrap();

    assert_eq!(result.category, "concept");
    assert_eq!(
        result
            .file_path
            .parent()
            .and_then(|parent| parent.file_name())
            .and_then(|name| name.to_str()),
        Some("notes"),
        "the concept alias must land in the concept folder, got: {:?}",
        result.file_path
    );
    let written = fs::read_to_string(&result.file_path).unwrap();
    assert!(written.contains("category: concept"));
}

#[test]
fn test_flush_supersedes_frontmatter_links_resolved_relative_path() {
    let temp = tempdir().unwrap();
    let vault_root = temp.path();
    let cards = vault_root.join("20_Cards");
    fs::create_dir_all(&cards).unwrap();
    fs::write(
        cards.join("adr-010-legacy-setnx-lock.md"),
        "---\ntitle: ADR-010 Legacy Setnx Lock\nstatus: active\n---\n\n# ADR-010 Legacy Setnx Lock\n",
    )
    .unwrap();

    let engine = FlushEngine::new(vault_root).unwrap();
    let result = engine
        .flush(FlushRequest {
            title: "ADR-011 Distributed Lock".to_string(),
            summary: Some("Replaces the legacy SETNX lock".to_string()),
            content: "## Non-Negotiable Invariants\n- [LOCK-011] (deny) [src/**/*.rs] forbidden: \"setnx_lock\" | remediation: \"Use the distributed lock\"".to_string(),
            category: "card".to_string(),
            tags: vec![],
            related_notes: vec![],
            dry_run: false,
            supersedes: vec!["adr-010-legacy-setnx-lock".to_string()],
        })
        .unwrap();

    assert_eq!(
        result.superseded_files.len(),
        1,
        "the resolved supersede target must be reported"
    );

    let written = fs::read_to_string(&result.file_path).unwrap();
    assert!(
        written.contains("supersedes:\n  - \"[[20_Cards/adr-010-legacy-setnx-lock]]\""),
        "frontmatter must link the resolved vault-relative note, got:\n{written}"
    );

    let old = fs::read_to_string(cards.join("adr-010-legacy-setnx-lock.md")).unwrap();
    assert!(old.contains("status: superseded"), "got:\n{old}");
}
