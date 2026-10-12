use assert_cmd::Command;
use serde_json::json;
use std::fs;
use std::sync::Mutex;
use tempfile::tempdir;

use k0maru::mcp::McpServer;
use k0maru::storage::SqliteStorage;

static TEST_MUTEX: Mutex<()> = Mutex::new(());

#[test]
fn test_e2e_model_switching_and_dynamic_migration() {
    let _guard = TEST_MUTEX.lock().unwrap();
    std::env::set_var("K0MARU_FORCE_MOCK_EMBED", "1");

    let temp = tempdir().expect("tempdir");
    let vault_path = temp.path();

    // 1. Create initial test vault notes
    let proj_dir = vault_path.join("10_Projects");
    let card_dir = vault_path.join("20_Cards");
    fs::create_dir_all(&proj_dir).unwrap();
    fs::create_dir_all(&card_dir).unwrap();

    fs::write(
        proj_dir.join("DistributedDb.md"),
        "# 分布式数据库架构\n基于 Paxos 算法与多副本状态机复制的高可用架构。\n",
    )
    .unwrap();
    fs::write(
        card_dir.join("Raft.md"),
        "# Raft Consensus\nLeader election and log replication algorithm.\n",
    )
    .unwrap();

    // 2. Initial sync with 384d mock embedder
    let mut cmd_sync = Command::cargo_bin("k0maru").unwrap();
    cmd_sync
        .arg("sync")
        .arg("--vault")
        .arg(vault_path)
        .arg("--vector")
        .arg("--embed-model")
        .arg("mock")
        .arg("--json");
    let assert = cmd_sync.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let json_val: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json_val["vector"]["embedded_count"], 2);

    let cache_file = vault_path.join(".k0maru").join("cache.sqlite");
    {
        let storage = SqliteStorage::open(&cache_file).unwrap();
        assert_eq!(storage.get_vector_dimension().unwrap(), Some(384));
        assert_eq!(storage.vector_count().unwrap(), 2);
    }

    // 3. Switch to 512d Chinese-optimized model (bge-small-zh-v1.5)
    let mut cmd_migrate = Command::cargo_bin("k0maru").unwrap();
    cmd_migrate
        .arg("sync")
        .arg("--vault")
        .arg(vault_path)
        .arg("--vector")
        .arg("--embed-model")
        .arg("bge-small-zh-v1.5")
        .arg("--json");
    let assert_migrate = cmd_migrate.assert().success();
    let stdout_migrate = String::from_utf8(assert_migrate.get_output().stdout.clone()).unwrap();
    let json_val_migrate: serde_json::Value = serde_json::from_str(&stdout_migrate).unwrap();
    assert_eq!(json_val_migrate["vector"]["embedded_count"], 2);

    // 4. Verify SQLite storage was safely migrated to 512d
    {
        let storage = SqliteStorage::open(&cache_file).unwrap();
        assert_eq!(storage.get_vector_dimension().unwrap(), Some(512));
        assert_eq!(storage.vector_count().unwrap(), 2);
    }

    // 5. Search using 512d model
    let mut cmd_search = Command::cargo_bin("k0maru").unwrap();
    cmd_search
        .arg("search")
        .arg("分布式数据库")
        .arg("--vault")
        .arg(vault_path)
        .arg("--mode")
        .arg("vector")
        .arg("--embed-model")
        .arg("bge-small-zh-v1.5")
        .arg("--json");
    let assert_search = cmd_search.assert().success();
    let stdout_search = String::from_utf8(assert_search.get_output().stdout.clone()).unwrap();
    let results: serde_json::Value = serde_json::from_str(&stdout_search).unwrap();
    let hits = results.as_array().unwrap();
    assert!(!hits.is_empty());
    assert!(hits[0]["path"]
        .as_str()
        .unwrap()
        .contains("DistributedDb.md"));

    std::env::remove_var("K0MARU_FORCE_MOCK_EMBED");
}

#[test]
fn test_e2e_fastmcp_recall_memory_with_model_migration() {
    let _guard = TEST_MUTEX.lock().unwrap();
    std::env::set_var("K0MARU_FORCE_MOCK_EMBED", "1");

    let temp = tempdir().expect("tempdir");
    let vault_path = temp.path();

    let proj_dir = vault_path.join("10_Projects");
    fs::create_dir_all(&proj_dir).unwrap();
    fs::write(
        proj_dir.join("ZeroCopy.md"),
        "# Zero-Copy Stream Engine\nHigh throughput network ring buffer and DMA buffers.\n",
    )
    .unwrap();

    // 1. Initial sync with 384d model
    let mut cmd_sync = Command::cargo_bin("k0maru").unwrap();
    cmd_sync
        .arg("sync")
        .arg("--vault")
        .arg(vault_path)
        .arg("--vector")
        .arg("--embed-model")
        .arg("mock");
    cmd_sync.assert().success();

    let cache_file = vault_path.join(".k0maru").join("cache.sqlite");
    {
        let storage = SqliteStorage::open(&cache_file).unwrap();
        assert_eq!(storage.get_vector_dimension().unwrap(), Some(384));
    }

    // 2. Run FastMCP with 768d model (bge-base-en)
    let mut server = McpServer::new(vault_path).with_embed_model(Some("bge-base-en"));

    let rpc_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "recall_memory",
            "arguments": {
                "query": "Zero-Copy DMA ring buffer"
            }
        }
    });

    let raw_req = serde_json::to_string(&rpc_request).unwrap();
    let resp = server.handle_request(&raw_req);
    assert!(resp.is_some());

    let resp_val: serde_json::Value = serde_json::from_str(&resp.unwrap()).unwrap();
    assert!(resp_val.get("result").is_some());
    let content = resp_val["result"]["content"][0]["text"].as_str().unwrap();
    assert!(
        content.contains("ZeroCopy.md") || content.contains("Zero-Copy Stream Engine"),
        "FastMCP response should find ZeroCopy note: {}",
        content
    );

    // Verify storage was migrated to 768d
    {
        let storage = SqliteStorage::open(&cache_file).unwrap();
        assert_eq!(storage.get_vector_dimension().unwrap(), Some(768));
    }

    std::env::remove_var("K0MARU_FORCE_MOCK_EMBED");
}

#[test]
fn test_e2e_invalid_model_error_handling() {
    let _guard = TEST_MUTEX.lock().unwrap();

    let temp = tempdir().expect("tempdir");
    let vault_path = temp.path();

    let mut cmd = Command::cargo_bin("k0maru").unwrap();
    cmd.arg("sync")
        .arg("--vault")
        .arg(vault_path)
        .arg("--vector")
        .arg("--embed-model")
        .arg("invalid-transformer-9000");

    let assert = cmd.assert().failure();
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains("Unknown embedding model") || stderr.contains("Supported models"),
        "Stderr should report unknown model with suggestions: {}",
        stderr
    );
}
