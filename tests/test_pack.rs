use std::fs;
use std::path::Path;

use assert_cmd::Command;
use tempfile::tempdir;

use k0maru::guard::resolve_guard_rules;
use k0maru::pack::{
    export_pack, import_pack, install_pack, list_installed_packs, uninstall_pack,
    PackExportOptions, PackImportOptions, PackInstallOptions,
};
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

#[test]
fn test_pack_install_and_uninstall_isolated_lifecycle() {
    let source_vault = tempdir().unwrap();
    let cards_dir = source_vault.path().join("20_Cards");
    fs::create_dir_all(&cards_dir).unwrap();

    let note_content = r#"---
title: Redis Distributed Lock ADR
category: decision
tags:
  - topic/redis
invariants:
  - id: REDIS-001
    title: Deny Plain SETNX Without Expiry
    file_pattern: "*.rs"
    severity: deny
    forbidden: "redis.setnx"
    remediation: "Use SET key val NX EX 30 with mandatory expiry TTL"
---
# Redis Distributed Lock ADR

Distributed locks must always have an explicit TTL.
"#;
    fs::write(
        cards_dir.join("Redis_Distributed_Lock_ADR.md"),
        note_content,
    )
    .unwrap();

    let pack_opts = PackExportOptions {
        vault_path: source_vault.path().to_path_buf(),
        name: "redis-pack".to_string(),
        version: "0.1.0".to_string(),
        description: Some("Redis Distributed Invariants".to_string()),
        tag: None,
        query: None,
        files: Vec::new(),
    };

    let pack = export_pack(&pack_opts).expect("export_pack should succeed");
    let pack_file = source_vault.path().join("redis.pack.json");
    fs::write(&pack_file, serde_json::to_string_pretty(&pack).unwrap()).unwrap();

    let target_vault = tempdir().unwrap();
    let temp_home = tempdir().unwrap();

    let install_opts = PackInstallOptions {
        vault_path: target_vault.path().to_path_buf(),
        source_path: pack_file,
        home_override: Some(temp_home.path().to_path_buf()),
        dry_run: false,
    };
    let install_res = install_pack(&install_opts).expect("install_pack should succeed");
    assert_eq!(install_res.manifest.name, "redis-pack");
    assert_eq!(install_res.manifest.notes_count, 1);
    assert_eq!(install_res.manifest.guard_rules_count, 1);

    // Verify note is installed under <vault>/.k0maru/packs/<name>/ (NOT in root or 20_Cards/)
    let pack_dir = target_vault
        .path()
        .join(".k0maru")
        .join("packs")
        .join("redis-pack");
    assert!(
        pack_dir.exists(),
        "pack dir should exist under .k0maru/packs/redis-pack"
    );
    assert!(
        pack_dir.join("redis-distributed-lock-adr.md").exists(),
        "note should exist in pack dir"
    );
    assert!(
        !target_vault
            .path()
            .join("redis-distributed-lock-adr.md")
            .exists(),
        "note should NOT exist in vault root"
    );
    assert!(
        !target_vault.path().join("20_Cards").exists(),
        "20_Cards should NOT exist in target vault"
    );

    // Verify list_installed_packs returns 1 pack
    let installed =
        list_installed_packs(target_vault.path()).expect("list_installed_packs should succeed");
    assert_eq!(installed.len(), 1);
    assert_eq!(installed[0].name, "redis-pack");
    assert_eq!(installed[0].guard_rules_count, 1);

    // Verify resolve_guard_rules in target vault includes REDIS-001
    let active_rules =
        resolve_guard_rules(target_vault.path(), None).expect("resolve_guard_rules should succeed");
    assert!(
        active_rules.iter().any(|r| r.id == "REDIS-001"),
        "REDIS-001 should be active in target vault, found: {:?}",
        active_rules
    );

    // Call uninstall_pack
    let uninstall_res = uninstall_pack(target_vault.path(), "redis-pack", Some(temp_home.path()))
        .expect("uninstall_pack should succeed");
    assert_eq!(uninstall_res.name, "redis-pack");

    // Verify list_installed_packs is now empty
    let installed_after =
        list_installed_packs(target_vault.path()).expect("list_installed_packs should succeed");
    assert!(
        installed_after.is_empty(),
        "Installed packs should be empty after uninstall, found: {:?}",
        installed_after
    );

    // Verify resolve_guard_rules no longer contains REDIS-001
    let active_rules_after =
        resolve_guard_rules(target_vault.path(), None).expect("resolve_guard_rules should succeed");
    assert!(
        !active_rules_after.iter().any(|r| r.id == "REDIS-001"),
        "REDIS-001 should no longer be active after uninstall, found: {:?}",
        active_rules_after
    );
}

