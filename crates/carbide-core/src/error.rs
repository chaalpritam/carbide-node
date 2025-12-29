//! Error types for the Carbide Network

use thiserror::Error;

/// Main error type for Carbide operations
#[derive(Error, Debug)]
pub enum CarbideError {
    /// IO errors (file operations, network)
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Cryptographic errors
    #[error("Crypto error: {0}")]
    Crypto(String),

    /// Network/provider errors
    #[error("Provider error: {0}")]
    Provider(String),

    /// Discovery service errors
    #[error("Discovery error: {0}")]
    Discovery(String),

    /// Reputation system errors
    #[error("Reputation error: {0}")]
    Reputation(String),

    /// Resource not found
    #[error("Not found: {0}")]
    NotFound(String),

    /// Generic internal errors
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type alias for convenience
pub type Result<T> = std::result::Result<T, CarbideError>;
