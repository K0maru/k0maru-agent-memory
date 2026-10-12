use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_sync_with_embed_model_mock() {
    let temp = tempdir().expect("tempdir");
    let vault_path = temp.path();

    // Create a mock note in 10_Projects
    let project_dir = vault_path.join("10_Projects");
    fs::create_dir_all(&project_dir).expect("create project dir");
    fs::write(
        project_dir.join("System.md"),
        "# Pluggable Embeddings\nTesting multi-model vector embeddings architecture.\n",
    )
    .expect("write note");

    let mut cmd = Command::cargo_bin("k0maru").expect("cargo bin");
    cmd.arg("sync")
        .arg("--vault")
        .arg(vault_path)
        .arg("--vector")
        .arg("--embed-model")
        .arg("mock")
        .arg("--json");

    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let json_val: serde_json::Value = serde_json::from_str(&stdout).expect("valid json output");

    assert!(json_val.get("vector").is_some());
    let vector_stats = json_val.get("vector").unwrap();
    assert_eq!(vector_stats["embedded_count"], 1);
}

#[test]
fn test_doctor_json_reports_embedding_metadata() {
    let temp = tempdir().expect("tempdir");
    let vault_path = temp.path();

    let mut cmd = Command::cargo_bin("k0maru").expect("cargo bin");
    cmd.env("K0MARU_EMBED_MODEL", "bge-small-zh-v1.5")
        .arg("doctor")
        .arg("--vault")
        .arg(vault_path)
        .arg("--json");

    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let json_val: serde_json::Value = serde_json::from_str(&stdout).expect("valid json output");

    let items = json_val["items"].as_array().expect("items array");
    let embed_item = items
        .iter()
        .find(|i| i["name"] == "Embedding Engine")
        .expect("Embedding Engine item");

    let detail = embed_item["detail"].as_str().unwrap();
    assert!(
        detail.contains("bge-small-zh-v1.5") || detail.contains("512"),
        "Detail should mention model or dimensions: {}",
        detail
    );
}

#[test]
fn test_search_vector_with_embed_model_mock() {
    let temp = tempdir().expect("tempdir");
    let vault_path = temp.path();

    // 1. Create a note and sync it with mock embedder
    let project_dir = vault_path.join("10_Projects");
    fs::create_dir_all(&project_dir).expect("create project dir");
    fs::write(
        project_dir.join("Kernel.md"),
        "# Microkernel Architecture\nModular IPC and memory capability engine.\n",
    )
    .expect("write note");

    let mut sync_cmd = Command::cargo_bin("k0maru").expect("cargo bin");
    sync_cmd
        .arg("sync")
        .arg("--vault")
        .arg(vault_path)
        .arg("--vector")
        .arg("--embed-model")
        .arg("mock");
    sync_cmd.assert().success();

    // 2. Search using vector mode with mock embedder
    let mut search_cmd = Command::cargo_bin("k0maru").expect("cargo bin");
    search_cmd
        .arg("search")
        .arg("microkernel capability")
        .arg("--vault")
        .arg(vault_path)
        .arg("--mode")
        .arg("vector")
        .arg("--embed-model")
        .arg("mock")
        .arg("--json");

    let assert = search_cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let results: serde_json::Value = serde_json::from_str(&stdout).expect("valid json output");
    let hits = results.as_array().expect("hits array");
    assert!(!hits.is_empty(), "Should find at least 1 hit");
    assert!(hits[0]["path"].as_str().unwrap().contains("Kernel.md"));
}
