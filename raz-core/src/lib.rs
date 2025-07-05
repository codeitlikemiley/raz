//! # RAZ Core Library
//!
//! Universal command generator for Rust projects across multiple IDEs.
//!
//! ## Overview
//!
//! RAZ (Rust Action Zapper) provides intelligent, context-aware command generation
//! by analyzing project structure, dependencies, and cursor position to suggest
//! relevant commands for development workflows.
//!
//! ## Architecture
//!
//! - **Context Layer**: Analyzes project structure and file context
//! - **Provider Layer**: Framework-specific command generators
//! - **Filter Layer**: Rule engine for command filtering and prioritization
//! - **Command Layer**: Command execution and management
//! - **Config Layer**: Configuration management with hierarchical overrides

#[cfg(feature = "tree-sitter-support")]
pub mod ast;
pub mod browser;
pub mod cargo_options;
pub mod cargo_options_catalog;
pub mod commands;
mod config_adapter;

pub use config_adapter::ConfigManager;
pub mod context;
pub mod dioxus_validation;
pub mod error;
pub mod file_detection;
pub mod filters;
pub mod framework_detection;
pub mod providers;
pub mod rustc_options;
pub mod test_commands;
pub mod tree_sitter_test_detector;
pub mod universal_command_generator;

// Re-export main types
#[cfg(feature = "tree-sitter-support")]
pub use ast::{RustAnalyzer, SymbolContext as AstSymbolContext};
pub use cargo_options::*;
pub use commands::*;
// Config types are now from raz-config crate
pub use context::*;
pub use dioxus_validation::*;
pub use error::*;
pub use file_detection::{
    EntryPoint, EntryPointType, ExecutionCapabilities, FileDetector, FileExecutionContext,
    FileRole, RustProjectType, SingleFileType,
};
pub use filters::*;
pub use framework_detection::*;
pub use providers::*;
pub use raz_config::{
    CommandConfigBuilder, CommandOverride, ConfigBuilder, ConfigError, ConfigTemplates,
    ConfigValidator, EffectiveConfig, GlobalConfig, OverrideBuilder, OverrideMode, WorkspaceConfig,
};
pub use universal_command_generator::*;

use anyhow::Result;
use std::path::Path;
use std::sync::Arc;

/// Main entry point for the RAZ command generation system
pub struct RazCore {
    config: Arc<ConfigManager>,
    providers: Vec<Box<dyn CommandProvider>>,
    pub context_analyzer: Arc<ProjectAnalyzer>,
    filter_engine: Arc<FilterEngine>,
}

impl RazCore {
    /// Create a new RAZ instance with default configuration
    pub fn new() -> Result<Self> {
        let config = ConfigManager::new()?;
        Self::with_config(config)
    }

    /// Create a new RAZ instance with custom configuration
    pub fn with_config(config: ConfigManager) -> Result<Self> {
        let config = Arc::new(config);
        let context_analyzer = Arc::new(ProjectAnalyzer::new());
        let filter_engine = Arc::new(FilterEngine::new());

        let mut raz = Self {
            config,
            providers: Vec::new(),
            context_analyzer,
            filter_engine,
        };

        // Register built-in providers
        raz.register_builtin_providers()?;

        Ok(raz)
    }

    /// Analyze a workspace and return project context
    pub async fn analyze_workspace(&self, path: &Path) -> Result<ProjectContext> {
        self.context_analyzer
            .analyze_project(path)
            .await
            .map_err(anyhow::Error::new)
    }

    /// Generate commands for the given context
    pub async fn generate_commands(&self, context: &ProjectContext) -> Result<Vec<Command>> {
        let mut all_commands = Vec::new();

        // Collect commands from all providers
        for provider in &self.providers {
            if provider.can_handle(context) {
                let commands = provider.commands(context).await?;
                all_commands.extend(commands);
            }
        }

        // Apply filters and sorting
        let filtered_commands = self.filter_engine.apply_filters(&all_commands, context)?;

        Ok(filtered_commands)
    }

    /// Execute a command
    pub async fn execute_command(&self, command: &Command) -> Result<ExecutionResult> {
        command.execute().await.map_err(anyhow::Error::new)
    }

    /// Execute a command with browser launching support
    pub async fn execute_command_with_browser(
        &self,
        command: &Command,
        browser: Option<String>,
    ) -> Result<ExecutionResult> {
        command
            .execute_with_browser(true, browser)
            .await
            .map_err(anyhow::Error::new)
    }

    /// Register a custom command provider
    pub fn register_provider(&mut self, provider: Box<dyn CommandProvider>) {
        self.providers.push(provider);
    }

    /// Get effective configuration for a workspace
    pub fn get_config(&self, workspace: &Path) -> EffectiveConfig {
        self.config.get_effective_config(workspace)
    }

    /// Get command override for a specific file/context
    pub fn get_command_override(&self, workspace: &Path, key: &str) -> Option<CommandOverride> {
        self.config.get_command_override(workspace, key)
    }

    /// Set command override for a specific file/context
    pub fn set_command_override(
        &mut self,
        workspace: &Path,
        key: String,
        override_config: CommandOverride,
    ) -> Result<()> {
        // We need to work with a mutable reference to config
        // For now, let's just create a new ConfigManager to save
        let mut config_manager = ConfigManager::new()?;
        config_manager.set_command_override(workspace, key, override_config)
    }

    /// Generate universal commands for any file (stateless)
    pub async fn generate_universal_commands(
        &self,
        file_path: &Path,
        cursor: Option<Position>,
    ) -> Result<Vec<Command>> {
        self.generate_universal_commands_with_options(file_path, cursor, false)
            .await
    }

