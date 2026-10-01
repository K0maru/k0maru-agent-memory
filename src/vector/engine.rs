//! Embedding engine trait definition.

use crate::vector::error::VectorError;

/// Core abstraction for generating text vector embeddings.
///
/// Implementations must be thread-safe (`Send + Sync`) and object-safe
/// to enable dynamic dispatch (`Arc<dyn EmbeddingEngine>`).
pub trait EmbeddingEngine: Send + Sync {
    /// Generates vector embeddings for a batch of text slices.
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, VectorError>;

    /// Returns the embedding dimension produced by this engine.
    fn dimension(&self) -> usize;

    /// Returns the identifier / name of the embedding model.
    fn model_name(&self) -> &str;

    /// Convenience method to generate an embedding for a single text slice.
    fn embed(&self, text: &str) -> Result<Vec<f32>, VectorError> {
        let mut results = self.embed_batch(&[text])?;
        results
            .pop()
            .ok_or_else(|| VectorError::Internal("Empty embedding result from batch".to_string()))
    }
}
