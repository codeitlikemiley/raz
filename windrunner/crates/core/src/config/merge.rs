//! Configuration merging logic for cargo-runner
//!
//! Implements the merging hierarchy: root -> package (or workspace -> package)
//! The default mode is merge, but overrides can use force_replace to replace instead of merge

use super::{Config, Override};
use crate::error::Result;
use std::path::{Path, PathBuf};
use tracing::debug;

#[derive(Debug, Clone)]
pub struct ConfigInfo {
    pub root_config_path: Option<PathBuf>,
    pub workspace_config_path: Option<PathBuf>,
    pub package_config_path: Option<PathBuf>,
}

pub struct ConfigMerger {
    root_config: Option<Config>,
    workspace_config: Option<Config>,
    package_config: Option<Config>,
    config_info: ConfigInfo,
}

impl ConfigMerger {
    pub fn new() -> Self {
        Self {
            root_config: None,
            workspace_config: None,
            package_config: None,
            config_info: ConfigInfo {
                root_config_path: None,
                workspace_config_path: None,
                package_config_path: None,
            },
        }
    }

    /// Load all relevant configs for a given file path
    pub fn load_configs_for_path(&mut self, file_path: &Path) -> Result<()> {
        debug!("Loading configs for path: {:?}", file_path);

        // Find package-level config first
        if let Some(package_config_path) = Self::find_package_config(file_path) {
            debug!("Found package config at: {:?}", package_config_path);
            self.package_config = Some(Config::load_from_file(&package_config_path)?);
            self.config_info.package_config_path = Some(package_config_path);
        }

        // Find workspace config (if different from package)
        if let Some(workspace_config_path) = Self::find_workspace_config(file_path) {
            // Only load if it's different from package config
            if self.package_config.is_none()
                || self
                    .package_config
                    .as_ref()
                    .and_then(|_| Self::find_package_config(file_path))
                    != Some(workspace_config_path.clone())
            {
                debug!("Found workspace config at: {:?}", workspace_config_path);
                self.workspace_config = Some(Config::load_from_file(&workspace_config_path)?);
                self.config_info.workspace_config_path = Some(workspace_config_path);
            }
        }

        // Find PROJECT_ROOT config
        // First check if PROJECT_ROOT is set in environment
        let project_root = if let Ok(env_root) = std::env::var("PROJECT_ROOT") {
            PathBuf::from(env_root)
        } else {
            // If not set, use current directory or find .cargo-runner.json
            let cwd = std::env::current_dir().unwrap_or_default();
            // Walk up from current directory to find .cargo-runner.json
            let mut check_dir = cwd.as_path();
            let mut found_root = None;
            loop {
                if check_dir.join(".cargo-runner.json").exists() {
                    found_root = Some(check_dir.to_path_buf());
                    break;
                }
                match check_dir.parent() {
                    Some(parent) => check_dir = parent,
                    None => break,
                }
            }
            found_root.unwrap_or(cwd)
        };

        let root_config_path = project_root.join(".cargo-runner.json");
        if root_config_path.exists() {
            // Check if this is the same as package config to avoid loading twice
            // We need to handle the case where package config might be a relative path
            let is_same_as_package =
                if let Some(package_path) = &self.config_info.package_config_path {
                    // Try to canonicalize both paths
                    match (package_path.canonicalize(), root_config_path.canonicalize()) {
                        (Ok(p1), Ok(p2)) => p1 == p2,
                        _ => {
                            // If canonicalize fails, try absolute path comparison
                            let abs_package = if package_path.is_absolute() {
                                package_path.clone()
                            } else {
                                std::env::current_dir()
                                    .ok()
                                    .map(|cwd| cwd.join(package_path))
                                    .unwrap_or_else(|| package_path.clone())
                            };
                            abs_package == root_config_path
                        }
                    }
                } else {
                    false
                };

            if !is_same_as_package {
                debug!("Found root config at: {:?}", root_config_path);
                self.root_config = Some(Config::load_from_file(&root_config_path)?);
                self.config_info.root_config_path = Some(root_config_path);
            } else {
                debug!("Root config is same as package config, skipping duplicate load");
                self.config_info.root_config_path = self.config_info.package_config_path.clone();
            }
        }

        Ok(())
    }

    /// Get the merged configuration
    pub fn get_merged_config(&self) -> Config {
        let mut config = Config::default();

        // Start with root config
        if let Some(ref root) = self.root_config {
            config = self.merge_configs(config, root.clone(), false);
        }

        // Apply workspace config
        if let Some(ref workspace) = self.workspace_config {
            config = self.merge_configs(config, workspace.clone(), false);
        }

        // Apply package config
        if let Some(ref package) = self.package_config {
            config = self.merge_configs(config, package.clone(), false);
        }

        config
    }

