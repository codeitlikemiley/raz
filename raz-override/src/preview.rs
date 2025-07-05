use crate::{OverrideEntry, error::Result};
use raz_config::CommandOverride;
use serde::{Deserialize, Serialize};
use std::fmt;

/// A change that will be made by an override
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreviewChange {
    /// Add a new environment variable
    AddEnv { key: String, value: String },
    /// Remove an environment variable
    RemoveEnv { key: String },
    /// Modify an existing environment variable
    ModifyEnv {
        key: String,
        old_value: String,
        new_value: String,
    },

    /// Add a cargo option
    AddCargoOption { option: String },
    /// Remove a cargo option
    RemoveCargoOption { option: String },

    /// Add a rustc option
    AddRustcOption { option: String },
    /// Remove a rustc option
    RemoveRustcOption { option: String },

    /// Add command arguments
    AddArgs { args: Vec<String> },
    /// Remove command arguments
    RemoveArgs { args: Vec<String> },

    /// Change command
    ChangeCommand {
        old_command: String,
        new_command: String,
    },
}

impl fmt::Display for PreviewChange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PreviewChange::AddEnv { key, value } => {
                write!(f, "  + Set env var: {key}={value}")
            }
            PreviewChange::RemoveEnv { key } => {
                write!(f, "  - Remove env var: {key}")
            }
            PreviewChange::ModifyEnv {
                key,
                old_value,
                new_value,
            } => {
                write!(f, "  ~ Change env var: {key}={old_value} → {new_value}")
            }
            PreviewChange::AddCargoOption { option } => {
                write!(f, "  + Add cargo option: {option}")
            }
            PreviewChange::RemoveCargoOption { option } => {
                write!(f, "  - Remove cargo option: {option}")
            }
            PreviewChange::AddRustcOption { option } => {
                write!(f, "  + Add rustc option: {option}")
            }
            PreviewChange::RemoveRustcOption { option } => {
                write!(f, "  - Remove rustc option: {option}")
            }
            PreviewChange::AddArgs { args } => {
                write!(f, "  + Add args: {}", args.join(" "))
            }
            PreviewChange::RemoveArgs { args } => {
                write!(f, "  - Remove args: {}", args.join(" "))
            }
            PreviewChange::ChangeCommand {
                old_command,
                new_command,
            } => {
                write!(f, "  ~ Change command: {old_command} → {new_command}")
            }
        }
    }
}

/// Preview of changes that will be made by an override
#[derive(Debug, Clone)]
pub struct OverridePreview {
    /// The override entry
    pub entry: OverrideEntry,
    /// The base command that will be modified
    pub base_command: String,
    /// List of changes that will be applied
    pub changes: Vec<PreviewChange>,
    /// The final command after applying overrides
    pub final_command: String,
}

impl OverridePreview {
    /// Create a preview for an override
    pub fn new(
        entry: OverrideEntry,
        base_command: &str,
        base_env: &indexmap::IndexMap<String, String>,
        base_cargo_options: &[String],
        base_rustc_options: &[String],
        base_args: &[String],
    ) -> Result<Self> {
        let mut changes = Vec::new();
        let override_config = &entry.override_config;

        // Note: CommandOverride doesn't have a command field, only options and args

        // Check environment variable changes
        for (key, value) in &override_config.env {
            if let Some(old_value) = base_env.get(key) {
                if old_value != value {
                    changes.push(PreviewChange::ModifyEnv {
                        key: key.clone(),
                        old_value: old_value.clone(),
                        new_value: value.clone(),
                    });
                }
            } else {
                changes.push(PreviewChange::AddEnv {
                    key: key.clone(),
                    value: value.clone(),
                });
            }
        }

        // Check cargo options
        for option in &override_config.cargo_options {
            if !base_cargo_options.contains(option) {
                changes.push(PreviewChange::AddCargoOption {
                    option: option.clone(),
                });
            }
        }

        // Check rustc options
        for option in &override_config.rustc_options {
            if !base_rustc_options.contains(option) {
                changes.push(PreviewChange::AddRustcOption {
                    option: option.clone(),
                });
            }
        }

        // Check arguments
        for arg in &override_config.args {
            if !base_args.contains(arg) {
                changes.push(PreviewChange::AddArgs {
                    args: vec![arg.clone()],
                });
            }
        }

        // Build final command (simplified)
        let final_command = Self::build_final_command(
            override_config,
            base_command,
            base_cargo_options,
            base_rustc_options,
            base_args,
        );

        Ok(Self {
            entry,
            base_command: base_command.to_string(),
            changes,
            final_command,
        })
    }

