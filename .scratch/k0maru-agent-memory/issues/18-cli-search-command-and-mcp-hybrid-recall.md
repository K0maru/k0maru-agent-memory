# 18 — CLI Search Command & FastMCP Hybrid Memory Recall

**Type:** task  
**Status:** resolved  
**Blocked by:** 16, 17  

## Context
Surface the hybrid retrieval capabilities to human developers via a new CLI `search` command and to AI coding agents via the FastMCP `recall_memory` tool.

## Objectives & Deliverables
1. Add `k0maru search` subcommand in `src/main.rs`:
   ```bash
   k0maru search "<query>" --vault <path> [--mode hybrid|bm25|vector] [--limit 5] [--json]
   ```
   - Terminal output: Clean formatted list showing Document ID, Title, RRF/BM25 Score, and matched snippet/summary;
   - `--json` mode: Outputs full machine-readable JSON array of search hits.
2. Update `k0maru sync`:
   - Add `--vector` flag: `k0maru sync --vault <path> --vector`.
3. Update FastMCP `recall_memory` in `src/mcp/server.rs`:
   - Checks if vector table has entries:
     - If yes, executes `HybridSearchEngine::search(query, SearchMode::Hybrid)`;
     - If no, executes standard `SqliteStorage::search_fts`.
   - Returns structured snippets with backlinks and relevance scores.
4. End-to-end integration tests in `tests/test_cli_search_smoke.rs` and `tests/test_mcp_hybrid.rs`.

## Acceptance Criteria
- [x] `k0maru search "..."` returns expected results on both human-readable and `--json` formats.
- [x] FastMCP `recall_memory` transparently utilizes hybrid retrieval when available.
- [x] Deterministic exit codes and zero panics on empty or syntax-heavy search queries.
