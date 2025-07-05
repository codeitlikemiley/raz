use crate::{
    error::{ConfigError, Result},
    schema::ConfigVersion,
};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

type MigrationFn = Box<dyn Fn(JsonValue) -> Result<JsonValue>>;

pub struct ConfigMigrator {
    migrations: HashMap<(u32, u32), MigrationFn>,
}

impl ConfigMigrator {
    pub fn new() -> Self {
        let mut migrator = Self {
            migrations: HashMap::new(),
        };

        migrator.register_migrations();
        migrator
    }

    fn register_migrations(&mut self) {
        // Example: v0 to v1 migration (when we have legacy configs)
        // self.add_migration(0, 1, Box::new(|config| {
        //     let mut config = config;
        //     // Perform migration steps
        //     Ok(config)
        // }));
    }

    pub fn add_migration(&mut self, from_version: u32, to_version: u32, migration: MigrationFn) {
        self.migrations
            .insert((from_version, to_version), migration);
    }

    pub fn migrate_to_latest(&self, config_str: &str) -> Result<String> {
        let mut config: JsonValue = serde_json::from_str(config_str).map_err(|e| {
            ConfigError::MigrationError(format!("Failed to parse config as JSON: {e}"))
        })?;

        let current_version = self.detect_version(&config)?;
        let target_version = ConfigVersion::CURRENT;

        if current_version >= target_version {
            return Ok(config_str.to_string());
        }

        config = self.migrate_between(config, current_version, target_version)?;

        let toml_value: toml::Value = serde_json::from_value(config)
            .map_err(|e| ConfigError::MigrationError(e.to_string()))?;

        toml::to_string_pretty(&toml_value).map_err(ConfigError::SerializeError)
    }

    fn migrate_between(
        &self,
        mut config: JsonValue,
        from: ConfigVersion,
        to: ConfigVersion,
    ) -> Result<JsonValue> {
        let mut current = from.0;
        let target = to.0;

        while current < target {
            let next = current + 1;

            if let Some(migration) = self.migrations.get(&(current, next)) {
                config = migration(config)?;
            } else {
                return Err(ConfigError::MigrationError(format!(
                    "No migration path from v{current} to v{next}"
                )));
            }

            current = next;
        }

        // Update version in config
        if let Some(raz) = config.get_mut("raz") {
            if let Some(version) = raz.get_mut("version") {
                *version = JsonValue::Number(target.into());
            }
        }

        Ok(config)
    }

    fn detect_version(&self, config: &JsonValue) -> Result<ConfigVersion> {
        if let Some(raz) = config.get("raz") {
            if let Some(version) = raz.get("version") {
                if let Some(v) = version.as_u64() {
                    return Ok(ConfigVersion(v as u32));
                }
            }
        }

        // If no version field, assume it's v0 (legacy)
        Ok(ConfigVersion(0))
    }

    pub fn check_migration_needed(config_str: &str) -> Result<bool> {
        let config: toml::Value = toml::from_str(config_str)?;

        if let Some(raz) = config.get("raz") {
            if let Some(version) = raz.get("version") {
                if let Some(v) = version.as_integer() {
                    return Ok(ConfigVersion(v as u32) < ConfigVersion::CURRENT);
                }
            }
        }

        // No version means migration needed
        Ok(true)
    }
}

impl Default for ConfigMigrator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_detection() {
        let migrator = ConfigMigrator::new();

        let config_v1 = r#"{"raz": {"version": 1}}"#;
        let config: JsonValue = serde_json::from_str(config_v1).unwrap();
        let version = migrator.detect_version(&config).unwrap();
        assert_eq!(version.0, 1);

        let config_no_version = r#"{"raz": {}}"#;
        let config: JsonValue = serde_json::from_str(config_no_version).unwrap();
        let version = migrator.detect_version(&config).unwrap();
        assert_eq!(version.0, 0);
    }
}
