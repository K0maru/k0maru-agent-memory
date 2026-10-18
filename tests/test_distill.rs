use std::fs;
use std::str::FromStr;
use tempfile::tempdir;

use assert_cmd::Command;
use k0maru::convention::{FlushEngine, FlushRequest};
use k0maru::distill::{DistillEngine, DistillOptions, DistillTarget, DistilledSkill};
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
    };
    let res_live = engine.flush(req_live).unwrap();
    assert!(!res_live.dry_run);
    assert!(res_live.file_path.exists());
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
