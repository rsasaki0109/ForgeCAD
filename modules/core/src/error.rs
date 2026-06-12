use std::fmt;

use thiserror::Error;

/// Primary error type for OpenCAD operations.
#[derive(Debug, Error)]
pub enum OpenCadError {
    #[error("invalid id: {0}")]
    InvalidId(String),

    #[error("invalid unit: {0}")]
    InvalidUnit(String),

    #[error("invalid expression: {0}")]
    InvalidExpression(String),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("validation failed: {0}")]
    Validation(String),

    #[error("transaction error: {0}")]
    Transaction(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("{0}")]
    Other(String),
}

/// Convenience result alias used across OpenCAD crates.
pub type Result<T> = std::result::Result<T, OpenCadError>;

impl OpenCadError {
    pub fn validation(message: impl fmt::Display) -> Self {
        Self::Validation(message.to_string())
    }

    pub fn transaction(message: impl fmt::Display) -> Self {
        Self::Transaction(message.to_string())
    }

    pub fn not_found(message: impl fmt::Display) -> Self {
        Self::NotFound(message.to_string())
    }
}
