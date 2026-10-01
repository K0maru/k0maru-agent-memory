use std::sync::Arc;
use std::time::Instant;

use k0maru::vector::{EmbeddingEngine, MockEmbeddingEngine, VectorError};

fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

#[test]
fn test_mock_embedding_engine_dimension() {
    let default_engine = MockEmbeddingEngine::default();
    assert_eq!(default_engine.dimension(), 384);
    assert_eq!(default_engine.model_name(), "mock-embedding-384");

    let custom_dim_engine = MockEmbeddingEngine::new(128);
    assert_eq!(custom_dim_engine.dimension(), 128);
    assert_eq!(custom_dim_engine.model_name(), "mock-embedding-128");

    let with_name = MockEmbeddingEngine::with_model_name(768, "custom-mock-model");
    assert_eq!(with_name.dimension(), 768);
    assert_eq!(with_name.model_name(), "custom-mock-model");

    let vec_default = default_engine.embed("test text").unwrap();
    assert_eq!(vec_default.len(), 384);

    let vec_custom = custom_dim_engine.embed("test text").unwrap();
    assert_eq!(vec_custom.len(), 128);

    let vec_768 = with_name.embed("test text").unwrap();
    assert_eq!(vec_768.len(), 768);
}

#[test]
fn test_mock_embedding_engine_l2_normalization() {
    let engine = MockEmbeddingEngine::default();
    let sample_inputs = [
        "",
        "hello world",
        "12345 !@#$%^&*()_+",
        "Obsidian and LLM-Wiki cleanroom memory hub with zero-daemon",
        "这是一个包含中文字符和特殊标点的测试文本：嵌入向量必须严格进行 L2 归一化处理。",
        "The quick brown fox jumps over the lazy dog. A quick movement in the forest.",
        "a",
        "   \t\n   ",
    ];

    for input in sample_inputs {
        let vec = engine.embed(input).expect("Embedding should succeed");
        assert_eq!(vec.len(), engine.dimension());

        let norm_sq: f32 = vec.iter().map(|v| v * v).sum();
        let diff = (norm_sq - 1.0).abs();
        assert!(
            diff < 1e-4,
            "Input {:?} produced vector with L2 norm squared {}, diff {} exceeds tolerance",
            input,
            norm_sq,
            diff
        );
    }
}

#[test]
fn test_mock_embedding_engine_determinism() {
    let engine1 = MockEmbeddingEngine::default();
    let engine2 = MockEmbeddingEngine::default();

    let text_a = "Deterministic embedding test across multiple instances and runs";
    let text_b = "Another different string for distinct vector verification";

    let v1 = engine1.embed(text_a).unwrap();
    let v2 = engine1.embed(text_a).unwrap();
    let v3 = engine2.embed(text_a).unwrap();

    assert_eq!(v1, v2, "Same instance must return identical vector");
    assert_eq!(v1, v3, "Different instance must return identical vector");

    let v_b = engine1.embed(text_b).unwrap();
    assert_ne!(v1, v_b, "Distinct texts should produce distinct vectors");
}

#[test]
fn test_mock_embedding_engine_batch_consistency() {
    let engine = MockEmbeddingEngine::default();
    let texts = vec![
        "First batch item",
        "Second batch item",
        "Third item with some overlap",
        "Fourth completely random sentence",
    ];

    let batch_results = engine.embed_batch(&texts).unwrap();
    assert_eq!(batch_results.len(), texts.len());

    for (idx, text) in texts.iter().enumerate() {
        let single_result = engine.embed(text).unwrap();
        assert_eq!(
            batch_results[idx], single_result,
            "Batch item at {} must match single embed result",
            idx
        );
    }

    // Empty batch test
    let empty_batch = engine.embed_batch(&[]).unwrap();
    assert!(empty_batch.is_empty());
}

#[test]
fn test_mock_embedding_engine_semantic_similarity() {
    let engine = MockEmbeddingEngine::default();

    let text_a = "k0maru agent memory hub for obsidian second brain";
    let text_b = "k0maru agent memory system for obsidian notes";
    let text_c = "quantum chromodynamics and non-abelian gauge field theory";

    let vec_a = engine.embed(text_a).unwrap();
    let vec_b = engine.embed(text_b).unwrap();
    let vec_c = engine.embed(text_c).unwrap();

    // Since vectors are unit normalized, cosine similarity is simply the dot product
    let sim_ab = dot_product(&vec_a, &vec_b);
    let sim_ac = dot_product(&vec_a, &vec_c);

    println!("sim(A, B) = {}, sim(A, C) = {}", sim_ab, sim_ac);

    assert!(
        sim_ab > sim_ac,
        "Similar texts (sim={}) must have higher similarity than disparate texts (sim={})",
        sim_ab,
        sim_ac
    );
    assert!(
        sim_ab > 0.4,
        "Similar texts should have substantial cosine similarity (> 0.4), got {}",
        sim_ab
    );
    assert!(
        sim_ac < 0.25,
        "Disparate texts should have low cosine similarity (< 0.25), got {}",
        sim_ac
    );
}

#[test]
fn test_embedding_engine_object_safety_and_dyn() {
    let engine: Arc<dyn EmbeddingEngine> = Arc::new(MockEmbeddingEngine::default());

    assert_eq!(engine.dimension(), 384);
    assert_eq!(engine.model_name(), "mock-embedding-384");

    let texts = ["dyn test item 1", "dyn test item 2"];
    let batch = engine.embed_batch(&texts).unwrap();
    assert_eq!(batch.len(), 2);
    assert_eq!(batch[0].len(), 384);

    let single = engine.embed("dyn test single").unwrap();
    assert_eq!(single.len(), 384);
}

#[test]
fn test_mock_embedding_engine_performance() {
    let engine = MockEmbeddingEngine::default();
    let samples: Vec<String> = (0..100)
        .map(|i| {
            format!(
                "Document chunk {} describing system architecture and indexing pipelines",
                i
            )
        })
        .collect();
    let sample_refs: Vec<&str> = samples.iter().map(|s| s.as_str()).collect();

    let start = Instant::now();
    let results = engine.embed_batch(&sample_refs).unwrap();
    let elapsed = start.elapsed();

    assert_eq!(results.len(), 100);
    println!("Embedding 100 texts took {:?}", elapsed);

    // Each text must average far below 1ms (< 1000us)
    let avg_per_text = elapsed / 100;
    assert!(
        avg_per_text.as_micros() < 500,
        "Average per text must be < 500us, took {:?}",
        avg_per_text
    );
}

#[test]
fn test_vector_error_display_and_traits() {
    let err1 = VectorError::DimensionMismatch {
        expected: 384,
        actual: 128,
    };
    let err2 = VectorError::ModelNotFound("bge-small-en".to_string());
    let err3 = VectorError::InferenceError("ONNX session failure".to_string());
    let err4 = VectorError::Internal("Allocation failure".to_string());

    assert_eq!(
        err1.to_string(),
        "Vector dimension mismatch: expected 384, got 128"
    );
    assert_eq!(err2.to_string(), "Model not found: bge-small-en");
    assert_eq!(
        err3.to_string(),
        "Vector inference error: ONNX session failure"
    );
    assert_eq!(
        err4.to_string(),
        "Internal vector error: Allocation failure"
    );

    // Test clone and equality
    assert_eq!(err1.clone(), err1);
    assert_ne!(err1, err2);

    // Verify std::error::Error implementation
    let boxed: Box<dyn std::error::Error> = Box::new(err1);
    assert!(boxed.to_string().contains("dimension mismatch"));
}
