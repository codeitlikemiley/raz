use crate::{
    CommandConfig, FilterConfig, ProviderConfig, RazConfig, UiConfig,
    error::{ConfigError, Result},
    override_config::OverrideCollection,
    schema::ConfigVersion,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

static WORKSPACE_CONFIG_CACHE: Lazy<Mutex<HashMap<PathBuf, WorkspaceConfig>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub raz: Option<RazConfig>,
    pub providers_config: Option<ProviderConfig>,
    pub filters: Option<FilterConfig>,
    pub ui: Option<UiConfig>,
    pub commands: Option<Vec<CommandConfig>>,
    pub overrides: Option<OverrideCollection>,
    pub extends: Option<PathBuf>,
    #[serde(skip)]
    pub path: PathBuf,
}

impl WorkspaceConfig {
    pub fn new(workspace_path: PathBuf) -> Self {
        Self {
            raz: None,
            providers_config: None,
            filters: None,
            ui: None,
            commands: None,
            overrides: None,
            extends: None,
            path: workspace_path,
        }
    }

    pub fn load(workspace_path: impl AsRef<Path>) -> Result<Option<Self>> {
        let workspace_path = workspace_path.as_ref();
        let config_path = Self::find_config_path(workspace_path)?;

        let Some(config_path) = config_path else {
            return Ok(None);
        };

        {
            let cache = WORKSPACE_CONFIG_CACHE.lock().unwrap();
            if let Some(cached) = cache.get(&config_path) {
                return Ok(Some(cached.clone()));
            }
        }

        let contents = std::fs::read_to_string(&config_path)?;
        let mut config: Self = toml::from_str(&contents)?;
        config.path = config_path.parent().unwrap().to_path_buf();

        if let Some(extends_path) = &config.extends {
            let base_path = if extends_path.is_relative() {
                config.path.join(extends_path)
            } else {
                extends_path.clone()
            };

            if let Some(base_config) = Self::load(&base_path)? {
                config = config.merge_with_base(base_config);
            }
        }

        config.validate()?;

        {
            let mut cache = WORKSPACE_CONFIG_CACHE.lock().unwrap();
            cache.insert(config_path, config.clone());
        }

        Ok(Some(config))
    }

    pub fn save(&self) -> Result<()> {
        let config_path = self.path.join(crate::WORKSPACE_CONFIG_FILENAME);

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let contents = toml::to_string_pretty(self)?;
        std::fs::write(&config_path, contents)?;

        {
            let mut cache = WORKSPACE_CONFIG_CACHE.lock().unwrap();
            cache.insert(config_path, self.clone());
        }

        Ok(())
    }

    fn find_config_path(start_path: &Path) -> Result<Option<PathBuf>> {
        let mut current = start_path;

        loop {
            let config_path = current.join(crate::WORKSPACE_CONFIG_FILENAME);
            if config_path.exists() {
                return Ok(Some(config_path));
            }

            if let Some(parent) = current.parent() {
                current = parent;
            } else {
                break;
            }
        }

        Ok(None)
    }

    pub fn validate(&self) -> Result<()> {
        if let Some(raz_config) = &self.raz {
            if raz_config.version.needs_migration(&ConfigVersion::CURRENT) {
                return Err(ConfigError::VersionMismatch {
                    expected: ConfigVersion::CURRENT.0,
                    found: raz_config.version.0,
                });
            }
        }

        if let Some(extends_path) = &self.extends {
            if extends_path.canonicalize()? == self.path.canonicalize()? {
                return Err(ConfigError::CyclicInheritance);
            }
        }

        Ok(())
    }

    fn merge_with_base(mut self, base: WorkspaceConfig) -> Self {
        if self.raz.is_none() && base.raz.is_some() {
            self.raz = base.raz;
        }

        if self.providers_config.is_none() && base.providers_config.is_some() {
            self.providers_config = base.providers_config;
        }

        if self.filters.is_none() && base.filters.is_some() {
            self.filters = base.filters;
        }

        if self.ui.is_none() && base.ui.is_some() {
            self.ui = base.ui;
        }

        if self.commands.is_none() && base.commands.is_some() {
            self.commands = base.commands;
        } else if let (Some(mut commands), Some(base_commands)) =
            (self.commands.take(), base.commands)
        {
            let mut merged = base_commands;
            merged.append(&mut commands);
            self.commands = Some(merged);
        }

        if self.overrides.is_none() && base.overrides.is_some() {
            self.overrides = base.overrides;
        } else if let (Some(overrides), Some(base_overrides)) =
            (&mut self.overrides, base.overrides)
        {
            for (key, override_config) in base_overrides.overrides {
                overrides.overrides.entry(key).or_insert(override_config);
            }
        }

        self
    }

    pub fn clear_cache() {
        let mut cache = WORKSPACE_CONFIG_CACHE.lock().unwrap();
        cache.clear();
    }

    pub fn invalidate_cache_for(path: &Path) {
        let mut cache = WORKSPACE_CONFIG_CACHE.lock().unwrap();
        cache.retain(|k, _| !k.starts_with(path));
    }
}
