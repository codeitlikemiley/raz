//! Bevy game engine provider for game development commands
//!
//! Provides intelligent command suggestions for Bevy game projects including
//! development server, building for different platforms, and testing commands.

use crate::{
    Command, CommandBuilder, CommandCategory, CommandProvider, ProjectContext, ProjectType,
    RazResult, SymbolKind,
};
use async_trait::async_trait;

/// Provider for Bevy game engine commands
pub struct BevyProvider {
    priority: u8,
}

impl BevyProvider {
    pub fn new() -> Self {
        Self {
            priority: 85, // High priority for Bevy game projects
        }
    }
}

#[async_trait]
impl CommandProvider for BevyProvider {
    fn name(&self) -> &str {
        "bevy"
    }

    fn priority(&self) -> u8 {
        self.priority
    }

    fn can_handle(&self, context: &ProjectContext) -> bool {
        // Check if this is a Bevy project
        match &context.project_type {
            ProjectType::Bevy => true,
            ProjectType::Mixed(frameworks) => frameworks.contains(&ProjectType::Bevy),
            _ => {
                // Also check dependencies for bevy
                context
                    .dependencies
                    .iter()
                    .any(|dep| dep.name == "bevy" || dep.name.starts_with("bevy_"))
            }
        }
    }

    async fn commands(&self, context: &ProjectContext) -> RazResult<Vec<Command>> {
        let mut commands = Vec::new();

        // Development commands
        commands.extend(self.development_commands(context));

        // Build commands for different platforms
        commands.extend(self.build_commands(context));

        // Testing commands
        commands.extend(self.test_commands(context));

        // Context-aware commands
        commands.extend(self.context_aware_commands(context));

        // Asset and optimization commands
        commands.extend(self.asset_commands(context));

        Ok(commands)
    }
}

impl BevyProvider {
    /// Development and running commands
    fn development_commands(&self, _context: &ProjectContext) -> Vec<Command> {
        vec![
            CommandBuilder::new("bevy-run", "cargo")
                .label("Run Bevy Game")
                .description("Run the Bevy game in development mode")
                .arg("run")
                .category(CommandCategory::Run)
                .priority(95)
                .tag("dev")
                .tag("run")
                .tag("bevy")
                .estimated_duration(3)
                .build(),
            CommandBuilder::new("bevy-run-release", "cargo")
                .label("Run Release Build")
                .description("Run optimized release build of the game")
                .arg("run")
                .arg("--release")
                .category(CommandCategory::Run)
                .priority(85)
                .tag("run")
                .tag("release")
                .tag("bevy")
                .estimated_duration(5)
                .build(),
            CommandBuilder::new("bevy-run-features", "cargo")
                .label("Run with Dynamic Features")
                .description("Run with dynamic linking for faster iteration")
                .arg("run")
                .arg("--features")
                .arg("bevy/dynamic_linking")
                .category(CommandCategory::Run)
                .priority(88)
                .tag("dev")
                .tag("dynamic")
                .tag("bevy")
                .estimated_duration(4)
                .build(),
            CommandBuilder::new("bevy-check", "cargo")
                .label("Check Bevy Code")
                .description("Fast compile check without building")
                .arg("check")
                .category(CommandCategory::Lint)
                .priority(90)
                .tag("check")
                .tag("fast")
                .tag("bevy")
                .estimated_duration(8)
                .build(),
        ]
    }

