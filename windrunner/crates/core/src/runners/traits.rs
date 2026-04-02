//! Core traits for the runner architecture

use crate::{
    error::Result,
    types::{FileType, Runnable},
};
use std::path::Path;

/// Core trait that all command runners must implement
pub trait CommandRunner: Send + Sync {
    /// The configuration type this runner uses
    type Config;

    /// The command type this runner produces
    type Command: RunnerCommand;

    /// Detect all runnables in the given file
    fn detect_runnables(&self, file_path: &Path) -> Result<Vec<Runnable>>;

    /// Get the best runnable at a specific line
    fn get_runnable_at_line(&self, file_path: &Path, line: u32) -> Result<Option<Runnable>>;

    /// Build a command for the given runnable
    fn build_command(
        &self,
        runnable: &Runnable,
        config: &Self::Config,
        file_type: FileType,
    ) -> Result<Self::Command>;

    /// Validate that a command is valid before execution
    fn validate_command(&self, command: &Self::Command) -> Result<()>;

    /// Get the name of this runner
    fn name(&self) -> &'static str;
}

/// Trait for executable commands
pub trait RunnerCommand: Send + Sync {
    /// Get the command as a shell string
    fn to_shell_command(&self) -> String;

    /// Execute the command
    fn execute(&self) -> Result<std::process::ExitStatus>;

    /// Get the working directory for this command
    fn working_dir(&self) -> Option<&Path>;

    /// Get environment variables for this command
    fn env_vars(&self) -> &[(String, String)];

    /// Get the command arguments
    fn args(&self) -> &[String];
}
