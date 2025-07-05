use crate::{
    OverrideEntry,
    backup::BackupManager,
    error::{OverrideError, Result},
};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Override storage format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageFormat {
    version: u32,
    overrides: IndexMap<String, OverrideEntry>,
}

impl Default for StorageFormat {
    fn default() -> Self {
        Self {
            version: 1,
            overrides: IndexMap::new(),
        }
    }
}

/// Manages persistent storage of overrides
pub struct OverrideStorage {
    storage_path: PathBuf,
    #[allow(dead_code)]
    workspace_path: PathBuf,
    cache: RwLock<StorageFormat>,
    backup_manager: Arc<BackupManager>,
}

impl OverrideStorage {
    /// Create new storage instance
    pub fn new(workspace_path: &Path) -> Result<Self> {
        let storage_path = workspace_path.join(".raz").join("overrides.toml");

        // Ensure directory exists
        if let Some(parent) = storage_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Load existing data or create new
        let cache = if storage_path.exists() {
            let content = std::fs::read_to_string(&storage_path)?;
            toml::from_str(&content)?
        } else {
            StorageFormat::default()
        };

        // Create backup manager with default 5 backups
        let backup_manager = Arc::new(BackupManager::new(workspace_path, 5)?);

        Ok(Self {
            storage_path,
            workspace_path: workspace_path.to_path_buf(),
            cache: RwLock::new(cache),
            backup_manager,
        })
    }

    /// Save an override entry
    pub fn save(&self, entry: &OverrideEntry) -> Result<()> {
        let mut cache = self
            .cache
            .write()
            .map_err(|e| OverrideError::StorageError(format!("Lock poisoned: {e}")))?;

        // Create backup before modification
        let _ = self
            .backup_manager
            .create_backup(&*cache)
            .map_err(|e| log::warn!("Failed to create backup: {e}"));

        // Store by primary key
        cache
            .overrides
            .insert(entry.key.primary.clone(), entry.clone());

        // Persist to disk
        self.persist(&cache)?;

        Ok(())
    }

    /// Save an override entry with validation check
    pub fn save_with_validation(
        &self,
        entry: &OverrideEntry,
        validate_fn: impl FnOnce(&OverrideEntry) -> Result<()>,
    ) -> Result<()> {
        // Validate before any modifications
        validate_fn(entry)?;

        // If validation passes, save with backup
        self.save(entry)
    }

    /// Get override by primary key
    pub fn get_by_primary_key(&self, key: &str) -> Result<Option<OverrideEntry>> {
        let cache = self
            .cache
            .read()
            .map_err(|e| OverrideError::StorageError(format!("Lock poisoned: {e}")))?;

        Ok(cache.overrides.get(key).cloned())
    }

    /// Get override by any key (primary or fallback)
    pub fn get_by_key(&self, key: &str) -> Result<Option<OverrideEntry>> {
        let cache = self
            .cache
            .read()
            .map_err(|e| OverrideError::StorageError(format!("Lock poisoned: {e}")))?;

        // First try primary key
        if let Some(entry) = cache.overrides.get(key) {
            return Ok(Some(entry.clone()));
        }

        // Then search fallbacks
        for entry in cache.overrides.values() {
            if entry.key.fallbacks.contains(&key.to_string()) {
                return Ok(Some(entry.clone()));
            }
        }

        Ok(None)
    }

    /// Get all overrides for a file
    pub fn get_by_file(&self, file_path: &Path) -> Result<Vec<OverrideEntry>> {
        let cache = self
            .cache
            .read()
            .map_err(|e| OverrideError::StorageError(format!("Lock poisoned: {e}")))?;

        let normalized_path = file_path.to_string_lossy().replace('\\', "/");

        Ok(cache
            .overrides
            .values()
            .filter(|entry| {
                let entry_path = entry
                    .metadata
                    .file_path
                    .to_string_lossy()
                    .replace('\\', "/");
                entry_path == normalized_path
            })
            .cloned()
            .collect())
    }

    /// Delete an override
    pub fn delete(&self, key: &str) -> Result<bool> {
        let mut cache = self
            .cache
            .write()
            .map_err(|e| OverrideError::StorageError(format!("Lock poisoned: {e}")))?;

        let removed = cache.overrides.shift_remove(key).is_some();

        if removed {
            self.persist(&cache)?;
        }

        Ok(removed)
    }

    /// List all overrides
    pub fn list_all(&self) -> Result<Vec<OverrideEntry>> {
        let cache = self
            .cache
            .read()
            .map_err(|e| OverrideError::StorageError(format!("Lock poisoned: {e}")))?;

        Ok(cache.overrides.values().cloned().collect())
    }

    /// Clear all overrides
    pub fn clear(&self) -> Result<()> {
        let mut cache = self
            .cache
            .write()
            .map_err(|e| OverrideError::StorageError(format!("Lock poisoned: {e}")))?;

        cache.overrides.clear();
        self.persist(&cache)?;

        Ok(())
    }

    /// Migrate overrides from old format
    pub fn migrate_from_legacy(&self, legacy_data: &str) -> Result<Vec<String>> {
        let mut migrated_keys = Vec::new();

        // Parse legacy format (simplified for now)
        // In real implementation, this would handle the old format
        if let Ok(legacy_map) = toml::from_str::<toml::Value>(legacy_data) {
            if let Some(overrides) = legacy_map.get("overrides").and_then(|v| v.as_table()) {
                for (old_key, _value) in overrides {
                    // Convert old key format to new
                    // Old: "file:line:column"
                    // New: "file:function_name" or "file:L<line>"

                    log::info!("Migrating override: {old_key}");
                    migrated_keys.push(old_key.clone());
                }
            }
        }

        Ok(migrated_keys)
    }