#[test]
fn test_dsh_directory_install_and_uninstall_mounts_omp_skills_and_memory() {
    let dsh_dir = tempdir().unwrap();

    // 1. harness.json
    let harness_json = serde_json::json!({
        "id": "mujoco-dsh",
        "name": "mujoco-dsh",
        "version": "0.2.0",
        "description": "MuJoCo simulation harness"
    });
    fs::write(
        dsh_dir.path().join("harness.json"),
        serde_json::to_string_pretty(&harness_json).unwrap(),
    )
    .unwrap();

    // 2. AGENTS.md with guard-rules / invariants
    let agents_md = r#"---
title: MuJoCo Simulation Agents
category: guide
tags:
  - simulation
  - mujoco
invariants:
  - id: MUJOCO-001
    title: Validate Timestep Bounds
    file_pattern: "*.py"
    severity: deny
    forbidden: "step(dt=0)"
    remediation: "dt must be > 0"
guard_rules:
  - id: MUJOCO-001
    title: Validate Timestep Bounds
    file_pattern: "*.py"
    severity: deny
    forbidden: "step(dt=0)"
    remediation: "dt must be > 0"
---

# MuJoCo Simulation Agents

## Non-Negotiable Invariants
```yaml
- id: MUJOCO-001
  title: Validate Timestep Bounds
  file_pattern: "*.py"
  severity: deny
  forbidden: "step(dt=0)"
  remediation: "dt must be > 0"
```
"#;
    fs::write(dsh_dir.path().join("AGENTS.md"), agents_md).unwrap();

    // 3. skills/mujoco-sim/SKILL.md
    let skill_dir = dsh_dir.path().join("skills").join("mujoco-sim");
    fs::create_dir_all(&skill_dir).unwrap();
    let skill_content = r#"---
name: mujoco-sim
description: Run physics simulation steps safely
---

# MuJoCo Sim Skill

Run physics simulation steps safely.
"#;
    fs::write(skill_dir.join("SKILL.md"), skill_content).unwrap();

    let target_vault = tempdir().unwrap();
    let temp_home = tempdir().unwrap();

    let install_opts = PackInstallOptions {
        vault_path: target_vault.path().to_path_buf(),
        source_path: dsh_dir.path().to_path_buf(),
        home_override: Some(temp_home.path().to_path_buf()),
        dry_run: false,
    };
    let install_res = install_pack(&install_opts).expect("install_pack should succeed");
    assert_eq!(install_res.manifest.name, "mujoco-dsh");
    assert_eq!(install_res.manifest.version, "0.2.0");
    assert_eq!(install_res.manifest.source_kind, "dsh");
    assert_eq!(
        install_res.manifest.installed_skills,
        vec!["mujoco-sim".to_string()]
    );

    // Verifies temp_home.join(".omp/agent/skills/mujoco-sim/SKILL.md") exists
    let omp_skill_file = temp_home
        .path()
        .join(".omp")
        .join("agent")
        .join("skills")
        .join("mujoco-sim")
        .join("SKILL.md");
    assert!(
        omp_skill_file.exists(),
        "Mounted omp skill file should exist at {}",
        omp_skill_file.display()
    );

    // Verifies resolve_guard_rules finds MUJOCO-001
    let active_rules =
        resolve_guard_rules(target_vault.path(), None).expect("resolve_guard_rules should succeed");
    assert!(
        active_rules.iter().any(|r| r.id == "MUJOCO-001"),
        "MUJOCO-001 should be active in target vault, found: {:?}",
        active_rules
    );

    // Call uninstall_pack
    let uninstall_res = uninstall_pack(target_vault.path(), "mujoco-dsh", Some(temp_home.path()))
        .expect("uninstall_pack should succeed");
    assert_eq!(uninstall_res.name, "mujoco-dsh");
    assert_eq!(uninstall_res.removed_skills, vec!["mujoco-sim".to_string()]);

    // Verifies uninstall_pack cleanly removes both <vault>/.k0maru/packs/mujoco-dsh and temp_home.join(".omp/agent/skills/mujoco-sim")
    let pack_dir = target_vault
        .path()
        .join(".k0maru")
        .join("packs")
        .join("mujoco-dsh");
    assert!(
        !pack_dir.exists(),
        "Pack dir in vault should be removed: {}",
        pack_dir.display()
    );

    let omp_skill_dir = temp_home
        .path()
        .join(".omp")
        .join("agent")
        .join("skills")
        .join("mujoco-sim");
    assert!(
        !omp_skill_dir.exists(),
        "OMP skill directory should be removed: {}",
        omp_skill_dir.display()
    );

    // Verifies list_installed_packs is empty
    let installed_after =
        list_installed_packs(target_vault.path()).expect("list_installed_packs should succeed");
    assert!(
        installed_after.is_empty(),
        "Installed packs should be empty after uninstall, found: {:?}",
        installed_after
    );

    // Verifies resolve_guard_rules no longer contains MUJOCO-001
    let active_rules_after =
        resolve_guard_rules(target_vault.path(), None).expect("resolve_guard_rules should succeed");
    assert!(
        !active_rules_after.iter().any(|r| r.id == "MUJOCO-001"),
        "MUJOCO-001 should no longer be active after uninstall, found: {:?}",
        active_rules_after
    );
}

