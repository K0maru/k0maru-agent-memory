use std::fs;
use std::path::Path;

use assert_cmd::Command;
use tempfile::tempdir;

use k0maru::guard::resolve_guard_rules;
use k0maru::pack::{export_pack, import_pack, PackExportOptions, PackImportOptions};

#[test]
fn test_pack_export_and_import_into_flat_karpathy_wiki() {
    let source_vault = tempdir().unwrap();
    let cards_dir = source_vault.path().join("20_Cards");
    fs::create_dir_all(&cards_dir).unwrap();

    let note_content = r#"---
title: Redis Lock ADR
tags:
  - topic/redis
invariants:
  - id: REDIS-001
    title: Mandatory Redlock Algorithm
    file_pattern: "*.rs"
    severity: deny
    forbidden: "SETNX"
    remediation: "Use Redlock instead of bare SETNX"
---
# Redis Lock ADR

Always use Redlock. See [[20_Cards/Distributed_Systems|Distributed Systems]] for context.
"#;
    fs::write(cards_dir.join("Redis_Lock_ADR.md"), note_content).unwrap();

    let export_opts = PackExportOptions {
        vault_path: source_vault.path().to_path_buf(),
        name: "redis-pack".to_string(),
        version: "0.1.0".to_string(),
        description: Some("Redis lock memory pack".to_string()),
        tag: Some("topic/redis".to_string()),
        query: None,
        files: vec![],
    };

    let pack = export_pack(&export_opts).expect("export_pack should succeed");
    assert_eq!(pack.notes.len(), 1, "pack should have exactly 1 note");
    assert_eq!(
        pack.guard_rules.len(),
        1,
        "pack should have exactly 1 guard rule"
    );
    assert_eq!(pack.guard_rules[0].id, "REDIS-001");

    let note = &pack.notes[0];
    assert!(
        note.content
            .contains("[[Distributed_Systems|Distributed Systems]]"),
        "note content should contain normalized wikilink without folder prefix, got: {}",
        note.content
    );
    assert!(
        !note.content.contains("20_Cards/Distributed_Systems"),
        "note content should not contain 20_Cards prefix, got: {}",
        note.content
    );

    // Target Flat Karpathy LLM-Wiki: root note existing-note.md and NO subdirectories
    let flat_wiki = tempdir().unwrap();
    fs::write(
        flat_wiki.path().join("existing-note.md"),
        "# Existing Note\nA flat root note.\n",
    )
    .unwrap();

    let import_opts = PackImportOptions {
        vault_path: flat_wiki.path().to_path_buf(),
        dry_run: false,
    };
    let import_res = import_pack(&pack, &import_opts).expect("import_pack should succeed");
    assert_eq!(import_res.imported_notes.len(), 1);

    // Verify imported note lands directly in the root of the flat wiki (no 20_Cards/ created)
    assert!(
        !flat_wiki.path().join("20_Cards").exists(),
        "20_Cards directory must NOT be created in flat wiki"
    );

    let imported_item = &import_res.imported_notes[0];
    let imported_file_path = flat_wiki.path().join(&imported_item.relative_path);
    assert!(
        imported_file_path.exists(),
        "imported note file must exist at {}",
        imported_file_path.display()
    );
    assert_eq!(
        imported_item.relative_path.parent(),
        Some(Path::new("")),
        "imported note relative path parent must be empty root"
    );

    // Verify rule REDIS-001 is active in the flat wiki
    let active_rules = resolve_guard_rules(flat_wiki.path(), None).unwrap();
    assert!(
        active_rules.iter().any(|r| r.id == "REDIS-001"),
        "REDIS-001 rule should be active in flat wiki, found: {:?}",
        active_rules
    );
}

