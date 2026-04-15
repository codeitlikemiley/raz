//! Clean API for command building with encapsulated config resolution

pub mod bazel;
pub mod cargo;
mod config_access;
mod config_resolver;
pub mod rustc;

pub use self::bazel::BazelCommandBuilder;
pub use self::cargo::{
    BenchmarkCommandBuilder, BinaryCommandBuilder, DocTestCommandBuilder, ModuleTestCommandBuilder,
    TestCommandBuilder,
};
pub use self::config_access::ConfigAccess;
pub use self::config_resolver::ConfigResolver;
pub use self::rustc::{RustcCommandBuilder, SingleFileScriptBuilder};

use crate::{
    command::Command,
    config::Config,
    error::Result,
    types::{FileType, FunctionIdentity, Runnable, RunnableKind},
};
use std::path::Path;

/// Main entry point for building commands
///
/// # Example
/// ```no_run
/// use cargo_runner_core::command::builder::CommandBuilder;
/// use cargo_runner_core::types::{Runnable, RunnableKind, Scope, Position};
/// use std::path::{Path, PathBuf};
///
/// let runnable = Runnable {
///     label: "test_example".to_string(),
///     kind: RunnableKind::Test {
///         test_name: "test_example".to_string(),
///         is_async: false,
///     },
///     scope: Scope {
///         start: Position::new(1, 0),
///         end: Position::new(10, 0),
///         kind: cargo_runner_core::types::ScopeKind::Function,
///         name: Some("test_example".to_string()),
///     },
///     module_path: "my_crate::tests".to_string(),
///     file_path: PathBuf::from("src/tests.rs"),
///     extended_scope: None,
/// };
///
/// let command = CommandBuilder::for_runnable(&runnable, cargo_runner_core::types::FileType::CargoProject)
///     .with_package("my-package")
///     .with_project_root(Path::new("/path/to/project"))
///     .build()
///     .unwrap();
/// ```
pub struct CommandBuilder<'a> {
    runnable: &'a Runnable,
    package_name: Option<String>,
    project_root: Option<&'a Path>,
    config_override: Option<Config>,
    file_type: FileType,
}

impl<'a> CommandBuilder<'a> {
    /// Create a new command builder for a runnable
    pub fn for_runnable(runnable: &'a Runnable, file_type: FileType) -> Self {
        Self {
            runnable,
            package_name: None,
            project_root: None,
            config_override: None,
            file_type,
        }
    }

    /// Set the package name
    pub fn with_package(mut self, package: impl Into<String>) -> Self {
        self.package_name = Some(package.into());
        self
    }

    /// Set the project root
    pub fn with_project_root(mut self, root: &'a Path) -> Self {
        self.project_root = Some(root);
        self
    }

    /// Override the configuration (for testing or special cases)
    pub fn with_config(mut self, config: Config) -> Self {
        self.config_override = Some(config);
        self
    }

    /// Build the command
    pub fn build(self) -> Result<Command> {
        let file_type = self.file_type;
        tracing::debug!(
            "CommandBuilder::build: file_type={:?}, runnable.kind={:?}, file_path={:?}",
            self.file_type,
            self.runnable.kind,
            self.runnable.file_path
        );

        // Create the function identity
        let identity = FunctionIdentity {
            package: self.package_name.clone(),
            module_path: if self.runnable.module_path.is_empty() {
                None
            } else {
                Some(self.runnable.module_path.clone())
            },
            file_path: Some(self.runnable.file_path.clone()),
            function_name: self.runnable.get_function_name(),
            file_type: Some(file_type),
        };

        // Resolve configuration
        let config = if let Some(config) = self.config_override {
            config
        } else {
            ConfigResolver::new(self.project_root, &identity).resolve()?
        };

        // Standalone routing optimization
        if file_type == FileType::Standalone {
            if matches!(self.runnable.kind, RunnableKind::DocTest { .. }) {
                return Err(crate::error::Error::UnsupportedRunnable {
                    context: "standalone files doctest",
                });
            }
            if matches!(self.runnable.kind, RunnableKind::SingleFileScript { .. }) {
                return SingleFileScriptBuilder::build(
                    self.runnable,
                    self.package_name.as_deref(),
                    &config,
                    file_type,
                );
            }
            return RustcCommandBuilder::build(
                self.runnable,
                self.package_name.as_deref(),
                &config,
                file_type,
            );
        }

        // Delegate to specific builders based on file type first, then kind
        match (file_type, &self.runnable.kind) {
            (crate::types::FileType::Standalone, _) => unreachable!(),
            // Single file scripts
            (FileType::SingleFileScript, _) => SingleFileScriptBuilder::build(
                self.runnable,
                self.package_name.as_deref(),
                &config,
                file_type,
            ),

            // Cargo projects use cargo commands
            (FileType::CargoProject, RunnableKind::DocTest { .. }) => DocTestCommandBuilder::build(
                self.runnable,
                self.package_name.as_deref(),
                &config,
                file_type,
            ),
            (FileType::CargoProject, RunnableKind::Test { .. }) => TestCommandBuilder::build(
                self.runnable,
                self.package_name.as_deref(),
                &config,
                file_type,
            ),
            (FileType::CargoProject, RunnableKind::Binary { .. }) => {
                tracing::debug!(
                    "Routing to BinaryCommandBuilder for package {:?}",
                    self.package_name
                );
                BinaryCommandBuilder::build(
                    self.runnable,
                    self.package_name.as_deref(),
                    &config,
                    file_type,
                )
            }
            (FileType::CargoProject, RunnableKind::ModuleTests { .. }) => {
                ModuleTestCommandBuilder::build(
                    self.runnable,
                    self.package_name.as_deref(),
                    &config,
                    file_type,
                )
            }
            (FileType::CargoProject, RunnableKind::Benchmark { .. }) => {
                BenchmarkCommandBuilder::build(
                    self.runnable,
                    self.package_name.as_deref(),
                    &config,
                    file_type,
                )
            }
            (FileType::CargoProject, RunnableKind::Standalone { .. }) => {
                // This shouldn't happen, but fallback to rustc
                RustcCommandBuilder::build(
                    self.runnable,
                    self.package_name.as_deref(),
                    &config,
                    file_type,
                )
            }
            (FileType::CargoProject, RunnableKind::SingleFileScript { .. }) => {
                SingleFileScriptBuilder::build(
                    self.runnable,
                    self.package_name.as_deref(),
                    &config,
                    file_type,
                )
            }
        }
    }
}

/// Trait for building specific command types
pub trait CommandBuilderImpl: ConfigAccess {
    fn build(
        runnable: &Runnable,
        package: Option<&str>,
        config: &Config,
        file_type: FileType,
    ) -> Result<Command>;
}
