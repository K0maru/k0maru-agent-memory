//! Deterministic, zero-dependency mock embedding provider for tests and offline usage.

use crate::vector::engine::EmbeddingEngine;
use crate::vector::error::VectorError;
use xxhash_rust::xxh3::xxh3_64;

/// Default embedding dimension for the mock engine (aligned with sqlite-vec default).
pub const DEFAULT_DIMENSION: usize = 384;

/// SplitMix64 pseudo-random generator for fast, deterministic vector generation.
#[derive(Debug, Clone)]
struct SplitMix64(u64);

impl SplitMix64 {
    #[inline]
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    #[inline]
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }

    #[inline]
    fn next_f32(&mut self) -> f32 {
        let val = (self.next_u64() >> 40) as u32;
        ((val as f32) / 8388607.5) - 1.0
    }
}

/// Deterministic mock embedding engine that generates L2-normalized unit vectors.
///
/// Features:
/// - Deterministic: identical inputs always yield identical vectors.
/// - Semantic overlap: shared words and character patterns produce higher cosine similarity.
/// - Unit normalized: output vectors are strictly L2-normalized ($\sum v_i^2 \approx 1.0$).
/// - High performance: zero external dependencies, <1ms execution per batch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MockEmbeddingEngine {
    dimension: usize,
    model_name: String,
}

impl MockEmbeddingEngine {
    /// Default embedding dimension (384).
    pub const DEFAULT_DIMENSION: usize = DEFAULT_DIMENSION;

    /// Creates a new `MockEmbeddingEngine` with the specified output dimension.
    pub fn new(dimension: usize) -> Self {
        assert!(dimension > 0, "Embedding dimension must be greater than 0");
        Self {
            dimension,
            model_name: format!("mock-embedding-{}", dimension),
        }
    }

    /// Creates a new `MockEmbeddingEngine` with the specified dimension and custom model name.
    pub fn with_model_name(dimension: usize, model_name: impl Into<String>) -> Self {
        assert!(dimension > 0, "Embedding dimension must be greater than 0");
        Self {
            dimension,
            model_name: model_name.into(),
        }
    }

    /// Generates an L2-normalized deterministic vector for a single input text.
    pub fn embed_single(&self, text: &str) -> Vec<f32> {
        let mut vec = vec![0.0f32; self.dimension];

        let lower = text.to_lowercase();
        let words: Vec<&str> = lower
            .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
            .filter(|s| !s.is_empty())
            .collect();

        // 1. Word token projections
        for word in &words {
            let word_hash = xxh3_64(word.as_bytes());
            let mut rng = SplitMix64::new(word_hash);
            for x in vec.iter_mut() {
                *x += rng.next_f32();
            }
        }

        // 2. Character bigram projections for subword similarity (limited to first 64 words for performance)
        let words_for_bigrams = if words.len() > 64 {
            &words[..64]
        } else {
            &words[..]
        };

        for word in words_for_bigrams {
            for (c1, c2) in word.chars().zip(word.chars().skip(1)) {
                let mut buf = [0u8; 8];
                let len1 = c1.encode_utf8(&mut buf[0..4]).len();
                let len2 = c2.encode_utf8(&mut buf[len1..8]).len();
                let bi_hash = xxh3_64(&buf[..len1 + len2]);
                let mut rng = SplitMix64::new(bi_hash);
                for x in vec.iter_mut() {
                    *x += rng.next_f32() * 0.2;
                }
            }
        }

        // 3. Global text hash projection (provides non-zero baseline even for empty/whitespace input)
        let text_hash = xxh3_64(text.as_bytes());
        let mut rng = SplitMix64::new(text_hash);
        for x in vec.iter_mut() {
            *x += rng.next_f32() * 0.1;
        }

        // 4. L2 Normalization in double-precision to ensure unit length sum(v_i^2) == 1.0
        let norm_sq: f64 = vec.iter().map(|&x| (x as f64) * (x as f64)).sum();
        if norm_sq > 1e-18 {
            let inv_norm = (1.0 / norm_sq.sqrt()) as f32;
            for x in vec.iter_mut() {
                *x *= inv_norm;
            }
        } else {
            vec[0] = 1.0;
        }

        vec
    }
}

impl Default for MockEmbeddingEngine {
    fn default() -> Self {
        Self::new(Self::DEFAULT_DIMENSION)
    }
}

impl EmbeddingEngine for MockEmbeddingEngine {
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, VectorError> {
        let embeddings = texts.iter().map(|text| self.embed_single(text)).collect();
        Ok(embeddings)
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }
}
