//! Leptos framework provider for web development commands
//!
//! Provides intelligent command suggestions for Leptos projects including
//! development server, building, testing, and deployment commands.

use crate::{
    Command, CommandBuilder, CommandCategory, CommandProvider, ProjectContext, ProjectType,
    RazResult, SymbolKind,
};
use async_trait::async_trait;

/// Provider for Leptos web framework commands
pub struct LeptosProvider {
    priority: u8,
}

impl LeptosProvider {
    pub fn new() -> Self {
        Self {
            priority: 90, // High priority for Leptos projects
        }
    }
}

#[async_trait]
impl CommandProvider for LeptosProvider {
    fn name(&self) -> &str {
        "leptos"
    }

    fn priority(&self) -> u8 {
        self.priority
    }

    fn can_handle(&self, context: &ProjectContext) -> bool {
        // Check if this is a Leptos project
        match &context.project_type {
            ProjectType::Leptos => true,
            ProjectType::Mixed(frameworks) => frameworks.contains(&ProjectType::Leptos),
            _ => {
                // Also check dependencies for leptos
                context.dependencies.iter().any(|dep| {
                    dep.name == "leptos" || dep.name == "leptos_axum" || dep.name == "leptos_actix"
                })
            }
        }
    }

    async fn commands(&self, context: &ProjectContext) -> RazResult<Vec<Command>> {
        let mut commands = Vec::new();

        // Development commands
        commands.extend(self.development_commands(context));

        // Build commands
        commands.extend(self.build_commands(context));

        // Testing commands
        commands.extend(self.test_commands(context));

        // Context-aware commands based on cursor position
        commands.extend(self.context_aware_commands(context));

        // Deployment commands
        commands.extend(self.deployment_commands(context));

        Ok(commands)
    }
}

impl LeptosProvider {
    /// Development server and watch commands
    fn development_commands(&self, context: &ProjectContext) -> Vec<Command> {
        vec![
            CommandBuilder::new("leptos-watch", "cargo-leptos")
                .label("Leptos Dev Watch")
                .description("Development server with auto-reload (recommended for development)")
                .arg("watch")
                .category(CommandCategory::Run)
                .priority(95)
                .tag("dev")
                .tag("watch")
                .tag("leptos")
                .cwd(context.workspace_root.clone())
                .estimated_duration(5)
                .build(),
            CommandBuilder::new("leptos-watch-hot", "cargo-leptos")
                .label("Leptos Hot Reload")
                .description("Development with hot-reload (requires nightly)")
                .arg("watch")
                .arg("--hot-reload")
                .category(CommandCategory::Run)
                .priority(92)
                .tag("dev")
                .tag("hot-reload")
                .tag("leptos")
                .cwd(context.workspace_root.clone())
                .estimated_duration(5)
                .build(),
            CommandBuilder::new("leptos-serve", "cargo-leptos")
                .label("Leptos Serve")
                .description("Serve in hydrate mode (production-like, no auto-reload)")
                .arg("serve")
                .category(CommandCategory::Run)
                .priority(85)
                .tag("serve")
                .tag("leptos")
                .cwd(context.workspace_root.clone())
                .estimated_duration(5)
                .build(),
            CommandBuilder::new("leptos-serve-release", "cargo-leptos")
                .label("Leptos Serve Release")
                .description("Serve optimized release build")
                .arg("serve")
                .arg("--release")
                .category(CommandCategory::Run)
                .priority(80)
                .tag("serve")
                .tag("release")
                .tag("leptos")
                .cwd(context.workspace_root.clone())
                .estimated_duration(8)
                .build(),
        ]
    }

    /// Build commands for different targets
    fn build_commands(&self, context: &ProjectContext) -> Vec<Command> {
        vec![
            CommandBuilder::new("leptos-build", "cargo-leptos")
                .label("Leptos Build")
                .description("Build Leptos app (both server SSR and client WASM)")
                .arg("build")
                .category(CommandCategory::Build)
                .priority(85)
                .tag("build")
                .tag("production")
                .tag("leptos")
                .cwd(context.workspace_root.clone())
                .estimated_duration(30)
                .build(),
            CommandBuilder::new("leptos-build-release", "cargo-leptos")
                .label("Leptos Release Build")
                .description("Build optimized release version")
                .arg("build")
                .arg("--release")
                .category(CommandCategory::Build)
                .priority(82)
                .tag("build")
                .tag("release")
                .tag("leptos")
                .cwd(context.workspace_root.clone())
                .estimated_duration(60)
                .build(),
            CommandBuilder::new("leptos-build-precompress", "cargo-leptos")
                .label("Leptos Build Compressed")
                .description("Build release with precompressed assets (gzip & brotli)")
                .arg("build")
                .arg("--release")
                .arg("--precompress")
                .category(CommandCategory::Build)
                .priority(80)
                .tag("build")
                .tag("release")
                .tag("production")
                .tag("leptos")
                .cwd(context.workspace_root.clone())
                .estimated_duration(70)
                .build(),
        ]
    }

