#![doc = include_str!("../README.md")]

pub mod builder;
pub mod effective;
pub mod error;
pub mod global;
pub mod hierarchy;
pub mod migration;
pub mod override_config;
pub mod schema;
pub mod templates;
pub mod validation;
pub mod workspace;

pub use templates::ConfigTemplates;

pub use builder::{CommandConfigBuilder, ConfigBuilder, OverrideBuilder};
pub use effective::EffectiveConfig;
pub use error::{ConfigError, Result};
pub use global::GlobalConfig;
pub use hierarchy::{ConfigHierarchy, ConfigLevel, ConfigLocation};
pub use migration::ConfigMigrator;
pub use override_config::{CommandOverride, OverrideMode, OverrideSettings};
pub use schema::{ConfigSchema, ConfigVersion};
pub use validation::ConfigValidator;
pub use workspace::WorkspaceConfig;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RazConfig {
    pub version: ConfigVersion,
    pub enabled: bool,
    pub providers: Vec<String>,
    pub cache_dir: Option<PathBuf>,
    pub cache_ttl: Option<u64>,
    pub parallel_execution: Option<bool>,
    pub max_concurrent_jobs: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderConfig {
    pub cargo: Option<toml::Value>,
    pub rustc: Option<toml::Value>,
    pub leptos: Option<toml::Value>,
    pub dioxus: Option<toml::Value>,
    pub bevy: Option<toml::Value>,
    pub custom: Option<toml::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterConfig {
    pub priority_commands: Vec<String>,
    pub ignore_commands: Vec<String>,
    pub auto_fix: Option<bool>,
    pub show_warnings: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub color: Option<bool>,
    pub progress: Option<bool>,
    pub verbose: Option<bool>,
    pub format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: Option<indexmap::IndexMap<String, String>>,
    pub working_dir: Option<PathBuf>,
    pub description: Option<String>,
}

pub const GLOBAL_CONFIG_PATH: &str = "~/.config/raz/config.toml";
pub const WORKSPACE_CONFIG_FILENAME: &str = ".raz/config.toml";
pub const CONFIG_CURRENT_VERSION: u32 = 1;
