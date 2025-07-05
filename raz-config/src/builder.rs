use crate::{
    CommandConfig, FilterConfig, GlobalConfig, ProviderConfig, UiConfig,
    override_config::{CommandOverride, OverrideCollection, OverrideMode},
    schema::ConfigVersion,
};
use std::path::PathBuf;

pub struct ConfigBuilder {
    config: GlobalConfig,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        let mut config = GlobalConfig::new();
        config.raz.providers.clear(); // Start with empty providers
        Self { config }
    }

    pub fn version(mut self, version: ConfigVersion) -> Self {
        self.config.raz.version = version;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.config.raz.enabled = enabled;
        self
    }

    pub fn provider(mut self, provider: impl Into<String>) -> Self {
        self.config.raz.providers.push(provider.into());
        self
    }

    pub fn providers(mut self, providers: Vec<String>) -> Self {
        self.config.raz.providers = providers;
        self
    }

    pub fn cache_dir(mut self, path: PathBuf) -> Self {
        self.config.raz.cache_dir = Some(path);
        self
    }

    pub fn cache_ttl(mut self, seconds: u64) -> Self {
        self.config.raz.cache_ttl = Some(seconds);
        self
    }

    pub fn parallel_execution(mut self, enabled: bool) -> Self {
        self.config.raz.parallel_execution = Some(enabled);
        self
    }

    pub fn max_concurrent_jobs(mut self, jobs: usize) -> Self {
        self.config.raz.max_concurrent_jobs = Some(jobs);
        self
    }

    pub fn provider_config(mut self, config: ProviderConfig) -> Self {
        self.config.providers_config = Some(config);
        self
    }

    pub fn filter_config(mut self, config: FilterConfig) -> Self {
        self.config.filters = Some(config);
        self
    }

    pub fn ui_config(mut self, config: UiConfig) -> Self {
        self.config.ui = Some(config);
        self
    }

    pub fn command(mut self, command: CommandConfig) -> Self {
        if self.config.commands.is_none() {
            self.config.commands = Some(Vec::new());
        }
        self.config.commands.as_mut().unwrap().push(command);
        self
    }

    pub fn override_config(mut self, override_config: CommandOverride) -> Self {
        if self.config.saved_overrides.is_none() {
            self.config.saved_overrides = Some(OverrideCollection::new());
        }
        self.config
            .saved_overrides
            .as_mut()
            .unwrap()
            .add(override_config);
        self
    }

    pub fn build(self) -> GlobalConfig {
        self.config
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CommandConfigBuilder {
    config: CommandConfig,
}

impl CommandConfigBuilder {
    pub fn new(name: impl Into<String>, command: impl Into<String>) -> Self {
        Self {
            config: CommandConfig {
                name: name.into(),
                command: command.into(),
                args: Vec::new(),
                env: None,
                working_dir: None,
                description: None,
            },
        }
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.config.args.push(arg.into());
        self
    }

    pub fn args(mut self, args: Vec<String>) -> Self {
        self.config.args = args;
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        if self.config.env.is_none() {
            self.config.env = Some(indexmap::IndexMap::new());
        }
        self.config
            .env
            .as_mut()
            .unwrap()
            .insert(key.into(), value.into());
        self
    }

    pub fn working_dir(mut self, path: PathBuf) -> Self {
        self.config.working_dir = Some(path);
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.config.description = Some(desc.into());
        self
    }

    pub fn build(self) -> CommandConfig {
        self.config
    }
}

pub struct OverrideBuilder {
    override_config: CommandOverride,
}

impl OverrideBuilder {
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            override_config: CommandOverride::new(key.into()),
        }
    }

    pub fn file(mut self, path: PathBuf) -> Self {
        self.override_config.file_path = Some(path);
        self
    }

    pub fn function(mut self, name: impl Into<String>) -> Self {
        self.override_config.function_name = Some(name.into());
        self
    }

    pub fn line_range(mut self, start: usize, end: usize) -> Self {
        self.override_config.line_range = Some((start, end));
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.override_config.env.insert(key.into(), value.into());
        self
    }

    pub fn cargo_option(mut self, option: impl Into<String>) -> Self {
        self.override_config.cargo_options.push(option.into());
        self
    }

    pub fn rustc_option(mut self, option: impl Into<String>) -> Self {
        self.override_config.rustc_options.push(option.into());
        self
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.override_config.args.push(arg.into());
        self
    }

    pub fn mode(mut self, mode: OverrideMode) -> Self {
        self.override_config.mode = mode;
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.override_config.description = Some(desc.into());
        self
    }

    pub fn build(self) -> CommandOverride {
        self.override_config
    }
}
