use std::fs;
use tempfile::TempDir;

use k0maru::adapters::ObsidianAdapter;
use k0maru::git::{parse_diff_stream, FileChangeStatus, GitInspector, ModifiedFile};
use k0maru::loadout::{estimate_tokens, LoadoutBuilder};
use k0maru::scanner::IncrementalScanner;
use k0maru::storage::SqliteStorage;

/// Adversarial Challenge 1: Token Bounds (<300 tokens strictly).
///
/// Requirement R2: "strictly bounding the injected payload to <300 tokens."
/// Worker claimed: "Mathematical guarantee: unconditionally enforces estimate_tokens(&markdown) < 300 across all input permutations."
///
/// Finding: When modified file paths contain long module names (e.g. 200+ characters),
/// `build_with_diff` does not truncate `active_changes_line` or apply a hard token ceiling
/// to the final markdown, causing the token estimate to exceed 299 tokens (observed: 323 tokens).
#[test]
fn test_reproduce_token_budget_exceeded_on_long_module_names() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    let projects_dir = root.join("10_Projects");
    fs::create_dir_all(&projects_dir).unwrap();

    let project_name =
        "k0maru-enterprise-deep-learning-and-context-management-system-platform-extra-long-name";
    let project_note = format!(
        r#"---
title: {project_name}
status: active
tags:
  - project
---
# {project_name}

> Enterprise high-throughput low-latency multi-agent context platform with distributed storage and indexing.
"#
    );
    fs::write(
        projects_dir.join(format!("{}.md", project_name)),
        project_note,
    )
    .unwrap();

    let adapter = ObsidianAdapter::new(root);
    let mut storage = SqliteStorage::in_memory().unwrap();
    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    scanner.sync(false).unwrap();

    let builder = LoadoutBuilder::new(&adapter, &storage);

    let long_mod_1 = "a".repeat(350);
    let long_mod_2 = "b".repeat(350);

    let modified = vec![
        ModifiedFile::new(format!("{long_mod_1}/file1.rs"), FileChangeStatus::Modified),
        ModifiedFile::new(format!("{long_mod_2}/file2.rs"), FileChangeStatus::Modified),
    ];

    let loadout_opt = builder.build_with_diff(project_name, &modified).unwrap();
    assert!(loadout_opt.is_some(), "Loadout should be generated");
    let loadout = loadout_opt.unwrap();

    let tokens = estimate_tokens(&loadout.markdown);
    println!("Observed token count: {}", tokens);

    // This assertion MUST pass for R2 compliance, but currently fails with 323 tokens.
    assert!(
        tokens < 300,
        "HARD BOUND VIOLATION! Tokens: {} >= 300 (Requirement R2 strictly requires <300 tokens)",
        tokens
    );
}

/// Adversarial Challenge 2: Unified Diff Parser Code Leakage.
///
/// In unified diffs from `git diff`, context lines begin with a space.
/// Any non-empty context line in Python, Go, YAML, Markdown, Bash, etc.,
/// that does not end with `;`, `{`, or `}` falls through to Format E and is
/// falsely parsed as a modified file path.
#[test]
fn test_reproduce_unified_diff_code_leakage() {
    let diff = r#"
diff --git a/server.py b/server.py
index abc..def 100644
--- a/server.py
+++ b/server.py
@@ -10,6 +10,7 @@
 def handle_request(req):
     # Validate session
     user = authenticate(req)
+    logger.info("request from user")
     return process_pipeline(user)
"#;
    let files = parse_diff_stream(diff.as_bytes()).unwrap();
    let paths: Vec<String> = files
        .iter()
        .map(|f| f.path.to_string_lossy().to_string())
        .collect();

    let leaked_code = paths.iter().any(|p| {
        p.contains("handle_request")
            || p.contains("authenticate")
            || p.contains("return process_pipeline")
    });

    assert!(
        !leaked_code,
        "Diff body code lines leaked into parsed file paths: {:?}",
        paths
    );
}

/// Adversarial Challenge 3: Quoted File Paths with Spaces.
///
/// Git quotes paths containing spaces (e.g. `diff --git "a/path with spaces.rs" "b/path with spaces.rs"`).
/// Splitting on whitespace corrupts the path into fragments.
#[test]
fn test_reproduce_quoted_paths_with_spaces() {
    let diff = r#"
diff --git "a/src/my core file.rs" "b/src/my core file.rs"
--- "a/src/my core file.rs"
+++ "b/src/my core file.rs"
@@ -1,2 +1,3 @@
+// modified
"#;
    let files = parse_diff_stream(diff.as_bytes()).unwrap();
    let paths: Vec<String> = files
        .iter()
        .map(|f| f.path.to_string_lossy().to_string())
        .collect();

    assert!(
        paths
            .iter()
            .any(|p| p.contains("my core file.rs") && !p.contains('\"') && !p.starts_with("b/")),
        "Path should be cleanly unquoted and resolved without prefix artifacts, got: {:?}",
        paths
    );
}

/// Adversarial Challenge 4: Empty Diffs and Submodule Commits (Edge Cases).
#[test]
fn test_edge_case_empty_diff_and_submodules() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    let non_git = GitInspector::inspect_diff(root, None).unwrap();
    assert!(non_git.is_empty());

    let empty_stream = parse_diff_stream("".as_bytes()).unwrap();
    assert!(empty_stream.is_empty());

    let submodule_diff = r#"
diff --git a/vendor/submod b/vendor/submod
index abc..def 160000
--- a/vendor/submod
+++ b/vendor/submod
@@ -1 +1 @@
-Subproject commit 1234567890abcdef1234567890abcdef12345678
+Subproject commit abcdef1234567890abcdef1234567890abcdef12
"#;
    let submod_files = parse_diff_stream(submodule_diff.as_bytes()).unwrap();
    assert_eq!(submod_files.len(), 1);
    assert_eq!(submod_files[0].path.to_string_lossy(), "vendor/submod");
}
