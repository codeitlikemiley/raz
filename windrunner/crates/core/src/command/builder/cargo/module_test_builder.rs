//! Module test command builder

use super::common::CargoBuilderHelper;
use crate::{
    command::{
        CargoCommand,
        builder::{CommandBuilderImpl, ConfigAccess},
    },
    config::Config,
    error::Result,
    types::{FileType, Runnable},
};

/// Module test command builder
pub struct ModuleTestCommandBuilder;

impl ConfigAccess for ModuleTestCommandBuilder {}
impl CargoBuilderHelper for ModuleTestCommandBuilder {}

impl CommandBuilderImpl for ModuleTestCommandBuilder {
    fn build(
        runnable: &Runnable,
        package: Option<&str>,
        config: &Config,
        file_type: FileType,
    ) -> Result<CargoCommand> {
        tracing::warn!(
            "ModuleTestCommandBuilder::build called for {:?}, package={:?}",
            runnable.file_path,
            package
        );
        let builder = ModuleTestCommandBuilder;
        let mut args = vec![];

        // Handle test framework configuration (same as TestCommandBuilder)
        if let Some(test_framework) = builder.get_test_framework(config, file_type) {
            // Add channel
            if let Some(channel) = &test_framework.channel {
                args.push(format!("+{}", channel));
            } else if let Some(channel) = builder.get_channel(config, file_type) {
                args.push(format!("+{}", channel));
            }

            // Add subcommand
            if let Some(subcommand) = &test_framework.subcommand {
                args.extend(subcommand.split_whitespace().map(String::from));
            } else {
                args.push("test".to_string());
            }

            // Add framework features
            if let Some(features) = &test_framework.features {
                args.extend(features.to_args());
            }

            // Add framework args
            if let Some(framework_args) = &test_framework.extra_args {
                args.extend(framework_args.clone());
            }
        } else {
            // Standard test command
            if let Some(channel) = builder.get_channel(config, file_type) {
                args.push(format!("+{}", channel));
            }
            args.push("test".to_string());
        }

        // Add package
        if let Some(pkg) = package {
            if !pkg.is_empty() {
                args.push("--package".to_string());
                args.push(pkg.to_string());
            }
        }

        // Get test framework for checking if we're using default cargo test
        let test_framework = builder.get_test_framework(config, file_type);

        // Add --bin for tests in binary files (like src/main.rs)
        builder.add_bin_target(&mut args, &runnable.file_path, package, test_framework)?;

        // Apply configuration
        builder.apply_args(&mut args, runnable, config, file_type);

        // Add module filter
        builder.add_module_filter(&mut args, runnable, config, file_type);

        let mut command = CargoCommand::new(args);

        // Set working directory to cargo root
        if let Some(cargo_root) = builder.find_cargo_root(&runnable.file_path) {
            command = command.with_working_dir(cargo_root.to_string_lossy().to_string());
        }

        // Apply test framework env
        if let Some(test_framework) = builder.get_test_framework(config, file_type) {
            if let Some(extra_env) = &test_framework.extra_env {
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

impl ModuleTestCommandBuilder {
    fn add_bin_target(
        &self,
        args: &mut Vec<String>,
        file_path: &std::path::Path,
        package: Option<&str>,
        test_framework: Option<&crate::config::TestFramework>,
    ) -> Result<()> {
        let path_str = file_path.to_str().unwrap_or("");

        // Check if we're using default cargo test command
        let is_default_test = args.contains(&"test".to_string()) && {
            if let Some(tf) = &test_framework {
                // If we have a test framework, check if it's using default cargo test
                tf.command.as_ref().map(|c| c == "cargo").unwrap_or(true) && tf.subcommand.is_none()
            } else {
                // No test framework means we're using default cargo
                true
            }
        };

        // For tests in library source files (src/**/*.rs, excluding main.rs and bin/), add --lib flag
        // Only if using default cargo test command
        if is_default_test
            && ((path_str.contains("/src/")
                || path_str.starts_with("src/")
                || path_str == "lib.rs")
                && !path_str.ends_with("/main.rs")
                && !path_str.ends_with("main.rs")
                && !path_str.contains("/bin/"))
        {
            tracing::debug!(
                "Adding --lib for module tests in library source file: {}",
                path_str
            );
            args.push("--lib".to_string());
            return Ok(());
        }

        // For tests in example files (examples/*.rs), add --example flag
        // Only if using default cargo test command
        if is_default_test && (path_str.contains("/examples/") || path_str.starts_with("examples/"))
        {
            if let Some(stem) = file_path.file_stem() {
                let example_name = stem.to_string_lossy();
                tracing::debug!(
                    "Adding --example {} for module tests in examples/{}.rs",
                    example_name,
                    example_name
                );
                args.push("--example".to_string());
                args.push(example_name.to_string());
            }
            return Ok(());
        }

        // For tests in binary files (src/main.rs, src/bin/*.rs), add --bin flag
        // Only if using default cargo test command
        if is_default_test && (path_str.ends_with("/src/main.rs") || path_str.contains("/src/bin/"))
        {
            if path_str.ends_with("/src/main.rs") {
                // For src/main.rs, use package name as binary name
                if let Some(pkg) = package {
                    tracing::debug!("Adding --bin {} for module tests in src/main.rs", pkg);
                    args.push("--bin".to_string());
                    args.push(pkg.to_string());
                }
            } else if let Some(stem) = file_path.file_stem() {
                // For src/bin/*.rs, use file stem as binary name
                let bin_name = stem.to_string_lossy();
                tracing::debug!(
                    "Adding --bin {} for module tests in src/bin/{}.rs",
                    bin_name,
                    bin_name
                );
                args.push("--bin".to_string());
                args.push(bin_name.to_string());
            }
        }

        Ok(())
    }

    fn add_module_filter(
        &self,
        args: &mut Vec<String>,
        runnable: &Runnable,
        config: &Config,
        file_type: FileType,
    ) {
        // For module tests, we need to extract the module name from the RunnableKind
        if let crate::types::RunnableKind::ModuleTests { module_name } = &runnable.kind {
            // For file-level commands on lib.rs files, don't add module filter
            // This allows running all tests in the library with just --lib
            // But we should still add the module filter if module_name is not empty
            let path_str = runnable.file_path.to_str().unwrap_or("");
            tracing::debug!(
                "add_module_filter: path_str={}, args={:?}, contains_lib={}, module_name={}",
                path_str,
                args,
                args.contains(&"--lib".to_string()),
                module_name
            );

            // Skip module filter only for empty module names (file-level commands)
            if args.contains(&"--lib".to_string())
                && (path_str.ends_with("/lib.rs")
                    || path_str == "lib.rs"
                    || path_str.ends_with("/src/lib.rs"))
                && module_name.is_empty()
            {
                tracing::debug!("Skipping module filter for file-level lib.rs command");
                // Still apply test binary args if any
                self.apply_test_binary_args(args, runnable, config, file_type);
                return;
            }

            args.push("--".to_string());

            // Use the full module path if available, otherwise just the module name
            if !runnable.module_path.is_empty() {
                tracing::debug!("Using runnable.module_path: {}", runnable.module_path);
                args.push(runnable.module_path.clone());
            } else {
                tracing::debug!("Using module_name from RunnableKind: {}", module_name);
                args.push(module_name.clone());
            }

            // Apply test binary args
            self.apply_test_binary_args(args, runnable, config, file_type);
        }
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

    fn apply_test_binary_args(
        &self,
        args: &mut Vec<String>,
        runnable: &Runnable,
        config: &Config,
        file_type: FileType,
    ) {
        // Apply override test binary args
        if let Some(override_config) = self.get_override(runnable, config, file_type) {
            if let Some(override_cargo) = &override_config.cargo {
                if let Some(extra_args) = &override_cargo.extra_test_binary_args {
                    args.extend(extra_args.clone());
                }
            }
        }

        // Apply global test binary args
        if let Some(extra_args) = self.get_extra_test_binary_args(config, file_type) {
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
