use crate::{OverrideEntry, OverrideError, OverrideSystem, ResolutionStrategy, Result};
use colored::*;
use std::path::Path;

/// Inspector for debugging and examining overrides
pub struct OverrideInspector {
    system: OverrideSystem,
}

impl OverrideInspector {
    /// Create a new inspector for a workspace
    pub fn new(workspace_path: &Path) -> Result<Self> {
        let system = OverrideSystem::new(workspace_path)?;
        Ok(Self { system })
    }

    /// List all overrides with detailed information
    pub fn list_all_detailed(&self) -> Result<()> {
        let overrides = self.system.list_all()?;

        if overrides.is_empty() {
            println!("{}", "No overrides found.".yellow());
            return Ok(());
        }

        println!("{}", format!("Found {} overrides:", overrides.len()).bold());
        println!();

        for (idx, entry) in overrides.iter().enumerate() {
            self.print_override_entry(idx + 1, entry);
        }

        Ok(())
    }

    /// List overrides for a specific file
    pub fn list_by_file(&self, file_path: &Path) -> Result<()> {
        let overrides = self.system.list_by_file(file_path)?;

        if overrides.is_empty() {
            println!(
                "{}",
                format!("No overrides found for file: {}", file_path.display()).yellow()
            );
            return Ok(());
        }

        println!(
            "{}",
            format!(
                "Found {} overrides for {}:",
                overrides.len(),
                file_path.display()
            )
            .bold()
        );
        println!();

        for (idx, entry) in overrides.iter().enumerate() {
            self.print_override_entry(idx + 1, entry);
        }

        Ok(())
    }

    /// Show detailed information about a specific override
    pub fn inspect_override(&self, key: &str) -> Result<()> {
        let overrides = self.system.list_all()?;

        let entry = overrides
            .iter()
            .find(|e| e.key.primary == key || e.key.fallbacks.contains(&key.to_string()))
            .ok_or_else(|| OverrideError::OverrideNotFound(key.to_string()))?;

        println!("{}", "Override Details:".bold().blue());
        println!();

        // Key information
        println!("{}", "Keys:".bold());
        println!("  Primary: {}", entry.key.primary.green());
        if !entry.key.fallbacks.is_empty() {
            println!("  Fallbacks:");
            for fallback in &entry.key.fallbacks {
                println!("    - {}", fallback.cyan());
            }
        }
        println!();

        // Metadata
        println!("{}", "Metadata:".bold());
        println!(
            "  File: {}",
            entry.metadata.file_path.display().to_string().blue()
        );
        if let Some(func_name) = &entry.metadata.function_name {
            println!("  Function: {}", func_name.yellow());
        }
        if let Some(line) = entry.metadata.original_line {
            println!("  Original Line: {}", line.to_string().magenta());
        }
        println!(
            "  Created: {}",
            entry.metadata.created_at.format("%Y-%m-%d %H:%M:%S UTC")
        );
        println!(
            "  Modified: {}",
            entry.metadata.modified_at.format("%Y-%m-%d %H:%M:%S UTC")
        );
        if let Some(notes) = &entry.metadata.notes {
            println!("  Notes: {notes}");
        }
        println!();

        // Override configuration
        println!("{}", "Override Configuration:".bold());
        println!("  Key: {}", entry.override_config.key.green());

        if !entry.override_config.env.is_empty() {
            println!("  Environment Variables:");
            for (key, value) in &entry.override_config.env {
                println!("    {} = {}", key.cyan(), value);
            }
        }

        if !entry.override_config.cargo_options.is_empty() {
            println!("  Cargo Options:");
            for opt in &entry.override_config.cargo_options {
                println!("    {opt}");
            }
        }

        if !entry.override_config.rustc_options.is_empty() {
            println!("  Rustc Options:");
            for opt in &entry.override_config.rustc_options {
                println!("    {opt}");
            }
        }

        if !entry.override_config.args.is_empty() {
            println!("  Additional Arguments:");
            for arg in &entry.override_config.args {
                println!("    {arg}");
            }
        }

        Ok(())
    }

    /// Debug resolution process for a given context
    pub fn debug_resolution(
        &mut self,
        file_path: &Path,
        line: usize,
        column: Option<usize>,
    ) -> Result<()> {
        let context = self.system.get_function_context(file_path, line, column)?;

        println!("{}", "Resolution Debug Information:".bold().blue());
        println!();

        // Context information
        println!("{}", "Context:".bold());
        println!("  File: {}", context.file_path.display().to_string().blue());
        println!("  Line: {}", line.to_string().magenta());
        if let Some(col) = column {
            println!("  Column: {}", col.to_string().magenta());
        }
        if let Some(func_name) = &context.function_name {
            println!("  Detected Function: {}", func_name.yellow());
        } else {
            println!("  Detected Function: {}", "None".red());
        }
        println!();

        // Resolution candidates
        let candidates = self.system.debug_resolution(&context)?;

        if candidates.is_empty() {
            println!("{}", "No resolution candidates found.".yellow());
            return Ok(());
        }

        println!(
            "{}",
            format!("Found {} resolution candidates:", candidates.len()).bold()
        );
        println!();

        for (idx, (entry, strategy, score)) in candidates.iter().enumerate() {
            println!("{}. {} (score: {:.2})", idx + 1, entry.key.primary, score);
            println!("   Strategy: {}", self.format_strategy(strategy));
            println!(
                "   Function: {}",
                entry.metadata.function_name.as_deref().unwrap_or("N/A")
            );
            if let Some(line) = entry.metadata.original_line {
                println!("   Original Line: {line}");
            }
            println!();
        }

        // Final resolution result
        if let Some((override_config, strategy)) = self.system.resolve_with_strategy(&context)? {
            println!("{}", "Final Resolution:".bold().green());
            println!("  Override Key: {}", override_config.key);
            println!("  Strategy: {}", self.format_strategy(&strategy));
        } else {
            println!("{}", "No override resolved.".red());
        }

        Ok(())
    }

