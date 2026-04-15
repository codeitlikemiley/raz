//! Module test command builder

use super::common::CargoBuilderHelper;
use crate::{
    command::{
        Command,
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
    ) -> Result<Command> {
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
                args.push(format!("+{channel}"));
            } else if let Some(channel) = builder.get_channel(config, file_type) {
                args.push(format!("+{channel}"));
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
                args.push(format!("+{channel}"));
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

        // For file-level lib.rs commands (empty module_name), don't add --lib
        // so that integration tests and doc tests also run.
        let is_file_level_lib =
            if let crate::types::RunnableKind::ModuleTests { module_name } = &runnable.kind {
                let path_str = runnable.file_path.to_str().unwrap_or("");
                module_name.is_empty()
                    && (path_str.ends_with("/lib.rs")
                        || path_str == "lib.rs"
                        || path_str.ends_with("/src/lib.rs"))
            } else {
                false
            };

        // Add target flags (--lib, --bin, --test, etc.) unless this is a
        // file-level lib.rs command where we want to run ALL tests.
        if !is_file_level_lib {
            builder.add_bin_target(&mut args, &runnable.file_path, package, test_framework)?;
        }

        // Apply configuration
        builder.apply_args(&mut args, runnable, config, file_type);

        // Add module filter
        builder.add_module_filter(&mut args, runnable, config, file_type);

        let mut command = Command::cargo(args);

        // Set working directory to cargo root
        if let Some(cargo_root) = builder.find_cargo_root(&runnable.file_path) {
            command = command.with_working_dir(cargo_root.to_string_lossy().to_string());
        }

        // Apply test framework env
        if let Some(test_framework) = builder.get_test_framework(config, file_type) {
            if let Some(extra_env) = &test_framework.extra_env {
                for (key, value) in extra_env {
                    command.env.insert(key.clone(), value.clone());
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

        // For integration test files (tests/*.rs), add --test flag
        // Only if using default cargo test command
        if is_default_test {
            let is_integration_test = file_path
                .parent()
                .and_then(|p| p.file_name())
                .map(|name| name == "tests")
                .unwrap_or(false);
            if is_integration_test {
                if let Some(stem) = file_path.file_stem() {
                    let test_name = stem.to_string_lossy();
                    tracing::debug!(
                        "Adding --test {} for module tests in tests/{}.rs",
                        test_name,
                        test_name
                    );
                    args.push("--test".to_string());
                    args.push(test_name.to_string());
                }
                return Ok(());
            }
        }

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
            let path_str = runnable.file_path.to_str().unwrap_or("");
            tracing::debug!(
                "add_module_filter: path_str={}, args={:?}, module_name={}",
                path_str,
                args,
                module_name
            );

            // Skip module filter for file-level commands (empty module_name) on
            // lib.rs — we want to run ALL tests in the package.
            let is_lib_rs = path_str.ends_with("/lib.rs")
                || path_str == "lib.rs"
                || path_str.ends_with("/src/lib.rs");
            if is_lib_rs && module_name.is_empty() {
                tracing::debug!("Skipping module filter for file-level lib.rs command");
                args.push("--".to_string());
                self.apply_test_binary_args(args, runnable, config, file_type);
                return;
            }

            // Skip module filter for file-level commands on integration test files
            // — --test <stem> already scopes to the correct binary.
            let is_integration_test = runnable
                .file_path
                .parent()
                .and_then(|p| p.file_name())
                .map(|name| name == "tests")
                .unwrap_or(false);
            if is_integration_test && module_name.is_empty() {
                tracing::debug!("Skipping module filter for file-level integration test command");
                args.push("--".to_string());
                self.apply_test_binary_args(args, runnable, config, file_type);
                return;
            }

            args.push("--".to_string());

            // Use the full module path if available, otherwise just the module name
            // For integration tests, strip the file-level prefix (tests::<file_stem>)
            // since --test <stem> already scopes to the correct binary.
            let effective_path = if is_integration_test {
                Self::strip_integration_test_prefix(
                    &runnable.file_path,
                    if !runnable.module_path.is_empty() {
                        &runnable.module_path
                    } else {
                        module_name
                    },
                )
            } else if !runnable.module_path.is_empty() {
                runnable.module_path.clone()
            } else {
                module_name.clone()
            };

            if !effective_path.is_empty() {
                tracing::debug!("Using effective module path: {}", effective_path);
                args.push(effective_path);
            }

            // Apply test binary args
            self.apply_test_binary_args(args, runnable, config, file_type);
        }
    }

    /// Strip the file-level module prefix for integration tests.
    ///
    /// Integration test files get a synthetic module path of `tests::<file_stem>`
    /// from the module resolver, but cargo's integration test binaries don't use
    /// that prefix — tests are at the root namespace. Strip it so the filter works.
    fn strip_integration_test_prefix(file_path: &std::path::Path, module_path: &str) -> String {
        let file_stem = file_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let prefix = format!("tests::{file_stem}");

        if module_path == prefix {
            String::new()
        } else if let Some(rest) = module_path.strip_prefix(&format!("{prefix}::")) {
            rest.to_string()
        } else {
            module_path.to_string()
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
        command: &mut Command,
        runnable: &Runnable,
        config: &Config,
        file_type: FileType,
    ) {
        // Apply override env vars
        if let Some(override_config) = self.get_override(runnable, config, file_type) {
            if let Some(override_cargo) = &override_config.cargo {
                if let Some(extra_env) = &override_cargo.extra_env {
                    for (key, value) in extra_env {
                        command.env.insert(key.clone(), value.clone());
                    }
                }
            }
        }
    }
}
