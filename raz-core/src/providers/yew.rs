//! Yew framework provider for web development commands
//!
//! Provides intelligent command suggestions for Yew projects including
//! development server, building, testing, and deployment commands.

use crate::{
    Command, CommandBuilder, CommandCategory, CommandProvider, ProjectContext, ProjectType,
    RazResult, SymbolKind,
};
use async_trait::async_trait;

/// Provider for Yew web framework commands
pub struct YewProvider {
    priority: u8,
}

impl YewProvider {
    pub fn new() -> Self {
        Self {
            priority: 90, // High priority for Yew projects
        }
    }
}

#[async_trait]
impl CommandProvider for YewProvider {
    fn name(&self) -> &str {
        "yew"
    }

    fn priority(&self) -> u8 {
        self.priority
    }

    fn can_handle(&self, context: &ProjectContext) -> bool {
        // Check if this is a Yew project
        match &context.project_type {
            ProjectType::Yew => true,
            ProjectType::Mixed(frameworks) => frameworks.contains(&ProjectType::Yew),
            _ => {
                // Also check dependencies for yew
                context.dependencies.iter().any(|dep| {
                    dep.name == "yew" || dep.name == "yew-router" || dep.name == "yew-hooks"
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

        // Deployment and utility commands
        commands.extend(self.deployment_commands(context));

        Ok(commands)
    }
}

impl YewProvider {
    /// Development server and watch commands
    fn development_commands(&self, context: &ProjectContext) -> Vec<Command> {
        vec![
            CommandBuilder::new("yew-serve", "trunk")
                .label("Yew Dev Server")
                .description("Development server with auto-reload (recommended for development)")
                .arg("serve")
                .category(CommandCategory::Run)
                .priority(95)
                .tag("dev")
                .tag("serve")
                .tag("yew")
                .cwd(context.workspace_root.clone())
                .estimated_duration(5)
                .build(),
            CommandBuilder::new("yew-serve-open", "trunk")
                .label("Yew Dev Server + Open")
                .description("Development server with auto-reload and open browser")
                .arg("serve")
                .arg("--open")
                .category(CommandCategory::Run)
                .priority(92)
                .tag("dev")
                .tag("serve")
                .tag("open")
                .tag("yew")
                .cwd(context.workspace_root.clone())
                .estimated_duration(5)
                .build(),
            CommandBuilder::new("yew-serve-port", "trunk")
                .label("Yew Dev Server (Port 8080)")
                .description("Development server on custom port 8080")
                .arg("serve")
                .arg("--port")
                .arg("8080")
                .category(CommandCategory::Run)
                .priority(85)
                .tag("dev")
                .tag("serve")
                .tag("port")
                .tag("yew")
                .cwd(context.workspace_root.clone())
                .estimated_duration(5)
                .build(),
            CommandBuilder::new("yew-watch", "trunk")
                .label("Yew Watch Build")
                .description("Watch for changes and rebuild (no server)")
                .arg("watch")
                .category(CommandCategory::Build)
                .priority(80)
                .tag("watch")
                .tag("build")
                .tag("yew")
                .cwd(context.workspace_root.clone())
                .estimated_duration(5)
                .build(),
        ]
    }

    /// Build commands for different targets
    fn build_commands(&self, context: &ProjectContext) -> Vec<Command> {
        vec![
            CommandBuilder::new("yew-build", "trunk")
                .label("Yew Build")
                .description("Build Yew app for development")
                .arg("build")
                .category(CommandCategory::Build)
                .priority(85)
                .tag("build")
                .tag("yew")
                .cwd(context.workspace_root.clone())
                .estimated_duration(30)
                .build(),
            CommandBuilder::new("yew-build-release", "trunk")
                .label("Yew Release Build")
                .description("Build optimized release version")
                .arg("build")
                .arg("--release")
                .category(CommandCategory::Build)
                .priority(82)
                .tag("build")
                .tag("release")
                .tag("yew")
                .cwd(context.workspace_root.clone())
                .estimated_duration(60)
                .build(),
            CommandBuilder::new("yew-build-public-url", "trunk")
                .label("Yew Build with Public URL")
                .description("Build with custom public URL for deployment")
                .arg("build")
                .arg("--release")
                .arg("--public-url")
                .arg("/app/")
                .category(CommandCategory::Build)
                .priority(70)
                .tag("build")
                .tag("release")
                .tag("deploy")
                .tag("yew")
                .cwd(context.workspace_root.clone())
                .estimated_duration(65)
                .build(),
            CommandBuilder::new("yew-build-dist", "trunk")
                .label("Yew Build to Custom Dist")
                .description("Build to custom dist directory")
                .arg("build")
                .arg("--release")
                .arg("--dist")
                .arg("build")
                .category(CommandCategory::Build)
                .priority(65)
                .tag("build")
                .tag("release")
                .tag("custom")
                .tag("yew")
                .cwd(context.workspace_root.clone())
                .estimated_duration(65)
                .build(),
        ]
    }

    /// Testing commands
    fn test_commands(&self, context: &ProjectContext) -> Vec<Command> {
        vec![
            CommandBuilder::new("yew-test", "cargo")
                .label("Yew Tests")
                .description("Run Rust tests for Yew components")
                .arg("test")
                .category(CommandCategory::Test)
                .priority(75)
                .tag("test")
                .tag("yew")
                .cwd(context.workspace_root.clone())
                .estimated_duration(15)
                .build(),
            CommandBuilder::new("yew-test-wasm", "wasm-pack")
                .label("Yew WASM Tests")
                .description("Run tests in WASM environment")
                .arg("test")
                .arg("--headless")
                .arg("--chrome")
                .category(CommandCategory::Test)
                .priority(72)
                .tag("test")
                .tag("wasm")
                .tag("browser")
                .tag("yew")
                .cwd(context.workspace_root.clone())
                .estimated_duration(30)
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
                .tag("yew")
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
                                    .tag("yew")
                                    .cwd(context.workspace_root.clone())
                                    .estimated_duration(5)
                                    .build(),
                            );
                        }

                        // Component-related commands for functions that look like Yew components
                        if symbol.name.chars().next().is_some_and(|c| c.is_uppercase()) {
                            commands.push(
                                CommandBuilder::new("yew-dev-component", "trunk")
                                    .label("Dev with Component Focus")
                                    .description(format!(
                                        "Develop focusing on component: {}",
                                        symbol.name
                                    ))
                                    .arg("serve")
                                    .arg("--open")
                                    .category(CommandCategory::Run)
                                    .priority(88)
                                    .tag("component")
                                    .tag("dev")
                                    .tag("yew")
                                    .cwd(context.workspace_root.clone())
                                    .estimated_duration(8)
                                    .build(),
                            );
                        }
                    }
                    SymbolKind::Struct => {
                        // Struct-related commands (possibly component props or state)
                        commands.push(
                            CommandBuilder::new("yew-check-struct", "cargo")
                                .label("Check Struct Usage")
                                .description(format!("Check usage of struct: {}", symbol.name))
                                .arg("check")
                                .arg("--message-format=json")
                                .category(CommandCategory::Lint)
                                .priority(70)
                                .tag("check")
                                .tag("struct")
                                .tag("yew")
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
                    CommandBuilder::new("yew-build-check", "trunk")
                        .label("Build & Check")
                        .description("Build and check component compilation")
                        .arg("build")
                        .category(CommandCategory::Build)
                        .priority(80)
                        .tag("components")
                        .tag("build")
                        .tag("yew")
                        .cwd(context.workspace_root.clone())
                        .estimated_duration(20)
                        .build(),
                );
            }
        }

        commands
    }

    /// Deployment and utility commands
    fn deployment_commands(&self, context: &ProjectContext) -> Vec<Command> {
        vec![
            CommandBuilder::new("yew-clean", "trunk")
                .label("Clean Yew Build")
                .description("Clean trunk build artifacts")
                .arg("clean")
                .category(CommandCategory::Clean)
                .priority(60)
                .tag("clean")
                .tag("yew")
                .cwd(context.workspace_root.clone())
                .estimated_duration(2)
                .build(),
            CommandBuilder::new("cargo-clean", "cargo")
                .label("Clean Cargo Build")
                .description("Clean cargo build artifacts")
                .arg("clean")
                .category(CommandCategory::Clean)
                .priority(55)
                .tag("clean")
                .tag("cargo")
                .tag("yew")
                .cwd(context.workspace_root.clone())
                .estimated_duration(5)
                .build(),
            CommandBuilder::new("yew-config", "trunk")
                .label("Trunk Config Check")
                .description("Check Trunk.toml configuration")
                .arg("config")
                .arg("show")
                .category(CommandCategory::Generate)
                .priority(50)
                .tag("config")
                .tag("trunk")
                .tag("yew")
                .cwd(context.workspace_root.clone())
                .estimated_duration(2)
                .build(),
        ]
    }
}

impl Default for YewProvider {
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