    /// Build commands for different platforms and targets
    fn build_commands(&self, context: &ProjectContext) -> Vec<Command> {
        let mut commands = vec![
            CommandBuilder::new("bevy-build", "cargo")
                .label("Build Bevy Game")
                .description("Build the game for the current platform")
                .arg("build")
                .category(CommandCategory::Build)
                .priority(80)
                .tag("build")
                .tag("bevy")
                .estimated_duration(45)
                .build(),
            CommandBuilder::new("bevy-build-release", "cargo")
                .label("Build Release")
                .description("Build optimized release version")
                .arg("build")
                .arg("--release")
                .category(CommandCategory::Build)
                .priority(85)
                .tag("build")
                .tag("release")
                .tag("bevy")
                .estimated_duration(90)
                .build(),
        ];

        // Add platform-specific builds if we detect cross-compilation setup
        if context.dependencies.iter().any(|d| d.name.contains("wasm")) {
            commands.push(
                CommandBuilder::new("bevy-build-wasm", "cargo")
                    .label("Build for WASM")
                    .description("Build Bevy game for web (WebAssembly)")
                    .arg("build")
                    .arg("--target")
                    .arg("wasm32-unknown-unknown")
                    .arg("--release")
                    .category(CommandCategory::Build)
                    .priority(75)
                    .tag("build")
                    .tag("wasm")
                    .tag("web")
                    .tag("bevy")
                    .estimated_duration(120)
                    .build(),
            );
        }

        // Windows cross-compilation
        commands.push(
            CommandBuilder::new("bevy-build-windows", "cargo")
                .label("Build for Windows")
                .description("Cross-compile for Windows")
                .arg("build")
                .arg("--target")
                .arg("x86_64-pc-windows-gnu")
                .arg("--release")
                .category(CommandCategory::Build)
                .priority(70)
                .tag("build")
                .tag("windows")
                .tag("cross")
                .tag("bevy")
                .estimated_duration(100)
                .build(),
        );

        commands
    }

    /// Testing commands
    fn test_commands(&self, _context: &ProjectContext) -> Vec<Command> {
        vec![
            CommandBuilder::new("bevy-test", "cargo")
                .label("Run Bevy Tests")
                .description("Run unit and integration tests")
                .arg("test")
                .category(CommandCategory::Test)
                .priority(75)
                .tag("test")
                .tag("bevy")
                .estimated_duration(20)
                .build(),
            CommandBuilder::new("bevy-test-headless", "cargo")
                .label("Headless Tests")
                .description("Run tests without rendering (headless)")
                .arg("test")
                .arg("--features")
                .arg("bevy/headless")
                .category(CommandCategory::Test)
                .priority(80)
                .tag("test")
                .tag("headless")
                .tag("bevy")
                .estimated_duration(15)
                .build(),
            CommandBuilder::new("bevy-test-doc", "cargo")
                .label("Test Documentation")
                .description("Test code examples in documentation")
                .arg("test")
                .arg("--doc")
                .category(CommandCategory::Test)
                .priority(65)
                .tag("test")
                .tag("doc")
                .tag("bevy")
                .estimated_duration(25)
                .build(),
        ]
    }

