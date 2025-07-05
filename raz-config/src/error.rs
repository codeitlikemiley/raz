use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Configuration file not found: {0}")]
    FileNotFound(PathBuf),

    #[error("Failed to read configuration file: {0}")]
    ReadError(#[from] std::io::Error),

    #[error("Failed to parse configuration: {0}")]
    ParseError(#[from] toml::de::Error),

    #[error("Failed to serialize configuration: {0}")]
    SerializeError(#[from] toml::ser::Error),

    #[error("Invalid configuration: {0}")]
    ValidationError(String),

    #[error("Configuration version mismatch: expected {expected}, found {found}")]
    VersionMismatch { expected: u32, found: u32 },

    #[error("Migration failed: {0}")]
    MigrationError(String),

    #[error("Invalid home directory")]
    InvalidHomeDir,

    #[error("Invalid workspace path: {0}")]
    InvalidWorkspacePath(PathBuf),

    #[error("Cyclic configuration inheritance detected")]
    CyclicInheritance,

    #[error("Invalid override key: {0}")]
    InvalidOverrideKey(String),
}

pub type Result<T> = std::result::Result<T, ConfigError>;
