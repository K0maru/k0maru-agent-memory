//! Error types for vector operations and embedding engines.

use std::fmt;

/// Errors that can occur during embedding generation or vector operations.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum VectorError {
    /// Dimension mismatch between expected model dimensions and actual vector size.
    DimensionMismatch { expected: usize, actual: usize },
    /// Requested embedding model could not be found or loaded.
    ModelNotFound(String),
    /// Inference failure during model execution.
    InferenceError(String),
    /// Internal engine failure.
    Internal(String),
}

impl fmt::Display for VectorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DimensionMismatch { expected, actual } => {
                write!(
                    f,
                    "Vector dimension mismatch: expected {}, got {}",
                    expected, actual
                )
            }
            Self::ModelNotFound(msg) => write!(f, "Model not found: {}", msg),
            Self::InferenceError(msg) => write!(f, "Vector inference error: {}", msg),
            Self::Internal(msg) => write!(f, "Internal vector error: {}", msg),
        }
    }
}

impl std::error::Error for VectorError {}
