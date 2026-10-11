//! Local ONNX-based embedding engine using FastEmbed.

use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

use crate::vector::engine::EmbeddingEngine;
use crate::vector::error::VectorError;

/// Resolves the default cache directory for embedding models.
///
/// Precedence:
/// 1. Explicit path if provided;
/// 2. `~/.k0maru/models/` if `$HOME` or `$USERPROFILE` is set;
/// 3. `.k0maru/models/` in the current working directory.
pub fn resolve_cache_dir(explicit: Option<PathBuf>) -> PathBuf {
    if let Some(p) = explicit {
        return p;
    }
    if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
        PathBuf::from(home).join(".k0maru").join("models")
    } else {
        PathBuf::from(".k0maru").join("models")
    }
}

/// Local CPU-based ONNX embedding backend using FastEmbed.
///
/// Thread-safe (`Send + Sync`) and object-safe for dynamic dispatch (`Arc<dyn EmbeddingEngine>`).
#[derive(Clone)]
pub struct FastEmbedBackend {
    model: Arc<Mutex<TextEmbedding>>,
    model_name: String,
    dimension: usize,
}

impl FastEmbedBackend {
    /// Creates a new `FastEmbedBackend` with default model (`all-MiniLM-L6-v2`, 384 dimensions).
    ///
    /// If `cache_dir` is `None`, defaults to `~/.k0maru/models/`.
    pub fn new(cache_dir: Option<PathBuf>) -> Result<Self, VectorError> {
        Self::with_model(EmbeddingModel::AllMiniLML6V2, cache_dir)
    }

    /// Creates a new `FastEmbedBackend` with a specific [`EmbeddingModel`].
    pub fn with_model(
        model: EmbeddingModel,
        cache_dir: Option<PathBuf>,
    ) -> Result<Self, VectorError> {
        let (dim, name) = match TextEmbedding::get_model_info(&model) {
            Ok(info) => {
                let name = if model == EmbeddingModel::AllMiniLML6V2 {
                    "all-MiniLM-L6-v2".to_string()
                } else {
                    info.model_code.clone()
                };
                (info.dim, name)
            }
            Err(_) => (384, "all-MiniLM-L6-v2".to_string()),
        };

        let cache_path = resolve_cache_dir(cache_dir);
        if let Err(e) = std::fs::create_dir_all(&cache_path) {
            return Err(VectorError::ModelInitFailed(format!(
                "Failed to create model cache directory '{}': {}",
                cache_path.display(),
                e
            )));
        }

        let options = InitOptions::new(model)
            .with_cache_dir(cache_path)
            .with_show_download_progress(false);

        let text_embedding = TextEmbedding::try_new(options).map_err(|e| {
            VectorError::ModelInitFailed(format!("FastEmbed model initialization failed: {}", e))
        })?;

        Ok(Self {
            model: Arc::new(Mutex::new(text_embedding)),
            model_name: name,
            dimension: dim,
        })
    }

    /// Creates a `FastEmbedBackend` wrapping an existing [`TextEmbedding`] instance.
    pub fn from_embedding(
        text_embedding: TextEmbedding,
        model_name: Option<String>,
        dimension: Option<usize>,
    ) -> Self {
        Self {
            model: Arc::new(Mutex::new(text_embedding)),
            model_name: model_name.unwrap_or_else(|| "all-MiniLM-L6-v2".to_string()),
            dimension: dimension.unwrap_or(384),
        }
    }

    /// Creates a `FastEmbedBackend` wrapping an existing `Arc<Mutex<TextEmbedding>>`.
    pub fn from_arc(
        model: Arc<Mutex<TextEmbedding>>,
        model_name: String,
        dimension: usize,
    ) -> Self {
        Self {
            model,
            model_name,
            dimension,
        }
    }
}

impl fmt::Debug for FastEmbedBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FastEmbedBackend")
            .field("model_name", &self.model_name)
            .field("dimension", &self.dimension)
            .finish()
    }
}

impl EmbeddingEngine for FastEmbedBackend {
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, VectorError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let guard = self.model.lock().map_err(|e| {
            VectorError::Internal(format!("Failed to acquire fastembed model lock: {}", e))
        })?;

        let texts_vec: Vec<&str> = texts.to_vec();
        let embeddings = guard.embed(texts_vec, None).map_err(|e| {
            VectorError::InferenceError(format!("FastEmbed inference failed: {}", e))
        })?;

        Ok(embeddings)
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }
}
