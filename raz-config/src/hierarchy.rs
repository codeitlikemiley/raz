//! Configuration hierarchy management
//!
//! Implements a three-level configuration system:
//! 1. Project-level (.raz in nearest parent with Cargo.toml or existing .raz)
//! 2. Workspace-level (.raz in Cargo workspace root)
//! 3. Global-level (~/.raz)

use crate::error::{ConfigError, Result};
use std::env;
use std::path::{Path, PathBuf};

/// Configuration levels in order of precedence
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConfigLevel {
    /// Project-specific configuration
    Project,
    /// Workspace-level configuration
    Workspace,
    /// Global user configuration
    Global,
}

/// Represents a configuration location
#[derive(Debug, Clone)]
pub struct ConfigLocation {
    pub level: ConfigLevel,
    pub path: PathBuf,
    pub exists: bool,
}

/// Configuration hierarchy manager
pub struct ConfigHierarchy {
    locations: Vec<ConfigLocation>,
}

impl ConfigHierarchy {
    /// Discover configuration hierarchy for a given path
    pub fn discover(start_path: &Path) -> Result<Self> {
        let mut locations = Vec::new();

        // 1. Find project-level config
        if let Some(project_root) = find_project_root(start_path) {
            let config_path = project_root.join(".raz");
            locations.push(ConfigLocation {
                level: ConfigLevel::Project,
                path: config_path.clone(),
                exists: config_path.exists(),
            });
        }

        // 2. Find workspace-level config (if different from project)
        if let Some(workspace_root) = find_workspace_root(start_path) {
            let config_path = workspace_root.join(".raz");

            // Only add if it's different from project level
            if !locations.iter().any(|loc| loc.path == config_path) {
                locations.push(ConfigLocation {
                    level: ConfigLevel::Workspace,
                    path: config_path.clone(),
                    exists: config_path.exists(),
                });
            }
        }

        // 3. Global config
        if let Some(global_path) = get_global_config_path() {
            locations.push(ConfigLocation {
                level: ConfigLevel::Global,
                path: global_path.clone(),
                exists: global_path.exists(),
            });
        }

        Ok(Self { locations })
    }

    /// Get all configuration locations in order of precedence
    pub fn locations(&self) -> &[ConfigLocation] {
        &self.locations
    }

    /// Get the most specific existing configuration location
    pub fn primary_location(&self) -> Option<&ConfigLocation> {
        self.locations.iter().find(|loc| loc.exists)
    }

    /// Get or create the most appropriate configuration location
    pub fn get_or_create_location(
        &self,
        prefer_level: Option<ConfigLevel>,
    ) -> Result<&ConfigLocation> {
        // If a specific level is preferred and exists, use it
        if let Some(level) = prefer_level {
            if let Some(loc) = self.locations.iter().find(|l| l.level == level) {
                return Ok(loc);
            }
        }

        // Otherwise, use the most specific location (first in list)
        self.locations.first().ok_or_else(|| {
            ConfigError::ValidationError("No configuration location available".to_string())
        })
    }

    /// Initialize a configuration at the specified level
    pub fn init_config(&self, level: ConfigLevel) -> Result<PathBuf> {
        let location = self
            .locations
            .iter()
            .find(|loc| loc.level == level)
            .ok_or_else(|| {
                ConfigError::ValidationError(format!("No {level:?} level configuration path found"))
            })?;

        // Create the .raz directory if it doesn't exist
        std::fs::create_dir_all(&location.path)?;

        Ok(location.path.clone())
    }

    /// Get the path for override storage at the appropriate level
    pub fn get_override_storage_path(&self) -> Result<PathBuf> {
        // For overrides, prefer project level if available
        let location = self.get_or_create_location(Some(ConfigLevel::Project))?;
        Ok(location.path.join("overrides.toml"))
    }

    /// Aggregate override files from all levels
    pub fn get_all_override_paths(&self) -> Vec<PathBuf> {
        self.locations
            .iter()
            .filter(|loc| loc.exists)
            .map(|loc| loc.path.join("overrides.toml"))
            .filter(|path| path.exists())
            .collect()
    }
}

/// Find the nearest project root (directory with Cargo.toml or existing .raz)
fn find_project_root(start_path: &Path) -> Option<PathBuf> {
    let mut current = if start_path.is_file() {
        start_path.parent()?
    } else {
        start_path
    };

    loop {
        // Check for Cargo.toml (indicating a Rust project)
        if current.join("Cargo.toml").exists() {
            return Some(current.to_path_buf());
        }

        // Check for existing .raz directory (for non-Cargo projects)
        if current.join(".raz").exists() {
            return Some(current.to_path_buf());
        }

        current = current.parent()?;
    }
}

/// Find the Cargo workspace root
fn find_workspace_root(start_path: &Path) -> Option<PathBuf> {
    let mut current = if start_path.is_file() {
        start_path.parent()?
    } else {
        start_path
    };

    let mut _last_cargo_root = None;

    loop {
        let cargo_toml = current.join("Cargo.toml");
        if cargo_toml.exists() {
            // Check if this is a workspace root
            if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                if content.contains("[workspace]") {
                    return Some(current.to_path_buf());
                }
            }
            _last_cargo_root = Some(current.to_path_buf());
        }

        current = current.parent()?;
    }
}

/// Get the global configuration directory path
fn get_global_config_path() -> Option<PathBuf> {
    if let Ok(home) = env::var("HOME") {
        Some(PathBuf::from(home).join(".raz"))
    } else if let Ok(userprofile) = env::var("USERPROFILE") {
        // Windows fallback
        Some(PathBuf::from(userprofile).join(".raz"))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_config_hierarchy_discovery() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        // Create a workspace structure
        let workspace_cargo = "[workspace]\nmembers = [\"crate1\"]";
        fs::write(root.join("Cargo.toml"), workspace_cargo).unwrap();
        fs::create_dir_all(root.join(".raz")).unwrap();

        // Create a member crate
        let crate1 = root.join("crate1");
        fs::create_dir_all(&crate1).unwrap();
        fs::write(crate1.join("Cargo.toml"), "[package]\nname = \"crate1\"").unwrap();

        // Test from within the crate
        let hierarchy = ConfigHierarchy::discover(&crate1).unwrap();
        let locations = hierarchy.locations();

        // Should have project (crate1), workspace (root), and global
        assert!(locations.len() >= 2); // At least project and workspace

        // First should be project level
        assert_eq!(locations[0].level, ConfigLevel::Project);
        assert_eq!(locations[0].path, crate1.join(".raz"));

        // Second should be workspace level
        assert_eq!(locations[1].level, ConfigLevel::Workspace);
        assert_eq!(locations[1].path, root.join(".raz"));
    }

    #[test]
    fn test_standalone_file_config() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        // Create a standalone Rust file
        let rust_file = root.join("standalone.rs");
        fs::write(&rust_file, "fn main() {}").unwrap();

        // Create .raz directory in the same location
        fs::create_dir_all(root.join(".raz")).unwrap();

        let hierarchy = ConfigHierarchy::discover(&rust_file).unwrap();
        let locations = hierarchy.locations();

        // Should find the .raz directory
        assert!(!locations.is_empty());
        assert_eq!(locations[0].level, ConfigLevel::Project);
        assert_eq!(locations[0].path, root.join(".raz"));
        assert!(locations[0].exists);
    }
}
