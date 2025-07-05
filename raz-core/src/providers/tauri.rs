//! Tauri framework provider for desktop app development commands
//!
//! Provides intelligent command suggestions for Tauri projects including
//! development server, building for different platforms, and app distribution.

use crate::{
    Command, CommandBuilder, CommandCategory, CommandProvider, ProjectContext, ProjectType,
    RazResult, SymbolKind,
};
use async_trait::async_trait;

/// Provider for Tauri desktop app framework commands
pub struct TauriProvider {
    priority: u8,
}

impl TauriProvider {
    pub fn new() -> Self {
        Self {
            priority: 88, // High priority for Tauri app projects
        }
    }
}

#[async_trait]
impl CommandProvider for TauriProvider {
    fn name(&self) -> &str {
        "tauri"
    }

    fn priority(&self) -> u8 {
        self.priority
    }

    fn can_handle(&self, context: &ProjectContext) -> bool {
        // Check if this is a Tauri project
        match &context.project_type {
            ProjectType::Tauri => true,
            ProjectType::Mixed(frameworks) => frameworks.contains(&ProjectType::Tauri),
            _ => {
                // Also check dependencies for tauri
                context.dependencies.iter().any(|dep|
                    dep.name == "tauri" ||
                    dep.name == "tauri-build" ||
                    dep.name.starts_with("tauri-")
                ) ||
                // Check for tauri.conf.json or src-tauri directory
                context.workspace_root.join("src-tauri").exists() ||
                context.workspace_root.join("tauri.conf.json").exists()
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

        // Distribution and signing commands
        commands.extend(self.distribution_commands(context));

        Ok(commands)
    }
}

impl TauriProvider {
    /// Development server and debugging commands
    fn development_commands(&self, _context: &ProjectContext) -> Vec<Command> {
        vec![
            CommandBuilder::new("tauri-dev", "cargo")
                .label("Tauri Dev Server")
                .description("Start Tauri development server with hot reload")
                .arg("tauri")
                .arg("dev")
                .category(CommandCategory::Run)
                .priority(95)
                .tag("dev")
                .tag("serve")
                .tag("tauri")
                .estimated_duration(8)
                .build(),
            CommandBuilder::new("tauri-dev-debug", "cargo")
                .label("Tauri Debug Mode")
                .description("Start development with debugging enabled")
                .arg("tauri")
                .arg("dev")
                .arg("--debug")
                .category(CommandCategory::Run)
                .priority(90)
                .tag("dev")
                .tag("debug")
                .tag("tauri")
                .estimated_duration(10)
                .build(),
            CommandBuilder::new("tauri-dev-release", "cargo")
                .label("Tauri Dev Release")
                .description("Run in development mode with release build")
                .arg("tauri")
                .arg("dev")
                .arg("--release")
                .category(CommandCategory::Run)
                .priority(85)
                .tag("dev")
                .tag("release")
                .tag("tauri")
                .estimated_duration(30)
                .build(),
            CommandBuilder::new("tauri-dev-no-watch", "cargo")
                .label("Tauri Dev (No Watch)")
                .description("Start development without file watching")
                .arg("tauri")
                .arg("dev")
                .arg("--no-watch")
                .category(CommandCategory::Run)
                .priority(80)
                .tag("dev")
                .tag("no-watch")
                .tag("tauri")
                .estimated_duration(6)
                .build(),
            CommandBuilder::new("tauri-info", "cargo")
                .label("Tauri System Info")
                .description("Show Tauri environment and system information")
                .arg("tauri")
                .arg("info")
                .category(CommandCategory::Generate)
                .priority(75)
                .tag("info")
                .tag("system")
                .tag("tauri")
                .estimated_duration(2)
                .build(),
            CommandBuilder::new("tauri-init", "cargo")
                .label("Initialize Tauri")
                .description("Add Tauri to existing project")
                .arg("tauri")
                .arg("init")
                .category(CommandCategory::Generate)
                .priority(70)
                .tag("init")
                .tag("setup")
                .tag("tauri")
                .estimated_duration(5)
                .build(),
        ]
    }

    /// Build commands for different platforms and targets
    fn build_commands(&self, context: &ProjectContext) -> Vec<Command> {
        let mut commands = vec![
            CommandBuilder::new("tauri-build", "cargo")
                .label("Build Tauri App")
                .description("Build the Tauri application for current platform")
                .arg("tauri")
                .arg("build")
                .category(CommandCategory::Build)
                .priority(85)
                .tag("build")
                .tag("tauri")
                .estimated_duration(60)
                .build(),
            CommandBuilder::new("tauri-build-debug", "cargo")
                .label("Build Debug")
                .description("Build debug version of the app")
                .arg("tauri")
                .arg("build")
                .arg("--debug")
                .category(CommandCategory::Build)
                .priority(80)
                .tag("build")
                .tag("debug")
                .tag("tauri")
                .estimated_duration(45)
                .build(),
        ];

        // Add platform-specific builds based on features or environment
        let is_cross_platform = context.dependencies.iter().any(|d| {
            d.features
                .iter()
                .any(|f| f.contains("updater") || f.contains("api-all"))
        });

        if is_cross_platform {
            commands.extend(vec![
                CommandBuilder::new("tauri-build-windows", "cargo")
                    .label("Build for Windows")
                    .description("Cross-compile for Windows")
                    .arg("tauri")
                    .arg("build")
                    .arg("--target")
                    .arg("x86_64-pc-windows-msvc")
                    .category(CommandCategory::Build)
                    .priority(75)
                    .tag("build")
                    .tag("windows")
                    .tag("cross")
                    .tag("tauri")
                    .estimated_duration(90)
                    .build(),
                CommandBuilder::new("tauri-build-macos", "cargo")
                    .label("Build for macOS")
                    .description("Cross-compile for macOS")
                    .arg("tauri")
                    .arg("build")
                    .arg("--target")
                    .arg("x86_64-apple-darwin")
                    .category(CommandCategory::Build)
                    .priority(75)
                    .tag("build")
                    .tag("macos")
                    .tag("cross")
                    .tag("tauri")
                    .estimated_duration(100)
                    .build(),
                CommandBuilder::new("tauri-build-linux", "cargo")
                    .label("Build for Linux")
                    .description("Cross-compile for Linux")
                    .arg("tauri")
                    .arg("build")
                    .arg("--target")
                    .arg("x86_64-unknown-linux-gnu")
                    .category(CommandCategory::Build)
                    .priority(75)
                    .tag("build")
                    .tag("linux")
                    .tag("cross")
                    .tag("tauri")
                    .estimated_duration(85)
                    .build(),
            ]);
        }

        commands
    }

    /// Testing commands
    fn test_commands(&self, _context: &ProjectContext) -> Vec<Command> {
        vec![
            CommandBuilder::new("tauri-test", "cargo")
                .label("Run Tauri Tests")
                .description("Run unit and integration tests")
                .arg("test")
                .category(CommandCategory::Test)
                .priority(75)
                .tag("test")
                .tag("tauri")
                .estimated_duration(25)
                .build(),
            CommandBuilder::new("tauri-test-core", "cargo")
                .label("Test Core Logic")
                .description("Test Rust core logic without UI")
                .arg("test")
                .arg("--package")
                .arg("src-tauri")
                .category(CommandCategory::Test)
                .priority(80)
                .tag("test")
                .tag("core")
                .tag("tauri")
                .estimated_duration(15)
                .build(),
            CommandBuilder::new("tauri-test-webdriver", "cargo")
                .label("WebDriver Tests")
                .description("Run end-to-end tests with WebDriver")
                .arg("tauri")
                .arg("build")
                .arg("--debug")
                .args(vec![
                    "&&".to_string(),
                    "cargo".to_string(),
                    "test".to_string(),
                    "--test".to_string(),
                    "webdriver".to_string(),
                ])
                .category(CommandCategory::Test)
                .priority(70)
                .tag("test")
                .tag("e2e")
                .tag("webdriver")
                .tag("tauri")
                .estimated_duration(60)
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
                        // Command-related functions
                        if symbol.name.starts_with("cmd_")
                            || symbol.name.contains("command")
                            || symbol.modifiers.contains(&"tauri::command".to_string())
                        {
                            commands.push(
                                CommandBuilder::new("tauri-test-command", "cargo")
                                    .label("Test Tauri Command")
                                    .description(format!("Test command function: {}", symbol.name))
                                    .arg("test")
                                    .arg(format!("test_{}", symbol.name))
                                    .category(CommandCategory::Test)
                                    .priority(85)
                                    .tag("command")
                                    .tag("test")
                                    .tag("tauri")
                                    .estimated_duration(8)
                                    .build(),
                            );
                        }

                        // Event handler functions
                        if symbol.name.contains("event") || symbol.name.contains("handler") {
                            commands.push(
                                CommandBuilder::new("tauri-dev-events", "cargo")
                                    .label("Debug Event Handlers")
                                    .description(format!(
                                        "Run with event debugging: {}",
                                        symbol.name
                                    ))
                                    .arg("tauri")
                                    .arg("dev")
                                    .arg("--debug")
                                    .category(CommandCategory::Run)
                                    .priority(80)
                                    .tag("events")
                                    .tag("debug")
                                    .tag("tauri")
                                    .estimated_duration(12)
                                    .build(),
                            );
                        }

                        // Test function commands
                        if symbol.name.starts_with("test_")
                            || symbol.modifiers.contains(&"test".to_string())
                        {
                            commands.push(
                                CommandBuilder::new("tauri-test-current", "cargo")
                                    .label("Test Current Function")
                                    .description(format!("Run test: {}", symbol.name))
                                    .arg("test")
                                    .arg(&symbol.name)
                                    .arg("--")
                                    .arg("--nocapture")
                                    .category(CommandCategory::Test)
                                    .priority(90)
                                    .tag("test")
                                    .tag("current")
                                    .tag("tauri")
                                    .estimated_duration(5)
                                    .build(),
                            );
                        }
                    }
                    SymbolKind::Struct => {
                        // State management structs
                        if symbol.name.ends_with("State") || symbol.name.contains("Config") {
                            commands.push(
                                CommandBuilder::new("tauri-check-state", "cargo")
                                    .label("Check State Management")
                                    .description(format!("Validate state struct: {}", symbol.name))
                                    .arg("check")
                                    .category(CommandCategory::Lint)
                                    .priority(75)
                                    .tag("state")
                                    .tag("check")
                                    .tag("tauri")
                                    .estimated_duration(10)
                                    .build(),
                            );
                        }
                    }
                    _ => {}
                }
            }

            // File-based commands
            if file_context.path.to_string_lossy().contains("main.rs")
                && file_context.path.to_string_lossy().contains("src-tauri")
            {
                commands.push(
                    CommandBuilder::new("tauri-check-main", "cargo")
                        .label("Check Main App")
                        .description("Check main Tauri application logic")
                        .arg("check")
                        .arg("--package")
                        .arg("src-tauri")
                        .category(CommandCategory::Lint)
                        .priority(85)
                        .tag("main")
                        .tag("check")
                        .tag("tauri")
                        .estimated_duration(8)
                        .build(),
                );
            }

            if file_context.path.to_string_lossy().contains("command") {
                commands.push(
                    CommandBuilder::new("tauri-test-commands", "cargo")
                        .label("Test All Commands")
                        .description("Test all Tauri command handlers")
                        .arg("test")
                        .arg("commands")
                        .category(CommandCategory::Test)
                        .priority(80)
                        .tag("commands")
                        .tag("test")
                        .tag("tauri")
                        .estimated_duration(20)
                        .build(),
                );
            }
        }

