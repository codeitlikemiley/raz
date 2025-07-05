use crate::{
    CommandConfig, FilterConfig, ProviderConfig, RazConfig, UiConfig,
    global::GlobalConfig,
    override_config::{CommandOverride, OverrideCollection},
    workspace::WorkspaceConfig,
};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct EffectiveConfig {
    pub raz: RazConfig,
    pub providers_config: ProviderConfig,
    pub filters: FilterConfig,
    pub ui: UiConfig,
    pub commands: Vec<CommandConfig>,
    pub overrides: OverrideCollection,
}

impl EffectiveConfig {
    pub fn new(global: GlobalConfig, workspace: Option<WorkspaceConfig>) -> Self {
        let mut effective = Self::from_global(global);

        if let Some(workspace) = workspace {
            effective.merge_workspace(workspace);
        }

        effective
    }

    fn from_global(global: GlobalConfig) -> Self {
        Self {
            raz: global.raz,
            providers_config: global.providers_config.unwrap_or_default(),
            filters: global.filters.unwrap_or_else(|| FilterConfig {
                priority_commands: vec![],
                ignore_commands: vec![],
                auto_fix: Some(false),
                show_warnings: Some(true),
            }),
            ui: global.ui.unwrap_or_else(|| UiConfig {
                color: Some(true),
                progress: Some(true),
                verbose: Some(false),
                format: Some("default".to_string()),
            }),
            commands: global.commands.unwrap_or_default(),
            overrides: global.saved_overrides.unwrap_or_default(),
        }
    }

    fn merge_workspace(&mut self, workspace: WorkspaceConfig) {
        if let Some(raz_config) = workspace.raz {
            if raz_config.enabled != self.raz.enabled {
                self.raz.enabled = raz_config.enabled;
            }

            if !raz_config.providers.is_empty() {
                self.raz.providers = raz_config.providers;
            }

            if raz_config.cache_dir.is_some() {
                self.raz.cache_dir = raz_config.cache_dir;
            }

            if raz_config.cache_ttl.is_some() {
                self.raz.cache_ttl = raz_config.cache_ttl;
            }

            if raz_config.parallel_execution.is_some() {
                self.raz.parallel_execution = raz_config.parallel_execution;
            }

            if raz_config.max_concurrent_jobs.is_some() {
                self.raz.max_concurrent_jobs = raz_config.max_concurrent_jobs;
            }
        }

        if let Some(providers) = workspace.providers_config {
            self.merge_provider_config(providers);
        }

        if let Some(filters) = workspace.filters {
            self.merge_filter_config(filters);
        }

        if let Some(ui) = workspace.ui {
            self.merge_ui_config(ui);
        }

        if let Some(mut commands) = workspace.commands {
            self.commands.append(&mut commands);
        }

        if let Some(overrides) = workspace.overrides {
            for (key, override_config) in overrides.overrides {
                self.overrides.overrides.insert(key, override_config);
            }
        }
    }

    fn merge_provider_config(&mut self, providers: ProviderConfig) {
        if providers.cargo.is_some() {
            self.providers_config.cargo = providers.cargo;
        }

        if providers.rustc.is_some() {
            self.providers_config.rustc = providers.rustc;
        }

        if providers.leptos.is_some() {
            self.providers_config.leptos = providers.leptos;
        }

        if providers.dioxus.is_some() {
            self.providers_config.dioxus = providers.dioxus;
        }

        if providers.bevy.is_some() {
            self.providers_config.bevy = providers.bevy;
        }

        if providers.custom.is_some() {
            self.providers_config.custom = providers.custom;
        }
    }

    fn merge_filter_config(&mut self, filters: FilterConfig) {
        if !filters.priority_commands.is_empty() {
            self.filters.priority_commands = filters.priority_commands;
        }

        if !filters.ignore_commands.is_empty() {
            self.filters.ignore_commands = filters.ignore_commands;
        }

        if filters.auto_fix.is_some() {
            self.filters.auto_fix = filters.auto_fix;
        }

        if filters.show_warnings.is_some() {
            self.filters.show_warnings = filters.show_warnings;
        }
    }

    fn merge_ui_config(&mut self, ui: UiConfig) {
        if ui.color.is_some() {
            self.ui.color = ui.color;
        }

        if ui.progress.is_some() {
            self.ui.progress = ui.progress;
        }

        if ui.verbose.is_some() {
            self.ui.verbose = ui.verbose;
        }

        if ui.format.is_some() {
            self.ui.format = ui.format;
        }
    }

    pub fn apply_runtime_override(&mut self, override_config: CommandOverride) {
        let key = override_config.generate_key();
        self.overrides.overrides.insert(key, override_config);
    }

    pub fn find_override(
        &self,
        file: Option<&PathBuf>,
        function: Option<&str>,
        line: Option<usize>,
    ) -> Option<&CommandOverride> {
        self.overrides.find_best_match(file, function, line)
    }

    pub fn is_provider_enabled(&self, provider: &str) -> bool {
        self.raz.providers.iter().any(|p| p == provider)
    }

    pub fn get_provider_config(&self, provider: &str) -> Option<&toml::Value> {
        match provider {
            "cargo" => self.providers_config.cargo.as_ref(),
            "rustc" => self.providers_config.rustc.as_ref(),
            "leptos" => self.providers_config.leptos.as_ref(),
            "dioxus" => self.providers_config.dioxus.as_ref(),
            "bevy" => self.providers_config.bevy.as_ref(),
            _ => self.providers_config.custom.as_ref(),
        }
    }

    pub fn should_ignore_command(&self, command: &str) -> bool {
        self.filters.ignore_commands.iter().any(|c| c == command)
    }

    pub fn is_priority_command(&self, command: &str) -> bool {
        self.filters.priority_commands.iter().any(|c| c == command)
    }
}
