use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OverrideError {
    #[error("Failed to parse source file: {0}")]
    ParseError(String),

    #[error("Function not found at position {line}:{column} in {file}")]
    FunctionNotFound {
        file: PathBuf,
        line: usize,
        column: usize,
    },

    #[error("Invalid override key: {0}")]
    InvalidKey(String),

    #[error("Override not found for key: {0}")]
    OverrideNotFound(String),

    #[error("Failed to read file: {0}")]
    FileReadError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] toml::ser::Error),

    #[error("Deserialization error: {0}")]
    DeserializationError(#[from] toml::de::Error),

    #[error("Configuration error: {0}")]
    ConfigError(#[from] raz_config::ConfigError),

    #[error("Tree-sitter error: {0}")]
    TreeSitterError(String),

    #[error("Multiple functions found matching '{0}', please be more specific")]
    AmbiguousFunction(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Migration error: {0}")]
    MigrationError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),
}

pub type Result<T> = std::result::Result<T, OverrideError>;
