use std::path::PathBuf;
use tempfile::tempdir;

use k0maru::convention::{ConventionSniffer, NoteCategory};
use k0maru::distill::{DistilledSkill, SkillExtractor};

#[test]
fn test_note_category_skill_support() {
    assert_eq!(NoteCategory::Skill.as_str(), "skill");
    assert_eq!(NoteCategory::from_str_loose("skill"), NoteCategory::Skill);
    assert_eq!(NoteCategory::from_str_loose("skills"), NoteCategory::Skill);
    assert_eq!(
        NoteCategory::from_str_loose("playbook"),
        NoteCategory::Skill
    );
    assert_eq!(
        NoteCategory::from_str_loose("playbooks"),
        NoteCategory::Skill
    );
    assert_eq!(NoteCategory::from_str_loose("recipe"), NoteCategory::Skill);
    assert_eq!(NoteCategory::from_str_loose("recipes"), NoteCategory::Skill);
    assert_eq!(
        NoteCategory::from_str_loose("troubleshooting"),
        NoteCategory::Skill
    );
}

#[test]
fn test_vault_convention_sniff_skills_dir() {
    let tmp = tempdir().unwrap();
    let skills_dir = tmp.path().join("skills");
    std::fs::create_dir_all(&skills_dir).unwrap();

    let convention = ConventionSniffer::sniff(tmp.path());
    assert_eq!(
        convention.target_subdir_for("skill"),
        PathBuf::from("skills")
    );
}

#[test]
fn test_distilled_skill_to_markdown() {
    let skill = DistilledSkill {
        title: "Fix SQLite-Vec Linking Failure".to_string(),
        trigger_context: "Building on macOS ARM64 with cargo test".to_string(),
        root_cause: "Dynamic linker cannot find libsqlite_vec.dylib".to_string(),
        remediation: "Run `cargo build --features sqlite-vec` with DYLD_LIBRARY_PATH set"
            .to_string(),
        prevention_rules: vec![
            "Always specify rpath in build.rs for bundled C extensions".to_string(),
            "Verify sqlite-vec virtual table presence using k0maru doctor".to_string(),
        ],
        tags: vec!["topic/rust".to_string(), "topic/sqlite".to_string()],
        related_notes: vec!["SQLite-Vec Architecture".to_string()],
    };

    let md = skill.to_markdown();
    assert!(md.contains("# Fix SQLite-Vec Linking Failure"));
    assert!(md.contains("## 🎯 触发上下文与应用场景 (Trigger Context)"));
    assert!(md.contains("## 🔍 故障根因与错误特征 (Root Cause & Signatures)"));
    assert!(md.contains("## 🛠️ 修复策略与执行命令 (Remediation & Commands)"));
    assert!(md.contains("## 🛡️ 防范规约与常青法则 (Prevention Rules & Best Practices)"));
    assert!(md.contains("[[SQLite-Vec Architecture]]"));
    assert!(md.contains("type: skill") || md.contains("type/skill"));
}

#[test]
fn test_skill_extractor_rust_compiler_error() {
    let trace = r#"
$ cargo test --test test_vector_sync
   Compiling k0maru v0.6.0 (/workspace/k0maru)
error[E0382]: use of moved value: `storage`
  --> src/scanner/vector_sync.rs:42:9
   |
38 |         let storage = Arc::new(storage);
   |             ------- move occurs because `storage` has type `SqliteStorage`, which does not implement the `Copy` trait
...
42 |         storage.insert_vector(&doc_id, &vec)?;
   |         ^^^^^^^ value used here after move

error: could not compile `k0maru` (lib test) due to 1 previous error
$ git checkout -- src/scanner/vector_sync.rs
$ cargo test
test result: ok. 195 passed; 0 failed
"#;

    let skill = SkillExtractor::extract(
        trace,
        Some("Fix storage moved value error in vector_sync.rs"),
        Some("Resolve E0382 Move Error in Vector Sync"),
    );

    assert_eq!(skill.title, "Resolve E0382 Move Error in Vector Sync");
    assert!(
        skill.root_cause.contains("error[E0382]")
            || skill.root_cause.contains("use of moved value")
    );
    assert!(
        skill.trigger_context.contains("cargo test")
            || skill.trigger_context.contains("vector_sync.rs")
    );
    assert!(
        skill.remediation.contains("git checkout")
            || skill.remediation.contains("cargo test")
            || skill.remediation.contains("vector_sync.rs")
    );
    assert!(!skill.prevention_rules.is_empty());
    assert!(
        skill.tags.contains(&"topic/rust".to_string())
            || skill.tags.contains(&"type/skill".to_string())
    );
}

#[test]
fn test_skill_extractor_python_traceback() {
    let trace = r#"
$ python3 scripts/fetch_models.py --target onnx
Traceback (most recent call last):
  File "scripts/fetch_models.py", line 88, in <module>
    main()
  File "scripts/fetch_models.py", line 42, in main
    api_key = os.environ['HF_TOKEN']
  File "/usr/lib/python3.10/os.py", line 680, in __getitem__
    raise KeyError(key) from None
KeyError: 'HF_TOKEN'
$ export HF_TOKEN="dummy_token"
$ python3 scripts/fetch_models.py --target onnx
[OK] Downloaded model weights successfully
"#;

    let skill = SkillExtractor::extract(
        trace,
        Some("Missing HF_TOKEN environment variable during model download"),
        None,
    );

    assert!(
        skill.title.contains("KeyError")
            || skill.title.contains("HF_TOKEN")
            || skill.title.contains("fetch_models")
    );
    assert!(
        skill.root_cause.contains("KeyError: 'HF_TOKEN'") || skill.root_cause.contains("HF_TOKEN")
    );
    assert!(
        skill.remediation.contains("export HF_TOKEN") || skill.remediation.contains("HF_TOKEN")
    );
    assert!(!skill.prevention_rules.is_empty());
}

#[test]
fn test_skill_extractor_shell_command_failure() {
    let trace = r#"
$ sqlite-vec-dump --db ./cache.sqlite
zsh: command not found: sqlite-vec-dump
exit status 127
$ brew install sqlite-vec-cli
$ sqlite-vec-dump --db ./cache.sqlite
Total 384-dim vectors: 42
"#;

    let skill = SkillExtractor::extract(trace, None, None);
    assert!(skill.root_cause.contains("command not found") || skill.root_cause.contains("127"));
    assert!(
        skill.remediation.contains("brew install") || skill.remediation.contains("sqlite-vec-cli")
    );
}

#[test]
fn test_skill_extractor_fallback_unstructured() {
    let trace = "Build failed unexpectedly with code 1. Check network connectivity.";
    let skill = SkillExtractor::extract(trace, None, None);
    assert!(!skill.title.is_empty());
    assert!(!skill.root_cause.is_empty());
    assert!(!skill.prevention_rules.is_empty());
}
