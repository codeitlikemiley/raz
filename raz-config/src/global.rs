use crate::{
    CommandConfig, FilterConfig, ProviderConfig, RazConfig, UiConfig,
    error::{ConfigError, Result},
    override_config::{OverrideCollection, OverrideSettings},
    schema::ConfigVersion,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    pub raz: RazConfig,
    pub providers_config: Option<ProviderConfig>,
    pub filters: Option<FilterConfig>,
    pub ui: Option<UiConfig>,
    pub commands: Option<Vec<CommandConfig>>,
    pub overrides: Option<OverrideSettings>,
    pub saved_overrides: Option<OverrideCollection>,
}

impl GlobalConfig {
    pub fn new() -> Self {
        Self {
            raz: RazConfig {
                version: ConfigVersion::CURRENT,
                enabled: true,
                providers: vec!["cargo".to_string(), "rustc".to_string()],
                cache_dir: None,
                cache_ttl: Some(3600),
                parallel_execution: Some(true),
                max_concurrent_jobs: Some(4),
            },
            providers_config: None,
            filters: None,
            ui: None,
            commands: None,
            overrides: None,
            saved_overrides: None,
        }
    }

    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            return Ok(Self::new());
        }

        let contents = std::fs::read_to_string(&config_path)?;
        let config: Self = toml::from_str(&contents)?;

        config.validate()?;

        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let contents = toml::to_string_pretty(self)?;
        std::fs::write(&config_path, contents)?;

        Ok(())
    }

    pub fn config_path() -> Result<PathBuf> {
        let home = dirs::home_dir().ok_or(ConfigError::InvalidHomeDir)?;
        Ok(home.join(".config").join("raz").join("config.toml"))
    }

    pub fn validate(&self) -> Result<()> {
        if self.raz.version.needs_migration(&ConfigVersion::CURRENT) {
            return Err(ConfigError::VersionMismatch {
                expected: ConfigVersion::CURRENT.0,
                found: self.raz.version.0,
            });
        }

        if self.raz.providers.is_empty() {
            return Err(ConfigError::ValidationError(
                "At least one provider must be enabled".to_string(),
            ));
        }

        if let Some(ttl) = self.raz.cache_ttl {
            if ttl == 0 {
                return Err(ConfigError::ValidationError(
                    "Cache TTL must be greater than 0".to_string(),
                ));
            }
        }

        if let Some(jobs) = self.raz.max_concurrent_jobs {
            if jobs == 0 {
                return Err(ConfigError::ValidationError(
                    "Max concurrent jobs must be greater than 0".to_string(),
                ));
            }
        }

        Ok(())
    }

    pub fn merge_with(&mut self, other: GlobalConfig) {
        self.raz.enabled = other.raz.enabled;

        if !other.raz.providers.is_empty() {
            self.raz.providers = other.raz.providers;
        }

        if other.raz.cache_dir.is_some() {
            self.raz.cache_dir = other.raz.cache_dir;
        }

        if other.raz.cache_ttl.is_some() {
            self.raz.cache_ttl = other.raz.cache_ttl;
        }

        if other.raz.parallel_execution.is_some() {
            self.raz.parallel_execution = other.raz.parallel_execution;
        }

        if other.raz.max_concurrent_jobs.is_some() {
            self.raz.max_concurrent_jobs = other.raz.max_concurrent_jobs;
        }

        if other.providers_config.is_some() {
            self.providers_config = other.providers_config;
        }

        if other.filters.is_some() {
            self.filters = other.filters;
        }

        if other.ui.is_some() {
            self.ui = other.ui;
        }

        if other.commands.is_some() {
            self.commands = other.commands;
        }

        if other.overrides.is_some() {
            self.overrides = other.overrides;
        }

        if other.saved_overrides.is_some() {
            self.saved_overrides = other.saved_overrides;
        }
    }
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self::new()
    }
}
