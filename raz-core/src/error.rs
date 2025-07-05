//! Error types for RAZ Core

use std::path::PathBuf;
use thiserror::Error;

/// Main error type for RAZ operations
#[derive(Error, Debug)]
pub enum RazError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {message}")]
    Config { message: String },

    #[error("Project analysis failed: {message}")]
    Analysis { message: String },

    #[error("Command execution failed: {command}")]
    Execution { command: String },

    #[error("Provider error: {provider}: {message}")]
    Provider { provider: String, message: String },

    #[error("Invalid workspace: {path}")]
    InvalidWorkspace { path: PathBuf },

    #[error("Parsing error: {message}")]
    Parse { message: String },

    #[error("Feature not available: {feature}")]
    FeatureUnavailable { feature: String },

    #[error("Timeout: {operation}")]
    Timeout { operation: String },

    #[error("Internal error: {message}")]
    Internal { message: String },
}

impl RazError {
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config {
            message: message.into(),
        }
    }

    pub fn analysis(message: impl Into<String>) -> Self {
        Self::Analysis {
            message: message.into(),
        }
    }

    pub fn execution(command: impl Into<String>) -> Self {
        Self::Execution {
            command: command.into(),
        }
    }

    pub fn provider(provider: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Provider {
            provider: provider.into(),
            message: message.into(),
        }
    }

    pub fn invalid_workspace(path: impl Into<PathBuf>) -> Self {
        Self::InvalidWorkspace { path: path.into() }
    }

    pub fn parse(message: impl Into<String>) -> Self {
        Self::Parse {
            message: message.into(),
        }
    }

    pub fn feature_unavailable(feature: impl Into<String>) -> Self {
        Self::FeatureUnavailable {
            feature: feature.into(),
        }
    }

    pub fn timeout(operation: impl Into<String>) -> Self {
        Self::Timeout {
            operation: operation.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }
}

/// Result type alias for RAZ operations
pub type RazResult<T> = Result<T, RazError>;
