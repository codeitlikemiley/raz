//! Binary command builder with binary framework support

use super::common::CargoBuilderHelper;
use crate::{
    command::{
        CargoCommand, CommandType,
        builder::{CommandBuilderImpl, ConfigAccess},
    },
    config::Config,
    error::Result,
    types::{FileType, Runnable},
};

/// Binary command builder with binary framework support
pub struct BinaryCommandBuilder;

impl ConfigAccess for BinaryCommandBuilder {}
impl CargoBuilderHelper for BinaryCommandBuilder {}

impl CommandBuilderImpl for BinaryCommandBuilder {
    fn build(
        runnable: &Runnable,
        package: Option<&str>,
        config: &Config,
        file_type: FileType,
    ) -> Result<CargoCommand> {
        tracing::debug!(
            "BinaryCommandBuilder::build called for {:?}, package={:?}",
            runnable.file_path,
            package
        );
        let builder = BinaryCommandBuilder;
        let mut args = vec![];
        let mut command_type = CommandType::Cargo;

        // Get binary framework for later use
        let binary_framework = builder.get_binary_framework(config, file_type);

        // Check for override that might change command/subcommand
        tracing::debug!("Checking for overrides...");
        let override_config = builder.get_override(runnable, config, file_type);
        let has_override_command = override_config
            .and_then(|o| o.cargo.as_ref())
            .and_then(|c| c.command.as_ref())
            .is_some();
        tracing::debug!("has_override_command: {}", has_override_command);

        // Handle override command first (takes precedence)
        if let Some(override_config) = override_config {
            if let Some(override_cargo) = &override_config.cargo {
                if let Some(cmd) = &override_cargo.command {
                    if cmd != "cargo" {
                        command_type = CommandType::Shell;
                        args.push(cmd.clone());

                        // Add subcommand if specified
                        if let Some(subcommand) = &override_cargo.subcommand {
                            args.extend(subcommand.split_whitespace().map(String::from));
                        }
                    } else {
                        // Standard cargo with channel
                        if let Some(channel) = &override_cargo.channel {
                            args.push(format!("+{}", channel));
                        } else if let Some(channel) = builder.get_channel(config, file_type) {
                            args.push(format!("+{}", channel));
                        }

                        if let Some(subcommand) = &override_cargo.subcommand {
                            args.extend(subcommand.split_whitespace().map(String::from));
                        } else {
                            args.push("run".to_string());
                        }
                    }
                }
            }
        }

        // Only use binary framework if no override command was found
        if !has_override_command && binary_framework.is_some() {
            let binary_framework = binary_framework.as_ref().unwrap();
            // Handle custom command
            if let Some(cmd) = &binary_framework.command {
                if cmd != "cargo" {
                    command_type = CommandType::Shell;
                    args.push(cmd.clone());

                    // Add subcommand if specified
                    if let Some(subcommand) = &binary_framework.subcommand {
                        args.extend(subcommand.split_whitespace().map(String::from));
                    }
                } else {
                    // Standard cargo with channel
                    if let Some(channel) = &binary_framework.channel {
                        args.push(format!("+{}", channel));
                    } else if let Some(channel) = builder.get_channel(config, file_type) {
                        args.push(format!("+{}", channel));
                    }

                    // Add subcommand
                    if let Some(subcommand) = &binary_framework.subcommand {
                        args.extend(subcommand.split_whitespace().map(String::from));
                    } else {
                        args.push("run".to_string());
                    }
                }
            } else {
                // No custom command, use standard cargo
                if let Some(channel) = &binary_framework.channel {
                    args.push(format!("+{}", channel));
                } else if let Some(channel) = builder.get_channel(config, file_type) {
                    args.push(format!("+{}", channel));
                }

                if let Some(subcommand) = &binary_framework.subcommand {
                    args.extend(subcommand.split_whitespace().map(String::from));
                } else {
                    args.push("run".to_string());
                }
            }

            // Add framework features (only for cargo commands)
            if command_type == CommandType::Cargo {
                if let Some(features) = &binary_framework.features {
                    args.extend(features.to_args());
                }
            }

            // Add framework args
            if let Some(framework_args) = &binary_framework.extra_args {
                args.extend(framework_args.clone());
            }
        } else if !has_override_command {
            // Standard binary command (only if no override was applied)
            if let Some(channel) = builder.get_channel(config, file_type) {
                args.push(format!("+{}", channel));
            }
            args.push("run".to_string());
        }

        // Add package (only for cargo commands)
        if command_type == CommandType::Cargo {
            if let Some(pkg) = package {
                if !pkg.is_empty() {
                    args.push("--package".to_string());
                    args.push(pkg.to_string());
                }
            }

            // Add binary name
            // Check if we're using the default cargo run command
            // We're using default run if:
            // 1. We have "run" in the args
            // 2. We're using cargo (not a custom command like dx)
            // 3. No custom subcommand (like "leptos serve")
            let has_custom_command = binary_framework
                .as_ref()
                .and_then(|bf| bf.command.as_ref())
                .map(|cmd| cmd != "cargo")
                .unwrap_or(false);

            let has_custom_subcommand = binary_framework
                .as_ref()
                .and_then(|bf| bf.subcommand.as_ref())
                .is_some();

            let is_default_run =
                args.contains(&"run".to_string()) && !has_custom_command && !has_custom_subcommand;

            tracing::debug!(
                "Check default run: args={:?}, has_custom_command={}, has_custom_subcommand={}, is_default_run={}",
                args,
                has_custom_command,
                has_custom_subcommand,
                is_default_run
            );
            builder.add_target(&mut args, runnable, package, is_default_run)?;
        }

        // Apply configuration
        builder.apply_args(&mut args, runnable, config, file_type);

        let mut command = match command_type {
            CommandType::Shell => {
                // Extract the command from args[0]
                let cmd = args[0].clone();
                let cmd_args = args[1..].to_vec();
                CargoCommand::new_shell(cmd, cmd_args)
            }
            _ => CargoCommand::new(args),
        };

        // Set working directory to cargo root for all commands
        if let Some(cargo_root) = builder.find_cargo_root(&runnable.file_path) {
            command = command.with_working_dir(cargo_root.to_string_lossy().to_string());
        }

        // Apply binary framework env
        if let Some(ref binary_framework) = binary_framework {
            if let Some(extra_env) = &binary_framework.extra_env {
                for (key, value) in extra_env {
                    command.env.push((key.clone(), value.clone()));
                }
            }
        }

        builder.apply_common_config(
            &mut command,
            config,
            file_type,
            builder.get_extra_env(config, file_type),
        );
        builder.apply_env(&mut command, runnable, config, file_type);

        Ok(command)
    }
}