    /// Context-aware commands based on cursor position
    fn context_aware_commands(&self, context: &ProjectContext) -> Vec<Command> {
        let mut commands = Vec::new();

        if let Some(file_context) = &context.current_file {
            if let Some(symbol) = &file_context.cursor_symbol {
                match symbol.kind {
                    SymbolKind::Function => {
                        // System-related commands for functions that look like Bevy systems
                        if symbol.name.ends_with("_system")
                            || symbol.name.contains("update")
                            || symbol.name.contains("spawn")
                        {
                            commands.push(
                                CommandBuilder::new("bevy-run-system", "cargo")
                                    .label("Run with System Focus")
                                    .description(format!(
                                        "Run game focusing on system: {}",
                                        symbol.name
                                    ))
                                    .arg("run")
                                    .arg("--features")
                                    .arg("bevy/trace")
                                    .category(CommandCategory::Run)
                                    .priority(85)
                                    .tag("system")
                                    .tag("debug")
                                    .tag("bevy")
                                    .estimated_duration(8)
                                    .build(),
                            );
                        }

                        // Test function commands
                        if symbol.name.starts_with("test_")
                            || symbol.modifiers.contains(&"test".to_string())
                        {
                            commands.push(
                                CommandBuilder::new("bevy-test-current", "cargo")
                                    .label("Test Current System")
                                    .description(format!("Run test: {}", symbol.name))
                                    .arg("test")
                                    .arg(&symbol.name)
                                    .arg("--")
                                    .arg("--nocapture")
                                    .category(CommandCategory::Test)
                                    .priority(90)
                                    .tag("test")
                                    .tag("current")
                                    .tag("bevy")
                                    .estimated_duration(5)
                                    .build(),
                            );
                        }
                    }
                    SymbolKind::Struct => {
                        // Component-related commands
                        if symbol.name.ends_with("Component")
                            || symbol.modifiers.contains(&"Component".to_string())
                        {
                            commands.push(
                                CommandBuilder::new("bevy-inspect-component", "cargo")
                                    .label("Inspect Component")
                                    .description(format!(
                                        "Run with component inspector: {}",
                                        symbol.name
                                    ))
                                    .arg("run")
                                    .arg("--features")
                                    .arg("bevy-inspector-egui/default")
                                    .category(CommandCategory::Run)
                                    .priority(80)
                                    .tag("component")
                                    .tag("inspector")
                                    .tag("bevy")
                                    .estimated_duration(10)
                                    .build(),
                            );
                        }

                        // Resource-related commands
                        if symbol.name.ends_with("Resource") {
                            commands.push(
                                CommandBuilder::new("bevy-debug-resource", "cargo")
                                    .label("Debug Resource")
                                    .description(format!(
                                        "Run with resource debugging: {}",
                                        symbol.name
                                    ))
                                    .arg("run")
                                    .arg("--features")
                                    .arg("bevy/trace")
                                    .category(CommandCategory::Run)
                                    .priority(75)
                                    .tag("resource")
                                    .tag("debug")
                                    .tag("bevy")
                                    .estimated_duration(8)
                                    .build(),
                            );
                        }
                    }
                    _ => {}
                }
            }

            // File-based commands
            if file_context.path.to_string_lossy().contains("system") {
                commands.push(
                    CommandBuilder::new("bevy-check-systems", "cargo")
                        .label("Check Systems")
                        .description("Check all game systems for compilation errors")
                        .arg("check")
                        .arg("--features")
                        .arg("bevy/trace")
                        .category(CommandCategory::Lint)
                        .priority(80)
                        .tag("systems")
                        .tag("check")
                        .tag("bevy")
                        .estimated_duration(12)
                        .build(),
                );
            }

            if file_context.path.to_string_lossy().contains("asset") {
                commands.push(
                    CommandBuilder::new("bevy-check-assets", "cargo")
                        .label("Validate Assets")
                        .description("Check asset loading and validation")
                        .arg("run")
                        .arg("--features")
                        .arg("bevy/filesystem_watcher")
                        .category(CommandCategory::Run)
                        .priority(75)
                        .tag("assets")
                        .tag("validate")
                        .tag("bevy")
                        .estimated_duration(15)
                        .build(),
                );
            }
        }

        commands
    }

    /// Asset management and optimization commands
    fn asset_commands(&self, _context: &ProjectContext) -> Vec<Command> {
        vec![
            CommandBuilder::new("bevy-assets-watch", "cargo")
                .label("Watch Assets")
                .description("Run with asset hot-reloading enabled")
                .arg("run")
                .arg("--features")
                .arg("bevy/file_watcher")
                .category(CommandCategory::Run)
                .priority(85)
                .tag("assets")
                .tag("watch")
                .tag("bevy")
                .estimated_duration(5)
                .build(),
            CommandBuilder::new("bevy-optimize-assets", "cargo")
                .label("Optimize Assets")
                .description("Build with optimized asset processing")
                .arg("build")
                .arg("--release")
                .arg("--features")
                .arg("bevy/serialize")
                .category(CommandCategory::Build)
                .priority(70)
                .tag("assets")
                .tag("optimize")
                .tag("bevy")
                .estimated_duration(60)
                .build(),
            CommandBuilder::new("bevy-bundle-assets", "cargo")
                .label("Bundle Assets")
                .description("Create asset bundle for distribution")
                .arg("build")
                .arg("--release")
                .arg("--features")
                .arg("bevy/embedded_watcher")
                .category(CommandCategory::Deploy)
                .priority(65)
                .tag("bundle")
                .tag("assets")
                .tag("deploy")
                .tag("bevy")
                .estimated_duration(90)
                .build(),
        ]
    }
}

