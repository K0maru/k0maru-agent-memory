//! Embedding model catalog and specification types.

use std::fmt;
use std::str::FromStr;

use crate::vector::error::VectorError;

/// Supported embedding model architectures and variants.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
pub enum EmbeddingModelType {
    /// sentence-transformers/all-MiniLM-L6-v2 (384 dimensions, default general-purpose English/code)
    #[default]
    AllMiniLML6V2,
    /// BAAI/bge-small-en-v1.5 (384 dimensions, high-accuracy English retrieval)
    BgeSmallEnV15,
    /// BAAI/bge-small-zh-v1.5 (512 dimensions, Chinese-optimized semantic search)
    BgeSmallZhV15,
    /// BAAI/bge-base-en-v1.5 (768 dimensions, deep semantic retrieval)
    BgeBaseEnV15,
    /// sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2 (384 dimensions, 50+ languages)
    ParaphraseMlMiniLmL12V2,
    /// Deterministic offline mock engine (384 dimensions by default)
    Mock,
}

impl EmbeddingModelType {
    /// Returns the vector dimension produced by this model variant.
    pub fn dimension(&self) -> usize {
        match self {
            Self::AllMiniLML6V2 => 384,
            Self::BgeSmallEnV15 => 384,
            Self::BgeSmallZhV15 => 512,
            Self::BgeBaseEnV15 => 768,
            Self::ParaphraseMlMiniLmL12V2 => 384,
            Self::Mock => 384,
        }
    }

    /// Returns the canonical CLI/configuration string name for this model.
    pub fn canonical_name(&self) -> &'static str {
        match self {
            Self::AllMiniLML6V2 => "all-minilm-l6-v2",
            Self::BgeSmallEnV15 => "bge-small-en-v1.5",
            Self::BgeSmallZhV15 => "bge-small-zh-v1.5",
            Self::BgeBaseEnV15 => "bge-base-en-v1.5",
            Self::ParaphraseMlMiniLmL12V2 => "paraphrase-multilingual-minilm-l12-v2",
            Self::Mock => "mock",
        }
    }

    /// Returns a list of all supported canonical model names.
    pub fn all_canonical_names() -> &'static [&'static str] {
        &[
            "all-minilm-l6-v2",
            "bge-small-en-v1.5",
            "bge-small-zh-v1.5",
            "bge-base-en-v1.5",
            "paraphrase-multilingual-minilm-l12-v2",
            "mock",
        ]
    }

    /// Converts this model type to a corresponding [`fastembed::EmbeddingModel`] if applicable.
    #[cfg(feature = "fastembed")]
    pub fn to_fastembed_model(&self) -> Option<fastembed::EmbeddingModel> {
        match self {
            Self::AllMiniLML6V2 => Some(fastembed::EmbeddingModel::AllMiniLML6V2),
            Self::BgeSmallEnV15 => Some(fastembed::EmbeddingModel::BGESmallENV15),
            Self::BgeSmallZhV15 => Some(fastembed::EmbeddingModel::BGESmallZHV15),
            Self::BgeBaseEnV15 => Some(fastembed::EmbeddingModel::BGEBaseENV15),
            Self::ParaphraseMlMiniLmL12V2 => {
                Some(fastembed::EmbeddingModel::ParaphraseMLMiniLML12V2)
            }
            Self::Mock => None,
        }
    }

    /// Parses a string into an [`EmbeddingModelType`], resolving aliases and case insensitivity.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Result<Self, VectorError> {
        <Self as FromStr>::from_str(s)
    }
}

impl fmt::Display for EmbeddingModelType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.canonical_name())
    }
}

impl FromStr for EmbeddingModelType {
    type Err = VectorError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.trim().to_lowercase().replace('_', "-");
        match normalized.as_str() {
            "all-minilm-l6-v2" | "all-minilm" | "minilm-l6-v2" | "minilm-l6" | "minilm" => {
                Ok(Self::AllMiniLML6V2)
            }
            "bge-small-en-v1.5" | "bge-small-en-v15" | "bge-small-en" | "bge-en" => {
                Ok(Self::BgeSmallEnV15)
            }
            "bge-small-zh-v1.5" | "bge-small-zh-v15" | "bge-small-zh" | "bge-zh" => {
                Ok(Self::BgeSmallZhV15)
            }
            "bge-base-en-v1.5" | "bge-base-en-v15" | "bge-base-en" => Ok(Self::BgeBaseEnV15),
            "paraphrase-multilingual-minilm-l12-v2"
            | "paraphrase-multilingual"
            | "paraphrase-minilm"
            | "multilingual-minilm" => Ok(Self::ParaphraseMlMiniLmL12V2),
            "mock" | "mock-embedding" => Ok(Self::Mock),
            _ => Err(VectorError::ModelNotFound(format!(
                "Unknown embedding model '{}'. Supported models: {}",
                s,
                Self::all_canonical_names().join(", ")
            ))),
        }
    }
}
