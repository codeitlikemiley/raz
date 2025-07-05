//! Command providers for different frameworks and tools

use crate::{Command, CommandBuilder, CommandCategory, ProjectContext, ProjectType, RazResult};
use async_trait::async_trait;

// Framework providers
pub mod bevy;
pub mod dioxus;
pub mod leptos;
pub mod tauri;
pub mod yew;

pub use bevy::BevyProvider;
pub use dioxus::DioxusProvider;
pub use leptos::LeptosProvider;
pub use tauri::TauriProvider;
pub use yew::YewProvider;

/// Trait for implementing command providers
#[async_trait]
pub trait CommandProvider: Send + Sync {
    /// Unique name for this provider
    fn name(&self) -> &str;

    /// Generate commands for the given context
    async fn commands(&self, context: &ProjectContext) -> RazResult<Vec<Command>>;

    /// Priority for this provider (higher = more important)
    fn priority(&self) -> u8 {
        50
    }

    /// Whether this provider can handle the given context
    fn can_handle(&self, context: &ProjectContext) -> bool {
        let _ = context;
        true
    }
}

/// Built-in cargo provider that handles standard Rust project commands
pub struct CargoProvider {
    name: String,
}

impl Default for CargoProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl CargoProvider {
    pub fn new() -> Self {
        Self {
            name: "cargo".to_string(),
        }
    }

    /// Resolve test context for enhanced command generation
    async fn resolve_test_context(
        &self,
        context: &ProjectContext,
    ) -> RazResult<Option<crate::TestContext>> {
        let Some(ref file_context) = context.current_file else {
            return Ok(None);
        };

        // Only proceed if we're in a Rust file
        if file_context.language != crate::Language::Rust {
            return Ok(None);
        }

        let mut test_context = crate::TestContext::new();

        // 1. Resolve package name from workspace structure
        test_context.package_name = self.resolve_package_name(context, &file_context.path)?;

        // 2. Resolve target type from file path
        test_context.target_type = self.resolve_target_type(context, &file_context.path)?;

        // 3. Build module path from file path and detected modules
        test_context.module_path = self.build_module_path(&file_context.path, file_context)?;

        // 4. Set test name if cursor is on a test function
        if let Some(ref cursor_symbol) = file_context.cursor_symbol {
            if cursor_symbol.kind == crate::SymbolKind::Test
                || (cursor_symbol.kind == crate::SymbolKind::Function
                    && cursor_symbol.name.starts_with("test_"))
            {
                test_context.test_name = Some(cursor_symbol.name.clone());
            }
        }

        // 5. Add features and environment variables
        test_context.features = context.active_features.clone();
        test_context.env_vars = context
            .env_vars
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        test_context.working_dir = Some(context.workspace_root.clone());

        // Only return if we have meaningful test information
        if test_context.is_precise() || test_context.package_name.is_some() {
            Ok(Some(test_context))
        } else {
            Ok(None)
        }
    }

    /// Build module path from file path and file context
    fn build_module_path(
        &self,
        file_path: &std::path::Path,
        file_context: &crate::FileContext,
    ) -> RazResult<Vec<String>> {
        let mut module_path = Vec::new();

        // Start with file-based module path
        if let Some(base_path) = crate::FileAnalyzer::extract_module_path(file_path) {
            module_path = base_path
                .split("::")
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect();
        }

        // Look for test modules in the symbols
        let has_test_module = file_context.symbols.iter().any(|s| {
            s.kind == crate::SymbolKind::Module && (s.name == "tests" || s.name.contains("test"))
        });

        // If we have test modules and we're in a test context, add "tests" to the path
        if has_test_module && !module_path.iter().any(|m| m == "tests") {
            // Check if we're likely in a test context based on file content or cursor symbol
            let likely_in_test = file_context
                .cursor_symbol
                .as_ref()
                .map(|s| s.kind == crate::SymbolKind::Test)
                .unwrap_or(false);

            if likely_in_test {
                module_path.push("tests".to_string());
            }
        }

        Ok(module_path)
    }