    /// Merge two configs, respecting force_replace settings
    fn merge_configs(
        &self,
        mut base: Config,
        override_config: Config,
        force_replace: bool,
    ) -> Config {
        // Merge cargo config
        if let Some(override_cargo) = override_config.cargo {
            if base.cargo.is_none() {
                base.cargo = Some(override_cargo);
            } else if let Some(ref mut base_cargo) = base.cargo {
                self.merge_cargo_config(base_cargo, override_cargo, force_replace);
            }
        }

        // Merge rustc config
        if let Some(override_rustc) = override_config.rustc {
            if base.rustc.is_none() {
                base.rustc = Some(override_rustc);
            } else if let Some(ref mut base_rustc) = base.rustc {
                self.merge_rustc_config(base_rustc, override_rustc, force_replace);
            }
        }

        // Merge single_file_script config
        if let Some(override_sfs) = override_config.single_file_script {
            if base.single_file_script.is_none() {
                base.single_file_script = Some(override_sfs);
            } else if let Some(ref mut base_sfs) = base.single_file_script {
                self.merge_single_file_script_config(base_sfs, override_sfs, force_replace);
            }
        }

        // Merge overrides - these are function-specific, so we merge the arrays
        self.merge_overrides(&mut base.overrides, override_config.overrides);

        // Cache settings are internal and not merged

        base
    }

    /// Get information about which config files were loaded
    pub fn get_config_info(&self) -> &ConfigInfo {
        &self.config_info
    }

    fn merge_cargo_config(
        &self,
        base: &mut super::CargoConfig,
        override_config: super::CargoConfig,
        force_replace: bool,
    ) {
        if override_config.command.is_some() {
            base.command = override_config.command;
        }
        if override_config.subcommand.is_some() {
            base.subcommand = override_config.subcommand;
        }
        if override_config.channel.is_some() {
            base.channel = override_config.channel;
        }
        if override_config.package.is_some() {
            base.package = override_config.package;
        }

        // Merge features
        if override_config.features.is_some() {
            base.features =
                super::Features::merge(base.features.as_ref(), override_config.features.as_ref());
        }

        // Merge extra_args with deduplication
        if let Some(ref extra_args) = override_config.extra_args {
            if force_replace || base.extra_args.is_none() {
                base.extra_args = Some(extra_args.clone());
            } else if let Some(ref mut base_args) = base.extra_args {
                for arg in extra_args {
                    if !base_args.contains(arg) {
                        base_args.push(arg.clone());
                    }
                }
            }
        }

        // Merge extra_test_binary_args with deduplication
        if let Some(ref extra_test_args) = override_config.extra_test_binary_args {
            if force_replace || base.extra_test_binary_args.is_none() {
                base.extra_test_binary_args = Some(extra_test_args.clone());
            } else if let Some(ref mut base_args) = base.extra_test_binary_args {
                for arg in extra_test_args {
                    if !base_args.contains(arg) {
                        base_args.push(arg.clone());
                    }
                }
            }
        }

        // Merge env
        if let Some(ref env) = override_config.extra_env {
            if force_replace || base.extra_env.is_none() {
                base.extra_env = Some(env.clone());
            } else if let Some(ref mut base_env) = base.extra_env {
                base_env.extend(env.clone());
            }
        }

        // Merge test_framework
        if override_config.test_framework.is_some() {
            base.test_framework = override_config.test_framework;
        }

        // Merge binary_framework
        if override_config.binary_framework.is_some() {
            base.binary_framework = override_config.binary_framework;
        }

        // Merge linked_projects
        // Special handling: if PROJECT_ROOT is set and base already has linked_projects,
        // don't override them with local config
        if override_config.linked_projects.is_some() {
            if std::env::var("PROJECT_ROOT").is_ok() && base.linked_projects.is_some() {
                // Keep the root config's linked_projects when PROJECT_ROOT is set
                // This ensures subdirectory configs don't lose the workspace-wide linked_projects
            } else {
                base.linked_projects = override_config.linked_projects;
            }
        }
    }

    fn merge_rustc_config(
        &self,
        base: &mut super::RustcConfig,
        override_config: super::RustcConfig,
        force_replace: bool,
    ) {
        // Merge test_framework
        if let Some(test_framework) = override_config.test_framework {
            if force_replace || base.test_framework.is_none() {
                base.test_framework = Some(test_framework);
            } else if let Some(ref mut base_framework) = base.test_framework {
                self.merge_rustc_framework(base_framework, test_framework);
            }
        }

        // Merge binary_framework
        if let Some(binary_framework) = override_config.binary_framework {
            if force_replace || base.binary_framework.is_none() {
                base.binary_framework = Some(binary_framework);
            } else if let Some(ref mut base_framework) = base.binary_framework {
                self.merge_rustc_framework(base_framework, binary_framework);
            }
        }

        // Merge benchmark_framework
        if let Some(benchmark_framework) = override_config.benchmark_framework {
            if force_replace || base.benchmark_framework.is_none() {
                base.benchmark_framework = Some(benchmark_framework);
            } else if let Some(ref mut base_framework) = base.benchmark_framework {
                self.merge_rustc_framework(base_framework, benchmark_framework);
            }
        }
    }

