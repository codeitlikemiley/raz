//! Configuration validation for ensuring configs are valid before saving

use crate::{
    config::{Config, Override},
    error::Result,
    runners::options::CommandOptions,
    runners::validation::{ValidationRuleSet, bazel_validation_rules, cargo_validation_rules},
};

/// Trait for validating configurations
pub trait ConfigValidator {
    /// Validate the entire configuration
    fn validate(&self, config: &Config) -> Result<()>;

    /// Validate a specific override
    fn validate_override(&self, override_config: &Override) -> Result<()>;
}

/// Main configuration validator
pub struct MainConfigValidator {
    cargo_rules: ValidationRuleSet,
    bazel_rules: ValidationRuleSet,
}

impl MainConfigValidator {
    pub fn new() -> Self {
        Self {
            cargo_rules: cargo_validation_rules(),
            bazel_rules: bazel_validation_rules(),
        }
    }

    /// Parse command line arguments into CommandOptions
    fn parse_args_to_options(&self, args: &[String]) -> Result<CommandOptions> {
        let mut options = CommandOptions::default();

        let mut i = 0;
        while i < args.len() {
            let arg = &args[i];

            match arg.as_str() {
                // Package selection
                "-p" | "--package" => {
                    i += 1;
                    if i < args.len() {
                        options.package_selection.package.push(args[i].clone());
                    }
                }
                "--workspace" => {
                    options.package_selection.workspace = true;
                }
                "--all" => {
                    options.package_selection.all = true;
                }
                "--exclude" => {
                    i += 1;
                    if i < args.len() {
                        options.package_selection.exclude.push(args[i].clone());
                    }
                }

                // Target selection
                "--lib" => {
                    options.target_selection.lib = true;
                }
                "--bins" => {
                    options.target_selection.bins = true;
                }
                "--bin" => {
                    i += 1;
                    if i < args.len() {
                        options.target_selection.bin.push(args[i].clone());
                    }
                }
                "--examples" => {
                    options.target_selection.examples = true;
                }
                "--example" => {
                    i += 1;
                    if i < args.len() {
                        options.target_selection.example.push(args[i].clone());
                    }
                }
                "--doc" => {
                    options.target_selection.doc = true;
                }
                "--all-targets" => {
                    options.target_selection.all_targets = true;
                }

                // Feature selection
                "-F" | "--features" => {
                    i += 1;
                    if i < args.len() {
                        // Split comma-separated features
                        options
                            .feature_selection
                            .features
                            .extend(args[i].split(',').map(|s| s.trim().to_string()));
                    }
                }
                "--all-features" => {
                    options.feature_selection.all_features = true;
                }
                "--no-default-features" => {
                    options.feature_selection.no_default_features = true;
                }

                // Compilation options
                "-r" | "--release" => {
                    options.compilation_options.release = true;
                }
                "--profile" => {
                    i += 1;
                    if i < args.len() {
                        options.compilation_options.profile = Some(args[i].clone());
                    }
                }
                "-j" | "--jobs" => {
                    i += 1;
                    if i < args.len() {
                        if let Ok(jobs) = args[i].parse() {
                            options.compilation_options.jobs = Some(jobs);
                        }
                    }
                }

                // Output options
                "-q" | "--quiet" => {
                    options.output_options.quiet = true;
                }
                "-v" | "--verbose" => {
                    options.output_options.verbose += 1;
                }
                "-vv" => {
                    options.output_options.verbose = 2;
                }

                _ => {
                    // Skip unknown arguments
                }
            }

            i += 1;
        }

        Ok(options)
    }
}

impl ConfigValidator for MainConfigValidator {
    fn validate(&self, config: &Config) -> Result<()> {
        // Validate all overrides
        for override_config in &config.overrides {
            self.validate_override(override_config)?;
        }

        // Validate cargo config if present
        if let Some(cargo_config) = &config.cargo {
            // Validate extra_args if present
            if let Some(extra_args) = &cargo_config.extra_args {
                let options = self.parse_args_to_options(extra_args)?;
                self.cargo_rules.validate(&options)?;
            }

            // Validate binary_framework args
            if let Some(framework) = &cargo_config.binary_framework {
                if let Some(extra_args) = &framework.extra_args {
                    let options = self.parse_args_to_options(extra_args)?;
                    self.cargo_rules.validate(&options)?;
                }
            }

            // Validate test_framework args
            if let Some(framework) = &cargo_config.test_framework {
                if let Some(extra_args) = &framework.extra_args {
                    let options = self.parse_args_to_options(extra_args)?;
                    self.cargo_rules.validate(&options)?;
                }
            }
        }

        // Validate bazel config if present
        if let Some(bazel_config) = &config.bazel {
            // Validate framework args for each framework type
            let frameworks = [
                &bazel_config.test_framework,
                &bazel_config.binary_framework,
                &bazel_config.benchmark_framework,
                &bazel_config.doc_test_framework,
            ];

            for framework_opt in frameworks {
                if let Some(framework) = framework_opt {
                    if let Some(extra_args) = &framework.extra_args {
                        let options = self.parse_args_to_options(extra_args)?;
                        self.bazel_rules.validate(&options)?;
                    }
                }
            }
        }

        Ok(())
    }

    fn validate_override(&self, override_config: &Override) -> Result<()> {
        // Validate cargo override
        if let Some(cargo_override) = &override_config.cargo {
            if let Some(extra_args) = &cargo_override.extra_args {
                let options = self.parse_args_to_options(extra_args)?;
                self.cargo_rules.validate(&options)?;
            }
        }

        // Validate bazel override (flat BazelOverride shape)
        if let Some(bazel_override) = &override_config.bazel {
            // Validate base args
            if let Some(args) = &bazel_override.args {
                let options = self.parse_args_to_options(args)?;
                self.bazel_rules.validate(&options)?;
            }
            // Validate extra_args
            if let Some(extra) = &bazel_override.extra_args {
                let options = self.parse_args_to_options(extra)?;
                self.bazel_rules.validate(&options)?;
            }
        }

        Ok(())
    }
}

/// Extension trait for Config to add validation
impl Config {
    /// Validate this configuration
    pub fn validate(&self) -> Result<()> {
        let validator = MainConfigValidator::new();
        validator.validate(self)
    }

    /// Save with validation
    pub fn save_validated(&self, path: &std::path::Path) -> Result<()> {
        // Validate before saving
        self.validate()?;

        // Use the regular save_to_file method
        self.save_to_file(path)
    }
}
