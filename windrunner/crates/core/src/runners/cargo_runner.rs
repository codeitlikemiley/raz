//! Cargo-specific implementation of CommandRunner

use std::path::Path;

use crate::{
    command::CargoCommand,
    config::Config,
    error::Result,
    patterns::RunnableDetector,
    types::{FileType, Runnable},
};

use super::{
    common::{get_cargo_package_name, resolve_module_paths, resolve_module_path_single},
    traits::{CommandRunner, RunnerCommand},
};

/// Cargo-specific command runner
pub struct CargoRunner;

impl CargoRunner {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }
}

impl CommandRunner for CargoRunner {
    type Config = Config;
    type Command = CargoCommand;

    fn detect_runnables(&self, file_path: &Path) -> Result<Vec<Runnable>> {
        // RunnableDetector has its own mutable state, so we need to create a new instance
        let mut detector = RunnableDetector::new()?;
        let mut runnables = detector.detect_runnables(file_path, None)?;

        // Now resolve module paths for all runnables
        if !runnables.is_empty() {
            // Get package name from Cargo.toml
            let package_name = get_cargo_package_name(file_path);
            resolve_module_paths(&mut runnables, file_path, package_name.as_deref())?;
        }

        Ok(runnables)
    }

    fn get_runnable_at_line(&self, file_path: &Path, line: u32) -> Result<Option<Runnable>> {
        let mut detector = RunnableDetector::new()?;
        if let Some(mut runnable) = detector.get_best_runnable_at_line(file_path, line)? {
            // Resolve module path for the runnable
            let package_name = get_cargo_package_name(file_path);
            resolve_module_path_single(&mut runnable, file_path, package_name.as_deref())?;
            Ok(Some(runnable))
        } else {
            Ok(None)
        }
    }

    fn build_command(
        &self,
        runnable: &Runnable,
        _config: &Self::Config,
        _file_type: FileType,
    ) -> Result<Self::Command> {
        use crate::command::builder::CommandBuilder;

        // Get the actual package name from Cargo.toml
        let package = get_cargo_package_name(&runnable.file_path);

        // Build command using CommandBuilder
        let mut builder = CommandBuilder::for_runnable(runnable);
        if let Some(pkg) = package {
            builder = builder.with_package(pkg);
        }

        let command = builder.build()?;

        Ok(command)
    }

    fn validate_command(&self, command: &Self::Command) -> Result<()> {
        // Basic validation - ensure command has required components
        if command.args.is_empty() {
            return Err(crate::error::Error::Other(
                "Command has no arguments".to_string(),
            ));
        }

        // Could add more validation here based on cargo rules
        Ok(())
    }

    fn name(&self) -> &'static str {
        "cargo"
    }
}

// Implement RunnerCommand for CargoCommand
impl RunnerCommand for CargoCommand {
    fn to_shell_command(&self) -> String {
        crate::command::CargoCommand::to_shell_command(self)
    }

    fn execute(&self) -> Result<std::process::ExitStatus> {
        crate::command::CargoCommand::execute(self).map_err(|e| crate::error::Error::IoError(e))
    }

    fn working_dir(&self) -> Option<&Path> {
        self.working_dir.as_ref().map(|s| Path::new(s))
    }

    fn env_vars(&self) -> &[(String, String)] {
        &self.env
    }

    fn args(&self) -> &[String] {
        &self.args
    }
}
