//! Validation rules engine for command options

use super::options::CommandOptions;
use crate::error::Result;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("Conflicting options: {0}")]
    ConflictingOptions(String),

    #[error("Missing required option: {0}")]
    MissingRequired(String),

    #[error("Invalid value: {0}")]
    InvalidValue(String),

    #[error("Mutually exclusive options: {0}")]
    MutuallyExclusive(String),
}

/// Base trait for validation rules
pub trait ValidationRule: Send + Sync {
    /// Validate the given options
    fn validate(&self, options: &CommandOptions) -> std::result::Result<(), ValidationError>;

    /// Get a description of this rule
    fn description(&self) -> &str;

    /// Get the name of this rule
    fn name(&self) -> &str;
}

/// Rule for mutually exclusive options
pub struct MutuallyExclusiveRule {
    pub name: String,
    pub fields: Vec<String>,
    pub message: String,
}

impl ValidationRule for MutuallyExclusiveRule {
    fn validate(&self, options: &CommandOptions) -> std::result::Result<(), ValidationError> {
        let active_count = self
            .fields
            .iter()
            .filter(|field| is_field_active(options, field))
            .count();

        if active_count > 1 {
            return Err(ValidationError::MutuallyExclusive(self.message.clone()));
        }

        Ok(())
    }

    fn description(&self) -> &str {
        &self.message
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Rule for conflicting options
pub struct ConflictingOptionsRule {
    pub name: String,
    pub option1: String,
    pub option2: String,
    pub message: String,
}

impl ValidationRule for ConflictingOptionsRule {
    fn validate(&self, options: &CommandOptions) -> std::result::Result<(), ValidationError> {
        let opt1_active = is_field_active(options, &self.option1);
        let opt2_active = is_field_active(options, &self.option2);

        if opt1_active && opt2_active {
            return Err(ValidationError::ConflictingOptions(self.message.clone()));
        }

        Ok(())
    }

    fn description(&self) -> &str {
        &self.message
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Rule for required fields when condition is met
pub struct RequiredIfRule {
    pub name: String,
    pub condition_field: String,
    pub required_field: String,
    pub message: String,
}

impl ValidationRule for RequiredIfRule {
    fn validate(&self, options: &CommandOptions) -> std::result::Result<(), ValidationError> {
        let condition_met = is_field_active(options, &self.condition_field);
        let required_present = is_field_active(options, &self.required_field);

        if condition_met && !required_present {
            return Err(ValidationError::MissingRequired(self.message.clone()));
        }

        Ok(())
    }

    fn description(&self) -> &str {
        &self.message
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Collection of validation rules
pub struct ValidationRuleSet {
    rules: Vec<Box<dyn ValidationRule>>,
}

impl ValidationRuleSet {
    pub fn new() -> Self {
        Self { rules: vec![] }
    }

    pub fn add_rule(&mut self, rule: Box<dyn ValidationRule>) {
        self.rules.push(rule);
    }

    pub fn validate(&self, options: &CommandOptions) -> Result<()> {
        for rule in &self.rules {
            rule.validate(options)
                .map_err(|e| crate::error::Error::Other(e.to_string()))?;
        }
        Ok(())
    }
}

/// Get cargo-specific validation rules
pub fn cargo_validation_rules() -> ValidationRuleSet {
    let mut rules = ValidationRuleSet::new();

    // Feature conflicts
    rules.add_rule(Box::new(ConflictingOptionsRule {
        name: "feature_conflict".to_string(),
        option1: "all_features".to_string(),
        option2: "no_default_features".to_string(),
        message: "--all-features and --no-default-features cannot be used together".to_string(),
    }));

    rules.add_rule(Box::new(ConflictingOptionsRule {
        name: "feature_specific_conflict".to_string(),
        option1: "all_features".to_string(),
        option2: "features".to_string(),
        message: "--all-features makes --features redundant".to_string(),
    }));

    // Target conflicts for run command
    rules.add_rule(Box::new(MutuallyExclusiveRule {
        name: "run_target_exclusive".to_string(),
        fields: vec!["lib".to_string(), "bin".to_string(), "example".to_string()],
        message: "Only one target type can be specified for run command".to_string(),
    }));

    // Test-specific conflicts
    rules.add_rule(Box::new(ConflictingOptionsRule {
        name: "doc_test_conflict".to_string(),
        option1: "doc".to_string(),
        option2: "lib".to_string(),
        message: "--doc cannot be used with other target selections".to_string(),
    }));

    // Package selection conflicts
    rules.add_rule(Box::new(ConflictingOptionsRule {
        name: "package_workspace_conflict".to_string(),
        option1: "package".to_string(),
        option2: "workspace".to_string(),
        message: "--package and --workspace are mutually exclusive".to_string(),
    }));

    // Compilation conflicts
    rules.add_rule(Box::new(ConflictingOptionsRule {
        name: "release_profile_conflict".to_string(),
        option1: "release".to_string(),
        option2: "profile".to_string(),
        message: "--release and --profile cannot be used together".to_string(),
    }));

    // Output conflicts
    rules.add_rule(Box::new(ConflictingOptionsRule {
        name: "quiet_verbose_conflict".to_string(),
        option1: "quiet".to_string(),
        option2: "verbose".to_string(),
        message: "--quiet and --verbose cannot be used together".to_string(),
    }));

    rules
}

/// Get bazel-specific validation rules
pub fn bazel_validation_rules() -> ValidationRuleSet {
    let mut rules = ValidationRuleSet::new();

    // Add Bazel-specific rules here
    // For now, using some basic rules
    rules.add_rule(Box::new(ConflictingOptionsRule {
        name: "bazel_config_conflict".to_string(),
        option1: "config".to_string(),
        option2: "no_config".to_string(),
        message: "Cannot specify both config and no-config".to_string(),
    }));

    rules
}

/// Helper function to check if a field is active in CommandOptions
fn is_field_active(options: &CommandOptions, field_path: &str) -> bool {
    match field_path {
        // Feature selection
        "all_features" => options.feature_selection.all_features,
        "no_default_features" => options.feature_selection.no_default_features,
        "features" => !options.feature_selection.features.is_empty(),

        // Target selection
        "lib" => options.target_selection.lib,
        "bin" => !options.target_selection.bin.is_empty() || options.target_selection.bins,
        "example" => {
            !options.target_selection.example.is_empty() || options.target_selection.examples
        }
        "doc" => options.target_selection.doc,

        // Package selection
        "package" => !options.package_selection.package.is_empty(),
        "workspace" => options.package_selection.workspace,

        // Compilation options
        "release" => options.compilation_options.release,
        "profile" => options.compilation_options.profile.is_some(),

        // Output options
        "quiet" => options.output_options.quiet,
        "verbose" => options.output_options.verbose > 0,

        _ => false,
    }
}