    fn merge_rustc_framework(
        &self,
        base: &mut super::RustcFramework,
        override_framework: super::RustcFramework,
    ) {
        // Merge build phase
        if let Some(build) = override_framework.build {
            if base.build.is_none() {
                base.build = Some(build);
            } else if let Some(ref mut base_build) = base.build {
                self.merge_rustc_phase_config(base_build, build);
            }
        }

        // Merge exec phase
        if let Some(exec) = override_framework.exec {
            if base.exec.is_none() {
                base.exec = Some(exec);
            } else if let Some(ref mut base_exec) = base.exec {
                self.merge_rustc_phase_config(base_exec, exec);
            }
        }
    }

    fn merge_rustc_phase_config(
        &self,
        base: &mut super::RustcPhaseConfig,
        override_phase: super::RustcPhaseConfig,
    ) {
        // Override command if provided
        if override_phase.command.is_some() {
            base.command = override_phase.command;
        }

        // Override args if provided (don't merge, replace)
        if override_phase.args.is_some() {
            base.args = override_phase.args;
        }

        // Merge extra_args with deduplication
        if let Some(ref extra_args) = override_phase.extra_args {
            if base.extra_args.is_none() {
                base.extra_args = Some(extra_args.clone());
            } else if let Some(ref mut base_args) = base.extra_args {
                for arg in extra_args {
                    if !base_args.contains(arg) {
                        base_args.push(arg.clone());
                    }
                }
            }
        }

        // Merge extra_test_binary_args with deduplication
        if let Some(ref extra_test_binary_args) = override_phase.extra_test_binary_args {
            if base.extra_test_binary_args.is_none() {
                base.extra_test_binary_args = Some(extra_test_binary_args.clone());
            } else if let Some(ref mut base_args) = base.extra_test_binary_args {
                for arg in extra_test_binary_args {
                    if !base_args.contains(arg) {
                        base_args.push(arg.clone());
                    }
                }
            }
        }
    }

    fn merge_single_file_script_config(
        &self,
        base: &mut super::SingleFileScriptConfig,
        override_config: super::SingleFileScriptConfig,
        force_replace: bool,
    ) {
        // Merge extra_args with deduplication
        if let Some(ref extra_args) = override_config.extra_args {
            if force_replace || base.extra_args.is_none() {
                base.extra_args = Some(extra_args.clone());
            } else if let Some(ref mut base_args) = base.extra_args {
                for arg in extra_args {
                    if !base_args.contains(arg) {
                        base_args.push(arg.clone());
                    }
                }
            }
        }

        // Merge env
        if let Some(ref env) = override_config.extra_env {
            if force_replace || base.extra_env.is_none() {
                base.extra_env = Some(env.clone());
            } else if let Some(ref mut base_env) = base.extra_env {
                base_env.extend(env.clone());
            }
        }

        // Merge extra_test_binary_args with deduplication
        if let Some(ref extra_test_binary_args) = override_config.extra_test_binary_args {
            if force_replace || base.extra_test_binary_args.is_none() {
                base.extra_test_binary_args = Some(extra_test_binary_args.clone());
            } else if let Some(ref mut base_args) = base.extra_test_binary_args {
                for arg in extra_test_binary_args {
                    if !base_args.contains(arg) {
                        base_args.push(arg.clone());
                    }
                }
            }
        }
    }

    /// Merge override arrays, handling force_replace per override
    fn merge_overrides(&self, base_overrides: &mut Vec<Override>, new_overrides: Vec<Override>) {
        for new_override in new_overrides {
            // Check if we already have an override for this identity
            if let Some(existing) = base_overrides
                .iter_mut()
                .find(|o| o.identity == new_override.identity)
            {
                // Only merge the nested configs now

                // Merge nested configs
                if let Some(new_cargo) = new_override.cargo {
                    if existing.cargo.is_none() {
                        existing.cargo = Some(new_cargo);
                    } else if let Some(ref mut existing_cargo) = existing.cargo {
                        self.merge_cargo_config(existing_cargo, new_cargo, false);
                    }
                }

                if let Some(new_rustc) = new_override.rustc {
                    if existing.rustc.is_none() {
                        existing.rustc = Some(new_rustc);
                    } else if let Some(ref mut existing_rustc) = existing.rustc {
                        self.merge_rustc_config(existing_rustc, new_rustc, false);
                    }
                }

                if let Some(new_sfs) = new_override.single_file_script {
                    if existing.single_file_script.is_none() {
                        existing.single_file_script = Some(new_sfs);
                    } else if let Some(ref mut existing_sfs) = existing.single_file_script {
                        self.merge_single_file_script_config(existing_sfs, new_sfs, false);
                    }
                }
            } else {
                // No existing override for this identity, add the new one
                base_overrides.push(new_override);
            }
        }
    }

