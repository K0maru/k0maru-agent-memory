use std::fs;
use std::sync::Arc;
use tempfile::TempDir;

use serde_json::{json, Value};

use k0maru::adapters::ObsidianAdapter;
use k0maru::mcp::McpServer;
use k0maru::scanner::IncrementalScanner;
use k0maru::storage::SqliteStorage;
use k0maru::vector::MockEmbeddingEngine;

fn setup_hybrid_vault() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temporary directory for hybrid vault");
    let root = temp_dir.path();

    let cards_dir = root.join("20_Cards");
    let daily_dir = root.join("00_Daily");
    fs::create_dir_all(&cards_dir).expect("Failed to create 20_Cards directory");
    fs::create_dir_all(&daily_dir).expect("Failed to create 00_Daily directory");

    // Card 1: Rrf_Retrieval
    let card1 = r#"---
title: RRF Retrieval
hierarchy: L3Evergreen
tags:
  - search
  - rrf
---
# RRF Retrieval

Reciprocal Rank Fusion combines BM25 lexical keyword search and semantic vector embeddings.
Backlink target: [[Graph_Boost]].
"#;
    fs::write(cards_dir.join("Rrf_Retrieval.md"), card1).expect("write card1");

    // Card 2: Graph_Boost
    let card2 = r#"---
title: Graph Boost
hierarchy: L3Evergreen
tags:
  - search
  - graph
---
# Graph Boost

Graph boost elevates 1-hop connected notes in the hybrid retrieval pool.
Links to: [[Rrf_Retrieval]].
"#;
    fs::write(cards_dir.join("Graph_Boost.md"), card2).expect("write card2");

    // Daily Note linking Card 1
    let daily = r#"---
date: 2026-10-01
---
# 2026-10-01 Daily Log

Reviewed [[Rrf_Retrieval]] implementation for agent memory.
"#;
    fs::write(daily_dir.join("2026-10-01.md"), daily).expect("write daily");

    temp_dir
}

#[test]
fn test_mcp_hybrid_recall_with_vectors_via_handle_call_tool() {
    let vault = setup_hybrid_vault();
    let cache_dir = vault.path().join(".k0maru");
    let cache_path = cache_dir.join("cache.sqlite");

    let embedder = Arc::new(MockEmbeddingEngine::new(384));
    let adapter = ObsidianAdapter::new(vault.path());
    let mut storage = SqliteStorage::open(&cache_path).expect("open storage");

    // Sync vault documents and vectors
    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    let (stats, vec_stats) = scanner
        .sync_vault_with_vector(vault.path(), Some(embedder.clone()))
        .expect("sync with vector");

    assert_eq!(stats.added, 3);
    assert_eq!(vec_stats.embedded_count, 3);
    assert!(storage.has_vectors().expect("has_vectors"));

    // Instantiate MCP server
    let mut server = McpServer::new(vault.path()).with_embedder(embedder);

    // Call recall_memory via handle_call_tool
    let req_params = json!({
        "name": "recall_memory",
        "arguments": {
            "query": "Fusion",
            "limit": 5
        }
    });

    let resp = server.handle_call_tool(json!(1), &req_params);

    assert_eq!(resp["jsonrpc"], "2.0");
    assert_eq!(resp["id"], 1);
    assert_ne!(resp["result"]["isError"], true);

    let content_arr = resp["result"]["content"].as_array().expect("content array");
    assert_eq!(content_arr.len(), 1);
    let text = content_arr[0]["text"].as_str().expect("text string");

    // Must contain Title, Path, Score, Backlinks, and Snippet
    assert!(text.contains("RRF Retrieval"));
    assert!(text.contains("20_Cards/Rrf_Retrieval.md"));
    assert!(text.contains("Score: "));
    assert!(text.contains("Backlinks: "));
    assert!(text.contains("Reciprocal Rank Fusion"));
}

#[test]
fn test_mcp_hybrid_recall_via_handle_line_stdio() {
    let vault = setup_hybrid_vault();
    let cache_dir = vault.path().join(".k0maru");
    let cache_path = cache_dir.join("cache.sqlite");

    let embedder = Arc::new(MockEmbeddingEngine::new(384));
    let adapter = ObsidianAdapter::new(vault.path());
    let mut storage = SqliteStorage::open(&cache_path).expect("open storage");

    let mut scanner = IncrementalScanner::new(&adapter, &mut storage);
    scanner
        .sync_vault_with_vector(vault.path(), Some(embedder.clone()))
        .expect("sync with vector");

    let mut server = McpServer::new(vault.path()).with_embedder(embedder);

    let req_line = json!({
        "jsonrpc": "2.0",
        "id": 102,
        "method": "tools/call",
        "params": {
            "name": "recall_memory",
            "arguments": {
                "query": "Graph",
                "limit": 3
            }
        }
    })
    .to_string();

    let resp_str = server
        .handle_line(&req_line)
        .expect("should return response line");
    let resp: Value = serde_json::from_str(&resp_str).expect("valid json response");

    assert_eq!(resp["id"], 102);
    let text = resp["result"]["content"][0]["text"]
        .as_str()
        .expect("text content");

    assert!(text.contains("Graph Boost"));
    assert!(text.contains("Score: "));
    assert!(text.contains("Backlinks: "));
    assert!(text.contains("1-hop connected notes"));
}

#[test]
fn test_mcp_recall_memory_fallback_without_vectors() {
    let vault = setup_hybrid_vault();
    // Do NOT generate vectors, only standard cache sync
    let mut server = McpServer::new(vault.path());

    let req_params = json!({
        "name": "recall_memory",
        "arguments": {
            "query": "Fusion",
            "limit": 5
        }
    });

    let resp = server.handle_call_tool(json!(7), &req_params);

    assert_eq!(resp["id"], 7);
    assert_ne!(resp["result"]["isError"], true);

    let text = resp["result"]["content"][0]["text"]
        .as_str()
        .expect("text string");
    assert!(text.contains("RRF Retrieval"));
    assert!(text.contains("Score: "));
    assert!(text.contains("Backlinks: "));
}
