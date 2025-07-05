//! Validation error types

use thiserror::Error;

pub type ValidationResult<T> = Result<T, ValidationError>;

#[derive(Error, Debug, Clone)]
pub enum ValidationError {
    #[error("Unknown option '{option}' for command '{command}'")]
    UnknownOption {
        command: String,
        option: String,
        suggestions: Vec<String>,
    },

    #[error("Invalid value '{value}' for option '{option}': {reason}")]
    InvalidValue {
        option: String,
        value: String,
        reason: String,
    },

    #[error("Option '{option}' conflicts with '{conflicts_with}'")]
    OptionConflict {
        option: String,
        conflicts_with: String,
    },

    #[error("Option '{option}' requires '{required}' to be specified")]
    MissingRequirement { option: String, required: String },

    #[error("Option '{option}' is deprecated: {message}")]
    DeprecatedOption { option: String, message: String },

    #[error("Option '{option}' expects a value")]
    MissingValue { option: String },

    #[error("Option '{option}' does not accept a value")]
    UnexpectedValue { option: String, value: String },

    #[error("Provider error: {message}")]
    ProviderError { message: String },
}

impl ValidationError {
    /// Create an unknown option error with suggestions
    pub fn unknown_option(
        command: impl Into<String>,
        option: impl Into<String>,
        suggestions: Vec<String>,
    ) -> Self {
        Self::UnknownOption {
            command: command.into(),
            option: option.into(),
            suggestions,
        }
    }

    /// Create an invalid value error
    pub fn invalid_value(
        option: impl Into<String>,
        value: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::InvalidValue {
            option: option.into(),
            value: value.into(),
            reason: reason.into(),
        }
    }

    /// Create a conflict error
    pub fn conflict(option: impl Into<String>, conflicts_with: impl Into<String>) -> Self {
        Self::OptionConflict {
            option: option.into(),
            conflicts_with: conflicts_with.into(),
        }
    }

    /// Create a missing requirement error
    pub fn missing_requirement(option: impl Into<String>, required: impl Into<String>) -> Self {
        Self::MissingRequirement {
            option: option.into(),
            required: required.into(),
        }
    }

    /// Create a deprecated option error
    pub fn deprecated(option: impl Into<String>, message: impl Into<String>) -> Self {
        Self::DeprecatedOption {
            option: option.into(),
            message: message.into(),
        }
    }

    /// Get formatted error message with suggestions
    pub fn format_with_suggestions(&self) -> String {
        match self {
            Self::UnknownOption {
                command,
                option,
                suggestions,
            } => {
                let mut msg = format!("Unknown option '{option}' for command '{command}'");
                if !suggestions.is_empty() {
                    msg.push_str(&format!("\nDid you mean: {}", suggestions.join(", ")));
                }
                msg
            }
            _ => self.to_string(),
        }
    }
}
