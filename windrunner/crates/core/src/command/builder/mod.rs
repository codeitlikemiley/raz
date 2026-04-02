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
    command::CargoCommand,
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
/// let command = CommandBuilder::for_runnable(&runnable)
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
}

impl<'a> CommandBuilder<'a> {
    /// Create a new command builder for a runnable
    pub fn for_runnable(runnable: &'a Runnable) -> Self {
        Self {
            runnable,
            package_name: None,
            project_root: None,
            config_override: None,
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

    /// Check if a file is a cargo script file
    fn is_cargo_script_file(&self, file_path: &Path) -> Result<bool> {
        if file_path.extension().and_then(|s| s.to_str()) == Some("rs") {
            if let Ok(content) = std::fs::read_to_string(file_path) {
                if let Some(first_line) = content.lines().next() {
                    return Ok(first_line.starts_with("#!")
                        && first_line.contains("cargo")
                        && first_line.contains("-Zscript"));
                }
            }
        }
        Ok(false)
    }

    /// Check if a file is a standalone file (not part of a Cargo project structure)
    fn is_standalone_file(&self, file_path: &Path) -> bool {
        // First check if the file has appropriate content for the runnable type
        let has_appropriate_content = match &self.runnable.kind {
            RunnableKind::Binary { .. } | RunnableKind::Standalone { .. } => {
                // For binaries, check for main function
                if let Ok(content) = std::fs::read_to_string(file_path) {
                    content.contains("fn main(") || content.contains("fn main (")
                } else {
                    return false;
                }
            }
            RunnableKind::Test { .. } | RunnableKind::ModuleTests { .. } => {
                // For tests, check for #[test] or #[cfg(test)]
                if let Ok(content) = std::fs::read_to_string(file_path) {
                    content.contains("#[test]") || content.contains("#[cfg(test)]")
                } else {
                    return false;
                }
            }
            RunnableKind::Benchmark { .. } => {
                // For benchmarks, check for #[bench]
                if let Ok(content) = std::fs::read_to_string(file_path) {
                    content.contains("#[bench]")
                } else {
                    return false;
                }
            }
            _ => return false, // Other types are not standalone
        };

        if !has_appropriate_content {
            return false;
        }

        // Check if file is part of a Cargo project
        let cargo_root = file_path
            .ancestors()
            .find(|p| p.join("Cargo.toml").exists());

        match cargo_root {
            None => true, // No Cargo.toml found, definitely standalone
            Some(root) => {
                // Check if the file is in a standard Cargo source location
                if let Ok(relative) = file_path.strip_prefix(root) {
                    let path_str = relative.to_str().unwrap_or("");

                    // Check standard Cargo project locations based on runnable type
                    match &self.runnable.kind {
                        RunnableKind::Binary { .. } | RunnableKind::Standalone { .. } => {
                            if path_str == "src/main.rs"
                                || path_str.starts_with("src/bin/")
                                || path_str.starts_with("examples/")
                            {
                                return false; // In standard location, not standalone
                            }
                        }
                        RunnableKind::Test { .. } | RunnableKind::ModuleTests { .. } => {
                            if path_str.starts_with("tests/")
                                || path_str.starts_with("src/")
                                || path_str.starts_with("examples/")
                            {
                                return false; // In standard location, not standalone
                            }
                        }
                        RunnableKind::Benchmark { .. } => {
                            if path_str.starts_with("benches/") {
                                return false; // In standard bench location, not standalone
                            }
                        }
                        _ => {}
                    }

                    // Check if it's listed in Cargo.toml
                    if let Ok(manifest) = std::fs::read_to_string(root.join("Cargo.toml")) {
                        // Simple check for [[bin]] entries
                        if manifest.contains("[[bin]]") && manifest.contains(&path_str) {
                            return false; // Listed in Cargo.toml, not standalone
                        }
                    }
                }

                // If we get here, it has main() but isn't in a standard location
                // and isn't listed in Cargo.toml, so it's standalone
                true
            }
        }
    }

    /// Detect the file type based on the runnable
    fn detect_file_type(&self) -> Result<FileType> {
        tracing::debug!(
            "detect_file_type called for kind={:?}, path={:?}",
            self.runnable.kind,
            self.runnable.file_path
        );

        match &self.runnable.kind {
            RunnableKind::Standalone { .. } => {
                if self.is_cargo_script_file(&self.runnable.file_path)? {
                    Ok(FileType::SingleFileScript)
                } else {
                    Ok(FileType::Standalone)
                }
            }
            RunnableKind::SingleFileScript { .. } => Ok(FileType::SingleFileScript),
            _ => {
                // Check cargo script FIRST since it's more specific
                if self.is_cargo_script_file(&self.runnable.file_path)? {
                    return Ok(FileType::SingleFileScript);
                }

                // Check if file is part of a Cargo project BEFORE checking standalone
                // First, resolve the file path to absolute if it's relative
                let file_path = if self.runnable.file_path.is_absolute() {
                    self.runnable.file_path.clone()
                } else {
                    std::env::current_dir()
                        .ok()
                        .map(|cwd| cwd.join(&self.runnable.file_path))
                        .unwrap_or_else(|| self.runnable.file_path.clone())
                };

                let cargo_root = file_path
                    .ancestors()
                    .find(|p| p.join("Cargo.toml").exists());

                match cargo_root {
                    Some(root) => {
                        // Check if the file is in a standard Cargo source location
                        if let Ok(relative) = file_path.strip_prefix(root) {
                            let path_str = relative.to_str().unwrap_or("");

                            tracing::debug!("detect_file_type: relative path = {}", path_str);

                            // Check standard Cargo project locations
                            if path_str == "src/main.rs"
                                || path_str.starts_with("src/bin/")
                                || path_str.starts_with("src/")
                                || path_str.starts_with("tests/")
                                || path_str.starts_with("examples/")
                                || path_str.starts_with("benches/")
                            {
                                tracing::debug!(
                                    "detect_file_type: detected as CargoProject (standard location)"
                                );
                                Ok(FileType::CargoProject)
                            } else if self.is_standalone_file(&file_path) {
                                // File is in a Cargo project but not in standard location and has appropriate content
                                tracing::debug!(
                                    "detect_file_type: detected as Standalone (non-standard location with appropriate content)"
                                );
                                Ok(FileType::Standalone)
                            } else {
                                // File is in a Cargo project but not in standard location
                                tracing::debug!(
                                    "detect_file_type: detected as CargoProject (non-standard location)"
                                );
                                Ok(FileType::CargoProject)
                            }
                        } else {
                            Ok(FileType::CargoProject)
                        }
                    }
                    None => {
                        tracing::debug!("detect_file_type: no Cargo.toml found");
                        // No Cargo.toml found, check if it's standalone
                        if self.is_standalone_file(&file_path) {
                            tracing::debug!(
                                "detect_file_type: detected as Standalone (no cargo root, has appropriate content)"
                            );
                            Ok(FileType::Standalone)
                        } else {
                            tracing::debug!(
                                "detect_file_type: detected as CargoProject (no cargo root, default)"
                            );
                            Ok(FileType::CargoProject)
                        }
                    }
                }
            }
        }
    }

    /// Build the command
    pub fn build(self) -> Result<CargoCommand> {
        let file_type = self.detect_file_type()?;
        tracing::debug!(
            "CommandBuilder::build: detected file_type={:?}, runnable.kind={:?}, file_path={:?}",
            file_type,
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

        // Delegate to specific builders based on file type first, then kind
        match (file_type, &self.runnable.kind) {
            // Standalone files use rustc directly
            (FileType::Standalone, RunnableKind::Test { .. }) => RustcCommandBuilder::build(
                &self.runnable,
                self.package_name.as_deref(),
                &config,
                file_type,
            ),
            (FileType::Standalone, RunnableKind::Binary { .. }) => RustcCommandBuilder::build(
                &self.runnable,
                self.package_name.as_deref(),
                &config,
                file_type,
            ),
            (FileType::Standalone, RunnableKind::ModuleTests { .. }) => RustcCommandBuilder::build(
                &self.runnable,
                self.package_name.as_deref(),
                &config,
                file_type,
            ),
            (FileType::Standalone, RunnableKind::Benchmark { .. }) => RustcCommandBuilder::build(
                &self.runnable,
                self.package_name.as_deref(),
                &config,
                file_type,
            ),
            (FileType::Standalone, RunnableKind::Standalone { .. }) => RustcCommandBuilder::build(
                &self.runnable,
                self.package_name.as_deref(),
                &config,
                file_type,
            ),
            (FileType::Standalone, RunnableKind::DocTest { .. }) => {
                // Doc tests in standalone files aren't supported
                Err(crate::error::Error::ParseError(
                    "Doc tests are not supported in standalone files".to_string(),
                ))
            }
            (FileType::Standalone, RunnableKind::SingleFileScript { .. }) => {
                // This shouldn't happen - single file script should be FileType::SingleFileScript
                SingleFileScriptBuilder::build(
                    &self.runnable,
                    self.package_name.as_deref(),
                    &config,
                    file_type,
                )
            }

            // Single file scripts
            (FileType::SingleFileScript, _) => SingleFileScriptBuilder::build(
                &self.runnable,
                self.package_name.as_deref(),
                &config,
                file_type,
            ),

            // Cargo projects use cargo commands
            (FileType::CargoProject, RunnableKind::DocTest { .. }) => DocTestCommandBuilder::build(
                &self.runnable,
                self.package_name.as_deref(),
                &config,
                file_type,
            ),
            (FileType::CargoProject, RunnableKind::Test { .. }) => {
                tracing::debug!("Routing to TestCommandBuilder");
                TestCommandBuilder::build(
                    &self.runnable,
                    self.package_name.as_deref(),
                    &config,
                    file_type,
                )
            }
            (FileType::CargoProject, RunnableKind::Binary { .. }) => {
                tracing::debug!(
                    "Routing to BinaryCommandBuilder for package {:?}",
                    self.package_name
                );
                let result = BinaryCommandBuilder::build(
                    &self.runnable,
                    self.package_name.as_deref(),
                    &config,
                    file_type,
                );
                if let Ok(ref cmd) = result {
                    tracing::debug!("BinaryCommandBuilder returned command: {:?}", cmd.args);
                }
                result
            }
            (FileType::CargoProject, RunnableKind::ModuleTests { .. }) => {
                ModuleTestCommandBuilder::build(
                    &self.runnable,
                    self.package_name.as_deref(),
                    &config,
                    file_type,
                )
            }
            (FileType::CargoProject, RunnableKind::Benchmark { .. }) => {
                BenchmarkCommandBuilder::build(
                    &self.runnable,
                    self.package_name.as_deref(),
                    &config,
                    file_type,
                )
            }
            (FileType::CargoProject, RunnableKind::Standalone { .. }) => {
                // This shouldn't happen, but fallback to rustc
                RustcCommandBuilder::build(
                    &self.runnable,
                    self.package_name.as_deref(),
                    &config,
                    file_type,
                )
            }
            (FileType::CargoProject, RunnableKind::SingleFileScript { .. }) => {
                SingleFileScriptBuilder::build(
                    &self.runnable,
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
    ) -> Result<CargoCommand>;
}
