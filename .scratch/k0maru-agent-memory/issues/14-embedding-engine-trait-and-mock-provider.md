# 14 — EmbeddingEngine Trait & Deterministic Mock Provider

**Type:** task  
**Status:** resolved  
**Blocked by:** None  

## Context
To decouple vector search from specific deep learning runtimes and maintain fast, isolated unit tests, we establish an `EmbeddingEngine` trait. This trait abstracts batch text embedding generation, dimension configuration, and error handling.

## Objectives & Deliverables
1. Define the `EmbeddingEngine` trait in `src/vector/mod.rs` or `src/vector/engine.rs`:
   ```rust
   pub trait EmbeddingEngine: Send + Sync {
       fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, VectorError>;
       fn dimension(&self) -> usize;
       fn model_name(&self) -> &str;
   }
   ```
2. Implement `MockEmbeddingEngine`:
   - Generates deterministic, normalized 384-dimensional unit vectors based on text hash / character frequencies;
   - Zero network dependencies, zero ONNX runtime overhead, completes in <1ms;
   - Ensures identical inputs produce identical vectors, and semantically similar synthetic strings have higher cosine similarity than disparate strings.
3. Define error types in `src/vector/error.rs` (`VectorError`).
4. Unit tests in `tests/test_embedding_engine.rs`:
   - Verifies normalization ($\sum v_i^2 \approx 1.0$);
   - Verifies dimension matching;
   - Verifies batch consistency.

## Acceptance Criteria
- [x] `EmbeddingEngine` trait is cleanly designed and object-safe (`Arc<dyn EmbeddingEngine>`).
- [x] `MockEmbeddingEngine` provides repeatable unit vectors without external dependencies.
- [x] 100% test coverage for trait and mock provider.