    /// Persist cache to disk
    fn persist(&self, cache: &StorageFormat) -> Result<()> {
        let content = toml::to_string_pretty(cache)?;
        std::fs::write(&self.storage_path, content)?;
        Ok(())
    }

    /// Update execution status of an override
    pub fn update_execution_status(&self, key: &str, success: bool) -> Result<()> {
        let mut cache = self
            .cache
            .write()
            .map_err(|e| OverrideError::StorageError(format!("Lock poisoned: {e}")))?;

        if let Some(entry) = cache.overrides.get_mut(key) {
            let now = chrono::Utc::now();
            entry.metadata.last_execution_time = Some(now);
            entry.metadata.last_execution_success = Some(success);

            if success {
                entry.metadata.validation_status = crate::ValidationStatus::Validated;
                entry.metadata.failure_count = 0;
            } else {
                entry.metadata.failure_count += 1;
                entry.metadata.validation_status =
                    crate::ValidationStatus::Failed(entry.metadata.failure_count);
            }

            entry.metadata.modified_at = now;

            // Persist changes
            self.persist(&cache)?;
        }

        Ok(())
    }

    /// Rollback to the last backup
    pub fn rollback_to_last_backup(&self) -> Result<()> {
        if let Some(backup_info) = self.backup_manager.get_last_backup()? {
            let restored: StorageFormat = self.backup_manager.restore_backup(&backup_info.path)?;

            let mut cache = self
                .cache
                .write()
                .map_err(|e| OverrideError::StorageError(format!("Lock poisoned: {e}")))?;

            *cache = restored;
            self.persist(&cache)?;

            log::info!("Rolled back to backup from {}", backup_info.created_at);
            Ok(())
        } else {
            Err(OverrideError::StorageError(
                "No backup available for rollback".to_string(),
            ))
        }
    }

    /// Get backup manager reference
    pub fn backup_manager(&self) -> &BackupManager {
        &self.backup_manager
    }

    /// Export overrides for backup
    pub fn export(&self) -> Result<String> {
        let cache = self
            .cache
            .read()
            .map_err(|e| OverrideError::StorageError(format!("Lock poisoned: {e}")))?;

        Ok(toml::to_string_pretty(&*cache)?)
    }

    /// Import overrides from backup
    pub fn import(&self, data: &str) -> Result<usize> {
        let imported: StorageFormat = toml::from_str(data)?;

        let mut cache = self
            .cache
            .write()
            .map_err(|e| OverrideError::StorageError(format!("Lock poisoned: {e}")))?;

        let count = imported.overrides.len();

        // Merge with existing (imported takes precedence)
        for (key, entry) in imported.overrides {
            cache.overrides.insert(key, entry);
        }

        self.persist(&cache)?;

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CommandOverride, FunctionContext, OverrideKey, OverrideMetadata};
    use tempfile::TempDir;

    #[test]
    fn test_storage_operations() {
        let temp_dir = TempDir::new().unwrap();
        let storage = OverrideStorage::new(temp_dir.path()).unwrap();

        // Create test entry
        let context = FunctionContext {
            file_path: PathBuf::from("src/test.rs"),
            function_name: Some("test_func".to_string()),
            line_number: 10,
            context: None,
        };

        let key = OverrideKey::new(&context).unwrap();
        let entry = OverrideEntry {
            key: key.clone(),
            override_config: CommandOverride::new("test".to_string()),
            metadata: OverrideMetadata {
                created_at: chrono::Utc::now(),
                modified_at: chrono::Utc::now(),
                file_path: context.file_path.clone(),
                function_name: context.function_name.clone(),
                original_line: Some(context.line_number),
                notes: None,
                validation_status: crate::ValidationStatus::Pending,
                last_execution_time: None,
                last_execution_success: None,
                failure_count: 0,
            },
        };

        // Test save
        storage.save(&entry).unwrap();

        // Test get by primary key
        let retrieved = storage.get_by_primary_key(&key.primary).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().key.primary, key.primary);

        // Test get by file
        let file_overrides = storage.get_by_file(&context.file_path).unwrap();
        assert_eq!(file_overrides.len(), 1);

        // Test delete
        assert!(storage.delete(&key.primary).unwrap());
        assert!(storage.get_by_primary_key(&key.primary).unwrap().is_none());
    }

    #[test]
    fn test_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let storage_path = temp_dir.path();

        // Create and save
        {
            let storage = OverrideStorage::new(storage_path).unwrap();

            let context = FunctionContext {
                file_path: PathBuf::from("src/main.rs"),
                function_name: Some("main".to_string()),
                line_number: 1,
                context: None,
            };

            let key = OverrideKey::new(&context).unwrap();
            let entry = OverrideEntry {
                key,
                override_config: CommandOverride::new("run".to_string()),
                metadata: OverrideMetadata {
                    created_at: chrono::Utc::now(),
                    modified_at: chrono::Utc::now(),
                    file_path: context.file_path,
                    function_name: context.function_name,
                    original_line: Some(context.line_number),
                    notes: Some("Test override".to_string()),
                    validation_status: crate::ValidationStatus::Pending,
                    last_execution_time: None,
                    last_execution_success: None,
                    failure_count: 0,
                },
            };

            storage.save(&entry).unwrap();
        }

        // Load and verify
        {
            let storage = OverrideStorage::new(storage_path).unwrap();
            let all = storage.list_all().unwrap();
            assert_eq!(all.len(), 1);
            assert_eq!(all[0].metadata.notes.as_deref(), Some("Test override"));
        }
    }
}