    /// Export overrides for inspection
    pub fn export_overrides(&self, output_path: Option<&Path>) -> Result<()> {
        let export_data = self.system.export()?;

        if let Some(path) = output_path {
            std::fs::write(path, &export_data)?;
            println!(
                "{}",
                format!("Exported overrides to: {}", path.display()).green()
            );
        } else {
            println!("{}", "Exported Override Data:".bold());
            println!("{export_data}");
        }

        Ok(())
    }

    /// Show statistics about overrides
    pub fn show_statistics(&self) -> Result<()> {
        let overrides = self.system.list_all()?;

        println!("{}", "Override Statistics:".bold().blue());
        println!();

        println!("Total Overrides: {}", overrides.len());

        // Count by file
        let mut file_counts = std::collections::HashMap::new();
        for entry in &overrides {
            *file_counts
                .entry(entry.metadata.file_path.display().to_string())
                .or_insert(0) += 1;
        }

        println!();
        println!("{}", "Overrides by File:".bold());
        for (file, count) in file_counts.iter() {
            println!("  {}: {}", file.blue(), count);
        }

        // Count by function
        let function_count = overrides
            .iter()
            .filter(|e| e.metadata.function_name.is_some())
            .count();
        let file_level_count = overrides.len() - function_count;

        println!();
        println!("{}", "Override Types:".bold());
        println!("  Function-level: {function_count}");
        println!("  File-level: {file_level_count}");

        // Environment variables usage
        let mut env_vars = std::collections::HashSet::new();
        for entry in &overrides {
            for key in entry.override_config.env.keys() {
                env_vars.insert(key.clone());
            }
        }

        if !env_vars.is_empty() {
            println!();
            println!("{}", "Environment Variables Used:".bold());
            for var in env_vars {
                println!("  - {}", var.cyan());
            }
        }

        Ok(())
    }

    // Helper functions

    fn print_override_entry(&self, idx: usize, entry: &OverrideEntry) {
        println!("{}. {}", idx, entry.key.primary.green());
        println!(
            "   File: {}",
            entry.metadata.file_path.display().to_string().blue()
        );
        if let Some(func_name) = &entry.metadata.function_name {
            println!("   Function: {}", func_name.yellow());
        }
        if let Some(line) = entry.metadata.original_line {
            println!("   Line: {}", line.to_string().magenta());
        }
        println!("   Override: {}", entry.override_config.key);
        println!();
    }

    fn format_strategy(&self, strategy: &ResolutionStrategy) -> colored::ColoredString {
        match strategy {
            ResolutionStrategy::Exact => "Exact Key Match".green(),
            ResolutionStrategy::Fallback => "Fallback Key".cyan(),
            ResolutionStrategy::FuzzyFunction => "Fuzzy Function Match".yellow(),
            ResolutionStrategy::LineProximity => "Line Proximity".magenta(),
            ResolutionStrategy::HashFallback => "Hash Fallback".red(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CommandOverride, FunctionContext};
    use tempfile::TempDir;

    #[test]
    fn test_inspector_list_all() {
        let temp_dir = TempDir::new().unwrap();
        let mut system = OverrideSystem::new(temp_dir.path()).unwrap();

        // Add some test overrides
        let context1 = FunctionContext {
            file_path: "src/main.rs".into(),
            function_name: Some("main".to_string()),
            line_number: 10,
            context: None,
        };

        let key1 = system.generate_key(&context1).unwrap();
        let mut override1 = CommandOverride::new("test1".to_string());
        override1.env.insert("DEBUG".to_string(), "1".to_string());

        system.save_override(key1, override1, &context1).unwrap();

        // Create inspector and test
        let inspector = OverrideInspector::new(temp_dir.path()).unwrap();

        // Should not panic
        inspector.list_all_detailed().unwrap();
    }

    #[test]
    fn test_inspector_debug_resolution() {
        let temp_dir = TempDir::new().unwrap();
        let mut system = OverrideSystem::new(temp_dir.path()).unwrap();

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

        // Add override for process_data
        let context = FunctionContext {
            file_path: test_file.clone(),
            function_name: Some("process_data".to_string()),
            line_number: 2,
            context: None,
        };

        let key = system.generate_key(&context).unwrap();
        let override_config = CommandOverride::new("debug-test".to_string());
        system
            .save_override(key, override_config, &context)
            .unwrap();

        // Create inspector and debug resolution
        let mut inspector = OverrideInspector::new(temp_dir.path()).unwrap();

        // Should not panic
        inspector.debug_resolution(&test_file, 2, None).unwrap();
    }
}
