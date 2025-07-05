use crate::{CommandOverride, FunctionContext, OverrideError, OverrideSystem, Result};
use colored::*;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Legacy override format (cursor position-based)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LegacyOverride {
    #[serde(flatten)]
    config: CommandOverride,
}

/// Legacy storage format
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LegacyStorageFormat {
    overrides: IndexMap<String, LegacyOverride>,
}

/// Migration report entry
#[derive(Debug, Clone)]
pub struct MigrationEntry {
    pub old_key: String,
    pub new_key: String,
    pub status: MigrationStatus,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MigrationStatus {
    Success,
    Warning,
    Failed,
}

/// Migration report
#[derive(Debug)]
pub struct MigrationReport {
    pub entries: Vec<MigrationEntry>,
    pub total: usize,
    pub successful: usize,
    pub warnings: usize,
    pub failed: usize,
}

impl MigrationReport {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
            total: 0,
            successful: 0,
            warnings: 0,
            failed: 0,
        }
    }

    fn add_entry(&mut self, entry: MigrationEntry) {
        match &entry.status {
            MigrationStatus::Success => self.successful += 1,
            MigrationStatus::Warning => self.warnings += 1,
            MigrationStatus::Failed => self.failed += 1,
        }
        self.entries.push(entry);
        self.total += 1;
    }

    pub fn print_summary(&self) {
        println!("{}", "Migration Report:".bold().blue());
        println!();
        println!("Total overrides: {}", self.total);
        println!("Successful: {}", self.successful.to_string().green());
        if self.warnings > 0 {
            println!("Warnings: {}", self.warnings.to_string().yellow());
        }
        if self.failed > 0 {
            println!("Failed: {}", self.failed.to_string().red());
        }
        println!();

        if !self.entries.is_empty() {
            println!("{}", "Details:".bold());
            for (idx, entry) in self.entries.iter().enumerate() {
                let status_symbol = match entry.status {
                    MigrationStatus::Success => "✓".green(),
                    MigrationStatus::Warning => "⚠".yellow(),
                    MigrationStatus::Failed => "✗".red(),
                };

                println!(
                    "{}. {} {} → {}",
                    idx + 1,
                    status_symbol,
                    entry.old_key.dimmed(),
                    entry.new_key
                );
                if !entry.message.is_empty() {
                    println!("   {}", entry.message.dimmed());
                }
            }
        }
    }
}

/// Override migrator for converting legacy overrides to new format
pub struct OverrideMigrator {
    system: OverrideSystem,
}

impl OverrideMigrator {
    /// Create a new migrator
    pub fn new(workspace_path: &Path) -> Result<Self> {
        let system = OverrideSystem::new(workspace_path)?;
        Ok(Self { system })
    }

    /// Parse legacy key format: "file:line:column" or "file:line"
    fn parse_legacy_key(key: &str) -> Result<(PathBuf, usize, Option<usize>)> {
        let parts: Vec<&str> = key.rsplitn(3, ':').collect();

        match parts.len() {
            3 => {
                // file:line:column
                let column = parts[0]
                    .parse::<usize>()
                    .map_err(|_| OverrideError::InvalidKey("Invalid column number".to_string()))?;
                let line = parts[1]
                    .parse::<usize>()
                    .map_err(|_| OverrideError::InvalidKey("Invalid line number".to_string()))?;
                let file_path = PathBuf::from(parts[2]);
                Ok((file_path, line, Some(column)))
            }
            2 => {
                // file:line or Windows path like C:\file
                if let Ok(line) = parts[0].parse::<usize>() {
                    let file_path = PathBuf::from(parts[1]);
                    Ok((file_path, line, None))
                } else {
                    Err(OverrideError::InvalidKey(format!(
                        "Invalid legacy key format: {key}"
                    )))
                }
            }
            _ => Err(OverrideError::InvalidKey(format!(
                "Invalid legacy key format: {key}"
            ))),
        }
    }

