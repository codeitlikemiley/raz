//! Leptos framework options provider

use crate::error::{ValidationError, ValidationResult};
use crate::provider::{OptionDef, OptionProvider, OptionValueType, ValueValidator};
use std::collections::HashMap;

/// Provider for leptos command options
pub struct LeptosProvider {
    options: HashMap<String, Vec<OptionDef>>,
}

impl Default for LeptosProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl LeptosProvider {
    pub fn new() -> Self {
        Self {
            options: build_leptos_options(),
        }
    }
}

impl OptionProvider for LeptosProvider {
    fn name(&self) -> &str {
        "leptos"
    }

    fn get_options(&self, command: &str) -> Vec<OptionDef> {
        // Handle leptos subcommands
        let leptos_command = if let Some(stripped) = command.strip_prefix("leptos ") {
            stripped // Remove "leptos " prefix
        } else {
            command
        };

        self.options
            .get(leptos_command)
            .cloned()
            .unwrap_or_default()
    }

    fn validate(&self, command: &str, option: &str, value: Option<&str>) -> ValidationResult<()> {
        let options = self.get_options(command);

        // Find the option definition
        let option_def = options
            .iter()
            .find(|def| def.name == option)
            .ok_or_else(|| ValidationError::unknown_option(command, option, vec![]))?;

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
            "leptos build".to_string(),
            "leptos serve".to_string(),
            "leptos watch".to_string(),
            "leptos new".to_string(),
            "leptos end-to-end".to_string(),
        ]
    }

    fn supports_command(&self, command: &str) -> bool {
        command.starts_with("leptos ") || self.get_commands().contains(&command.to_string())
    }
}

/// Build the leptos options catalog
fn build_leptos_options() -> HashMap<String, Vec<OptionDef>> {
    let mut options = HashMap::new();

    // Leptos build command options
    options.insert(
        "build".to_string(),
        vec![
            OptionDef::flag("--release", "Build in release mode"),
            OptionDef::multiple(
                "--bin-features",
                "Features to enable for binary compilation",
                ValueValidator::Any,
            ),
            OptionDef::multiple(
                "--lib-features",
                "Features to enable for library compilation",
                ValueValidator::Any,
            ),
            OptionDef::flag("--precompress", "Pre-compress static assets"),
            OptionDef::single(
                "--bin-target-dir",
                "Directory for binary artifacts",
                ValueValidator::DirectoryPath,
            ),
            OptionDef::single(
                "--lib-target-dir",
                "Directory for library artifacts",
                ValueValidator::DirectoryPath,
            ),
            OptionDef::single(
                "--site-root",
                "Root directory for the site",
                ValueValidator::DirectoryPath,
            ),
            OptionDef::single(
                "--site-pkg-dir",
                "Directory for site packages",
                ValueValidator::DirectoryPath,
            ),
            OptionDef::single(
                "--site-addr",
                "Address to serve the site",
                ValueValidator::Any,
            ),
            OptionDef::single(
                "--reload-port",
                "Port for reload server",
                ValueValidator::Number,
            ),
            OptionDef::flag("--hot-reload", "Enable hot reloading"),
            OptionDef::flag("--no-default-features", "Disable default features"),
        ],
    );

    // Leptos serve command options
    options.insert(
        "serve".to_string(),
        vec![
            OptionDef::flag("--release", "Serve release build"),
            OptionDef::single("--port", "Port to serve on", ValueValidator::Number),
            OptionDef::single("--host", "Host to serve on", ValueValidator::Any),
            OptionDef::flag("--hot-reload", "Enable hot reloading"),
            OptionDef::single(
                "--reload-port",
                "Port for reload server",
                ValueValidator::Number,
            ),
            OptionDef::single(
                "--site-root",
                "Root directory for the site",
                ValueValidator::DirectoryPath,
            ),
            OptionDef::single(
                "--site-pkg-dir",
                "Directory for site packages",
                ValueValidator::DirectoryPath,
            ),
            OptionDef::multiple(
                "--bin-features",
                "Features to enable for binary compilation",
                ValueValidator::Any,
            ),
            OptionDef::multiple(
                "--lib-features",
                "Features to enable for library compilation",
                ValueValidator::Any,
            ),
        ],
    );

    // Leptos watch command options
    options.insert(
        "watch".to_string(),
        vec![
            OptionDef::flag("--release", "Watch release build"),
            OptionDef::flag("--hot-reload", "Enable hot reloading"),
            OptionDef::single(
                "--reload-port",
                "Port for reload server",
                ValueValidator::Number,
            ),
            OptionDef::single(
                "--site-root",
                "Root directory for the site",
                ValueValidator::DirectoryPath,
            ),
            OptionDef::multiple(
                "--bin-features",
                "Features to enable for binary compilation",
                ValueValidator::Any,
            ),
            OptionDef::multiple(
                "--lib-features",
                "Features to enable for library compilation",
                ValueValidator::Any,
            ),
            OptionDef::flag("--no-default-features", "Disable default features"),
        ],
    );

    // Leptos new command options
    options.insert(
        "new".to_string(),
        vec![
            OptionDef::single("--name", "Name of the new project", ValueValidator::Any),
            OptionDef::single(
                "--template",
                "Template to use",
                ValueValidator::Enum(vec![
                    "start".to_string(),
                    "start-axum".to_string(),
                    "start-actix".to_string(),
                    "csr".to_string(),
                    "ssr".to_string(),
                ]),
            ),
            OptionDef::flag("--git", "Initialize git repository"),
            OptionDef::flag("--no-git", "Don't initialize git repository"),
        ],
    );

    // Leptos end-to-end command options
    options.insert(
        "end-to-end".to_string(),
        vec![
            OptionDef::flag("--release", "Run end-to-end tests in release mode"),
            OptionDef::single("--port", "Port for test server", ValueValidator::Number),
            OptionDef::multiple(
                "--bin-features",
                "Features to enable for binary compilation",
                ValueValidator::Any,
            ),
            OptionDef::multiple(
                "--lib-features",
                "Features to enable for library compilation",
                ValueValidator::Any,
            ),
        ],
    );

    options
}