        commands
    }

    /// Distribution and signing commands
    fn distribution_commands(&self, context: &ProjectContext) -> Vec<Command> {
        let mut commands = vec![
            CommandBuilder::new("tauri-bundle", "cargo")
                .label("Create App Bundle")
                .description("Create platform-specific app bundle")
                .arg("tauri")
                .arg("build")
                .arg("--bundles")
                .arg("all")
                .category(CommandCategory::Deploy)
                .priority(75)
                .tag("bundle")
                .tag("deploy")
                .tag("tauri")
                .estimated_duration(90)
                .build(),
            CommandBuilder::new("tauri-icon", "cargo")
                .label("Generate App Icons")
                .description("Generate app icons from source image")
                .arg("tauri")
                .arg("icon")
                .category(CommandCategory::Generate)
                .priority(65)
                .tag("icon")
                .tag("generate")
                .tag("tauri")
                .estimated_duration(5)
                .build(),
            CommandBuilder::new("tauri-add", "cargo")
                .label("Add Tauri Plugin")
                .description("Add a Tauri plugin to the project")
                .arg("tauri")
                .arg("add")
                .category(CommandCategory::Generate)
                .priority(60)
                .tag("plugin")
                .tag("add")
                .tag("tauri")
                .estimated_duration(10)
                .build(),
            CommandBuilder::new("tauri-migrate", "cargo")
                .label("Migrate to Tauri v2")
                .description("Migrate project from Tauri v1 to v2")
                .arg("tauri")
                .arg("migrate")
                .category(CommandCategory::Generate)
                .priority(55)
                .tag("migrate")
                .tag("upgrade")
                .tag("tauri")
                .estimated_duration(20)
                .build(),
        ];

        // Add mobile development commands if mobile features are detected
        let has_mobile = context.dependencies.iter().any(|d| {
            d.features
                .iter()
                .any(|f| f.contains("mobile") || f.contains("android") || f.contains("ios"))
        });

        if has_mobile {
            commands.extend(vec![
                CommandBuilder::new("tauri-android-dev", "cargo")
                    .label("Android Dev")
                    .description("Run Tauri app on Android device/emulator")
                    .arg("tauri")
                    .arg("android")
                    .arg("dev")
                    .category(CommandCategory::Run)
                    .priority(70)
                    .tag("mobile")
                    .tag("android")
                    .tag("dev")
                    .tag("tauri")
                    .estimated_duration(60)
                    .build(),
                CommandBuilder::new("tauri-ios-dev", "cargo")
                    .label("iOS Dev")
                    .description("Run Tauri app on iOS device/simulator")
                    .arg("tauri")
                    .arg("ios")
                    .arg("dev")
                    .category(CommandCategory::Run)
                    .priority(70)
                    .tag("mobile")
                    .tag("ios")
                    .tag("dev")
                    .tag("tauri")
                    .estimated_duration(60)
                    .build(),
            ]);
        }

        // Add signing commands if we detect signing configuration
        if context
            .dependencies
            .iter()
            .any(|d| d.features.iter().any(|f| f.contains("updater")))
        {
            commands.extend(vec![
                CommandBuilder::new("tauri-sign", "cargo")
                    .label("Sign Application")
                    .description("Sign the application for distribution")
                    .arg("tauri")
                    .arg("signer")
                    .arg("sign")
                    .category(CommandCategory::Deploy)
                    .priority(70)
                    .tag("sign")
                    .tag("security")
                    .tag("deploy")
                    .tag("tauri")
                    .estimated_duration(15)
                    .build(),
                CommandBuilder::new("tauri-updater", "cargo")
                    .label("Generate Update")
                    .description("Generate update package for auto-updater")
                    .arg("tauri")
                    .arg("build")
                    .arg("--config")
                    .arg("updater.active=true")
                    .category(CommandCategory::Deploy)
                    .priority(70)
                    .tag("updater")
                    .tag("deploy")
                    .tag("tauri")
                    .estimated_duration(75)
                    .build(),
            ]);
        }

        commands
    }
}