    /// Migrate a single legacy override
    fn migrate_single(
        &mut self,
        legacy_key: &str,
        legacy_override: &LegacyOverride,
    ) -> MigrationEntry {
        // Parse the legacy key
        let (file_path, line, column) = match Self::parse_legacy_key(legacy_key) {
            Ok(parsed) => parsed,
            Err(e) => {
                return MigrationEntry {
                    old_key: legacy_key.to_string(),
                    new_key: String::new(),
                    status: MigrationStatus::Failed,
                    message: format!("Failed to parse legacy key: {e}"),
                };
            }
        };

        // Try to get function context
        let context = match self.system.get_function_context(&file_path, line, column) {
            Ok(ctx) => ctx,
            Err(_) => {
                // Fallback to line-based key
                FunctionContext {
                    file_path: file_path.clone(),
                    function_name: None,
                    line_number: line,
                    context: column.map(|c| format!("column:{c}")),
                }
            }
        };

        // Generate new key
        let new_key = match self.system.generate_key(&context) {
            Ok(key) => key,
            Err(e) => {
                return MigrationEntry {
                    old_key: legacy_key.to_string(),
                    new_key: String::new(),
                    status: MigrationStatus::Failed,
                    message: format!("Failed to generate new key: {e}"),
                };
            }
        };

        // Check if function was detected successfully
        let had_function_detection_error = self
            .system
            .get_function_context(&file_path, line, column)
            .is_err();

        // Save the override with new key
        match self
            .system
            .save_override(new_key.clone(), legacy_override.config.clone(), &context)
        {
            Ok(_) => {
                let status = if had_function_detection_error {
                    MigrationStatus::Warning
                } else {
                    MigrationStatus::Success
                };

                let message = if context.function_name.is_some() {
                    "Migrated to function-based key".to_string()
                } else if had_function_detection_error {
                    "Could not detect function, migrated to line-based key".to_string()
                } else {
                    "Migrated to line-based key".to_string()
                };

                MigrationEntry {
                    old_key: legacy_key.to_string(),
                    new_key: new_key.primary,
                    status,
                    message,
                }
            }
            Err(e) => MigrationEntry {
                old_key: legacy_key.to_string(),
                new_key: new_key.primary,
                status: MigrationStatus::Failed,
                message: format!("Failed to save override: {e}"),
            },
        }
    }

    /// Migrate overrides from a legacy file
    pub fn migrate_from_file(&mut self, legacy_file: &Path) -> Result<MigrationReport> {
        if !legacy_file.exists() {
            return Err(OverrideError::MigrationError(format!(
                "Legacy file not found: {}",
                legacy_file.display()
            )));
        }

        let content = std::fs::read_to_string(legacy_file)?;
        self.migrate_from_string(&content)
    }

    /// Migrate overrides from a legacy string
    pub fn migrate_from_string(&mut self, content: &str) -> Result<MigrationReport> {
        let legacy_format: LegacyStorageFormat = toml::from_str(content).map_err(|e| {
            OverrideError::MigrationError(format!("Failed to parse legacy format: {e}"))
        })?;

        let mut report = MigrationReport::new();

        for (legacy_key, legacy_override) in &legacy_format.overrides {
            let entry = self.migrate_single(legacy_key, legacy_override);
            report.add_entry(entry);
        }

        Ok(report)
    }

    /// Auto-detect and migrate legacy overrides
    pub fn auto_migrate(&mut self, workspace_path: &Path) -> Result<Option<MigrationReport>> {
        let legacy_file = workspace_path.join(".raz").join("overrides.toml");

        if !legacy_file.exists() {
            return Ok(None);
        }

        // Check if it's already in new format
        let content = std::fs::read_to_string(&legacy_file)?;
        if content.contains("version") && content.contains("[[overrides]]") {
            // Already in new format
            return Ok(None);
        }

        // Backup the old file
        let backup_file = workspace_path.join(".raz").join("overrides.toml.backup");
        std::fs::copy(&legacy_file, &backup_file)?;

        println!(
            "{}",
            format!(
                "Found legacy overrides. Backing up to: {}",
                backup_file.display()
            )
            .yellow()
        );

        // Perform migration
        let report = self.migrate_from_file(&legacy_file)?;

        Ok(Some(report))
    }

