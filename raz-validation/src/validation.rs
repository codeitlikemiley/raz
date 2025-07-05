//! Validation configuration and levels

use serde::{Deserialize, Serialize};

/// Configuration for validation behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// Validation level
    pub level: ValidationLevel,

    /// Enabled providers
    pub providers: Vec<String>,

    /// Custom options for specific commands
    pub custom_options: std::collections::HashMap<String, Vec<CustomOption>>,

    /// Minimum similarity score for suggestions
    pub suggestion_threshold: i64,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            level: ValidationLevel::Normal,
            providers: vec!["cargo".to_string(), "leptos".to_string()],
            custom_options: std::collections::HashMap::new(),
            suggestion_threshold: 30,
        }
    }
}

/// Validation strictness levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValidationLevel {
    /// No validation - allow any options
    Off,

    /// Minimal validation - only check known conflicts
    Minimal,

    /// Normal validation - validate known options, warn on unknown
    Normal,

    /// Strict validation - reject unknown options
    Strict,
}

impl ValidationLevel {
    /// Check if this level performs validation
    pub fn is_enabled(self) -> bool {
        !matches!(self, Self::Off)
    }

    /// Check if this level is strict
    pub fn is_strict(self) -> bool {
        matches!(self, Self::Strict)
    }

    /// Check if this level allows unknown options
    pub fn allows_unknown(self) -> bool {
        matches!(self, Self::Off | Self::Minimal | Self::Normal)
    }
}

/// Custom option definition for configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomOption {
    /// Option name
    pub name: String,

    /// Option type
    #[serde(rename = "type")]
    pub option_type: String,

    /// Description
    pub description: Option<String>,

    /// Valid values (for enum types)
    pub values: Option<Vec<String>>,

    /// Conflicts with these options
    pub conflicts_with: Option<Vec<String>>,

    /// Requires these options
    pub requires: Option<Vec<String>>,
}

impl ValidationConfig {
    /// Create a new config with specified level
    pub fn with_level(level: ValidationLevel) -> Self {
        Self {
            level,
            ..Default::default()
        }
    }

    /// Add a provider to the config
    pub fn add_provider(mut self, provider: impl Into<String>) -> Self {
        self.providers.push(provider.into());
        self
    }

    /// Set suggestion threshold
    pub fn with_suggestion_threshold(mut self, threshold: i64) -> Self {
        self.suggestion_threshold = threshold;
        self
    }

    /// Check if a provider is enabled
    pub fn is_provider_enabled(&self, provider: &str) -> bool {
        self.providers.contains(&provider.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_level_methods() {
        assert!(ValidationLevel::Normal.is_enabled());
        assert!(!ValidationLevel::Off.is_enabled());

        assert!(ValidationLevel::Strict.is_strict());
        assert!(!ValidationLevel::Normal.is_strict());

        assert!(ValidationLevel::Normal.allows_unknown());
        assert!(!ValidationLevel::Strict.allows_unknown());
    }

    #[test]
    fn test_config_defaults() {
        let config = ValidationConfig::default();
        assert_eq!(config.level, ValidationLevel::Normal);
        assert!(config.providers.contains(&"cargo".to_string()));
    }

    #[test]
    fn test_config_builder() {
        let config = ValidationConfig::with_level(ValidationLevel::Strict)
            .add_provider("dioxus")
            .with_suggestion_threshold(50);

        assert_eq!(config.level, ValidationLevel::Strict);
        assert!(config.is_provider_enabled("dioxus"));
        assert_eq!(config.suggestion_threshold, 50);
    }
}