    /// Generate universal commands with force standalone option
    pub async fn generate_universal_commands_with_options(
        &self,
        file_path: &Path,
        cursor: Option<Position>,
        force_standalone: bool,
    ) -> Result<Vec<Command>> {
        let context =
            FileDetector::detect_context_with_options(file_path, cursor, force_standalone)
                .map_err(anyhow::Error::new)?;

        // Determine workspace and override key
        let workspace = context.get_workspace_root();
        let override_key = Self::build_override_key(file_path, &context, cursor);

        UniversalCommandGenerator::generate_commands_with_overrides(
            &context,
            cursor,
            workspace,
            override_key.as_deref(),
        )
        .map_err(anyhow::Error::new)
    }

    /// Generate universal commands with runtime override
    pub async fn generate_universal_commands_with_override(
        &self,
        file_path: &Path,
        cursor: Option<Position>,
        override_input: &str,
    ) -> Result<Vec<Command>> {
        let context =
            FileDetector::detect_context(file_path, cursor).map_err(anyhow::Error::new)?;

        // Determine workspace and override key
        let workspace = context.get_workspace_root();
        let override_key = Self::build_override_key(file_path, &context, cursor);

        UniversalCommandGenerator::generate_commands_with_runtime_override(
            &context,
            cursor,
            workspace,
            override_key.as_deref(),
            override_input,
        )
        .map_err(anyhow::Error::new)
    }

    /// Build override key from file path and context
    pub fn build_override_key(
        file_path: &Path,
        context: &FileExecutionContext,
        cursor: Option<Position>,
    ) -> Option<String> {
        let file_str = file_path.to_string_lossy();

        // If cursor is provided, try to find specific test or function
        if let Some(pos) = cursor {
            // Look for test at cursor
            if let Some(test_entry) =
                UniversalCommandGenerator::find_test_at_cursor(&context.entry_points, pos)
            {
                return Some(format!("{}:{}", file_str, test_entry.name));
            }

            // Look for any entry point at cursor
            for entry in &context.entry_points {
                if entry.line_range.0 <= pos.line && pos.line <= entry.line_range.1 {
                    return Some(format!("{}:{}", file_str, entry.name));
                }
            }

            // Use tree-sitter AST analysis to find the function at cursor position
            #[cfg(feature = "tree-sitter-support")]
            if let Ok(mut analyzer) = crate::ast::RustAnalyzer::new() {
                // Read the file content for parsing
                if let Ok(source) = std::fs::read_to_string(file_path) {
                    if let Ok(tree) = analyzer.parse(&source) {
                        if let Ok(Some(symbol)) = analyzer.symbol_at_position(&tree, &source, pos) {
                            return Some(format!("{}:{}", file_str, symbol.name));
                        }
                    }
                }
            }

            // Fallback: Use entry points if tree-sitter is not available or fails
            // Look for any entry point that contains the cursor
            for entry in &context.entry_points {
                if entry.line_range.0 <= pos.line && pos.line <= entry.line_range.1 {
                    return Some(format!("{}:{}", file_str, entry.name));
                }
            }
        }

        // Look for main function
        if context.entry_points.iter().any(|e| e.name == "main") {
            return Some(format!("{file_str}:main"));
        }

        // Just use file path
        Some(file_str.to_string())
    }

    /// Generate smart context-aware commands based on file and cursor position
    /// This method now delegates to universal command generation
    pub async fn generate_smart_commands(
        &self,
        workspace: &Path,
        file_path: Option<&Path>,
        cursor: Option<Position>,
    ) -> Result<Vec<Command>> {
        if let Some(file) = file_path {
            // Use universal command generation for files
            self.generate_universal_commands(file, cursor).await
        } else {
            // Fallback to workspace-based command generation
            let project_context = self.analyze_workspace(workspace).await?;
            self.generate_commands(&project_context).await
        }
    }

    fn register_builtin_providers(&mut self) -> Result<()> {
        // Register cargo provider (always available)
        self.register_provider(Box::new(providers::CargoProvider::new()));

        // Register documentation provider
        self.register_provider(Box::new(providers::DocProvider::new()));

        // Register framework providers
        self.register_provider(Box::new(providers::LeptosProvider::new()));
        self.register_provider(Box::new(providers::DioxusProvider::new()));
        self.register_provider(Box::new(providers::BevyProvider::new()));
        self.register_provider(Box::new(providers::TauriProvider::new()));
        self.register_provider(Box::new(providers::YewProvider::new()));

        Ok(())
    }
}

impl Default for RazCore {
    fn default() -> Self {
        Self::new().expect("Failed to create default RAZ instance")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_raz_core_creation() {
        let raz = RazCore::new().unwrap();
        assert!(!raz.providers.is_empty());
    }

    #[tokio::test]
    async fn test_workspace_analysis() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        fs::write(
            &cargo_toml,
            r#"
            [package]
            name = "test-project"
            version = "0.1.0"
            edition = "2021"
        "#,
        )
        .unwrap();

        let raz = RazCore::new().unwrap();
        let context = raz.analyze_workspace(temp_dir.path()).await.unwrap();

        assert_eq!(context.workspace_root, temp_dir.path());
        assert_eq!(context.project_type, ProjectType::Binary);
    }

    #[tokio::test]
    async fn test_command_generation() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        fs::write(
            &cargo_toml,
            r#"
            [package]
            name = "test-project"
            version = "0.1.0"
            edition = "2021"
        "#,
        )
        .unwrap();

        let raz = RazCore::new().unwrap();
        let context = raz.analyze_workspace(temp_dir.path()).await.unwrap();
        let commands = raz.generate_commands(&context).await.unwrap();

        assert!(!commands.is_empty());
        assert!(commands.iter().any(|c| c.command == "cargo"));
    }
}