    /// Build the final command string
    fn build_final_command(
        override_config: &CommandOverride,
        base_command: &str,
        base_cargo_options: &[String],
        base_rustc_options: &[String],
        base_args: &[String],
    ) -> String {
        let command = base_command;

        let mut parts = vec![command.to_string()];

        // Add cargo options
        parts.extend(base_cargo_options.iter().cloned());
        parts.extend(override_config.cargo_options.iter().cloned());

        // Add rustc options (if any)
        if !base_rustc_options.is_empty() || !override_config.rustc_options.is_empty() {
            parts.push("--".to_string());
            parts.extend(base_rustc_options.iter().cloned());
            parts.extend(override_config.rustc_options.iter().cloned());
        }

        // Add args
        if !base_args.is_empty() || !override_config.args.is_empty() {
            parts.push("--".to_string());
            parts.extend(base_args.iter().cloned());
            parts.extend(override_config.args.iter().cloned());
        }

        parts.join(" ")
    }

    /// Check if this preview has any changes
    pub fn has_changes(&self) -> bool {
        !self.changes.is_empty()
    }

    /// Get a summary of changes
    pub fn summary(&self) -> String {
        if self.changes.is_empty() {
            return "No changes".to_string();
        }

        let env_changes = self
            .changes
            .iter()
            .filter(|c| {
                matches!(
                    c,
                    PreviewChange::AddEnv { .. }
                        | PreviewChange::RemoveEnv { .. }
                        | PreviewChange::ModifyEnv { .. }
                )
            })
            .count();

        let option_changes = self
            .changes
            .iter()
            .filter(|c| {
                matches!(
                    c,
                    PreviewChange::AddCargoOption { .. }
                        | PreviewChange::RemoveCargoOption { .. }
                        | PreviewChange::AddRustcOption { .. }
                        | PreviewChange::RemoveRustcOption { .. }
                )
            })
            .count();

        let arg_changes = self
            .changes
            .iter()
            .filter(|c| {
                matches!(
                    c,
                    PreviewChange::AddArgs { .. } | PreviewChange::RemoveArgs { .. }
                )
            })
            .count();

        let mut parts = Vec::new();
        if env_changes > 0 {
            parts.push(format!("{env_changes} env"));
        }
        if option_changes > 0 {
            parts.push(format!("{option_changes} options"));
        }
        if arg_changes > 0 {
            parts.push(format!("{arg_changes} args"));
        }

        format!("{} changes", parts.join(", "))
    }
}

impl fmt::Display for OverridePreview {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Override Preview for: {}", self.entry.key)?;
        writeln!(f, "Base command: {}", self.base_command)?;
        writeln!(f)?;

        if self.changes.is_empty() {
            writeln!(f, "No changes will be made.")?;
        } else {
            writeln!(f, "Changes to be applied:")?;
            for change in &self.changes {
                writeln!(f, "{change}")?;
            }
        }

        writeln!(f)?;
        writeln!(f, "Final command: {}", self.final_command)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FunctionContext, OverrideKey, OverrideMetadata};
    use indexmap::IndexMap;
    use std::path::PathBuf;

    #[test]
    fn test_preview_with_changes() {
        let mut override_config = CommandOverride::new("test".to_string());
        override_config
            .env
            .insert("RUST_LOG".to_string(), "debug".to_string());
        override_config.cargo_options.push("--release".to_string());
        override_config.args.push("--port".to_string());
        override_config.args.push("8080".to_string());

        let context = FunctionContext {
            file_path: PathBuf::from("src/main.rs"),
            function_name: Some("main".to_string()),
            line_number: 1,
            context: None,
        };

        let entry = OverrideEntry {
            key: OverrideKey::new(&context).unwrap(),
            override_config,
            metadata: OverrideMetadata {
                created_at: chrono::Utc::now(),
                modified_at: chrono::Utc::now(),
                file_path: context.file_path,
                function_name: context.function_name,
                original_line: Some(context.line_number),
                notes: None,
                validation_status: crate::ValidationStatus::Pending,
                failure_count: 0,
                last_execution_success: None,
                last_execution_time: None,
            },
        };

        let base_env = IndexMap::new();
        let base_cargo_options = vec![];
        let base_rustc_options = vec![];
        let base_args = vec![];

        let preview = OverridePreview::new(
            entry,
            "run",
            &base_env,
            &base_cargo_options,
            &base_rustc_options,
            &base_args,
        )
        .unwrap();

        assert!(preview.has_changes());
        assert_eq!(preview.changes.len(), 4); // env add, cargo option add, args add (2 items)

        let summary = preview.summary();
        assert!(summary.contains("env"));
        assert!(summary.contains("options"));
        assert!(summary.contains("args"));
    }
}
