//! DocTest command builder

use super::common::CargoBuilderHelper;
use crate::{
    command::{
        CargoCommand,
        builder::{CommandBuilderImpl, ConfigAccess},
    },
    config::Config,
    error::Result,
    types::{FileType, Runnable, RunnableKind},
};

/// DocTest command builder
pub struct DocTestCommandBuilder;

impl ConfigAccess for DocTestCommandBuilder {}
impl CargoBuilderHelper for DocTestCommandBuilder {}

impl CommandBuilderImpl for DocTestCommandBuilder {
    fn build(
        runnable: &Runnable,
        package: Option<&str>,
        config: &Config,
        file_type: FileType,
    ) -> Result<CargoCommand> {
        let builder = DocTestCommandBuilder;

        // Get the test_id from the runnable
        let test_id = match &runnable.kind {
            RunnableKind::DocTest {
                struct_or_module_name,
                method_name,
            } => {
                // Strip "impl " prefix if present (used to differentiate impl blocks from structs)
                let clean_name = struct_or_module_name
                    .strip_prefix("impl ")
                    .unwrap_or(struct_or_module_name);

                if let Some(method) = method_name {
                    format!("{}::{}", clean_name, method)
                } else {
                    clean_name.to_string()
                }
            }
            _ => {
                return Err(crate::error::Error::ParseError(
                    "Expected DocTest runnable".to_string(),
                ));
            }
        };

        let mut args = vec![];

        // Add channel
        if let Some(channel) = builder.get_channel(config, file_type) {
            args.push(format!("+{}", channel));
        }

        args.push("test".to_string());
        args.push("--doc".to_string());

        // Add package
        if let Some(pkg) = package {
            if !pkg.is_empty() {
                args.push("--package".to_string());
                args.push(pkg.to_string());
            }
        }

        // Apply configuration
        builder.apply_args(&mut args, runnable, config, file_type);

        // Add doc test filter
        args.push("--".to_string());
        args.push(test_id.clone());

        // Apply test binary args
        builder.apply_test_binary_args(&mut args, runnable, config, file_type);

        let mut command = CargoCommand::new(args);

        // Set working directory to cargo root
        if let Some(cargo_root) = builder.find_cargo_root(&runnable.file_path) {
            command = command.with_working_dir(cargo_root.to_string_lossy().to_string());
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

impl DocTestCommandBuilder {
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
