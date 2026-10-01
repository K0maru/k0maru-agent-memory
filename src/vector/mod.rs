//! Vector embeddings and similarity engine module.

pub mod engine;
pub mod error;
#[cfg(feature = "fastembed")]
pub mod fastembed;
pub mod mock;

pub use engine::EmbeddingEngine;
pub use error::VectorError;
#[cfg(feature = "fastembed")]
pub use fastembed::{resolve_cache_dir, FastEmbedBackend};
pub use mock::MockEmbeddingEngine;

use std::path::PathBuf;
use std::sync::Arc;

/// Creates the default embedding engine for vector indexing and semantic retrieval.
///
/// If the `fastembed` feature is enabled, this function attempts to initialize
/// the local ONNX-based `FastEmbedBackend` with the specified `cache_dir`.
/// If the model cannot be downloaded or initialized (e.g. offline without cached weights),
/// it issues a warning and gracefully falls back to `MockEmbeddingEngine`.
///
/// If the `fastembed` feature is disabled, it returns `MockEmbeddingEngine`.
pub fn default_embedding_engine(
    cache_dir: Option<PathBuf>,
) -> Result<Arc<dyn EmbeddingEngine>, VectorError> {
    if std::env::var("K0MARU_FORCE_MOCK_EMBED").is_ok() {
        return Ok(Arc::new(MockEmbeddingEngine::new(384)));
    }

    #[cfg(feature = "fastembed")]
    {
        match FastEmbedBackend::new(cache_dir) {
            Ok(backend) => Ok(Arc::new(backend)),
            Err(e) => {
                eprintln!(
                    "⚠️  Warning: FastEmbed initialization failed ({}). Falling back to MockEmbeddingEngine.",
                    e
                );
                Ok(Arc::new(MockEmbeddingEngine::new(384)))
            }
        }
    }

    #[cfg(not(feature = "fastembed"))]
    {
        let _ = cache_dir;
        Ok(Arc::new(MockEmbeddingEngine::new(384)))
    }
}