    /// Dry run migration to preview changes
    pub fn dry_run(&mut self, legacy_file: &Path) -> Result<MigrationReport> {
        if !legacy_file.exists() {
            return Err(OverrideError::MigrationError(format!(
                "Legacy file not found: {}",
                legacy_file.display()
            )));
        }

        let content = std::fs::read_to_string(legacy_file)?;
        let legacy_format: LegacyStorageFormat = toml::from_str(&content).map_err(|e| {
            OverrideError::MigrationError(format!("Failed to parse legacy format: {e}"))
        })?;

        let mut report = MigrationReport::new();

        for (legacy_key, _) in &legacy_format.overrides {
            let (file_path, line, column) = match Self::parse_legacy_key(legacy_key) {
                Ok(parsed) => parsed,
                Err(e) => {
                    report.add_entry(MigrationEntry {
                        old_key: legacy_key.to_string(),
                        new_key: String::new(),
                        status: MigrationStatus::Failed,
                        message: format!("Failed to parse legacy key: {e}"),
                    });
                    continue;
                }
            };

            // Try to get function context
            let (new_key, status, message) =
                match self.system.get_function_context(&file_path, line, column) {
                    Ok(ctx) => match self.system.generate_key(&ctx) {
                        Ok(key) => {
                            let msg = if ctx.function_name.is_some() {
                                "Will migrate to function-based key".to_string()
                            } else {
                                "Will migrate to line-based key".to_string()
                            };
                            (key.primary, MigrationStatus::Success, msg)
                        }
                        Err(e) => (
                            String::new(),
                            MigrationStatus::Failed,
                            format!("Failed to generate key: {e}"),
                        ),
                    },
                    Err(e) => {
                        // Try fallback
                        let fallback_context = FunctionContext {
                            file_path: file_path.clone(),
                            function_name: None,
                            line_number: line,
                            context: column.map(|c| format!("column:{c}")),
                        };

                        match self.system.generate_key(&fallback_context) {
                            Ok(key) => (
                                key.primary,
                                MigrationStatus::Warning,
                                format!("Could not detect function ({e}), will use line-based key"),
                            ),
                            Err(e) => (
                                String::new(),
                                MigrationStatus::Failed,
                                format!("Failed to generate key: {e}"),
                            ),
                        }
                    }
                };

            report.add_entry(MigrationEntry {
                old_key: legacy_key.to_string(),
                new_key,
                status,
                message,
            });
        }

        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_parse_legacy_key() {
        // Test file:line:column format
        let (file, line, column) = OverrideMigrator::parse_legacy_key("src/main.rs:42:10").unwrap();
        assert_eq!(file, PathBuf::from("src/main.rs"));
        assert_eq!(line, 42);
        assert_eq!(column, Some(10));

        // Test file:line format
        let (file, line, column) = OverrideMigrator::parse_legacy_key("src/lib.rs:100").unwrap();
        assert_eq!(file, PathBuf::from("src/lib.rs"));
        assert_eq!(line, 100);
        assert_eq!(column, None);
    }

    #[test]
    fn test_migration_basic() {
        let temp_dir = TempDir::new().unwrap();
        let mut migrator = OverrideMigrator::new(temp_dir.path()).unwrap();

        // Create a test file
        let test_file = temp_dir.path().join("test.rs");
        std::fs::write(
            &test_file,
            r#"
fn process_data(input: &str) -> String {
    input.to_uppercase()
}

fn main() {
    let result = process_data("hello");
    println!("{}", result);
}
"#,
        )
        .unwrap();

        // Create legacy format
        let legacy_content = r#"
[overrides."test.rs:2:5"]
key = "test-override"
mode = "replace"
cargo_options = []
rustc_options = []
args = []
created_at = "2024-01-01T00:00:00Z"

[overrides."test.rs:2:5".env]
RUST_LOG = "debug"
"#;

        let report = migrator.migrate_from_string(legacy_content).unwrap();
        assert_eq!(report.total, 1);

        // Should have 1 warning (file doesn't exist so function detection fails)
        assert_eq!(report.warnings, 1);
        assert_eq!(report.failed, 0);

        // Verify the new key - should be line-based since function detection failed
        let entry = &report.entries[0];
        assert_eq!(entry.old_key, "test.rs:2:5");
        assert!(entry.new_key.contains("test.rs:L2"));
    }
}
