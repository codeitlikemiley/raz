//! Option provider trait and related types

use crate::error::{ValidationError, ValidationResult};
use serde::{Deserialize, Serialize};

/// Trait for providing option definitions and validation for a specific tool or framework
pub trait OptionProvider: Send + Sync {
    /// Get the name of this provider (e.g., "cargo", "leptos", "dioxus")
    fn name(&self) -> &str;

    /// Get all options available for a specific command
    fn get_options(&self, command: &str) -> Vec<OptionDef>;

    /// Validate a specific option and its value
    fn validate(&self, command: &str, option: &str, value: Option<&str>) -> ValidationResult<()>;

    /// Get commands supported by this provider
    fn get_commands(&self) -> Vec<String>;

    /// Check if this provider supports a specific command
    fn supports_command(&self, command: &str) -> bool {
        self.get_commands().contains(&command.to_string())
    }
}

/// Definition of a command-line option
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionDef {
    /// Option name (e.g., "--release", "--features")
    pub name: String,

    /// Type of value this option expects
    pub value_type: OptionValueType,

    /// Human-readable description
    pub description: String,

    /// Options that conflict with this one
    pub conflicts_with: Vec<String>,

    /// Options that are required when this one is used
    pub requires: Vec<String>,

    /// Deprecation message if this option is deprecated
    pub deprecated: Option<String>,

    /// Whether this option can appear multiple times
    pub repeatable: bool,
}

impl OptionDef {
    /// Create a new flag option
    pub fn flag(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value_type: OptionValueType::Flag,
            description: description.into(),
            conflicts_with: Vec::new(),
            requires: Vec::new(),
            deprecated: None,
            repeatable: false,
        }
    }

    /// Create a new single value option
    pub fn single(
        name: impl Into<String>,
        description: impl Into<String>,
        validator: ValueValidator,
    ) -> Self {
        Self {
            name: name.into(),
            value_type: OptionValueType::Single(validator),
            description: description.into(),
            conflicts_with: Vec::new(),
            requires: Vec::new(),
            deprecated: None,
            repeatable: false,
        }
    }

    /// Create a new multiple value option
    pub fn multiple(
        name: impl Into<String>,
        description: impl Into<String>,
        validator: ValueValidator,
    ) -> Self {
        Self {
            name: name.into(),
            value_type: OptionValueType::Multiple(validator),
            description: description.into(),
            conflicts_with: Vec::new(),
            requires: Vec::new(),
            deprecated: None,
            repeatable: false,
        }
    }

    /// Add conflicting options
    pub fn conflicts_with(mut self, options: Vec<String>) -> Self {
        self.conflicts_with = options;
        self
    }

    /// Add required options
    pub fn requires(mut self, options: Vec<String>) -> Self {
        self.requires = options;
        self
    }

    /// Mark as deprecated
    pub fn deprecated(mut self, message: impl Into<String>) -> Self {
        self.deprecated = Some(message.into());
        self
    }

    /// Mark as repeatable
    pub fn repeatable(mut self) -> Self {
        self.repeatable = true;
        self
    }
}

/// Type of value an option expects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptionValueType {
    /// Boolean flag (e.g., --release)
    Flag,

    /// Single value (e.g., --bin myapp)
    Single(ValueValidator),

    /// Multiple values (e.g., --features "a,b,c")
    Multiple(ValueValidator),
}

/// Validator for option values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValueValidator {
    /// Any string value
    Any,

    /// Must be a valid number
    Number,

    /// Must be one of the specified values
    Enum(Vec<String>),

    /// Must match the regex pattern
    Regex(String),

    /// Must be a valid file path
    FilePath,

    /// Must be a valid directory path
    DirectoryPath,

    /// Custom validation function name (for extensibility)
    Custom(String),
}

impl ValueValidator {
    /// Validate a value against this validator
    pub fn validate(&self, value: &str) -> ValidationResult<()> {
        match self {
            Self::Any => Ok(()),

            Self::Number => value
                .parse::<f64>()
                .map(|_| ())
                .map_err(|_| ValidationError::invalid_value("", value, "must be a number")),

            Self::Enum(valid_values) => {
                if valid_values.contains(&value.to_string()) {
                    Ok(())
                } else {
                    Err(ValidationError::invalid_value(
                        "",
                        value,
                        format!("must be one of: {}", valid_values.join(", ")),
                    ))
                }
            }

            Self::Regex(pattern) => {
                let regex = regex::Regex::new(pattern).map_err(|_| {
                    ValidationError::invalid_value("", value, "invalid regex pattern")
                })?;

                if regex.is_match(value) {
                    Ok(())
                } else {
                    Err(ValidationError::invalid_value(
                        "",
                        value,
                        format!("must match pattern: {pattern}"),
                    ))
                }
            }

            Self::FilePath => {
                // Basic file path validation - could be enhanced
                if value.is_empty() {
                    Err(ValidationError::invalid_value(
                        "",
                        value,
                        "file path cannot be empty",
                    ))
                } else {
                    Ok(())
                }
            }

            Self::DirectoryPath => {
                // Basic directory path validation - could be enhanced
                if value.is_empty() {
                    Err(ValidationError::invalid_value(
                        "",
                        value,
                        "directory path cannot be empty",
                    ))
                } else {
                    Ok(())
                }
            }

            Self::Custom(_name) => {
                // Custom validators would be implemented by providers
                Ok(())
            }
        }
    }
}

/// Built-in providers
pub mod cargo;
pub mod leptos;
