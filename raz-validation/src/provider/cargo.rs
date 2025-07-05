//! Cargo options provider

use crate::error::{ValidationError, ValidationResult};
use crate::provider::{OptionDef, OptionProvider, OptionValueType, ValueValidator};
use std::collections::HashMap;

/// Provider for cargo command options
pub struct CargoProvider {
    options: HashMap<String, Vec<OptionDef>>,
}

impl Default for CargoProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl CargoProvider {
    pub fn new() -> Self {
        Self {
            options: build_cargo_options(),
        }
    }
}

impl OptionProvider for CargoProvider {
    fn name(&self) -> &str {
        "cargo"
    }

    fn get_options(&self, command: &str) -> Vec<OptionDef> {
        // Get command-specific options
        let mut options = self.options.get(command).cloned().unwrap_or_default();

        // Add common cargo options that work with most commands
        if let Some(common) = self.options.get("common") {
            options.extend(common.clone());
        }

        options
    }

    fn validate(&self, command: &str, option: &str, value: Option<&str>) -> ValidationResult<()> {
        let options = self.get_options(command);

        // Find the option definition
        let option_def = options
            .iter()
            .find(|def| def.name == option)
            .ok_or_else(|| ValidationError::unknown_option(command, option, vec![]))?;

        // Check if deprecated
        if let Some(deprecated_msg) = &option_def.deprecated {
            return Err(ValidationError::deprecated(option, deprecated_msg));
        }

        // Validate value based on option type
        match (&option_def.value_type, value) {
            (OptionValueType::Flag, Some(val)) => Err(ValidationError::UnexpectedValue {
                option: option.to_string(),
                value: val.to_string(),
            }),
            (OptionValueType::Single(validator), Some(val)) => {
                validator.validate(val).map_err(|mut e| {
                    if let ValidationError::InvalidValue {
                        option: ref mut opt,
                        ..
                    } = e
                    {
                        *opt = option.to_string();
                    }
                    e
                })
            }
            (OptionValueType::Multiple(validator), Some(val)) => {
                // For multiple values, validate each comma-separated value
                for v in val.split(',') {
                    validator.validate(v.trim()).map_err(|mut e| {
                        if let ValidationError::InvalidValue {
                            option: ref mut opt,
                            ..
                        } = e
                        {
                            *opt = option.to_string();
                        }
                        e
                    })?;
                }
                Ok(())
            }
            (OptionValueType::Single(_) | OptionValueType::Multiple(_), None) => {
                Err(ValidationError::MissingValue {
                    option: option.to_string(),
                })
            }
            (OptionValueType::Flag, None) => Ok(()),
        }
    }

    fn get_commands(&self) -> Vec<String> {
        vec![
            "build".to_string(),
            "test".to_string(),
            "run".to_string(),
            "check".to_string(),
            "clippy".to_string(),
            "doc".to_string(),
            "clean".to_string(),
            "bench".to_string(),
            "install".to_string(),
            "publish".to_string(),
        ]
    }
}

