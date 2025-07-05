//! Smart options validation system for raz
//!
//! This crate provides a pluggable validation system that can validate command-line options
//! for different tools and frameworks (cargo, leptos, dioxus, etc.) with helpful error messages
//! and suggestions.

pub mod error;
pub mod provider;
pub mod registry;
pub mod suggestion;
pub mod validation;

pub use error::{ValidationError, ValidationResult};
pub use provider::{OptionDef, OptionProvider, OptionValueType, ValueValidator};
pub use registry::ValidationRegistry;
pub use validation::{ValidationConfig, ValidationLevel};

use std::collections::HashMap;

/// Main validation engine
pub struct ValidationEngine {
    registry: ValidationRegistry,
    config: ValidationConfig,
}

impl Default for ValidationEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ValidationEngine {
    /// Create a new validation engine with default providers
    pub fn new() -> Self {
        let mut registry = ValidationRegistry::new();

        // Register built-in providers
        registry.register_provider(Box::new(provider::cargo::CargoProvider::new()));

        Self {
            registry,
            config: ValidationConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: ValidationConfig) -> Self {
        let mut engine = Self::new();
        engine.config = config;
        engine
    }

    /// Validate a single option for a given command
    pub fn validate_option(
        &self,
        command: &str,
        option: &str,
        value: Option<&str>,
    ) -> ValidationResult<()> {
        if self.config.level == ValidationLevel::Off {
            return Ok(());
        }

        self.registry
            .validate_option(command, option, value, &self.config)
    }

    /// Validate multiple options for a command
    pub fn validate_options(
        &self,
        command: &str,
        options: &HashMap<String, Option<String>>,
    ) -> ValidationResult<()> {
        for (option, value) in options {
            self.validate_option(command, option, value.as_deref())?;
        }
        Ok(())
    }

    /// Get suggestions for a misspelled option
    pub fn suggest_option(&self, command: &str, option: &str) -> Vec<String> {
        self.registry.suggest_option(command, option)
    }

    /// Get all valid options for a command
    pub fn get_options(&self, command: &str) -> Vec<OptionDef> {
        self.registry.get_options(command)
    }

    /// Register a new provider
    pub fn register_provider(&mut self, provider: Box<dyn OptionProvider>) {
        self.registry.register_provider(provider);
    }
}
