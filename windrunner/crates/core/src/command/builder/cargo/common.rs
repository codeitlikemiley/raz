//! Common functionality for cargo builders

use crate::{
    config::{Config, Features},
    types::{FileType, FunctionIdentity, Runnable},
};

/// Helper trait for common builder functionality
pub trait CargoBuilderHelper {
    /// Find the cargo root directory for a given file with smart PROJECT_ROOT resolution
    fn find_cargo_root(&self, file_path: &std::path::Path) -> Option<std::path::PathBuf> {
        tracing::debug!("find_cargo_root called with: {:?}", file_path);

        // First, ensure we have an absolute path
        let abs_path = if file_path.is_absolute() {
            file_path.to_path_buf()
        } else {
            let cwd = std::env::current_dir().ok()?;
            tracing::debug!("CWD: {:?}", cwd);
            cwd.join(file_path)
        };

        tracing::debug!("Absolute path: {:?}", abs_path);

        // Smart resolution: Look for PROJECT_ROOT with linkedProjects
        if let Some(project_root) = self.find_project_root_with_linked_projects(&abs_path) {
            tracing::debug!(
                "Found PROJECT_ROOT with linkedProjects at: {:?}",
                project_root
            );

            // Load the config to get linkedProjects
            let config_path = project_root.join(".cargo-runner.json");
            if let Ok(config_str) = std::fs::read_to_string(&config_path) {
                if let Ok(config_json) = serde_json::from_str::<serde_json::Value>(&config_str) {
                    if let Some(linked_projects) = config_json
                        .get("cargo")
                        .and_then(|c| c.get("linkedProjects"))
                        .and_then(|lp| lp.as_array())
                    {
                        // Find which linked project contains our file
                        for project in linked_projects {
                            if let Some(cargo_toml_path) = project.as_str() {
                                // linked_projects contains paths to Cargo.toml files relative to PROJECT_ROOT
                                let cargo_toml_path = project_root.join(cargo_toml_path);

                                // Get the project directory (parent of Cargo.toml)
                                if let Some(project_dir) = cargo_toml_path.parent() {
                                    if abs_path.starts_with(project_dir) {
                                        tracing::debug!(
                                            "File belongs to linked project: {:?}",
                                            project_dir
                                        );
                                        return Some(project_dir.to_path_buf());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Fallback to standard cargo root detection
        let result = abs_path
            .ancestors()
            .find(|p| p.join("Cargo.toml").exists())
            .map(|p| p.to_path_buf());

        tracing::debug!("find_cargo_root result: {:?}", result);
        result
    }

    /// Find PROJECT_ROOT by looking for .cargo-runner.json with linkedProjects
    fn find_project_root_with_linked_projects(
        &self,
        from_path: &std::path::Path,
    ) -> Option<std::path::PathBuf> {
        // Check PROJECT_ROOT env first
        if let Ok(env_root) = std::env::var("PROJECT_ROOT") {
            let root = std::path::PathBuf::from(env_root);
            if self.has_linked_projects(&root) {
                return Some(root);
            }
        }

        // Walk up directory tree looking for .cargo-runner.json with linkedProjects
        let start_dir = if from_path.is_file() {
            from_path.parent()?
        } else {
            from_path
        };

        for ancestor in start_dir.ancestors() {
            if self.has_linked_projects(ancestor) {
                return Some(ancestor.to_path_buf());
            }
        }

        None
    }

    /// Check if a directory has .cargo-runner.json with linkedProjects
    fn has_linked_projects(&self, dir: &std::path::Path) -> bool {
        let config_path = dir.join(".cargo-runner.json");
        if config_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&config_path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    return json
                        .get("cargo")
                        .and_then(|c| c.get("linkedProjects"))
                        .and_then(|lp| lp.as_array())
                        .map(|arr| !arr.is_empty())
                        .unwrap_or(false);
                }
            }
        }
        false
    }

    fn get_override<'a>(
        &self,
        runnable: &Runnable,
        config: &'a Config,
        file_type: FileType,
    ) -> Option<&'a crate::config::Override> {
        let identity = self.create_identity(runnable, config, file_type);
        tracing::debug!("Looking for override for identity: {:?}", identity);
        let result = config.get_override_for(&identity);
        if result.is_some() {
            tracing::debug!("Found matching override!");
        } else {
            tracing::debug!("No matching override found");
        }
        result
    }

    fn create_identity(
        &self,
        runnable: &Runnable,
        config: &Config,
        file_type: FileType,
    ) -> FunctionIdentity {
        FunctionIdentity {
            package: config.cargo.as_ref().and_then(|c| c.package.clone()),
            module_path: if runnable.module_path.is_empty() {
                None
            } else {
                Some(runnable.module_path.clone())
            },
            file_path: Some(runnable.file_path.clone()),
            function_name: runnable.get_function_name(),
            file_type: Some(file_type),
        }
    }

    fn apply_features(
        &self,
        args: &mut Vec<String>,
        runnable: &Runnable,
        config: &Config,
        file_type: FileType,
        features: Option<&Features>,
    ) {
        // Features are only applicable to Cargo projects
        if file_type != FileType::CargoProject {
            return;
        }

        // Apply override features
        if let Some(override_config) = self.get_override(runnable, config, file_type) {
            if let Some(override_cargo) = &override_config.cargo {
                if let Some(features) = &override_cargo.features {
                    args.extend(features.to_args());
                    // Features are merged by default now
                }
            }
        }

        // Apply provided features
        if let Some(features) = features {
            args.extend(features.to_args());
        }
    }

    fn apply_common_config(
        &self,
        command: &mut crate::command::CargoCommand,
        _config: &Config,
        _file_type: FileType,
        extra_env: Option<&std::collections::HashMap<String, String>>,
    ) {
        // Apply environment variables based on file type
        if let Some(extra_env) = extra_env {
            for (key, value) in extra_env {
                command.env.push((key.clone(), value.clone()));
            }
        }
    }
}
