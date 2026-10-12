//! Vector embeddings and similarity engine module.

pub mod engine;
pub mod error;
#[cfg(feature = "fastembed")]
pub mod fastembed;
pub mod mock;
pub mod models;

pub use engine::EmbeddingEngine;
pub use error::VectorError;
#[cfg(feature = "fastembed")]
pub use fastembed::{resolve_cache_dir, FastEmbedBackend};
pub use mock::MockEmbeddingEngine;
pub use models::EmbeddingModelType;

use std::path::PathBuf;
use std::sync::Arc;

/// Resolves and instantiates an embedding engine based on the model name and cache directory.
///
/// Precedence hierarchy:
/// 1. Explicit `model_str` argument (e.g. "bge-small-zh-v1.5", "all-minilm-l6-v2", "mock");
/// 2. `K0MARU_EMBED_MODEL` environment variable;
/// 3. Default to `all-minilm-l6-v2`.
///
/// If `K0MARU_FORCE_MOCK_EMBED` is set or if the resolved model is `mock`,
/// a [`MockEmbeddingEngine`] configured with the target model's dimension and canonical name is returned.
///
/// If `fastembed` is enabled and initialization fails (e.g. offline and weights are uncached),
/// it issues a warning and gracefully falls back to [`MockEmbeddingEngine`].
pub fn resolve_embedding_engine(
    model_str: Option<&str>,
    cache_dir: Option<PathBuf>,
) -> Result<Arc<dyn EmbeddingEngine>, VectorError> {
    let model_type = if let Some(s) = model_str.filter(|s| !s.trim().is_empty()) {
        EmbeddingModelType::from_str(s)?
    } else if let Ok(env_val) = std::env::var("K0MARU_EMBED_MODEL") {
        if !env_val.trim().is_empty() {
            EmbeddingModelType::from_str(&env_val)?
        } else {
            EmbeddingModelType::default()
        }
    } else {
        EmbeddingModelType::default()
    };

    if std::env::var("K0MARU_FORCE_MOCK_EMBED").is_ok() || model_type == EmbeddingModelType::Mock {
        return Ok(Arc::new(MockEmbeddingEngine::new(model_type.dimension())));
    }

    #[cfg(feature = "fastembed")]
    {
        match FastEmbedBackend::with_model_type(model_type, cache_dir) {
            Ok(backend) => Ok(Arc::new(backend)),
            Err(e) => {
                eprintln!(
                    "⚠️  Warning: FastEmbed initialization for '{}' failed ({}). Falling back to MockEmbeddingEngine.",
                    model_type.canonical_name(),
                    e
                );
                Ok(Arc::new(MockEmbeddingEngine::new(model_type.dimension())))
            }
        }
    }

    #[cfg(not(feature = "fastembed"))]
    {
        let _ = cache_dir;
        Ok(Arc::new(MockEmbeddingEngine::new(model_type.dimension())))
    }
}

/// Creates the default embedding engine for vector indexing and semantic retrieval.
pub fn default_embedding_engine(
    cache_dir: Option<PathBuf>,
) -> Result<Arc<dyn EmbeddingEngine>, VectorError> {
    resolve_embedding_engine(None, cache_dir)
}