#[test]
fn test_pack_cli_install_list_uninstall_lifecycle() {
    let source_vault = tempdir().unwrap();
    let cards_dir = source_vault.path().join("20_Cards");
    fs::create_dir_all(&cards_dir).unwrap();

    let adr_content = r#"---
title: Auth Security ADR
category: decision
tags:
  - security
invariants:
  - id: AUTH-001
    title: Mandatory JWT Validation
    file_pattern: "*.rs"
    severity: deny
    forbidden: "insecure_jwt_decode"
    remediation: "Use secure token verification"
---
# Auth Security ADR
Invariants for authentication.
"#;
    fs::write(cards_dir.join("Auth_Security_ADR.md"), adr_content).unwrap();

    let pack_file = source_vault.path().join("auth-security.pack.json");

    let mut export_cmd = Command::cargo_bin("k0maru").unwrap();
    export_cmd
        .args([
            "pack",
            "export",
            "--name",
            "auth-security",
            "--vault",
            source_vault.path().to_str().unwrap(),
            "-o",
            pack_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    let target_vault = tempdir().unwrap();
    let temp_home = tempdir().unwrap();

    // CLI: k0maru pack install
    let mut install_cmd = Command::cargo_bin("k0maru").unwrap();
    install_cmd
        .args([
            "pack",
            "install",
            pack_file.to_str().unwrap(),
            "--vault",
            target_vault.path().to_str().unwrap(),
            "--home",
            temp_home.path().to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success();

    // CLI: k0maru pack list
    let mut list_cmd = Command::cargo_bin("k0maru").unwrap();
    let list_assert = list_cmd
        .args([
            "pack",
            "list",
            "--vault",
            target_vault.path().to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success();

    let list_stdout = String::from_utf8(list_assert.get_output().stdout.clone()).unwrap();
    let packs_json: serde_json::Value = serde_json::from_str(&list_stdout).unwrap();
    assert_eq!(packs_json.as_array().unwrap().len(), 1);
    assert_eq!(packs_json[0]["name"], "auth-security");

    // CLI: k0maru pack uninstall
    let mut uninstall_cmd = Command::cargo_bin("k0maru").unwrap();
    uninstall_cmd
        .args([
            "pack",
            "uninstall",
            "auth-security",
            "--vault",
            target_vault.path().to_str().unwrap(),
            "--home",
            temp_home.path().to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success();

    // CLI: k0maru pack list after uninstall
    let mut list_cmd_after = Command::cargo_bin("k0maru").unwrap();
    let list_assert_after = list_cmd_after
        .args([
            "pack",
            "list",
            "--vault",
            target_vault.path().to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success();
    let list_stdout_after =
        String::from_utf8(list_assert_after.get_output().stdout.clone()).unwrap();
    let packs_json_after: serde_json::Value = serde_json::from_str(&list_stdout_after).unwrap();
    assert_eq!(packs_json_after.as_array().unwrap().len(), 0);
}
