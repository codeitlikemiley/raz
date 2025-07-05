use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ConfigVersion(pub u32);

impl ConfigVersion {
    pub const V1: Self = Self(1);
    pub const CURRENT: Self = Self(crate::CONFIG_CURRENT_VERSION);

    pub fn is_compatible(&self, other: &Self) -> bool {
        self.0 <= other.0
    }

    pub fn needs_migration(&self, target: &Self) -> bool {
        self.0 < target.0
    }
}

impl fmt::Display for ConfigVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{}", self.0)
    }
}

impl Default for ConfigVersion {
    fn default() -> Self {
        Self::CURRENT
    }
}

#[derive(Debug, Clone)]
pub struct ConfigSchema {
    pub version: ConfigVersion,
    pub required_fields: Vec<String>,
    pub optional_fields: Vec<String>,
    pub deprecated_fields: Vec<String>,
}

impl ConfigSchema {
    pub fn v1() -> Self {
        Self {
            version: ConfigVersion::V1,
            required_fields: vec![
                "version".to_string(),
                "enabled".to_string(),
                "providers".to_string(),
            ],
            optional_fields: vec![
                "cache_dir".to_string(),
                "cache_ttl".to_string(),
                "parallel_execution".to_string(),
                "max_concurrent_jobs".to_string(),
                "providers_config".to_string(),
                "filters".to_string(),
                "ui".to_string(),
                "commands".to_string(),
                "overrides".to_string(),
            ],
            deprecated_fields: vec![],
        }
    }

    pub fn for_version(version: ConfigVersion) -> Self {
        match version.0 {
            1 => Self::v1(),
            _ => Self::v1(), // Default to latest
        }
    }

    pub fn validate_fields(&self, fields: &[String]) -> Result<(), Vec<String>> {
        let mut missing = Vec::new();

        for required in &self.required_fields {
            if !fields.contains(required) {
                missing.push(required.clone());
            }
        }

        if missing.is_empty() {
            Ok(())
        } else {
            Err(missing)
        }
    }

    pub fn get_deprecated_fields(&self, fields: &[String]) -> Vec<String> {
        fields
            .iter()
            .filter(|f| self.deprecated_fields.contains(f))
            .cloned()
            .collect()
    }
}
