//! Unified runner that manages all specific runners

mod build_system_detector;
mod file_command;
mod project_command;

use std::collections::HashMap;
use std::sync::Arc;

use crate::{build_system::BuildSystem, config::Config, error::Result, plugins::PluginRegistry};

use super::{bazel_runner::BazelRunner, cargo_runner::CargoRunner, traits::CommandRunner};

/// Unified runner that manages multiple command runners
pub struct UnifiedRunner {
    pub(crate) runners: HashMap<
        BuildSystem,
        Box<dyn CommandRunner<Config = Config, Command = crate::command::Command>>,
    >,
    pub(crate) plugins: PluginRegistry,
    pub(crate) config: Arc<Config>,
}

impl UnifiedRunner {
    /// Create a new unified runner with all available runners
    pub fn new() -> Result<Self> {
        let runners = init_runners()?;
        let config = Config::load()?;
        let plugins = PluginRegistry::with_defaults();

        Ok(Self {
            runners,
            plugins,
            config: Arc::new(config),
        })
    }

    /// Create with a specific config
    pub fn with_config(config: Config) -> Result<Self> {
        let runners = init_runners()?;
        let plugins = PluginRegistry::with_defaults();

        Ok(Self {
            runners,
            plugins,
            config: Arc::new(config),
        })
    }

    /// Get the appropriate runner for a build system
    pub fn get_runner(
        &self,
        build_system: &BuildSystem,
    ) -> Result<&dyn CommandRunner<Config = Config, Command = crate::command::Command>> {
        self.runners
            .get(build_system)
            .map(|r| r.as_ref())
            .ok_or_else(|| crate::error::Error::NoRunner(format!("{build_system:?}")))
    }

    /// Get the current configuration
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Update the configuration
    pub fn set_config(&mut self, config: Config) {
        self.config = Arc::new(config);
    }
}

// Convenience methods that mirror the old CargoRunner API for backward compatibility
fn init_runners() -> Result<
    HashMap<
        BuildSystem,
        Box<dyn CommandRunner<Config = Config, Command = crate::command::Command>>,
    >,
> {
    let mut runners = HashMap::new();
    runners.insert(
        BuildSystem::Cargo,
        Box::new(CargoRunner::new()?)
            as Box<dyn CommandRunner<Config = Config, Command = crate::command::Command>>,
    );
    runners.insert(
        BuildSystem::Bazel,
        Box::new(BazelRunner::new()?)
            as Box<dyn CommandRunner<Config = Config, Command = crate::command::Command>>,
    );
    Ok(runners)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    #[test]
    fn detect_build_system_finds_cargo_outside_home_boundary() {
        let temp_dir = TempDir::new().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(
            temp_dir.path().join("Cargo.toml"),
            r#"[package]
name = "outside-home"
version = "0.1.0"
edition = "2021"
"#,
        )
        .unwrap();
        fs::write(src_dir.join("main.rs"), "fn main() {}\n").unwrap();

        let runner = UnifiedRunner::new().unwrap();
        let build_system = runner
            .detect_build_system(&src_dir.join("main.rs"))
            .unwrap();

        assert_eq!(build_system, BuildSystem::Cargo);
    }

    #[test]
    fn detect_build_system_finds_bazel_outside_home_boundary() {
        let temp_dir = TempDir::new().unwrap();
        let app_dir = temp_dir.path().join("app");
        fs::create_dir_all(&app_dir).unwrap();
        fs::write(app_dir.join("BUILD.bazel"), "rust_binary(name = \"app\")\n").unwrap();
        fs::write(app_dir.join("main.rs"), "fn main() {}\n").unwrap();

        let runner = UnifiedRunner::new().unwrap();
        let build_system = runner
            .detect_build_system(&app_dir.join("main.rs"))
            .unwrap();

        assert_eq!(build_system, BuildSystem::Bazel);
    }

    #[test]
    fn get_file_command_falls_back_to_cargo_script() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(
            temp_dir.path().join("Cargo.toml"),
            r#"[package]
name = "script-workspace"
version = "0.1.0"
edition = "2021"
"#,
        )
        .unwrap();

        let script_path = temp_dir.path().join("power.rs");
        fs::write(
            &script_path,
            r#"#!/usr/bin/env -S cargo +nightly -Zscript
---cargo
[package]
edition = "2021"

[dependencies]
clap = { version = "4.5", features = ["derive"] }
---
fn main() {
    println!("hello");
}
"#,
        )
        .unwrap();

        let mut runner = UnifiedRunner::new().unwrap();
        let command = runner.get_file_command(Path::new(&script_path)).unwrap();

        let command = command.expect("expected a command for cargo script");
        let shell = command.to_shell_command();
        assert!(shell.contains("cargo"));
        assert!(shell.contains("+nightly"));
        assert!(shell.contains("-Zscript"));
        assert!(shell.contains("power.rs"));
    }

    #[test]
    fn get_file_command_falls_back_to_rust_script() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("power.rs");
        fs::write(
            &script_path,
            r#"#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! anyhow = "1"
//! clap = { version = "4.5", features = ["derive"] }
//! ```
//!
//! [package]
//! edition = "2024"
fn main() {
    println!("hello");
}
"#,
        )
        .unwrap();

        let mut runner = UnifiedRunner::new().unwrap();
        let command = runner.get_file_command(Path::new(&script_path)).unwrap();

        let command = command.expect("expected a command for rust-script");
        let shell = command.to_shell_command();
        assert!(shell.contains("rust-script"));
        assert!(shell.contains("power.rs"));
        assert!(!shell.contains("cargo +nightly -Zscript"));

        let line_command = runner
            .get_command_at_position_with_dir(Path::new(&script_path), Some(0))
            .unwrap();
        assert!(line_command.to_shell_command().contains("rust-script"));
    }
}