    /// Testing commands
    fn test_commands(&self, context: &ProjectContext) -> Vec<Command> {
        vec![
            CommandBuilder::new("leptos-test", "cargo-leptos")
                .label("Leptos Test")
                .description("Run tests for app, client and server")
                .arg("test")
                .category(CommandCategory::Test)
                .priority(75)
                .tag("test")
                .tag("leptos")
                .cwd(context.workspace_root.clone())
                .estimated_duration(15)
                .build(),
            CommandBuilder::new("leptos-end-to-end", "cargo-leptos")
                .label("Leptos E2E Tests")
                .description("Start server and run end-to-end tests")
                .arg("end-to-end")
                .category(CommandCategory::Test)
                .priority(72)
                .tag("test")
                .tag("e2e")
                .tag("leptos")
                .cwd(context.workspace_root.clone())
                .estimated_duration(90)
                .build(),
            CommandBuilder::new("cargo-test-workspace", "cargo")
                .label("Workspace Tests")
                .description("Run all tests in the workspace")
                .arg("test")
                .arg("--workspace")
                .category(CommandCategory::Test)
                .priority(70)
                .tag("test")
                .tag("workspace")
                .tag("leptos")
                .cwd(context.workspace_root.clone())
                .estimated_duration(30)
                .build(),
        ]
    }

    /// Context-aware commands based on cursor position and file content
    fn context_aware_commands(&self, context: &ProjectContext) -> Vec<Command> {
        let mut commands = Vec::new();

        if let Some(file_context) = &context.current_file {
            // Commands based on current symbol
            if let Some(symbol) = &file_context.cursor_symbol {
                match symbol.kind {
                    SymbolKind::Function => {
                        if symbol.name.starts_with("test_")
                            || symbol.modifiers.contains(&"test".to_string())
                        {
                            // Test-specific commands
                            commands.push(
                                CommandBuilder::new("cargo-test-current", "cargo")
                                    .label("Test Current Function")
                                    .description(format!("Run test function: {}", symbol.name))
                                    .arg("test")
                                    .arg(&symbol.name)
                                    .arg("--")
                                    .arg("--nocapture")
                                    .category(CommandCategory::Test)
                                    .priority(90)
                                    .tag("test")
                                    .tag("current")
                                    .tag("leptos")
                                    .cwd(context.workspace_root.clone())
                                    .estimated_duration(5)
                                    .build(),
                            );
                        }

                        // Component-related commands for functions that look like Leptos components
                        if symbol.name.chars().next().is_some_and(|c| c.is_uppercase()) {
                            commands.push(
                                CommandBuilder::new("leptos-dev-component", "cargo-leptos")
                                    .label("Dev with Component Focus")
                                    .description(format!(
                                        "Develop focusing on component: {}",
                                        symbol.name
                                    ))
                                    .arg("serve")
                                    .category(CommandCategory::Run)
                                    .priority(85)
                                    .tag("component")
                                    .tag("dev")
                                    .tag("leptos")
                                    .cwd(context.workspace_root.clone())
                                    .estimated_duration(10)
                                    .build(),
                            );
                        }
                    }
                    SymbolKind::Struct => {
                        // Struct-related commands (possibly state management)
                        commands.push(
                            CommandBuilder::new("leptos-check-struct", "cargo")
                                .label("Check Struct Usage")
                                .description(format!("Check usage of struct: {}", symbol.name))
                                .arg("check")
                                .arg("--message-format=json")
                                .category(CommandCategory::Lint)
                                .priority(70)
                                .tag("check")
                                .tag("struct")
                                .tag("leptos")
                                .cwd(context.workspace_root.clone())
                                .estimated_duration(8)
                                .build(),
                        );
                    }
                    _ => {}
                }
            }

            // File-based commands
            if file_context.path.to_string_lossy().contains("component") {
                commands.push(
                    CommandBuilder::new("leptos-build-check", "cargo-leptos")
                        .label("Build & Check")
                        .description("Build and check component compilation")
                        .arg("build")
                        .category(CommandCategory::Build)
                        .priority(80)
                        .tag("components")
                        .tag("build")
                        .tag("leptos")
                        .cwd(context.workspace_root.clone())
                        .estimated_duration(20)
                        .build(),
                );
            }
        }

        commands
    }

