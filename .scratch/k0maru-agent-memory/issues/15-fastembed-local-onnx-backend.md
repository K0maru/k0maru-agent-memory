# 15 — FastEmbed Local CPU ONNX Runtime Backend

**Type:** task  
**Status:** ready-for-agent  
**Blocked by:** 14  

## Context
For real-world semantic retrieval, we need a local CPU-based embedding model that requires zero external API keys, zero cloud latency, and zero ongoing costs. We implement `FastEmbedBackend` using the `fastembed` crate (or an ONNX runtime seam), defaulting to `all-MiniLM-L6-v2` (384-dim) or `bge-small-zh-v1.5`.

## Objectives & Deliverables
1. Integrate `fastembed` crate in `Cargo.toml` with feature-gate `vector` (optional/default):
   ```toml
   [features]
   default = ["vector"]
   vector = ["dep:fastembed", "dep:sqlite-vec"]
   ```
2. Implement `FastEmbedBackend`:
   - Loads ONNX model lazily from a local cache directory (`~/.k0maru/models/` or `<vault>/.k0maru/models/`);
   - Implements `EmbeddingEngine` trait for batch generation;
   - Supports configurable model selection (`TextEmbedding::try_new(InitOptions { model_name, cache_dir, .. })`).
3. Handle failure modes gracefully:
   - If model files cannot be loaded, returns explicit `VectorError::ModelInitFailed`;
   - Ensures memory is freed when engine is dropped.
4. Unit and integration tests (mocked or conditional on model availability).

## Acceptance Criteria
- [ ] `FastEmbedBackend` compiles and implements `EmbeddingEngine`.
- [ ] Only loads weights when explicitly invoked (never on `offload` or `version`).
- [ ] Thread-safe across concurrent queries.
