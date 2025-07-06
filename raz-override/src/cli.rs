use crate::{OverrideInspector, OverrideMigrator, OverrideSystem, Result};
use std::path::Path;

/// CLI commands for override debugging and inspection
pub struct OverrideCli {
    workspace_path: std::path::PathBuf,
}

impl OverrideCli {
    /// Create a new CLI handler for the given workspace
    pub fn new(workspace_path: &Path) -> Self {
        Self {
            workspace_path: workspace_path.to_path_buf(),
        }
    }

    /// List all overrides
    pub fn list_all(&self) -> Result<()> {
        let inspector = OverrideInspector::new(&self.workspace_path)?;
        inspector.list_all_detailed()
    }

    /// List overrides for a specific file
    pub fn list_by_file(&self, file_path: &Path) -> Result<()> {
        let inspector = OverrideInspector::new(&self.workspace_path)?;
        inspector.list_by_file(file_path)
    }

    /// Inspect a specific override by key
    pub fn inspect(&self, key: &str) -> Result<()> {
        let inspector = OverrideInspector::new(&self.workspace_path)?;
        inspector.inspect_override(key)
    }

    /// Debug resolution for a specific location
    pub fn debug_resolution(
        &self,
        file_path: &Path,
        line: usize,
        column: Option<usize>,
    ) -> Result<()> {
        let mut inspector = OverrideInspector::new(&self.workspace_path)?;
        inspector.debug_resolution(file_path, line, column)
    }

    /// Show override statistics
    pub fn stats(&self) -> Result<()> {
        let inspector = OverrideInspector::new(&self.workspace_path)?;
        inspector.show_statistics()
    }

    /// Export overrides
    pub fn export(&self, output_path: Option<&Path>) -> Result<()> {
        let inspector = OverrideInspector::new(&self.workspace_path)?;
        inspector.export_overrides(output_path)
    }

    /// Delete an override by key
    pub fn delete(&self, key: &str) -> Result<()> {
        let system = OverrideSystem::new(&self.workspace_path)?;
        if system.delete_override(key)? {
            println!("Override '{key}' deleted successfully.");
        } else {
            println!("Override '{key}' not found.");
        }
        Ok(())
    }

    /// Clear all overrides (with confirmation)
    pub fn clear_all(&self, force: bool) -> Result<()> {
        let system = OverrideSystem::new(&self.workspace_path)?;

        if !force {
            println!("This will delete ALL overrides. Are you sure? (y/N)");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            if !input.trim().eq_ignore_ascii_case("y") {
                println!("Operation cancelled.");
                return Ok(());
            }
        }

        system.clear_all()?;
        println!("All overrides cleared.");
        Ok(())
    }

    /// Clear overrides for a specific file (with confirmation)
    pub fn clear_by_file(&self, file_path: &Path, force: bool) -> Result<()> {
        let system = OverrideSystem::new(&self.workspace_path)?;

        // First check how many overrides exist for this file
        let overrides = system.list_by_file(file_path)?;
        if overrides.is_empty() {
            println!("No overrides found for file: {}", file_path.display());
            return Ok(());
        }

        if !force {
            println!(
                "This will delete {} override(s) for file: {}. Are you sure? (y/N)",
                overrides.len(),
                file_path.display()
            );
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            if !input.trim().eq_ignore_ascii_case("y") {
                println!("Operation cancelled.");
                return Ok(());
            }
        }

        let count = system.clear_by_file(file_path)?;
        println!(
            "Cleared {} override(s) for file: {}",
            count,
            file_path.display()
        );
        Ok(())
    }

    /// Import overrides from a file
    pub fn import(&self, file_path: &Path) -> Result<()> {
        let system = OverrideSystem::new(&self.workspace_path)?;
        let data = std::fs::read_to_string(file_path)?;
        let count = system.import(&data)?;
        println!("Imported {} overrides from {}", count, file_path.display());
        Ok(())
    }

    /// Migrate legacy overrides
    pub fn migrate(&self, legacy_file: Option<&Path>, dry_run: bool, auto: bool) -> Result<()> {
        let mut migrator = OverrideMigrator::new(&self.workspace_path)?;

        if auto {
            match migrator.auto_migrate(&self.workspace_path)? {
                Some(report) => {
                    println!("Migration completed!");
                    report.print_summary();
                }
                None => {
                    println!("No legacy overrides found or already migrated.");
                }
            }
        } else if let Some(file) = legacy_file {
            let report = if dry_run {
                println!("Dry run mode - no changes will be made");
                migrator.dry_run(file)?
            } else {
                migrator.migrate_from_file(file)?
            };
            report.print_summary();
        } else {
            println!("Please specify --file or use --auto");
        }

        Ok(())
    }
}

/// Helper to parse debug command arguments
pub fn parse_debug_args(args: &[String]) -> Result<(std::path::PathBuf, usize, Option<usize>)> {
    if args.is_empty() {
        return Err(crate::OverrideError::InvalidKey(
            "Usage: raz override debug <file> <line> [column]".to_string(),
        ));
    }

    let file_path = std::path::PathBuf::from(&args[0]);

    let line = args
        .get(1)
        .ok_or_else(|| crate::OverrideError::InvalidKey("Line number required".to_string()))?
        .parse::<usize>()
        .map_err(|_| crate::OverrideError::InvalidKey("Invalid line number".to_string()))?;

    let column = args.get(2).and_then(|s| s.parse::<usize>().ok());

    Ok((file_path, line, column))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_cli_list_all() {
        let temp_dir = TempDir::new().unwrap();
        let cli = OverrideCli::new(temp_dir.path());

        // Should not panic with empty overrides
        cli.list_all().unwrap();
    }

    #[test]
    fn test_parse_debug_args() {
        let args = vec![
            "src/main.rs".to_string(),
            "42".to_string(),
            "10".to_string(),
        ];

        let (file, line, column) = parse_debug_args(&args).unwrap();
        assert_eq!(file, std::path::PathBuf::from("src/main.rs"));
        assert_eq!(line, 42);
        assert_eq!(column, Some(10));

        // Without column
        let args = vec!["src/lib.rs".to_string(), "100".to_string()];

        let (file, line, column) = parse_debug_args(&args).unwrap();
        assert_eq!(file, std::path::PathBuf::from("src/lib.rs"));
        assert_eq!(line, 100);
        assert_eq!(column, None);
    }
}
