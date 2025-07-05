use crate::error::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Information about a backup file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    pub path: PathBuf,
    pub created_at: DateTime<Utc>,
    pub size: u64,
    pub override_count: usize,
}

/// Manages backup operations for override storage
pub struct BackupManager {
    backup_dir: PathBuf,
    max_backups: usize,
}

impl BackupManager {
    /// Create a new backup manager
    pub fn new(workspace_path: &Path, max_backups: usize) -> Result<Self> {
        let backup_dir = workspace_path.join(".raz").join("overrides.backup");

        // Ensure backup directory exists
        fs::create_dir_all(&backup_dir)?;

        Ok(Self {
            backup_dir,
            max_backups,
        })
    }

    /// Create a backup of the current state
    pub fn create_backup<T: Serialize>(&self, current_state: &T) -> Result<PathBuf> {
        // Generate timestamp-based filename
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S_%3f");
        let backup_filename = format!("overrides_{timestamp}.toml");
        let backup_path = self.backup_dir.join(&backup_filename);

        // Serialize and save
        let content = toml::to_string_pretty(current_state)?;
        fs::write(&backup_path, content)?;

        // Clean up old backups
        self.cleanup_old_backups()?;

        Ok(backup_path)
    }

    /// Restore from a backup file
    pub fn restore_backup<T: for<'de> Deserialize<'de>>(&self, backup_path: &Path) -> Result<T> {
        let content = fs::read_to_string(backup_path)?;
        let restored = toml::from_str(&content)?;
        Ok(restored)
    }

    /// Get the most recent backup
    pub fn get_last_backup(&self) -> Result<Option<BackupInfo>> {
        let backups = self.list_backups()?;
        Ok(backups.into_iter().next())
    }

    /// List all backups, sorted by creation time (newest first)
    pub fn list_backups(&self) -> Result<Vec<BackupInfo>> {
        let mut backups = Vec::new();

        // Read all backup files
        for entry in fs::read_dir(&self.backup_dir)? {
            let entry = entry?;
            let path = entry.path();

            // Skip non-toml files
            if path.extension().and_then(|s| s.to_str()) != Some("toml") {
                continue;
            }

            // Get file metadata
            let metadata = entry.metadata()?;
            let created_at = metadata
                .modified()
                .map(DateTime::<Utc>::from)
                .unwrap_or_else(|_| Utc::now());

            // Try to read override count
            let override_count = if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(data) = toml::from_str::<toml::Value>(&content) {
                    data.get("overrides")
                        .and_then(|v| v.as_table())
                        .map(|t| t.len())
                        .unwrap_or(0)
                } else {
                    0
                }
            } else {
                0
            };

            backups.push(BackupInfo {
                path,
                created_at,
                size: metadata.len(),
                override_count,
            });
        }

        // Sort by creation time, newest first
        backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(backups)
    }

    /// Clean up old backups, keeping only the most recent N
    pub fn cleanup_old_backups(&self) -> Result<()> {
        let backups = self.list_backups()?;

        // Skip if we're under the limit
        if backups.len() <= self.max_backups {
            return Ok(());
        }

        // Remove old backups
        for backup in backups.into_iter().skip(self.max_backups) {
            if let Err(e) = fs::remove_file(&backup.path) {
                log::warn!(
                    "Failed to remove old backup {}: {}",
                    backup.path.display(),
                    e
                );
            }
        }

        Ok(())
    }

    /// Find a known-good backup (one that was marked as validated)
    pub fn find_last_good_backup(&self) -> Result<Option<BackupInfo>> {
        // For now, just return the most recent backup
        // In the future, we could store validation metadata with backups
        self.get_last_backup()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;
    use tempfile::TempDir;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestData {
        version: u32,
        data: String,
    }

    #[test]
    fn test_backup_create_and_restore() {
        let temp_dir = TempDir::new().unwrap();
        let manager = BackupManager::new(temp_dir.path(), 5).unwrap();

        let test_data = TestData {
            version: 1,
            data: "test content".to_string(),
        };

        // Create backup
        let backup_path = manager.create_backup(&test_data).unwrap();
        assert!(backup_path.exists());

        // Restore backup
        let restored: TestData = manager.restore_backup(&backup_path).unwrap();
        assert_eq!(restored, test_data);
    }

    #[test]
    fn test_backup_cleanup() {
        let temp_dir = TempDir::new().unwrap();
        let manager = BackupManager::new(temp_dir.path(), 3).unwrap();

        // Create multiple backups
        for i in 0..5 {
            let test_data = TestData {
                version: i,
                data: format!("backup {i}"),
            };
            manager.create_backup(&test_data).unwrap();
            // Small delay to ensure different timestamps
            thread::sleep(Duration::from_millis(10));
        }

        // Check that only 3 backups remain
        let backups = manager.list_backups().unwrap();
        assert_eq!(backups.len(), 3);

        // Verify newest backups are kept
        for (i, backup) in backups.iter().enumerate() {
            let restored: TestData = manager.restore_backup(&backup.path).unwrap();
            assert_eq!(restored.version, 4 - i as u32);
        }
    }

    #[test]
    fn test_list_backups_ordering() {
        let temp_dir = TempDir::new().unwrap();
        let manager = BackupManager::new(temp_dir.path(), 10).unwrap();

        // Create backups with delays
        for i in 0..3 {
            let test_data = TestData {
                version: i,
                data: format!("backup {i}"),
            };
            manager.create_backup(&test_data).unwrap();
            thread::sleep(Duration::from_millis(50));
        }

        // List should be newest first
        let backups = manager.list_backups().unwrap();
        assert_eq!(backups.len(), 3);

        // Verify ordering
        for i in 0..2 {
            assert!(backups[i].created_at > backups[i + 1].created_at);
        }
    }
}