    /// Resolve package name from workspace structure
    fn resolve_package_name(
        &self,
        context: &ProjectContext,
        file_path: &std::path::Path,
    ) -> RazResult<Option<String>> {
        // For single-package projects
        if context.workspace_members.len() == 1 {
            return Ok(Some(context.workspace_members[0].name.clone()));
        }

        // For workspace projects, find which member contains this file
        for member in &context.workspace_members {
            let member_path = if member.path.is_absolute() {
                member.path.clone()
            } else {
                context.workspace_root.join(&member.path)
            };

            if file_path.starts_with(&member_path) {
                return Ok(Some(member.name.clone()));
            }
        }

        Ok(None)
    }

    /// Resolve target type from file path
    fn resolve_target_type(
        &self,
        context: &ProjectContext,
        file_path: &std::path::Path,
    ) -> RazResult<crate::TestTargetType> {
        let file_str = file_path.to_string_lossy();

        // Check for integration tests (tests/ directory)
        if file_str.contains("/tests/") {
            if let Some(test_name) = file_path.file_stem().and_then(|s| s.to_str()) {
                return Ok(crate::TestTargetType::Test(test_name.to_string()));
            }
        }

        // Check for examples (examples/ directory)
        if file_str.contains("/examples/") {
            if let Some(example_name) = file_path.file_stem().and_then(|s| s.to_str()) {
                return Ok(crate::TestTargetType::Example(example_name.to_string()));
            }
        }

        // Check for benchmarks (benches/ directory)
        if file_str.contains("/benches/") {
            if let Some(bench_name) = file_path.file_stem().and_then(|s| s.to_str()) {
                return Ok(crate::TestTargetType::Bench(bench_name.to_string()));
            }
        }

        // Check if it's a library file
        if file_str.contains("/src/lib.rs") || file_str.contains("/src/mod.rs") {
            return Ok(crate::TestTargetType::Lib);
        }

        // Check for binary files
        if file_str.contains("/src/main.rs") {
            return Ok(crate::TestTargetType::Bin("main".to_string()));
        }

        if file_str.contains("/src/bin/") {
            if let Some(bin_name) = file_path.file_stem().and_then(|s| s.to_str()) {
                return Ok(crate::TestTargetType::Bin(bin_name.to_string()));
            }
        }

        // Check against build targets for more precise detection
        for target in &context.build_targets {
            if file_path.starts_with(target.path.parent().unwrap_or(&context.workspace_root)) {
                return match target.target_type {
                    crate::TargetType::Binary => {
                        Ok(crate::TestTargetType::Bin(target.name.clone()))
                    }
                    crate::TargetType::Library => Ok(crate::TestTargetType::Lib),
                    crate::TargetType::Test => Ok(crate::TestTargetType::Test(target.name.clone())),
                    crate::TargetType::Bench => {
                        Ok(crate::TestTargetType::Bench(target.name.clone()))
                    }
                    crate::TargetType::Example => {
                        Ok(crate::TestTargetType::Example(target.name.clone()))
                    }
                };
            }
        }

        // Default to library
        Ok(crate::TestTargetType::Lib)
    }
}

