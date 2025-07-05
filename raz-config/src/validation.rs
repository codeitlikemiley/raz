use crate::{
    GlobalConfig, WorkspaceConfig,
    error::{ConfigError, Result},
    schema::{ConfigSchema, ConfigVersion},
};

pub struct ConfigValidator {
    schema: ConfigSchema,
}

impl ConfigValidator {
    pub fn new(version: ConfigVersion) -> Self {
        Self {
            schema: ConfigSchema::for_version(version),
        }
    }

    pub fn validate_global(&self, config: &GlobalConfig) -> Result<ValidationReport> {
        let mut report = ValidationReport::new();

        if config.raz.version != self.schema.version {
            report.add_error(format!(
                "Config version mismatch: expected {}, found {}",
                self.schema.version, config.raz.version
            ));
        }

        if config.raz.providers.is_empty() {
            report.add_error("No providers enabled".to_string());
        }

        for provider in &config.raz.providers {
            if !Self::is_valid_provider(provider) {
                report.add_warning(format!("Unknown provider: {provider}"));
            }
        }

        if let Some(cache_ttl) = config.raz.cache_ttl {
            if cache_ttl == 0 {
                report.add_error("Cache TTL must be greater than 0".to_string());
            } else if cache_ttl > 86400 {
                report.add_warning("Cache TTL is very high (> 24 hours)".to_string());
            }
        }

        if let Some(jobs) = config.raz.max_concurrent_jobs {
            if jobs == 0 {
                report.add_error("Max concurrent jobs must be greater than 0".to_string());
            } else if jobs > 32 {
                report.add_warning(
                    "Very high concurrent job limit may cause system instability".to_string(),
                );
            }
        }

        if let Some(cache_dir) = &config.raz.cache_dir {
            if !cache_dir.is_absolute() {
                report.add_error("Cache directory must be an absolute path".to_string());
            }
        }

        self.validate_commands(&config.commands, &mut report);

        if report.has_errors() {
            Err(ConfigError::ValidationError(report.format_errors()))
        } else {
            Ok(report)
        }
    }

    pub fn validate_workspace(&self, config: &WorkspaceConfig) -> Result<ValidationReport> {
        let mut report = ValidationReport::new();

        if let Some(raz_config) = &config.raz {
            if raz_config.version != self.schema.version {
                report.add_error(format!(
                    "Config version mismatch: expected {}, found {}",
                    self.schema.version, raz_config.version
                ));
            }
        }

        if let Some(extends) = &config.extends {
            if !extends.exists() {
                report.add_error(format!(
                    "Extended config file not found: {}",
                    extends.display()
                ));
            }
        }

        self.validate_commands(&config.commands, &mut report);

        if report.has_errors() {
            Err(ConfigError::ValidationError(report.format_errors()))
        } else {
            Ok(report)
        }
    }

    fn validate_commands(
        &self,
        commands: &Option<Vec<crate::CommandConfig>>,
        report: &mut ValidationReport,
    ) {
        if let Some(commands) = commands {
            let mut seen_names = std::collections::HashSet::new();

            for cmd in commands {
                if !seen_names.insert(&cmd.name) {
                    report.add_error(format!("Duplicate command name: {}", cmd.name));
                }

                if cmd.command.is_empty() {
                    report.add_error(format!("Empty command for: {}", cmd.name));
                }

                if let Some(working_dir) = &cmd.working_dir {
                    if !working_dir.is_absolute() {
                        report.add_warning(format!(
                            "Command '{}' uses relative working directory: {}",
                            cmd.name,
                            working_dir.display()
                        ));
                    }
                }
            }
        }
    }

    fn is_valid_provider(provider: &str) -> bool {
        matches!(
            provider,
            "cargo" | "rustc" | "leptos" | "dioxus" | "bevy" | "custom"
        )
    }
}

#[derive(Debug)]
pub struct ValidationReport {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationReport {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }

    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    pub fn format_errors(&self) -> String {
        self.errors.join("; ")
    }

    pub fn format_warnings(&self) -> String {
        self.warnings.join("; ")
    }
}

impl Default for ValidationReport {
    fn default() -> Self {
        Self::new()
    }
}