impl Default for BevyProvider {
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

    fn create_bevy_context() -> ProjectContext {
        ProjectContext {
            workspace_root: PathBuf::from("/test"),
            current_file: None,
            cursor_position: None,
            project_type: ProjectType::Bevy,
            dependencies: vec![Dependency {
                name: "bevy".to_string(),
                version: "0.12".to_string(),
                features: vec!["dynamic_linking".to_string()],
                optional: false,
                dev_dependency: false,
            }],
            workspace_members: vec![WorkspaceMember {
                name: "bevy-game".to_string(),
                path: PathBuf::from("/test"),
                package_type: ProjectType::Bevy,
            }],
            build_targets: vec![BuildTarget {
                name: "main".to_string(),
                target_type: TargetType::Binary,
                path: PathBuf::from("/test/src/main.rs"),
            }],
            active_features: vec!["dynamic_linking".to_string()],
            env_vars: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_bevy_provider_can_handle() {
        let provider = BevyProvider::new();
        let context = create_bevy_context();

        assert!(provider.can_handle(&context));
        assert_eq!(provider.name(), "bevy");
        assert_eq!(provider.priority(), 85);
    }

    #[tokio::test]
    async fn test_bevy_commands_generation() {
        let provider = BevyProvider::new();
        let context = create_bevy_context();

        let commands = provider.commands(&context).await.unwrap();

        assert!(!commands.is_empty());

        // Should have development commands
        assert!(commands.iter().any(|c| c.id == "bevy-run"));
        assert!(commands.iter().any(|c| c.id == "bevy-run-features"));

        // Should have build commands
        assert!(commands.iter().any(|c| c.id == "bevy-build"));
        assert!(commands.iter().any(|c| c.id == "bevy-build-release"));

        // Should have test commands
        assert!(commands.iter().any(|c| c.id == "bevy-test"));
        assert!(commands.iter().any(|c| c.id == "bevy-test-headless"));

        // Should have asset commands
        assert!(commands.iter().any(|c| c.id == "bevy-assets-watch"));
    }

    #[tokio::test]
    async fn test_bevy_wasm_commands() {
        let provider = BevyProvider::new();
        let mut context = create_bevy_context();

        // Add wasm dependency
        context.dependencies.push(Dependency {
            name: "wasm-bindgen".to_string(),
            version: "0.2".to_string(),
            features: vec![],
            optional: false,
            dev_dependency: false,
        });

        let commands = provider.commands(&context).await.unwrap();

        // Should have WASM build command
        assert!(commands.iter().any(|c| c.id == "bevy-build-wasm"));
    }

    #[tokio::test]
    async fn test_bevy_command_priorities() {
        let provider = BevyProvider::new();
        let context = create_bevy_context();

        let commands = provider.commands(&context).await.unwrap();

        // Run command should have highest priority
        let run_cmd = commands.iter().find(|c| c.id == "bevy-run").unwrap();
        assert_eq!(run_cmd.priority, 95);

        // All commands should have bevy tag
        assert!(
            commands
                .iter()
                .all(|c| c.tags.contains(&"bevy".to_string()))
        );
    }

    #[tokio::test]
    async fn test_bevy_dependency_detection() {
        let provider = BevyProvider::new();
        let mut context = create_bevy_context();
        context.project_type = ProjectType::Binary; // Not explicitly Game
        // But has bevy dependency (set in create_bevy_context)

        assert!(provider.can_handle(&context));
    }
}
