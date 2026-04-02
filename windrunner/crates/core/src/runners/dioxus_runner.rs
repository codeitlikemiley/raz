//! Dioxus runner — wraps CargoRunner with dx CLI detection.
//!
//! Detects Dioxus projects (presence of `Dioxus.toml` in any ancestor) and
//! surfaces that information for logging and future dispatch routing. Full
//! `dx serve` command generation is configured via `.cargo-runner.json`.

use std::path::Path;

use crate::{
    command::CargoCommand,
    config::Config,
    error::Result,
    types::{FileType, Runnable},
};

use super::{cargo_runner::CargoRunner, traits::CommandRunner};

/// Runner for Dioxus `dx`-managed projects.
///
/// For now this delegates everything to `CargoRunner` because detailed `dx`
/// command generation is handled via `.cargo-runner.json` overrides.
/// The primary value today is `detect()`, which lets `UnifiedRunner` log
/// and later route commands through the `dx` CLI.
pub struct DioxusRunner {
    base: CargoRunner,
}

impl DioxusRunner {
    pub fn new() -> Result<Self> {
        Ok(Self {
            base: CargoRunner::new()?,
        })
    }

    /// Returns `true` when `file_path` (or any ancestor) contains _both_
    /// a `Cargo.toml` and a `Dioxus.toml`, identifying a Dioxus project.
    pub fn detect(file_path: &Path) -> bool {
        let mut has_cargo = false;
        let mut has_dioxus = false;

        for ancestor in file_path.ancestors() {
            if ancestor.join("Dioxus.toml").exists() {
                has_dioxus = true;
            }
            if ancestor.join("Cargo.toml").exists() {
                has_cargo = true;
            }
            if has_cargo && has_dioxus {
                return true;
            }
        }
        false
    }
}

impl CommandRunner for DioxusRunner {
    type Config = Config;
    type Command = CargoCommand;

    fn detect_runnables(&self, file_path: &Path) -> Result<Vec<Runnable>> {
        self.base.detect_runnables(file_path)
    }

    fn get_runnable_at_line(&self, file_path: &Path, line: u32) -> Result<Option<Runnable>> {
        self.base.get_runnable_at_line(file_path, line)
    }

    fn build_command(
        &self,
        runnable: &Runnable,
        config: &Config,
        file_type: FileType,
    ) -> Result<CargoCommand> {
        // Delegate to CargoRunner; dx-specific overrides live in .cargo-runner.json
        self.base.build_command(runnable, config, file_type)
    }

    fn validate_command(&self, command: &CargoCommand) -> Result<()> {
        self.base.validate_command(command)
    }

    fn name(&self) -> &'static str {
        "dioxus"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn detects_dioxus_toml_in_ancestor() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("Dioxus.toml"), "").unwrap();
        std::fs::write(tmp.path().join("Cargo.toml"), "").unwrap();

        let sub = tmp.path().join("src");
        std::fs::create_dir_all(&sub).unwrap();
        let file = sub.join("main.rs");

        assert!(DioxusRunner::detect(&file));
    }

    #[test]
    fn no_dioxus_toml_returns_false() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("Cargo.toml"), "").unwrap();

        let file = tmp.path().join("src/lib.rs");
        assert!(!DioxusRunner::detect(&file));
    }
}