    /// Deployment and production commands
    fn deployment_commands(&self, context: &ProjectContext) -> Vec<Command> {
        vec![
            CommandBuilder::new("leptos-new", "cargo-leptos")
                .label("New Leptos Project")
                .description("Create a new Leptos project with wizard")
                .arg("new")
                .category(CommandCategory::Generate)
                .priority(70)
                .tag("new")
                .tag("generate")
                .tag("leptos")
                .cwd(context.workspace_root.clone())
                .estimated_duration(30)
                .build(),
            CommandBuilder::new("leptos-clean", "cargo")
                .label("Clean Leptos Build")
                .description("Clean build artifacts and temp files")
                .arg("clean")
                .category(CommandCategory::Clean)
                .priority(60)
                .tag("clean")
                .tag("leptos")
                .cwd(context.workspace_root.clone())
                .estimated_duration(5)
                .build(),
        ]
    }
}

impl Default for LeptosProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BuildTarget, Dependency, ProjectType, TargetType, WorkspaceMember};
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn create_leptos_context() -> ProjectContext {
        ProjectContext {
            workspace_root: PathBuf::from("/test"),
            current_file: None,
            cursor_position: None,
            project_type: ProjectType::Leptos,
            dependencies: vec![Dependency {
                name: "leptos".to_string(),
                version: "0.6".to_string(),
                features: vec!["csr".to_string()],
                optional: false,
                dev_dependency: false,
            }],
            workspace_members: vec![WorkspaceMember {
                name: "leptos-app".to_string(),
                path: PathBuf::from("/test"),
                package_type: ProjectType::Leptos,
            }],
            build_targets: vec![BuildTarget {
                name: "main".to_string(),
                target_type: TargetType::Binary,
                path: PathBuf::from("/test/src/main.rs"),
            }],
            active_features: vec!["csr".to_string()],
            env_vars: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_leptos_provider_can_handle() {
        let provider = LeptosProvider::new();
        let context = create_leptos_context();

        assert!(provider.can_handle(&context));
        assert_eq!(provider.name(), "leptos");
        assert_eq!(provider.priority(), 90);
    }

    #[tokio::test]
    async fn test_leptos_commands_generation() {
        let provider = LeptosProvider::new();
        let context = create_leptos_context();

        let commands = provider.commands(&context).await.unwrap();

        assert!(!commands.is_empty());

        // Should have development commands
        assert!(commands.iter().any(|c| c.id == "leptos-watch"));
        assert!(commands.iter().any(|c| c.id == "leptos-serve"));

        // Should have build commands
        assert!(commands.iter().any(|c| c.id == "leptos-build"));
        assert!(commands.iter().any(|c| c.id == "leptos-build-release"));

        // Should have test commands
        assert!(commands.iter().any(|c| c.id == "leptos-test"));

        // Should have deployment/utility commands
        assert!(commands.iter().any(|c| c.id == "leptos-new"));
    }

    #[tokio::test]
    async fn test_leptos_command_priorities() {
        let provider = LeptosProvider::new();
        let context = create_leptos_context();

        let commands = provider.commands(&context).await.unwrap();

        // Development watch should have highest priority
        let watch_cmd = commands.iter().find(|c| c.id == "leptos-watch").unwrap();
        assert_eq!(watch_cmd.priority, 95);

        // All commands should have leptos tag
        assert!(
            commands
                .iter()
                .all(|c| c.tags.contains(&"leptos".to_string()))
        );
    }

    #[tokio::test]
    async fn test_leptos_non_leptos_project() {
        let provider = LeptosProvider::new();
        let mut context = create_leptos_context();
        context.project_type = ProjectType::Binary;
        context.dependencies.clear();

        assert!(!provider.can_handle(&context));
    }

    #[tokio::test]
    async fn test_leptos_dependency_detection() {
        let provider = LeptosProvider::new();
        let mut context = create_leptos_context();
        context.project_type = ProjectType::Binary; // Not explicitly Leptos
        // But has leptos dependency (already set in create_leptos_context)

        assert!(provider.can_handle(&context));
    }
}
