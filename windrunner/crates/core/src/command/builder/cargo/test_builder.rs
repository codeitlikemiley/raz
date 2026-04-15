//! Test command builder with test framework support

use super::common::CargoBuilderHelper;
use crate::{
    command::{
        Command,
        builder::{CommandBuilderImpl, ConfigAccess},
    },
    config::Config,
    error::Result,
    types::{FileType, Runnable, RunnableKind},
};
use std::path::Path;

/// Test command builder with test framework support
pub struct TestCommandBuilder;

impl ConfigAccess for TestCommandBuilder {}
impl CargoBuilderHelper for TestCommandBuilder {}

impl CommandBuilderImpl for TestCommandBuilder {
    fn build(
        runnable: &Runnable,
        package: Option<&str>,
        config: &Config,
        file_type: FileType,
    ) -> Result<Command> {
        tracing::warn!(
            "TestCommandBuilder::build called for {:?}, package={:?}",
            runnable.file_path,
            package
        );
        let builder = TestCommandBuilder;
        let mut args = vec![];

        // Handle test framework configuration
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

        // Add target/bin/lib (for tests in specific files)
        tracing::debug!("Calling add_target for file: {:?}", runnable.file_path);
        builder.add_target(&mut args, &runnable.file_path, package, test_framework)?;

        // Apply configuration
        builder.apply_args(&mut args, runnable, config, file_type);

        // Add test filter
        builder.add_test_filter(&mut args, runnable, config, file_type);

        let mut command = Command::cargo(args);

        // Set working directory to cargo root
        if let Some(cargo_root) = builder.find_cargo_root(&runnable.file_path) {
            tracing::debug!(
                "Setting working directory for test command to: {:?}",
                cargo_root
            );
            command = command.with_working_dir(cargo_root.to_string_lossy().to_string());
        } else {
            tracing::debug!("No cargo root found for: {:?}", runnable.file_path);
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

impl TestCommandBuilder {
    fn add_target(
        &self,
        args: &mut Vec<String>,
        file_path: &Path,
        package: Option<&str>,
        test_framework: Option<&crate::config::TestFramework>,
    ) -> Result<()> {
        // Only add target flags when using the default `cargo test` subcommand.
        // Custom subcommands (nextest, cargo-llvm-cov, etc.) manage target flags themselves.
        let is_default_test = args.contains(&"test".to_string()) && {
            if let Some(tf) = &test_framework {
                tf.command.as_ref().map(|c| c == "cargo").unwrap_or(true) && tf.subcommand.is_none()
            } else {
                true
            }
        };

        if !is_default_test {
            return Ok(());
        }

        use crate::command::resolver::{
            BenchResolver, BinResolver, ExampleResolver, IntegrationTestResolver, LibResolver,
            ResolverChain,
        };

        let chain = ResolverChain::new()
            .push_resolver(IntegrationTestResolver)
            .push_resolver(BinResolver::new(package.map(str::to_string)))
            .push_resolver(ExampleResolver)
            .push_resolver(BenchResolver)
            .push_resolver(LibResolver);

        if let Some(flags) = chain.resolve(file_path, package) {
            tracing::debug!(
                "add_target: ResolverChain produced {:?} for {:?}",
                flags,
                file_path
            );
            args.extend(flags);
        } else {
            tracing::debug!("add_target: no resolver matched {:?}", file_path);
        }

        Ok(())
    }

    fn add_test_filter(
        &self,
        args: &mut Vec<String>,
        runnable: &Runnable,
        config: &Config,
        file_type: FileType,
    ) {
        if let RunnableKind::Test { test_name, .. } = &runnable.kind {
            args.push("--".to_string());

            // For integration tests, strip the file-level module prefix
            // (tests::<file_stem>) since --test <stem> already scopes to the
            // correct binary. Tests in integration test binaries are at the
            // root namespace.
            let effective_module_path =
                Self::effective_module_path_for_filter(&runnable.file_path, &runnable.module_path);

            // Build the full test path including module
            let full_test_path = if !effective_module_path.is_empty() {
                format!("{effective_module_path}::{test_name}")
            } else {
                test_name.clone()
            };

            args.push(full_test_path);

            // Add --exact flag for individual test functions
            args.push("--exact".to_string());

            // Apply test binary args
            self.apply_test_binary_args(args, runnable, config, file_type);
        }
    }

    /// For integration test files, strip the file-level module path prefix
    /// (`tests::<file_stem>`) since `--test <stem>` already scopes to the
    /// correct binary.
    fn effective_module_path_for_filter(file_path: &Path, module_path: &str) -> String {
        let is_integration_test = file_path
            .parent()
            .and_then(|p| p.file_name())
            .map(|name| name == "tests")
            .unwrap_or(false);

        if !is_integration_test || module_path.is_empty() {
            return module_path.to_string();
        }

        // Build the expected file-level prefix: tests::<file_stem>
        let file_stem = file_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let prefix = format!("tests::{file_stem}");

        if module_path == prefix {
            String::new() // No inline modules, test is at root level
        } else if let Some(rest) = module_path.strip_prefix(&format!("{prefix}::")) {
            rest.to_string() // Strip prefix, keep inline module path
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
