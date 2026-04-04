//! Leptos runner — wraps CargoRunner with native `cargo leptos` CLI dispatch.
//!
//! Detects Leptos projects (presence of `leptos` in `Cargo.toml`) and
//! routes Binary runnables through `cargo leptos serve` instead of `cargo run`.
//! Tests and benchmarks still go through CargoRunner since `cargo test` works.

use std::path::Path;

use crate::{
    command::{CargoCommand, CommandType},
    config::Config,
    error::Result,
    types::{FileType, Runnable, RunnableKind},
};

use super::{cargo_runner::CargoRunner, traits::CommandRunner};

/// Runner for Leptos projects managed by `cargo-leptos`.
///
/// When a Binary runnable is detected in a Leptos project, this runner
/// generates `cargo leptos serve` instead of `cargo run`. For tests and
/// benchmarks, it delegates to `CargoRunner` since `cargo test` works.
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
        match &runnable.kind {
            // Binary targets → cargo leptos serve (or override)
            RunnableKind::Binary { .. } => {
                // Check for overrides that customize the leptos command
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

                // Defaults: cargo leptos serve
                let mut subcommand = "serve".to_string();
                let mut extra_args: Vec<String> = Vec::new();
                let mut env: Vec<(String, String)> = Vec::new();

                // Apply overrides from .cargo-runner.json
                if let Some(ov) = override_config.and_then(|o| o.cargo.as_ref()) {
                    if let Some(sub) = &ov.subcommand {
                        // Strip redundant "leptos" prefix — the runner already adds it.
                        // This handles @cargo.leptos.watch → "leptos watch" → "watch"
                        subcommand = if let Some(stripped) = sub.strip_prefix("leptos ") {
                            stripped.to_string()
                        } else {
                            sub.clone()
                        };
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
                    "LeptosRunner: Binary detected — generating `cargo leptos {}`",
                    subcommand
                );

                let mut args = vec!["leptos".to_string(), subcommand];
                args.extend(extra_args);

                let cmd = CargoCommand {
                    command_type: CommandType::Cargo,
                    args,
                    working_dir: None,
                    env,
                    test_filter: None,
                };
                Ok(cmd)
            }
            // Tests, doctests, benchmarks → delegate to CargoRunner
            _ => self.base.build_command(runnable, config, file_type),
        }
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

    #[test]
    fn binary_runnable_produces_cargo_leptos_serve() {
        use crate::types::{Position, Scope, ScopeKind};

        let tmp = TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("Cargo.toml"),
            "[package]\nname = \"test\"\n[dependencies]\nleptos = \"0.7\"\n",
        )
        .unwrap();

        let runner = LeptosRunner {
            base: CargoRunner::new().unwrap(),
        };

        let runnable = Runnable {
            label: "main".to_string(),
            kind: RunnableKind::Binary { bin_name: None },
            file_path: tmp.path().join("src/main.rs"),
            module_path: String::new(),
            scope: Scope {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 10,
                    character: 0,
                },
                kind: ScopeKind::Function,
                name: Some("main".to_string()),
            },
            extended_scope: None,
        };

        let config = Config::default();
        let cmd = runner
            .build_command(&runnable, &config, FileType::CargoProject)
            .unwrap();

        assert_eq!(cmd.command_type, CommandType::Cargo);
        assert_eq!(cmd.to_shell_command(), "cargo leptos serve");
    }
}
