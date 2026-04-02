//! Bazel-specific implementation of CommandRunner

use std::path::Path;

use crate::{
    command::CargoCommand,
    config::Config,
    error::Result,
    patterns::RunnableDetector,
    types::{FileType, Runnable},
};

use super::{common::resolve_module_paths, traits::CommandRunner};

/// Bazel-specific command runner
pub struct BazelRunner;

impl BazelRunner {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }
}

impl CommandRunner for BazelRunner {
    type Config = Config;
    type Command = CargoCommand; // Reusing CargoCommand for now

    fn detect_runnables(&self, file_path: &Path) -> Result<Vec<Runnable>> {
        // For Bazel, we still parse Rust files the same way
        let mut detector = RunnableDetector::new()?;
        let mut runnables = detector.detect_runnables(file_path, None)?;
        
        // Resolve module paths using common function
        resolve_module_paths(&mut runnables, file_path, None)?;
        
        Ok(runnables)
    }

    fn get_runnable_at_line(&self, file_path: &Path, line: u32) -> Result<Option<Runnable>> {
        let mut detector = RunnableDetector::new()?;
        if let Some(mut runnable) = detector.get_best_runnable_at_line(file_path, line)? {
            // Resolve module path using common function
            use super::common::resolve_module_path_single;
            resolve_module_path_single(&mut runnable, file_path, None)?;
            Ok(Some(runnable))
        } else {
            Ok(None)
        }
    }

    fn build_command(
        &self,
        runnable: &Runnable,
        config: &Self::Config,
        file_type: FileType,
    ) -> Result<Self::Command> {
        use crate::command::builder::CommandBuilderImpl;
        use crate::command::builder::bazel::BazelCommandBuilder;

        // Build command using BazelCommandBuilder
        let command = BazelCommandBuilder::build(
            runnable, None, // Bazel doesn't use package in the same way
            config, file_type,
        )?;

        Ok(command)
    }

    fn validate_command(&self, command: &Self::Command) -> Result<()> {
        // Bazel-specific validation
        if command.args.is_empty() {
            return Err(crate::error::Error::Other(
                "Bazel command has no arguments".to_string(),
            ));
        }

        // Ensure it's a bazel command
        match &command.command_type {
            crate::command::CommandType::Bazel => Ok(()),
            _ => Err(crate::error::Error::Other(
                "Expected Bazel command type".to_string(),
            )),
        }
    }

    fn name(&self) -> &'static str {
        "bazel"
    }
}
