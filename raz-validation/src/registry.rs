//! Validation registry for managing option providers

use crate::error::{ValidationError, ValidationResult};
use crate::provider::{OptionDef, OptionProvider};
use crate::suggestion::OptionSuggester;
use crate::validation::ValidationConfig;
use std::collections::HashMap;

/// Registry that manages multiple option providers
pub struct ValidationRegistry {
    providers: Vec<Box<dyn OptionProvider>>,
    suggester: OptionSuggester,
}

impl Default for ValidationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ValidationRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            suggester: OptionSuggester::new(),
        }
    }

    /// Register a new option provider
    pub fn register_provider(&mut self, provider: Box<dyn OptionProvider>) {
        self.providers.push(provider);
    }

    /// Get all providers
    pub fn get_providers(&self) -> &[Box<dyn OptionProvider>] {
        &self.providers
    }

    /// Get options for a command from all applicable providers
    pub fn get_options(&self, command: &str) -> Vec<OptionDef> {
        let mut all_options = Vec::new();

        for provider in &self.providers {
            if provider.supports_command(command) {
                all_options.extend(provider.get_options(command));
            }
        }

        // Remove duplicates (prefer first occurrence)
        let mut seen = std::collections::HashSet::new();
        all_options.retain(|opt| seen.insert(opt.name.clone()));

        all_options
    }

    /// Validate an option using the appropriate provider
    pub fn validate_option(
        &self,
        command: &str,
        option: &str,
        value: Option<&str>,
        config: &ValidationConfig,
    ) -> ValidationResult<()> {
        // Find a provider that supports this command and has this option
        for provider in &self.providers {
            if provider.supports_command(command) {
                let options = provider.get_options(command);
                if options.iter().any(|opt| opt.name == option) {
                    return provider.validate(command, option, value);
                }
            }
        }

        // No provider recognized this option
        if config.level.is_strict() {
            let suggestions = self.suggest_option(command, option);
            Err(ValidationError::unknown_option(
                command,
                option,
                suggestions,
            ))
        } else {
            // In non-strict mode, allow unknown options
            Ok(())
        }
    }

    /// Get suggestions for a misspelled option
    pub fn suggest_option(&self, command: &str, option: &str) -> Vec<String> {
        let all_options = self.get_options(command);
        let option_names: Vec<&str> = all_options.iter().map(|opt| opt.name.as_str()).collect();

        self.suggester.suggest_options(option, &option_names)
    }

    /// Validate conflicts between options
    pub fn validate_conflicts(
        &self,
        command: &str,
        options: &HashMap<String, Option<String>>,
    ) -> ValidationResult<()> {
        let option_defs = self.get_options(command);
        let mut option_map: HashMap<String, &OptionDef> = HashMap::new();

        for def in &option_defs {
            option_map.insert(def.name.clone(), def);
        }

        // Check for conflicts
        for option_name in options.keys() {
            if let Some(option_def) = option_map.get(option_name) {
                for conflict in &option_def.conflicts_with {
                    if options.contains_key(conflict) {
                        return Err(ValidationError::conflict(option_name, conflict));
                    }
                }
            }
        }

        Ok(())
    }

    /// Validate required options
    pub fn validate_requirements(
        &self,
        command: &str,
        options: &HashMap<String, Option<String>>,
    ) -> ValidationResult<()> {
        let option_defs = self.get_options(command);
        let mut option_map: HashMap<String, &OptionDef> = HashMap::new();

        for def in &option_defs {
            option_map.insert(def.name.clone(), def);
        }

        // Check for missing requirements
        for option_name in options.keys() {
            if let Some(option_def) = option_map.get(option_name) {
                for required in &option_def.requires {
                    if !options.contains_key(required) {
                        return Err(ValidationError::missing_requirement(option_name, required));
                    }
                }
            }
        }

        Ok(())
    }

    /// Get provider by name
    pub fn get_provider(&self, name: &str) -> Option<&dyn OptionProvider> {
        self.providers
            .iter()
            .find(|p| p.name() == name)
            .map(|p| p.as_ref())
    }

    /// Check if any provider supports a command
    pub fn supports_command(&self, command: &str) -> bool {
        self.providers.iter().any(|p| p.supports_command(command))
    }
}
