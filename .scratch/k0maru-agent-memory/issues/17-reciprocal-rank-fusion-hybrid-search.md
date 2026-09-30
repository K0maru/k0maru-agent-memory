# 17 — Reciprocal Rank Fusion (RRF) Hybrid Search Engine

**Type:** task  
**Status:** ready-for-agent  
**Blocked by:** 13, 14  

## Context
Pure keyword search (BM25) fails on synonyms, while pure vector search is vulnerable to semantic drift and loses exact keyword precision. Hybrid search combining BM25, Cosine Vector similarity, and WikiLinks graph adjacency provides the highest retrieval fidelity.

## Objectives & Deliverables
1. Implement `HybridSearchEngine` in `src/storage/hybrid.rs`:
   - Executes BM25 query on FTS5 (`SqliteStorage::search_fts`);
   - Executes vector nearest-neighbor search (`SqliteStorage::search_vectors`);
   - Combines results using Reciprocal Rank Fusion (RRF):
     $$\text{RRF}(d) = \frac{w_{\text{bm25}}}{k + \text{rank}_{\text{bm25}}(d)} + \frac{w_{\text{vec}}}{k + \text{rank}_{\text{vec}}(d)}$$
     where $k = 60$, $w_{\text{bm25}} = 0.5$, $w_{\text{vec}} = 0.5$.
2. Implement Graph Boost:
   - Queries 1-hop links from `SqliteStorage::get_backlinks` and forward links;
   - If document is linked to/from a top-ranked result, add a graph boost (+0.05).
3. Support Search Modes:
   - `SearchMode::Hybrid` (default when vectors present);
   - `SearchMode::BM25` (lexical only);
   - `SearchMode::Vector` (semantic only).
4. Unit tests in `tests/test_hybrid_search.rs`:
   - Test RRF score calculation with tie-breaking;
   - Test cases where BM25 has exact match but vector is low;
   - Test cases where vector captures synonym that BM25 misses;
   - Test graph boost elevating connected notes.

## Acceptance Criteria
- [ ] RRF produces deterministic, normalized ranked list.
- [ ] Graph boost rewards densely connected Wiki nodes.
- [ ] Fallback to BM25 works seamlessly if vectors are disabled.
