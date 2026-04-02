//! Rustc-specific implementation for single-file Rust scripts

use std::path::Path;

use crate::{
    command::{CargoCommand, CommandType},
    config::Config,
    error::Result,
    patterns::RunnableDetector,
    types::{FileType, Runnable, RunnableKind},
};

use super::traits::CommandRunner;

/// Rustc-specific command runner for single-file scripts
pub struct RustcRunner;

impl RustcRunner {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }
}

impl CommandRunner for RustcRunner {
    type Config = Config;
    type Command = CargoCommand;

    fn detect_runnables(&self, file_path: &Path) -> Result<Vec<Runnable>> {
        // For single-file scripts, detect runnables the same way
        let mut detector = RunnableDetector::new()?;
        let runnables = detector.detect_runnables(file_path, None)?;
        Ok(runnables)
    }

    fn get_runnable_at_line(&self, file_path: &Path, line: u32) -> Result<Option<Runnable>> {
        let runnables = self.detect_runnables(file_path)?;

        // For rustc, if we're looking at the file level, we might want to compile and run the whole file
        if runnables.is_empty() || !runnables.iter().any(|r| r.scope.contains_line(line)) {
            // Create a file-level runnable
            let file_runnable = Runnable {
                scope: crate::types::Scope {
                    start: crate::types::Position::new(0, 0),
                    end: crate::types::Position::new(u32::MAX, 0),
                    kind: crate::types::ScopeKind::File(crate::types::FileScope::Standalone {
                        name: file_path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .map(|s| s.to_string()),
                    }),
                    name: Some(file_path.to_string_lossy().to_string()),
                },
                kind: RunnableKind::Binary {
                    bin_name: file_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_string()),
                },
                module_path: file_path.to_string_lossy().to_string(),
                file_path: file_path.to_path_buf(),
                extended_scope: None,
                label: file_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "file".to_string()),
            };
            return Ok(Some(file_runnable));
        }

        let mut detector = RunnableDetector::new()?;
        let best = detector.get_best_runnable_at_line(file_path, line)?;
        Ok(best)
    }

    fn build_command(
        &self,
        runnable: &Runnable,
        config: &Self::Config,
        file_type: FileType,
    ) -> Result<Self::Command> {
        use crate::command::builder::{CommandBuilderImpl, rustc::RustcCommandBuilder};

        // Use the RustcCommandBuilder
        RustcCommandBuilder::build(runnable, None, config, file_type)
    }

    fn validate_command(&self, command: &Self::Command) -> Result<()> {
        // Rustc-specific validation
        if command.args.is_empty() {
            return Err(crate::error::Error::Other(
                "Rustc command has no arguments".to_string(),
            ));
        }

        // Ensure it's a rustc command
        match &command.command_type {
            CommandType::Rustc => Ok(()),
            _ => Err(crate::error::Error::Other(
                "Expected Rustc command type".to_string(),
            )),
        }
    }

    fn name(&self) -> &'static str {
        "rustc"
    }
}
