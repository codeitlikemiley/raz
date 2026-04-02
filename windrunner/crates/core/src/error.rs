use std::io;

/// Errors that can occur during cargo-runner operations
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("IO error: {0}")]
    IoError(#[from] io::Error),

    #[error("Tree-sitter error: {0}")]
    TreeSitterError(String),

    #[error("Pattern detection error: {0}")]
    PatternError(String),

    #[error("Cache error: {0}")]
    CacheError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Module resolution error: {0}")]
    ModuleError(String),

    #[error("No runnable found at the specified location")]
    NoRunnableFound,

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("{0}")]
    Other(String),
}

/// Result type alias for cargo-runner operations
pub type Result<T> = std::result::Result<T, Error>;
