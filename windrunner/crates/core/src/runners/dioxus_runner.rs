//! Dioxus runner — wraps CargoRunner with native `dx` CLI dispatch.
//!
//! Detects Dioxus projects (presence of `Dioxus.toml` in any ancestor) and
//! routes Binary runnables through `dx serve` instead of `cargo run`.
//! Tests and benchmarks still go through CargoRunner since `cargo test` works.

use std::path::Path;

use crate::{
    command::{CargoCommand, CommandType},
    config::Config,
    error::Result,
    types::{FileType, Runnable, RunnableKind},
};

use super::{cargo_runner::CargoRunner, traits::CommandRunner};

/// Runner for Dioxus `dx`-managed projects.
///
/// When a Binary runnable is detected in a Dioxus project, this runner
/// generates `dx serve` instead of `cargo run`. For tests and benchmarks,
/// it delegates to `CargoRunner` since `cargo test` works correctly.
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
        match &runnable.kind {
            // Binary targets → dx serve (or override)
            RunnableKind::Binary { .. } => {
                // Check for overrides that customize the dx command
                let identity = crate::types::FunctionIdentity {
                    package: config.cargo.as_ref().and_then(|c| c.package.clone()),
                    module_path: if runnable.module_path.is_empty() {
                        None
                    } else {
                        Some(runnable.module_path.clone())
                    },
                    file_path: Some(runnable.file_path.clone()),
                    function_name: runnable.get_function_name(),
                    file_type: Some(file_type),
                };

                let override_config = config.get_override_for(&identity);

                // Defaults
                let mut command = "dx".to_string();
                let mut subcommand = "serve".to_string();
                let mut extra_args: Vec<String> = Vec::new();
                let mut env: Vec<(String, String)> = Vec::new();

                // Apply overrides from .cargo-runner.json
                if let Some(ov) = override_config.and_then(|o| o.cargo.as_ref()) {
                    if let Some(cmd) = &ov.command {
                        command = cmd.clone();
                    }
                    if let Some(sub) = &ov.subcommand {
                        subcommand = sub.clone();
                    }
                    if let Some(args) = &ov.extra_args {
                        extra_args.extend(args.clone());
                    }
                    if let Some(extra_env) = &ov.extra_env {
                        for (k, v) in extra_env {
                            env.push((k.clone(), v.clone()));
                        }
                    }
                }

                tracing::info!(
                    "DioxusRunner: Binary detected — generating `{} {}`",
                    command,
                    subcommand
                );

                let mut args = vec![command, subcommand];
                args.extend(extra_args);

                let cmd = CargoCommand {
                    command_type: CommandType::Shell,
                    args,
                    working_dir: None,
                    env,
                    test_filter: None,
                };
                Ok(cmd)
            }
            // Tests, doctests, benchmarks → delegate to CargoRunner
            // (cargo test works fine for Dioxus projects)
            _ => self.base.build_command(runnable, config, file_type),
        }
    }

    fn validate_command(&self, command: &CargoCommand) -> Result<()> {
        // Shell commands (dx serve) don't need cargo-specific validation
        if command.command_type == CommandType::Shell {
            return Ok(());
        }
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

    #[test]
    fn binary_runnable_produces_dx_serve() {
        use crate::types::{Position, Scope, ScopeKind};

        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("Dioxus.toml"), "").unwrap();
        std::fs::write(tmp.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();

        let runner = DioxusRunner {
            base: CargoRunner::new().unwrap(),
        };

        let runnable = Runnable {
            label: "main".to_string(),
            kind: RunnableKind::Binary { bin_name: None },
            file_path: tmp.path().join("src/main.rs"),
            module_path: String::new(),
            scope: Scope {
                start: Position { line: 0, character: 0 },
                end: Position { line: 10, character: 0 },
                kind: ScopeKind::Function,
                name: Some("main".to_string()),
            },
            extended_scope: None,
        };

        let config = Config::default();
        let cmd = runner
            .build_command(&runnable, &config, FileType::CargoProject)
            .unwrap();

        assert_eq!(cmd.command_type, CommandType::Shell);
        assert_eq!(cmd.to_shell_command(), "dx serve");
    }
}