#[async_trait]
impl CommandProvider for CargoProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn priority(&self) -> u8 {
        100 // High priority as cargo is fundamental to Rust projects
    }

    fn can_handle(&self, context: &ProjectContext) -> bool {
        // Cargo provider can handle any Rust project
        !matches!(context.project_type, ProjectType::Mixed(_))
    }

    async fn commands(&self, context: &ProjectContext) -> RazResult<Vec<Command>> {
        let mut commands = Vec::new();
        let cwd = context.workspace_root.clone();

        // Basic cargo commands
        commands.extend([
            // Build commands
            CommandBuilder::new("cargo-build", "cargo")
                .label("Build Project")
                .description("Build the project in debug mode")
                .arg("build")
                .category(CommandCategory::Build)
                .priority(80)
                .tag("cargo")
                .tag("build")
                .cwd(cwd.clone())
                .estimated_duration(30)
                .build(),
            CommandBuilder::new("cargo-build-release", "cargo")
                .label("Build Release")
                .description("Build the project in release mode with optimizations")
                .args(vec!["build".to_string(), "--release".to_string()])
                .category(CommandCategory::Build)
                .priority(70)
                .tag("cargo")
                .tag("build")
                .tag("release")
                .cwd(cwd.clone())
                .estimated_duration(120)
                .build(),
            // Test commands
            CommandBuilder::new("cargo-test", "cargo")
                .label("Run Tests")
                .description("Run all tests in the project")
                .arg("test")
                .category(CommandCategory::Test)
                .priority(85)
                .tag("cargo")
                .tag("test")
                .cwd(cwd.clone())
                .estimated_duration(60)
                .build(),
            CommandBuilder::new("cargo-test-release", "cargo")
                .label("Run Tests (Release)")
                .description("Run tests in release mode")
                .args(vec!["test".to_string(), "--release".to_string()])
                .category(CommandCategory::Test)
                .priority(60)
                .tag("cargo")
                .tag("test")
                .tag("release")
                .cwd(cwd.clone())
                .estimated_duration(90)
                .build(),
            // Check commands
            CommandBuilder::new("cargo-check", "cargo")
                .label("Check Code")
                .description("Check for compilation errors without building")
                .arg("check")
                .category(CommandCategory::Lint)
                .priority(90)
                .tag("cargo")
                .tag("check")
                .cwd(cwd.clone())
                .estimated_duration(15)
                .build(),
            CommandBuilder::new("cargo-clippy", "cargo")
                .label("Run Clippy")
                .description("Run Clippy lints to catch common mistakes")
                .arg("clippy")
                .category(CommandCategory::Lint)
                .priority(75)
                .tag("cargo")
                .tag("clippy")
                .tag("lint")
                .cwd(cwd.clone())
                .estimated_duration(30)
                .build(),
            // Format commands
            CommandBuilder::new("cargo-fmt", "cargo")
                .label("Format Code")
                .description("Format code using rustfmt")
                .arg("fmt")
                .category(CommandCategory::Format)
                .priority(70)
                .tag("cargo")
                .tag("format")
                .cwd(cwd.clone())
                .estimated_duration(5)
                .build(),
            CommandBuilder::new("cargo-fmt-check", "cargo")
                .label("Check Formatting")
                .description("Check if code is properly formatted")
                .args(vec!["fmt".to_string(), "--check".to_string()])
                .category(CommandCategory::Format)
                .priority(60)
                .tag("cargo")
                .tag("format")
                .tag("check")
                .cwd(cwd.clone())
                .estimated_duration(5)
                .build(),
            // Clean commands
            CommandBuilder::new("cargo-clean", "cargo")
                .label("Clean Build")
                .description("Remove build artifacts")
                .arg("clean")
                .category(CommandCategory::Clean)
                .priority(40)
                .tag("cargo")
                .tag("clean")
                .cwd(cwd.clone())
                .estimated_duration(5)
                .build(),
            // Update commands
            CommandBuilder::new("cargo-update", "cargo")
                .label("Update Dependencies")
                .description("Update dependencies to latest compatible versions")
                .arg("update")
                .category(CommandCategory::Update)
                .priority(30)
                .tag("cargo")
                .tag("update")
                .cwd(cwd.clone())
                .estimated_duration(30)
                .build(),
        ]);

        // Add run commands if binary targets exist
        let has_binary = context
            .build_targets
            .iter()
            .any(|target| target.target_type == crate::TargetType::Binary);

        if has_binary {
            commands.push(
                CommandBuilder::new("cargo-run", "cargo")
                    .label("Run Project")
                    .description("Run the main binary")
                    .arg("run")
                    .category(CommandCategory::Run)
                    .priority(95)
                    .tag("cargo")
                    .tag("run")
                    .cwd(cwd.clone())
                    .estimated_duration(60)
                    .build(),
            );

            commands.push(
                CommandBuilder::new("cargo-run-release", "cargo")
                    .label("Run Project (Release)")
                    .description("Run the main binary in release mode")
                    .args(vec!["run".to_string(), "--release".to_string()])
                    .category(CommandCategory::Run)
                    .priority(80)
                    .tag("cargo")
                    .tag("run")
                    .tag("release")
                    .cwd(cwd.clone())
                    .estimated_duration(90)
                    .build(),
            );
        }

        // Add example commands if examples exist
        let examples: Vec<_> = context
            .build_targets
            .iter()
            .filter(|target| target.target_type == crate::TargetType::Example)
            .collect();

        for example in examples {
            commands.push(
                CommandBuilder::new(format!("cargo-example-{}", example.name), "cargo")
                    .label(format!("Run Example: {}", example.name))
                    .description(format!("Run the '{}' example", example.name))
                    .args(vec![
                        "run".to_string(),
                        "--example".to_string(),
                        example.name.clone(),
                    ])
                    .category(CommandCategory::Run)
                    .priority(65)
                    .tag("cargo")
                    .tag("example")
                    .tag(&example.name)
                    .cwd(cwd.clone())
                    .estimated_duration(45)
                    .build(),
            );
        }

        // Add bench commands if bench targets exist
        let has_bench = context
            .build_targets
            .iter()
            .any(|target| target.target_type == crate::TargetType::Bench);

        if has_bench {
            commands.push(
                CommandBuilder::new("cargo-bench", "cargo")
                    .label("Run Benchmarks")
                    .description("Run all benchmarks")
                    .arg("bench")
                    .category(CommandCategory::Test)
                    .priority(50)
                    .tag("cargo")
                    .tag("bench")
                    .cwd(cwd.clone())
                    .estimated_duration(180)
                    .build(),
            );
        }

        // Add workspace-specific commands
        if context.workspace_members.len() > 1 {
            commands.extend([
                CommandBuilder::new("cargo-build-all", "cargo")
                    .label("Build All Packages")
                    .description("Build all packages in the workspace")
                    .args(vec!["build".to_string(), "--workspace".to_string()])
                    .category(CommandCategory::Build)
                    .priority(85)
                    .tag("cargo")
                    .tag("workspace")
                    .tag("build")
                    .cwd(cwd.clone())
                    .estimated_duration(120)
                    .build(),
                CommandBuilder::new("cargo-test-all", "cargo")
                    .label("Test All Packages")
                    .description("Run tests for all packages in the workspace")
                    .args(vec!["test".to_string(), "--workspace".to_string()])
                    .category(CommandCategory::Test)
                    .priority(80)
                    .tag("cargo")
                    .tag("workspace")
                    .tag("test")
                    .cwd(cwd.clone())
                    .estimated_duration(180)
                    .build(),
            ]);
        }

        // Add enhanced context-specific test commands
        if let Some(file_context) = &context.current_file {
            // Try to resolve test context for enhanced command generation
            if let Ok(Some(test_context)) = self.resolve_test_context(context).await {
                // Generate enhanced test commands using the new system
                let enhanced_commands =
                    crate::test_commands::TestCommandGenerator::generate_commands(
                        context,
                        Some(&test_context),
                    )?;
                commands.extend(enhanced_commands);
            } else {
                // Fallback to basic cursor-based test command
                if let Some(cursor_symbol) = &file_context.cursor_symbol {
                    if cursor_symbol.kind == crate::SymbolKind::Test {
                        commands.push(
                            CommandBuilder::new("cargo-test-current", "cargo")
                                .label(format!("Run Test: {}", cursor_symbol.name))
                                .description(format!(
                                    "Run the '{}' test function",
                                    cursor_symbol.name
                                ))
                                .args(vec!["test".to_string(), cursor_symbol.name.clone()])
                                .category(CommandCategory::Test)
                                .priority(100)
                                .tag("cargo")
                                .tag("test")
                                .tag("current")
                                .cwd(cwd.clone())
                                .estimated_duration(15)
                                .build(),
                        );
                    }
                }
            }
        }

        Ok(commands)
    }
}