impl Default for TauriProvider {
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

    fn create_tauri_context() -> ProjectContext {
        ProjectContext {
            workspace_root: PathBuf::from("/test"),
            current_file: None,
            cursor_position: None,
            project_type: ProjectType::Tauri,
            dependencies: vec![Dependency {
                name: "tauri".to_string(),
                version: "1.5".to_string(),
                features: vec!["api-all".to_string()],
                optional: false,
                dev_dependency: false,
            }],
            workspace_members: vec![WorkspaceMember {
                name: "tauri-app".to_string(),
                path: PathBuf::from("/test"),
                package_type: ProjectType::Tauri,
            }],
            build_targets: vec![BuildTarget {
                name: "main".to_string(),
                target_type: TargetType::Binary,
                path: PathBuf::from("/test/src-tauri/src/main.rs"),
            }],
            active_features: vec!["api-all".to_string()],
            env_vars: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_tauri_provider_can_handle() {
        let provider = TauriProvider::new();
        let context = create_tauri_context();

        assert!(provider.can_handle(&context));
        assert_eq!(provider.name(), "tauri");
        assert_eq!(provider.priority(), 88);
    }

    #[tokio::test]
    async fn test_tauri_commands_generation() {
        let provider = TauriProvider::new();
        let context = create_tauri_context();

        let commands = provider.commands(&context).await.unwrap();

        assert!(!commands.is_empty());

        // Should have development commands
        assert!(commands.iter().any(|c| c.id == "tauri-dev"));
        assert!(commands.iter().any(|c| c.id == "tauri-dev-debug"));

        // Should have build commands
        assert!(commands.iter().any(|c| c.id == "tauri-build"));
        assert!(commands.iter().any(|c| c.id == "tauri-build-debug"));

        // Should have test commands
        assert!(commands.iter().any(|c| c.id == "tauri-test"));
        assert!(commands.iter().any(|c| c.id == "tauri-test-core"));

        // Should have distribution commands
        assert!(commands.iter().any(|c| c.id == "tauri-bundle"));
        assert!(commands.iter().any(|c| c.id == "tauri-icon"));
    }

    #[tokio::test]
    async fn test_tauri_cross_platform_builds() {
        let provider = TauriProvider::new();
        let mut context = create_tauri_context();

        // Ensure cross-platform features are detected
        context.dependencies[0].features.push("updater".to_string());

        let commands = provider.commands(&context).await.unwrap();

        // Should have cross-platform build commands
        assert!(commands.iter().any(|c| c.id == "tauri-build-windows"));
        assert!(commands.iter().any(|c| c.id == "tauri-build-macos"));
        assert!(commands.iter().any(|c| c.id == "tauri-build-linux"));
    }

    #[tokio::test]
    async fn test_tauri_updater_commands() {
        let provider = TauriProvider::new();
        let mut context = create_tauri_context();

        // Add updater feature
        context.dependencies[0].features.push("updater".to_string());

        let commands = provider.commands(&context).await.unwrap();

        // Should have updater and signing commands
        assert!(commands.iter().any(|c| c.id == "tauri-sign"));
        assert!(commands.iter().any(|c| c.id == "tauri-updater"));
    }

    #[tokio::test]
    async fn test_tauri_command_priorities() {
        let provider = TauriProvider::new();
        let context = create_tauri_context();

        let commands = provider.commands(&context).await.unwrap();

        // Dev command should have highest priority
        let dev_cmd = commands.iter().find(|c| c.id == "tauri-dev").unwrap();
        assert_eq!(dev_cmd.priority, 95);

        // All commands should have tauri tag
        assert!(
            commands
                .iter()
                .all(|c| c.tags.contains(&"tauri".to_string()))
        );
    }

    #[tokio::test]
    async fn test_tauri_dependency_detection() {
        let provider = TauriProvider::new();
        let mut context = create_tauri_context();
        context.project_type = ProjectType::Binary; // Not explicitly App
        // But has tauri dependency (set in create_tauri_context)

        assert!(provider.can_handle(&context));
    }
}