/// Build the cargo options catalog
fn build_cargo_options() -> HashMap<String, Vec<OptionDef>> {
    let mut options = HashMap::new();

    // Common options that work with most cargo commands
    options.insert(
        "common".to_string(),
        vec![
            OptionDef::flag("--verbose", "Use verbose output (-v)"),
            OptionDef::flag("-v", "Use verbose output"),
            OptionDef::flag("--quiet", "Do not print cargo log messages"),
            OptionDef::flag("-q", "Do not print cargo log messages"),
            OptionDef::single(
                "--color",
                "Coloring",
                ValueValidator::Enum(vec![
                    "auto".to_string(),
                    "always".to_string(),
                    "never".to_string(),
                ]),
            ),
            OptionDef::flag("--frozen", "Require Cargo.lock and cache are up to date"),
            OptionDef::flag("--locked", "Require Cargo.lock is up to date"),
            OptionDef::flag("--offline", "Run without accessing the network"),
            OptionDef::single(
                "--config",
                "Override a configuration value",
                ValueValidator::Any,
            ),
        ],
    );

    // Build command options
    options.insert(
        "build".to_string(),
        vec![
            OptionDef::flag(
                "--release",
                "Build artifacts in release mode, with optimizations",
            ),
            OptionDef::flag("--dev", "Build artifacts in development mode"),
            OptionDef::single(
                "--target",
                "Build for the target triple",
                ValueValidator::Any,
            ),
            OptionDef::multiple(
                "--features",
                "Space or comma separated list of features to activate",
                ValueValidator::Any,
            ),
            OptionDef::flag("--all-features", "Activate all available features"),
            OptionDef::flag(
                "--no-default-features",
                "Do not activate the default feature",
            ),
            OptionDef::single(
                "--bin",
                "Build only the specified binary",
                ValueValidator::Any,
            ),
            OptionDef::flag("--bins", "Build all binaries"),
            OptionDef::single(
                "--lib",
                "Build only this package's library",
                ValueValidator::Any,
            ),
            OptionDef::flag("--workspace", "Build all packages in the workspace"),
            OptionDef::single("-j", "Number of parallel jobs", ValueValidator::Number)
                .conflicts_with(vec!["--jobs".to_string()]),
            OptionDef::single("--jobs", "Number of parallel jobs", ValueValidator::Number)
                .conflicts_with(vec!["-j".to_string()]),
        ],
    );

    // Test command options
    options.insert(
        "test".to_string(),
        vec![
            OptionDef::flag("--release", "Run tests in release mode"),
            OptionDef::single(
                "--target",
                "Build for the target triple",
                ValueValidator::Any,
            ),
            OptionDef::multiple(
                "--features",
                "Space or comma separated list of features to activate",
                ValueValidator::Any,
            ),
            OptionDef::flag("--all-features", "Activate all available features"),
            OptionDef::flag(
                "--no-default-features",
                "Do not activate the default feature",
            ),
            OptionDef::single(
                "--test",
                "Test only the specified test target",
                ValueValidator::Any,
            ),
            OptionDef::flag("--tests", "Test all tests"),
            OptionDef::single(
                "--bin",
                "Test only the specified binary",
                ValueValidator::Any,
            ),
            OptionDef::flag("--bins", "Test all binaries"),
            OptionDef::flag("--lib", "Test only this package's library"),
            OptionDef::flag("--doc", "Test only this library's documentation"),
            OptionDef::flag("--workspace", "Test all packages in the workspace"),
            OptionDef::flag("--no-run", "Compile, but don't run tests"),
            OptionDef::flag("--no-fail-fast", "Run all tests regardless of failure"),
        ],
    );

    // Run command options
    options.insert(
        "run".to_string(),
        vec![
            OptionDef::flag("--release", "Run in release mode"),
            OptionDef::single(
                "--target",
                "Build for the target triple",
                ValueValidator::Any,
            ),
            OptionDef::multiple(
                "--features",
                "Space or comma separated list of features to activate",
                ValueValidator::Any,
            ),
            OptionDef::flag("--all-features", "Activate all available features"),
            OptionDef::flag(
                "--no-default-features",
                "Do not activate the default feature",
            ),
            OptionDef::single("--bin", "Run the specified binary", ValueValidator::Any),
        ],
    );

    // Check command options (similar to build)
    options.insert(
        "check".to_string(),
        vec![
            OptionDef::flag("--release", "Check artifacts in release mode"),
            OptionDef::single(
                "--target",
                "Check for the target triple",
                ValueValidator::Any,
            ),
            OptionDef::multiple(
                "--features",
                "Space or comma separated list of features to activate",
                ValueValidator::Any,
            ),
            OptionDef::flag("--all-features", "Activate all available features"),
            OptionDef::flag(
                "--no-default-features",
                "Do not activate the default feature",
            ),
            OptionDef::single(
                "--bin",
                "Check only the specified binary",
                ValueValidator::Any,
            ),
            OptionDef::flag("--bins", "Check all binaries"),
            OptionDef::flag("--lib", "Check only this package's library"),
            OptionDef::flag("--workspace", "Check all packages in the workspace"),
        ],
    );

    // Clippy command options
    options.insert(
        "clippy".to_string(),
        vec![
            OptionDef::flag("--release", "Check artifacts in release mode"),
            OptionDef::single(
                "--target",
                "Check for the target triple",
                ValueValidator::Any,
            ),
            OptionDef::multiple(
                "--features",
                "Space or comma separated list of features to activate",
                ValueValidator::Any,
            ),
            OptionDef::flag("--all-features", "Activate all available features"),
            OptionDef::flag(
                "--no-default-features",
                "Do not activate the default feature",
            ),
            OptionDef::flag("--workspace", "Check all packages in the workspace"),
            OptionDef::flag("--fix", "Automatically apply lint suggestions"),
        ],
    );

    options
}