#[test]
fn test_pack_cli_export_inspect_import_into_decisions_wiki() {
    let source_vault = tempdir().unwrap();
    let cards_dir = source_vault.path().join("20_Cards");
    fs::create_dir_all(&cards_dir).unwrap();

    let adr_content = r#"---
title: Database Storage ADR
category: decision
tags:
  - architecture
invariants:
  - id: DB-001
    title: Deny Direct SQL Execution
    file_pattern: "*.rs"
    severity: deny
    forbidden: "raw_sql_exec"
    remediation: "Use query builder API"
---
# Database Storage ADR

Always use type-safe queries. Link to [[20_Cards/ORM_Guidelines|ORM Guidelines]].
"#;
    fs::write(cards_dir.join("Database_Storage_ADR.md"), adr_content).unwrap();

    let pack_file = source_vault.path().join("db-storage.pack.json");

    // 1. CLI: k0maru pack export
    let mut export_cmd = Command::cargo_bin("k0maru").unwrap();
    export_cmd
        .args([
            "pack",
            "export",
            "--name",
            "db-storage-pack",
            "--vault",
            source_vault.path().to_str().unwrap(),
            "-o",
            pack_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(
        pack_file.exists(),
        "pack JSON file should exist at output path"
    );
    let pack_json_str = fs::read_to_string(&pack_file).unwrap();
    assert!(pack_json_str.contains("db-storage-pack"));
    assert!(pack_json_str.contains("DB-001"));
    assert!(pack_json_str.contains("[[ORM_Guidelines|ORM Guidelines]]"));

    // Target vault with a decisions/ directory
    let target_vault = tempdir().unwrap();
    let decisions_dir = target_vault.path().join("decisions");
    fs::create_dir_all(&decisions_dir).unwrap();
    fs::write(
        decisions_dir.join("000-initial.md"),
        "# Initial Decision\nRecord of previous decision.\n",
    )
    .unwrap();

    // 2. CLI: k0maru pack inspect --json
    let mut inspect_cmd = Command::cargo_bin("k0maru").unwrap();
    let assert_inspect = inspect_cmd
        .args([
            "pack",
            "inspect",
            pack_file.to_str().unwrap(),
            "--vault",
            target_vault.path().to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success();

    let inspect_stdout = String::from_utf8(assert_inspect.get_output().stdout.clone()).unwrap();
    let inspect_report: serde_json::Value =
        serde_json::from_str(&inspect_stdout).expect("valid inspect report JSON");

    assert_eq!(inspect_report["name"], "db-storage-pack");
    assert_eq!(inspect_report["notes_count"], 1);
    assert_eq!(inspect_report["guard_rules_count"], 1);

    let predicted_paths = inspect_report["predicted_paths"]
        .as_array()
        .expect("predicted_paths should be an array");
    assert_eq!(predicted_paths.len(), 1);
    let pred_path = predicted_paths[0].as_str().unwrap();
    assert!(
        pred_path.starts_with("decisions"),
        "predicted path must start with decisions/, got: {}",
        pred_path
    );

    // 3. CLI: k0maru pack import --json
    let mut import_cmd = Command::cargo_bin("k0maru").unwrap();
    let assert_import = import_cmd
        .args([
            "pack",
            "import",
            pack_file.to_str().unwrap(),
            "--vault",
            target_vault.path().to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success();

    let import_stdout = String::from_utf8(assert_import.get_output().stdout.clone()).unwrap();
    let import_res: serde_json::Value =
        serde_json::from_str(&import_stdout).expect("valid import result JSON");

    assert_eq!(import_res["pack_name"], "db-storage-pack");
    let imported_notes = import_res["imported_notes"]
        .as_array()
        .expect("imported_notes should be an array");
    assert_eq!(imported_notes.len(), 1);

    let rel_path = imported_notes[0]["relative_path"].as_str().unwrap();
    assert!(
        rel_path.starts_with("decisions"),
        "imported note relative path must be inside decisions/, got: {}",
        rel_path
    );

    let imported_file = target_vault.path().join(rel_path);
    assert!(
        imported_file.exists(),
        "imported note file must exist on disk inside decisions/"
    );

    // Invariant check: rule DB-001 should be active in the target decisions wiki
    let active_rules = resolve_guard_rules(target_vault.path(), None).unwrap();
    assert!(
        active_rules.iter().any(|r| r.id == "DB-001"),
        "DB-001 rule should be active in target decisions wiki, found: {:?}",
        active_rules
    );
}