    /// Find the nearest .cargo-runner.json for a package
    fn find_package_config(path: &Path) -> Option<PathBuf> {
        let mut current = path;
        if current.is_file() {
            current = current.parent()?;
        }

        loop {
            // Check for config file
            let config_path = current.join(".cargo-runner.json");
            if config_path.exists() {
                return Some(config_path);
            }

            // Check if we've hit a Cargo.toml (package boundary)
            if current.join("Cargo.toml").exists() {
                // This is a package root, but no config found
                return None;
            }

            current = current.parent()?;
        }
    }

    /// Find workspace-level config
    fn find_workspace_config(path: &Path) -> Option<PathBuf> {
        let mut current = path;
        if current.is_file() {
            current = current.parent()?;
        }

        let mut _found_package_root = false;

        loop {
            let cargo_toml = current.join("Cargo.toml");
            if cargo_toml.exists() {
                // Check if this is a workspace
                if let Ok(contents) = std::fs::read_to_string(&cargo_toml) {
                    if contents.contains("[workspace]") {
                        let config_path = current.join(".cargo-runner.json");
                        if config_path.exists() {
                            return Some(config_path);
                        }
                    }
                }
                _found_package_root = true;
            }

            // If we've passed a package root and still looking, we might find workspace
            current = current.parent()?;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CargoConfig;
    use std::collections::HashMap;

    #[test]
    fn test_config_merging() {
        let base = Config {
            cargo: Some(CargoConfig {
                command: Some("cargo".to_string()),
                extra_args: Some(vec!["--release".to_string()]),
                extra_env: Some(HashMap::from([(
                    "RUST_LOG".to_string(),
                    "info".to_string(),
                )])),
                ..Default::default()
            }),
            ..Default::default()
        };

        let override_config = Config {
            cargo: Some(CargoConfig {
                channel: Some("nightly".to_string()),
                extra_args: Some(vec!["--features".to_string(), "foo".to_string()]),
                extra_env: Some(HashMap::from([(
                    "RUST_BACKTRACE".to_string(),
                    "1".to_string(),
                )])),
                ..Default::default()
            }),
            ..Default::default()
        };

        let merger = ConfigMerger::new();
        let merged = merger.merge_configs(base, override_config, false);

        let cargo_config = merged.cargo.unwrap();
        assert_eq!(cargo_config.command, Some("cargo".to_string()));
        assert_eq!(cargo_config.channel, Some("nightly".to_string()));
        assert_eq!(
            cargo_config.extra_args,
            Some(vec![
                "--release".to_string(),
                "--features".to_string(),
                "foo".to_string()
            ])
        );
        let env = cargo_config.extra_env.unwrap();
        assert_eq!(env.get("RUST_LOG"), Some(&"info".to_string()));
        assert_eq!(env.get("RUST_BACKTRACE"), Some(&"1".to_string()));
    }

    #[test]
    fn test_force_replace() {
        let base = Config {
            cargo: Some(CargoConfig {
                extra_args: Some(vec!["--release".to_string()]),
                extra_env: Some(HashMap::from([(
                    "RUST_LOG".to_string(),
                    "info".to_string(),
                )])),
                ..Default::default()
            }),
            ..Default::default()
        };

        let override_config = Config {
            cargo: Some(CargoConfig {
                extra_args: Some(vec!["--debug".to_string()]),
                extra_env: Some(HashMap::from([(
                    "RUST_LOG".to_string(),
                    "debug".to_string(),
                )])),
                ..Default::default()
            }),
            ..Default::default()
        };

        let merger = ConfigMerger::new();
        let merged = merger.merge_configs(base, override_config, true);

        let cargo_config = merged.cargo.unwrap();
        // With force_replace, args should be replaced, not merged
        assert_eq!(cargo_config.extra_args, Some(vec!["--debug".to_string()]));
        // Env should also be replaced
        assert_eq!(
            cargo_config.extra_env.unwrap().get("RUST_LOG"),
            Some(&"debug".to_string())
        );
    }
}