impl BinaryCommandBuilder {
    fn add_target(
        &self,
        args: &mut Vec<String>,
        runnable: &Runnable,
        package: Option<&str>,
        is_default_run: bool,
    ) -> Result<()> {
        tracing::debug!(
            "add_target: is_default_run={}, package={:?}, file_path={:?}",
            is_default_run,
            package,
            runnable.file_path
        );

        let path_str = runnable.file_path.to_str().unwrap_or("");

        // Check if this is an example file
        if path_str.contains("/examples/") {
            self.add_example(args, runnable)?;
            return Ok(());
        }

        // Otherwise, handle as binary
        self.add_binary(args, runnable, package, is_default_run)?;
        Ok(())
    }

    fn add_example(&self, args: &mut Vec<String>, runnable: &Runnable) -> Result<()> {
        if let Some(stem) = runnable.file_path.file_stem() {
            let example_name = stem.to_string_lossy();
            tracing::debug!(
                "Adding --example {} (file in examples/ directory)",
                example_name
            );
            args.push("--example".to_string());
            args.push(example_name.to_string());
        }
        Ok(())
    }

    fn add_binary(
        &self,
        args: &mut Vec<String>,
        runnable: &Runnable,
        package: Option<&str>,
        is_default_run: bool,
    ) -> Result<()> {
        // First check if we have an explicit binary name in the runnable
        if let crate::types::RunnableKind::Binary { bin_name } = &runnable.kind {
            if let Some(name) = bin_name {
                tracing::debug!("Using explicit bin_name: {}", name);
                args.push("--bin".to_string());
                args.push(name.clone());
                return Ok(());
            }
        }

        // For src/main.rs, we might need to add --bin with the package name
        // if there are multiple binaries in the project
        if let Some(stem) = runnable.file_path.file_stem() {
            let stem_str = stem.to_string_lossy();
            tracing::debug!("File stem: {}", stem_str);
            if stem_str != "main" {
                tracing::debug!("Adding --bin {} (non-main file)", stem_str);
                args.push("--bin".to_string());
                args.push(stem_str.to_string());
            } else {
                // For main.rs with default cargo run command, use the package name as the binary name
                // This handles the case where there are multiple binaries in the project
                if is_default_run && package.is_some() {
                    tracing::debug!(
                        "Adding --bin {} (main.rs with package name)",
                        package.unwrap()
                    );
                    args.push("--bin".to_string());
                    args.push(package.unwrap().to_string());
                } else {
                    tracing::debug!(
                        "Not adding --bin: is_default_run={}, package={:?}",
                        is_default_run,
                        package
                    );
                }
            }
        }
        Ok(())
    }

    fn apply_args(
        &self,
        args: &mut Vec<String>,
        runnable: &Runnable,
        config: &Config,
        file_type: FileType,
    ) {
        // Apply features first
        self.apply_features(
            args,
            runnable,
            config,
            file_type,
            self.get_features(config, file_type),
        );

        // Apply override args
        if let Some(override_config) = self.get_override(runnable, config, file_type) {
            if let Some(override_cargo) = &override_config.cargo {
                if let Some(extra_args) = &override_cargo.extra_args {
                    args.extend(extra_args.clone());
                }
            }
        }

        // Apply global args
        if let Some(extra_args) = self.get_extra_args(config, file_type) {
            args.extend(extra_args.clone());
        }
    }

    fn apply_env(
        &self,
        command: &mut CargoCommand,
        runnable: &Runnable,
        config: &Config,
        file_type: FileType,
    ) {
        // Apply override env vars
        if let Some(override_config) = self.get_override(runnable, config, file_type) {
            if let Some(override_cargo) = &override_config.cargo {
                if let Some(extra_env) = &override_cargo.extra_env {
                    for (key, value) in extra_env {
                        command.env.push((key.clone(), value.clone()));
                    }
                }
            }
        }
    }
}
