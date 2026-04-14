//! Configuration management for cargo-runner

mod bazel_config;
mod cargo_config;
mod features;
mod merge;
pub mod override_config;
pub mod override_manager;
mod plugin_policy;
mod rustc_config;
mod settings;
pub mod test_framework;
pub mod utils;

// Re-export main types
pub use bazel_config::{BazelConfig, BazelFramework, BazelOverride};
pub use cargo_config::{BinaryFramework, CargoConfig, SingleFileScriptConfig};
pub use features::Features;
pub use merge::{ConfigInfo, ConfigMerger};
pub use override_config::Override;
pub use override_manager::OverrideManager;
pub use plugin_policy::PluginPolicy;
pub use rustc_config::{RustcConfig, RustcFramework, RustcPhaseConfig};
pub use settings::Config;
pub use test_framework::TestFramework;
pub use utils::is_valid_channel;
