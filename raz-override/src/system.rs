use crate::{
    CommandOverride, OverrideEntry, OverrideMetadata, ValidationStatus,
    detector::FunctionDetector,
    error::{OverrideError, Result},
    key::{FunctionContext, OverrideKey},
    preview::OverridePreview,
    resolver::{OverrideResolver, ResolutionStrategy},
    storage::OverrideStorage,
};
use std::path::{Path, PathBuf};

/// Parameters for creating an override preview
pub struct PreviewParams<'a> {
    pub override_config: &'a CommandOverride,
    pub context: &'a FunctionContext,
    pub base_command: &'a str,
    pub base_env: &'a indexmap::IndexMap<String, String>,
    pub base_cargo_options: &'a [String],
    pub base_rustc_options: &'a [String],
    pub base_args: &'a [String],
}

/// Main override system that coordinates all components
pub struct OverrideSystem {
    workspace_path: PathBuf,
    pub(crate) storage: OverrideStorage,
    detector: FunctionDetector,
}

impl OverrideSystem {
    /// Create a new override system for testing (bypasses hierarchy)
    #[cfg(test)]
    pub fn new_for_test(workspace_path: &Path) -> Result<Self> {
        let storage = OverrideStorage::new_for_test(workspace_path)?;
        let detector = FunctionDetector::new()?;

        Ok(Self {
            workspace_path: workspace_path.to_path_buf(),
            storage,
            detector,
        })
    }

    /// Create a new override system for a workspace
    pub fn new(workspace_path: &Path) -> Result<Self> {
        let storage = OverrideStorage::new(workspace_path)?;
        let detector = FunctionDetector::new()?;

        Ok(Self {
            workspace_path: workspace_path.to_path_buf(),
            storage,
            detector,
        })
    }

    /// Generate a stable key from context
    pub fn generate_key(&mut self, context: &FunctionContext) -> Result<OverrideKey> {
        // Try to enhance context with detected function name if not provided
        let enhanced_context = if context.function_name.is_none() {
            if let Ok(source) = std::fs::read_to_string(&context.file_path) {
                if let Some(func_info) = self
                    .detector
                    .find_function_at_line(&source, context.line_number)?
                {
                    FunctionContext {
                        function_name: Some(func_info.name),
                        ..context.clone()
                    }
                } else {
                    context.clone()
                }
            } else {
                context.clone()
            }
        } else {
            context.clone()
        };

        OverrideKey::new(&enhanced_context)
    }

