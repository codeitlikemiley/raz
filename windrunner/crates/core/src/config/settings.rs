use crate::{
    error::{Error, Result},
    types::FunctionIdentity,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::{BazelConfig, CargoConfig, Override, RustcConfig, SingleFileScriptConfig};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Config {
    // Command-type specific configurations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cargo: Option<CargoConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rustc: Option<RustcConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub single_file_script: Option<SingleFileScriptConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bazel: Option<BazelConfig>,

    // Overrides for specific functions
    #[serde(default)]
    pub overrides: Vec<Override>,
}

impl Config {
    /// Load config using the standard merging strategy
    pub fn load() -> Result<Self> {
        use super::merge::ConfigMerger;

        let mut merger = ConfigMerger::new();

        // Always load from current directory to get package-specific configs
        if let Ok(cwd) = std::env::current_dir() {
            merger.load_configs_for_path(&cwd)?;
        }

        // The merger will automatically pick up PROJECT_ROOT config from env var
        Ok(merger.get_merged_config())
    }

    pub fn load_from_file(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let config = serde_json::from_str(&contents)
            .map_err(|e| Error::ConfigError(format!("Failed to parse config: {e}")))?;
        Ok(config)
    }

    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let contents = serde_json::to_string_pretty(self)
            .map_err(|e| Error::ConfigError(format!("Failed to serialize config: {e}")))?;
        std::fs::write(path, contents)?;
        Ok(())
    }

    pub fn get_override_for(&self, identity: &FunctionIdentity) -> Option<&Override> {
        tracing::debug!(
            "Config::get_override_for called with identity: {:?}",
            identity
        );
        tracing::debug!("Available overrides: {}", self.overrides.len());
        for (i, override_) in self.overrides.iter().enumerate() {
            tracing::debug!("  Override {}: {:?}", i, override_.identity);
            if override_.identity.matches(identity) {
                tracing::debug!("  -> MATCH!");
                return Some(override_);
            } else {
                tracing::debug!("  -> No match");
            }
        }
        None
    }

    pub fn find_config_file(start_path: &Path) -> Option<PathBuf> {
        let mut current = start_path;

        loop {
            let config_path = current.join(".cargo-runner.json");
            if config_path.exists() {
                return Some(config_path);
            }

            let config_path = current.join("cargo-runner.json");
            if config_path.exists() {
                return Some(config_path);
            }

            current = current.parent()?;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FileType, FunctionIdentity};
    use std::collections::HashMap;

    #[test]
    fn test_config_serialization() {
        let config = Config {
            cargo: Some(CargoConfig {
                command: Some("cargo".to_string()),
                channel: Some("nightly".to_string()),
                extra_args: Some(vec!["--release".to_string()]),
                ..Default::default()
            }),
            overrides: vec![Override {
                identity: FunctionIdentity {
                    package: Some("my_crate".to_string()),
                    module_path: Some("my_crate::tests::unit".to_string()),
                    file_path: None,
                    function_name: Some("test_addition".to_string()),
                    file_type: Some(FileType::CargoProject),
                },
                cargo: Some(CargoConfig {
                    command: Some("cargo".to_string()),
                    subcommand: Some("nextest".to_string()),
                    extra_args: Some(vec!["--nocapture".to_string()]),
                    extra_test_binary_args: Some(vec!["--test-threads=1".to_string()]),
                    extra_env: Some(HashMap::from([(
                        "RUST_LOG".to_string(),
                        "debug".to_string(),
                    )])),
                    ..Default::default()
                }),
                rustc: None,
                single_file_script: None,
                bazel: None,
            }],
            ..Default::default()
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        println!("Serialized config:\n{json}");

        let parsed: Config = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.overrides.len(), 1);
        assert_eq!(
            parsed.cargo.as_ref().unwrap().channel,
            Some("nightly".to_string())
        );
    }

    #[test]
    fn test_get_override_for() {
        let config = Config {
            overrides: vec![Override {
                identity: FunctionIdentity {
                    package: Some("my_crate".to_string()),
                    module_path: Some("my_crate::tests".to_string()),
                    file_path: None,
                    function_name: Some("test_foo".to_string()),
                    file_type: Some(FileType::CargoProject),
                },
                cargo: Some(CargoConfig {
                    extra_args: Some(vec!["--nocapture".to_string()]),
                    ..Default::default()
                }),
                ..Default::default()
            }],
            ..Default::default()
        };

        let identity = FunctionIdentity {
            package: Some("my_crate".to_string()),
            module_path: Some("my_crate::tests".to_string()),
            file_path: None,
            function_name: Some("test_foo".to_string()),
            file_type: Some(FileType::CargoProject),
        };

        let override_config = config.get_override_for(&identity);
        assert!(override_config.is_some());
        assert_eq!(
            override_config.unwrap().cargo.as_ref().unwrap().extra_args,
            Some(vec!["--nocapture".to_string()])
        );

        // Test with different identity
        let identity2 = FunctionIdentity {
            package: Some("my_crate".to_string()),
            module_path: Some("my_crate::tests".to_string()),
            file_path: None,
            function_name: Some("test_bar".to_string()),
            file_type: Some(FileType::CargoProject),
        };

        let override_config2 = config.get_override_for(&identity2);
        assert!(override_config2.is_none());
    }
}