/// Provider for project documentation commands
pub struct DocProvider;

impl DocProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DocProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CommandProvider for DocProvider {
    fn name(&self) -> &str {
        "doc"
    }

    fn priority(&self) -> u8 {
        60
    }

    async fn commands(&self, context: &ProjectContext) -> RazResult<Vec<Command>> {
        let cwd = context.workspace_root.clone();

        Ok(vec![
            CommandBuilder::new("cargo-doc", "cargo")
                .label("Generate Documentation")
                .description("Generate documentation for the project")
                .arg("doc")
                .category(CommandCategory::Generate)
                .priority(50)
                .tag("cargo")
                .tag("doc")
                .cwd(cwd.clone())
                .estimated_duration(60)
                .build(),
            CommandBuilder::new("cargo-doc-open", "cargo")
                .label("Generate & Open Documentation")
                .description("Generate documentation and open in browser")
                .args(vec!["doc".to_string(), "--open".to_string()])
                .category(CommandCategory::Generate)
                .priority(55)
                .tag("cargo")
                .tag("doc")
                .cwd(cwd)
                .estimated_duration(65)
                .build(),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BuildTarget, TargetType, WorkspaceMember};
    use std::path::PathBuf;

    fn create_test_context() -> ProjectContext {
        ProjectContext {
            workspace_root: PathBuf::from("/test/project"),
            current_file: None,
            cursor_position: None,
            project_type: ProjectType::Binary,
            dependencies: Vec::new(),
            workspace_members: vec![WorkspaceMember {
                name: "test-project".to_string(),
                path: PathBuf::from("/test/project"),
                package_type: ProjectType::Binary,
            }],
            build_targets: vec![BuildTarget {
                name: "main".to_string(),
                target_type: TargetType::Binary,
                path: PathBuf::from("/test/project/src/main.rs"),
            }],
            active_features: Vec::new(),
            env_vars: std::collections::HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_cargo_provider_basic_commands() {
        let provider = CargoProvider::new();
        let context = create_test_context();

        let commands = provider.commands(&context).await.unwrap();

        assert!(!commands.is_empty());
        assert!(commands.iter().any(|c| c.id == "cargo-build"));
        assert!(commands.iter().any(|c| c.id == "cargo-test"));
        assert!(commands.iter().any(|c| c.id == "cargo-run"));
        assert!(commands.iter().any(|c| c.id == "cargo-check"));
        assert!(commands.iter().any(|c| c.id == "cargo-clippy"));
        assert!(commands.iter().any(|c| c.id == "cargo-fmt"));
    }

    #[tokio::test]
    async fn test_cargo_provider_with_examples() {
        let provider = CargoProvider::new();
        let mut context = create_test_context();

        // Add an example target
        context.build_targets.push(BuildTarget {
            name: "hello".to_string(),
            target_type: TargetType::Example,
            path: PathBuf::from("/test/project/examples/hello.rs"),
        });

        let commands = provider.commands(&context).await.unwrap();

        assert!(commands.iter().any(|c| c.id == "cargo-example-hello"));
    }

    #[tokio::test]
    async fn test_cargo_provider_workspace() {
        let provider = CargoProvider::new();
        let mut context = create_test_context();

        // Add another workspace member
        context.workspace_members.push(WorkspaceMember {
            name: "test-lib".to_string(),
            path: PathBuf::from("/test/project/test-lib"),
            package_type: ProjectType::Library,
        });

        let commands = provider.commands(&context).await.unwrap();

        assert!(commands.iter().any(|c| c.id == "cargo-build-all"));
        assert!(commands.iter().any(|c| c.id == "cargo-test-all"));
    }

    #[tokio::test]
    async fn test_doc_provider() {
        let provider = DocProvider::new();
        let context = create_test_context();

        let commands = provider.commands(&context).await.unwrap();

        assert_eq!(commands.len(), 2);
        assert!(commands.iter().any(|c| c.id == "cargo-doc"));
        assert!(commands.iter().any(|c| c.id == "cargo-doc-open"));
    }
}