    /// Save an override with the given key
    pub fn save_override(
        &self,
        key: OverrideKey,
        override_config: CommandOverride,
        context: &FunctionContext,
    ) -> Result<()> {
        let entry = OverrideEntry {
            key,
            override_config,
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

        self.storage.save(&entry)
    }

    /// Save an override with validation
    pub fn save_override_with_validation(
        &self,
        key: OverrideKey,
        override_config: CommandOverride,
        context: &FunctionContext,
        command: &str,
    ) -> Result<()> {
        // Create entry
        let entry = OverrideEntry {
            key,
            override_config: override_config.clone(),
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

        // Validate before saving
        self.storage.save_with_validation(&entry, |entry| {
            use crate::parser::OverrideParser;

            // Create parser with validation
            let parser = OverrideParser::new(command);

            // Convert CommandOverride to ParsedOverride for validation
            let mut parsed = crate::parser::ParsedOverride::new();

            // Convert env vars
            for (k, v) in &entry.override_config.env {
                parsed.env.insert(k.clone(), v.clone());
            }

            // Convert cargo options
            for opt in &entry.override_config.cargo_options {
                let parts: Vec<&str> = opt.splitn(2, ' ').collect();
                if parts.len() == 2 {
                    parsed.options.insert(
                        parts[0].to_string(),
                        crate::parser::OptionValue::Single(parts[1].to_string()),
                    );
                } else {
                    parsed
                        .options
                        .insert(opt.clone(), crate::parser::OptionValue::Flag(true));
                }
            }

            // Add args
            parsed.extra_args = entry.override_config.args.clone();

            // Validate
            parser.validate(&parsed)
        })
    }

    /// Resolve an override from context
    pub fn resolve_override(
        &mut self,
        context: &FunctionContext,
    ) -> Result<Option<CommandOverride>> {
        let mut resolver = OverrideResolver::new(OverrideStorage::new(&self.workspace_path)?)?;
        match resolver.resolve(context)? {
            Some((entry, _strategy)) => Ok(Some(entry.override_config)),
            None => Ok(None),
        }
    }

    /// Resolve an override using the existing storage (for tests)
    #[cfg(test)]
    pub fn resolve_override_with_storage(
        &mut self,
        context: &FunctionContext,
    ) -> Result<Option<CommandOverride>> {
        // For tests, directly check the storage instead of creating a new resolver
        let entries = self.storage.list_all()?;
        for entry in entries {
            if entry.metadata.file_path == context.file_path {
                if let Some(ref function_name) = context.function_name {
                    if entry.metadata.function_name.as_ref() == Some(function_name) {
                        return Ok(Some(entry.override_config));
                    }
                } else if entry.metadata.original_line == Some(context.line_number) {
                    return Ok(Some(entry.override_config));
                }
            }
        }
        Ok(None)
    }

    /// Resolve override with strategy information
    pub fn resolve_with_strategy(
        &mut self,
        context: &FunctionContext,
    ) -> Result<Option<(CommandOverride, ResolutionStrategy)>> {
        let mut resolver = OverrideResolver::new(OverrideStorage::new(&self.workspace_path)?)?;
        match resolver.resolve(context)? {
            Some((entry, strategy)) => Ok(Some((entry.override_config, strategy))),
            None => Ok(None),
        }
    }

    /// Create a preview of changes
    pub fn preview_override(&self, params: PreviewParams) -> Result<OverridePreview> {
        let key = OverrideKey::new(params.context)?;

        let entry = OverrideEntry {
            key,
            override_config: params.override_config.clone(),
            metadata: OverrideMetadata {
                created_at: chrono::Utc::now(),
                modified_at: chrono::Utc::now(),
                file_path: params.context.file_path.clone(),
                function_name: params.context.function_name.clone(),
                original_line: Some(params.context.line_number),
                notes: None,
                validation_status: ValidationStatus::Pending,
                failure_count: 0,
                last_execution_success: None,
                last_execution_time: None,
            },
        };

        OverridePreview::new(
            entry,
            params.base_command,
            params.base_env,
            params.base_cargo_options,
            params.base_rustc_options,
            params.base_args,
        )
    }

    /// List all overrides
    pub fn list_all(&self) -> Result<Vec<OverrideEntry>> {
        self.storage.list_all()
    }

    /// List overrides for a specific file
    pub fn list_by_file(&self, file_path: &Path) -> Result<Vec<OverrideEntry>> {
        self.storage.get_by_file(file_path)
    }

    /// Delete an override by key
    pub fn delete_override(&self, key: &str) -> Result<bool> {
        self.storage.delete(key)
    }

    /// Clear all overrides
    pub fn clear_all(&self) -> Result<()> {
        self.storage.clear()
    }

    /// Clear overrides for a specific file
    pub fn clear_by_file(&self, file_path: &Path) -> Result<usize> {
        self.storage.clear_by_file(file_path)
    }

    /// Export overrides for backup
    pub fn export(&self) -> Result<String> {
        self.storage.export()
    }

    /// Import overrides from backup
    pub fn import(&self, data: &str) -> Result<usize> {
        self.storage.import(data)
    }

    /// Debug: Get resolution candidates
    pub fn debug_resolution(
        &mut self,
        context: &FunctionContext,
    ) -> Result<Vec<(OverrideEntry, ResolutionStrategy, f32)>> {
        let mut resolver = OverrideResolver::new(OverrideStorage::new(&self.workspace_path)?)?;
        resolver.get_resolution_candidates(context)
    }

    /// Get function context from file position
    pub fn get_function_context(
        &mut self,
        file_path: &Path,
        line: usize,
        column: Option<usize>,
    ) -> Result<FunctionContext> {
        let source = std::fs::read_to_string(file_path)?;

        let function_name = self
            .detector
            .find_function_at_line(&source, line)?
            .map(|f| f.name);

        Ok(FunctionContext {
            file_path: file_path.to_path_buf(),
            function_name,
            line_number: line,
            context: column.map(|c| format!("column:{c}")),
        })
    }

    /// Update execution status of an override
    pub fn update_execution_status(&self, key: &str, success: bool) -> Result<()> {
        self.storage.update_execution_status(key, success)
    }

    /// Rollback to last backup
    pub fn rollback_to_last_backup(&self) -> Result<()> {
        self.storage.rollback_to_last_backup()
    }

    /// Get overrides that have failed multiple times
    pub fn get_problematic_overrides(&self, failure_threshold: u32) -> Result<Vec<OverrideEntry>> {
        let all_overrides = self.list_all()?;
        Ok(all_overrides
            .into_iter()
            .filter(|entry| entry.metadata.failure_count >= failure_threshold)
            .collect())
    }

    /// Check if an override should be auto-rolled back
    pub fn should_auto_rollback(&self, key: &str, threshold: u32) -> Result<bool> {
        if let Some(entry) = self.storage.get_by_primary_key(key)? {
            Ok(entry.metadata.failure_count >= threshold)
        } else {
            Ok(false)
        }
    }
}

/// Convenience function to create override system from current directory
pub fn create_override_system() -> Result<OverrideSystem> {
    let cwd = std::env::current_dir().map_err(|e| {
        OverrideError::StorageError(format!("Failed to get current directory: {e}"))
    })?;
    OverrideSystem::new(&cwd)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_override_system_workflow() {
        let temp_dir = TempDir::new().unwrap();
        let mut system = OverrideSystem::new_for_test(temp_dir.path()).unwrap();

        // Create context
        let context = FunctionContext {
            file_path: PathBuf::from("src/main.rs"),
            function_name: Some("handle_request".to_string()),
            line_number: 42,
            context: None,
        };

        // Generate key
        let key = system.generate_key(&context).unwrap();
        assert_eq!(key.primary, "src/main.rs:handle_request");

        // Save override
        let mut override_config = CommandOverride::new("test".to_string());
        override_config
            .env
            .insert("RUST_LOG".to_string(), "debug".to_string());

        system
            .save_override(key.clone(), override_config.clone(), &context)
            .unwrap();

        // Resolve override (use test method that works with test storage)
        let resolved = system.resolve_override_with_storage(&context).unwrap();
        assert!(resolved.is_some());

        let resolved_config = resolved.unwrap();
        assert_eq!(resolved_config.key, "test");
        assert_eq!(
            resolved_config.env.get("RUST_LOG"),
            Some(&"debug".to_string())
        );

        // List overrides
        let all_overrides = system.list_all().unwrap();
        assert_eq!(all_overrides.len(), 1);

        // Delete override
        assert!(system.delete_override(&key.primary).unwrap());

        // Verify deleted
        let resolved_after = system.resolve_override_with_storage(&context).unwrap();
        assert!(resolved_after.is_none());
    }

    #[test]
    fn test_function_detection_integration() {
        let temp_dir = TempDir::new().unwrap();
        let mut system = OverrideSystem::new_for_test(temp_dir.path()).unwrap();

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

        // Get context for line inside process_data
        let context = system.get_function_context(&test_file, 2, None).unwrap();
        assert_eq!(context.function_name.as_deref(), Some("process_data"));

        // Get context for line inside main
        let context = system.get_function_context(&test_file, 6, None).unwrap();
        assert_eq!(context.function_name.as_deref(), Some("main"));
    }
}