    fn create_yew_context() -> ProjectContext {
        ProjectContext {
            workspace_root: PathBuf::from("/test"),
            current_file: None,
            cursor_position: None,
            project_type: ProjectType::Yew,
            dependencies: vec![Dependency {
                name: "yew".to_string(),
                version: "0.21".to_string(),
                features: vec!["web".to_string()],
                optional: false,
                dev_dependency: false,
            }],
            workspace_members: vec![WorkspaceMember {
                name: "yew-app".to_string(),
                path: PathBuf::from("/test"),
                package_type: ProjectType::Yew,
            }],
            build_targets: vec![BuildTarget {
                name: "main".to_string(),
                target_type: TargetType::Binary,
                path: PathBuf::from("/test/src/main.rs"),
            }],
            active_features: vec!["web".to_string()],
            env_vars: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_yew_provider_can_handle() {
        let provider = YewProvider::new();
        let context = create_yew_context();

        assert!(provider.can_handle(&context));
        assert_eq!(provider.name(), "yew");
        assert_eq!(provider.priority(), 90);
    }

    #[tokio::test]
    async fn test_yew_commands_generation() {
        let provider = YewProvider::new();
        let context = create_yew_context();

        let commands = provider.commands(&context).await.unwrap();

        assert!(!commands.is_empty());

        // Should have development commands
        assert!(commands.iter().any(|c| c.id == "yew-serve"));
        assert!(commands.iter().any(|c| c.id == "yew-serve-open"));

        // Should have build commands
        assert!(commands.iter().any(|c| c.id == "yew-build"));
        assert!(commands.iter().any(|c| c.id == "yew-build-release"));

        // Should have test commands
        assert!(commands.iter().any(|c| c.id == "yew-test"));

        // Should have utility commands
        assert!(commands.iter().any(|c| c.id == "yew-clean"));
    }

    #[tokio::test]
    async fn test_yew_command_priorities() {
        let provider = YewProvider::new();
        let context = create_yew_context();

        let commands = provider.commands(&context).await.unwrap();

        // Development serve should have highest priority
        let serve_cmd = commands.iter().find(|c| c.id == "yew-serve").unwrap();
        assert_eq!(serve_cmd.priority, 95);

        // All commands should have yew tag
        assert!(commands.iter().all(|c| c.tags.contains(&"yew".to_string())));
    }

    #[tokio::test]
    async fn test_yew_non_yew_project() {
        let provider = YewProvider::new();
        let mut context = create_yew_context();
        context.project_type = ProjectType::Binary;
        context.dependencies.clear();

        assert!(!provider.can_handle(&context));
    }

    #[tokio::test]
    async fn test_yew_dependency_detection() {
        let provider = YewProvider::new();
        let mut context = create_yew_context();
        context.project_type = ProjectType::Binary; // Not explicitly Yew
        // But has yew dependency (already set in create_yew_context)

        assert!(provider.can_handle(&context));
    }

    #[tokio::test]
    async fn test_yew_router_dependency_detection() {
        let provider = YewProvider::new();
        let mut context = create_yew_context();
        context.project_type = ProjectType::Binary;
        context.dependencies.clear();
        context.dependencies.push(Dependency {
            name: "yew-router".to_string(),
            version: "0.18".to_string(),
            features: vec![],
            optional: false,
            dev_dependency: false,
        });

        assert!(provider.can_handle(&context));
    }
}
