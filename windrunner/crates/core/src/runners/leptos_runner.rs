//! Leptos runner — wraps CargoRunner with cargo-leptos detection.
//!
//! Detects Leptos projects (presence of `leptos` in `Cargo.toml`) and
//! surfaces that information for logging and future dispatch routing.
//! Full `cargo leptos watch/build` command generation is configured via
//! `.cargo-runner.json`.

use std::path::Path;

use crate::{
    command::CargoCommand,
    config::Config,
    error::Result,
    types::{FileType, Runnable},
};

use super::{cargo_runner::CargoRunner, traits::CommandRunner};

/// Runner for Leptos projects managed by `cargo-leptos`.
///
/// For now this delegates everything to `CargoRunner` because detailed
/// `cargo leptos` command generation is handled via `.cargo-runner.json`
/// overrides. The primary value today is `detect()`.
pub struct LeptosRunner {
    base: CargoRunner,
}

impl LeptosRunner {
    pub fn new() -> Result<Self> {
        Ok(Self {
            base: CargoRunner::new()?,
        })
    }

    /// Returns `true` when any `Cargo.toml` in ancestor directories contains
    /// the string `"leptos"`, indicating a Leptos project.
    pub fn detect(file_path: &Path) -> bool {
        for ancestor in file_path.ancestors() {
            let cargo_toml = ancestor.join("Cargo.toml");
            if cargo_toml.exists() {
                match std::fs::read_to_string(&cargo_toml) {
                    Ok(content) if content.contains("leptos") => return true,
                    _ => {}
                }
            }
        }
        false
    }
}

impl CommandRunner for LeptosRunner {
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
        self.base.build_command(runnable, config, file_type)
    }

    fn validate_command(&self, command: &CargoCommand) -> Result<()> {
        self.base.validate_command(command)
    }

    fn name(&self) -> &'static str {
        "leptos"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn detects_leptos_in_cargo_toml() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("Cargo.toml"),
            "[dependencies]\nleptos = \"0.6\"\n",
        )
        .unwrap();
        let file = tmp.path().join("src/lib.rs");
        assert!(LeptosRunner::detect(&file));
    }

    #[test]
    fn non_leptos_cargo_toml_returns_false() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("Cargo.toml"),
            "[dependencies]\naxum = \"0.7\"\n",
        )
        .unwrap();
        let file = tmp.path().join("src/lib.rs");
        assert!(!LeptosRunner::detect(&file));
    }
}
