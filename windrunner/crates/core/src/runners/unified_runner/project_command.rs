use std::path::{Path, PathBuf};

use crate::{
    command::builder::rustc::single_file_script_builder::is_single_file_script_file,
    error::Result,
    parser::module_resolver::ModuleResolver,
    types::{Runnable, RunnableKind},
};

use super::UnifiedRunner;

impl UnifiedRunner {
    /// Get the override configuration for a specific runnable
    pub fn get_override_for_runnable(
        &self,
        runnable: &Runnable,
    ) -> Option<&crate::config::Override> {
        // Determine file type
        let file_type = match &runnable.kind {
            RunnableKind::SingleFileScript { .. } => crate::types::FileType::SingleFileScript,
            RunnableKind::Standalone { .. } => crate::types::FileType::Standalone,
            _ => crate::types::FileType::CargoProject,
        };

        // Create a FunctionIdentity from the runnable
        let identity = crate::types::FunctionIdentity {
            package: None, // TODO: Get package from runnable
            module_path: if runnable.module_path.is_empty() {
                None
            } else {
                Some(runnable.module_path.clone())
            },
            file_path: Some(runnable.file_path.clone()),
            function_name: match &runnable.kind {
                RunnableKind::Test { test_name, .. } => Some(test_name.clone()),
                RunnableKind::Benchmark { bench_name } => Some(bench_name.clone()),
                RunnableKind::DocTest {
                    struct_or_module_name,
                    method_name,
                } => {
                    if let Some(method) = method_name {
                        Some(format!("{struct_or_module_name}::{method}"))
                    } else {
                        Some(struct_or_module_name.clone())
                    }
                }
                _ => None,
            },
            file_type: Some(file_type),
        };

        self.config.get_override_for(&identity)
    }

    /// Resolve a file path, handling relative and absolute paths
    pub fn resolve_file_path(&mut self, file_path: &str) -> Result<PathBuf> {
        let path = Path::new(file_path);

        // If it's already an absolute path and exists, use it directly
        if path.is_absolute() && path.exists() {
            return Ok(path.to_path_buf());
        }

        // Try relative to current directory
        if let Ok(cwd) = std::env::current_dir() {
            let candidate = cwd.join(path);
            if candidate.exists() {
                return Ok(candidate);
            }
        }

        // Return the original path if we can't resolve it
        Ok(path.to_path_buf())
    }

    /// Detect the file type based on the file path and content
    pub fn detect_file_type(&self, file_path: &Path) -> Result<crate::types::FileType> {
        // Check for single-file script first (cargo script)
        if is_single_file_script_file(file_path) {
            return Ok(crate::types::FileType::SingleFileScript);
        }

        // Check if it's part of a cargo project
        if ModuleResolver::find_cargo_toml(file_path).is_some() {
            // Check if it's a library, binary, test, etc.
            let _file_name = file_path.file_name().and_then(|f| f.to_str()).unwrap_or("");
            Ok(crate::types::FileType::CargoProject)
        } else {
            // Standalone file
            Ok(crate::types::FileType::Standalone)
        }
    }

    /// Get the package name for a file path
    pub fn get_package_name_str(&self, file_path: &Path) -> Result<String> {
        let cargo_toml_path =
            ModuleResolver::find_cargo_toml(file_path).ok_or(crate::error::Error::NoCargoToml)?;

        let manifest = cargo_toml::Manifest::from_path(&cargo_toml_path)
            .map_err(crate::error::Error::CargoTomlParse)?;

        manifest
            .package
            .map(|p| p.name)
            .ok_or(crate::error::Error::NoPackageSection)
    }

    /// Find the config file path for a given file
    pub fn find_config_path(&self, file_path: &Path) -> Result<Option<PathBuf>> {
        let mut current_dir = if file_path.is_file() {
            file_path.parent().map(|p| p.to_path_buf())
        } else {
            Some(file_path.to_path_buf())
        };

        while let Some(dir) = current_dir {
            // Check for .cargo-runner.json
            let config_path = dir.join(".cargo-runner.json");
            if config_path.exists() {
                return Ok(Some(config_path));
            }

            // Check for cargo-runner.json
            let alt_config_path = dir.join("cargo-runner.json");
            if alt_config_path.exists() {
                return Ok(Some(alt_config_path));
            }

            // Move to parent directory
            current_dir = dir.parent().map(|p| p.to_path_buf());
        }

        Ok(None)
    }
}
