//! Dioxus framework provider for cross-platform app development
//!
//! Provides intelligent command suggestions for Dioxus projects including
//! desktop, web, mobile development and testing commands.

use crate::{
    Command, CommandBuilder, CommandCategory, CommandProvider, ProjectContext, ProjectType,
    RazResult, SymbolKind,
};
use async_trait::async_trait;

/// Provider for Dioxus cross-platform framework commands
pub struct DioxusProvider {
    priority: u8,
}

impl DioxusProvider {
    pub fn new() -> Self {
        Self {
            priority: 90, // High priority for Dioxus projects
        }
    }
}

#[async_trait]
impl CommandProvider for DioxusProvider {
    fn name(&self) -> &str {
        "dioxus"
    }

    fn priority(&self) -> u8 {
        self.priority
    }

    fn can_handle(&self, context: &ProjectContext) -> bool {
        // Check if this is a Dioxus project
        match &context.project_type {
            ProjectType::Dioxus => true,
            ProjectType::Mixed(frameworks) => frameworks.contains(&ProjectType::Dioxus),
            _ => {
                // Also check dependencies for dioxus
                context.dependencies.iter().any(|dep| {
                    dep.name == "dioxus"
                        || dep.name == "dioxus-web"
                        || dep.name == "dioxus-desktop"
                        || dep.name == "dioxus-mobile"
                })
            }
        }
    }

    async fn commands(&self, context: &ProjectContext) -> RazResult<Vec<Command>> {
        let mut commands = Vec::new();

        // Development commands
        commands.extend(self.development_commands(context));

        // Platform-specific build commands
        commands.extend(self.platform_commands(context));

        // Testing commands
        commands.extend(self.test_commands(context));

        // Context-aware commands
        commands.extend(self.context_aware_commands(context));

        // Bundle and deployment commands
        commands.extend(self.deployment_commands(context));

        Ok(commands)
    }
}

impl DioxusProvider {
    /// Detect the default platform for the Dioxus project
    fn detect_platform(&self, context: &ProjectContext) -> String {
        // 0. Check for environment variable override (highest priority)
        if let Ok(platform_override) = std::env::var("RAZ_PLATFORM_OVERRIDE") {
            return platform_override;
        }

        // 0.5. Check for saved command override in config
        // TODO: We need access to the config manager here to check for saved overrides
        // For now, the environment variable approach will work

        // 1. Check Dioxus.toml for default_platform
        if let Some(dioxus_config) = self.read_dioxus_config(context) {
            if let Some(platform) = dioxus_config.get("default_platform") {
                return platform.as_str().unwrap_or("desktop").to_string();
            }
        }

        // 2. Check Cargo.toml features to auto-detect platform
        let has_web = context
            .dependencies
            .iter()
            .any(|dep| dep.name == "dioxus-web" || dep.features.iter().any(|f| f == "web"));

        let has_desktop = context
            .dependencies
            .iter()
            .any(|dep| dep.name == "dioxus-desktop" || dep.features.iter().any(|f| f == "desktop"));

        let has_mobile = context
            .dependencies
            .iter()
            .any(|dep| dep.name == "dioxus-mobile" || dep.features.iter().any(|f| f == "mobile"));

        let has_fullstack = context.dependencies.iter().any(|dep| {
            dep.features
                .iter()
                .any(|f| f == "fullstack" || f == "server")
        });

        // Prioritize platforms based on dependencies
        if has_fullstack {
            "fullstack".to_string()
        } else if has_web {
            "web".to_string()
        } else if has_desktop {
            "desktop".to_string()
        } else if has_mobile {
            "mobile".to_string()
        } else {
            // Default fallback - desktop is most common for Dioxus users
            "desktop".to_string()
        }
    }

    /// Read Dioxus.toml configuration file
    fn read_dioxus_config(&self, context: &ProjectContext) -> Option<toml::Value> {
        let dioxus_toml_path = context.workspace_root.join("Dioxus.toml");
        if dioxus_toml_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&dioxus_toml_path) {
                if let Ok(config) = content.parse::<toml::Value>() {
                    return config.get("application").cloned();
                }
            }
        }
        None
    }

    /// Development server and watch commands
    fn development_commands(&self, context: &ProjectContext) -> Vec<Command> {
        let platform = self.detect_platform(context);

        vec![
            CommandBuilder::new("dx-serve", "dx")
                .label(format!("Dioxus Dev Server ({platform})"))
                .description(format!(
                    "Build, watch & serve with hot-reload for {platform} platform"
                ))
                .arg("serve")
                .arg("--platform")
                .arg(&platform)
                .category(CommandCategory::Run)
                .priority(95)
                .tag("dev")
                .tag("serve")
                .tag("hot-reload")
                .tag("dioxus")
                .tag(&platform)
                .estimated_duration(5)
                .build(),
            CommandBuilder::new("dx-serve-open", "dx")
                .label(format!("Dioxus Serve & Open ({platform})"))
                .description(format!(
                    "Start dev server and open browser for {platform} platform"
                ))
                .arg("serve")
                .arg("--platform")
                .arg(&platform)
                .arg("--open")
                .category(CommandCategory::Run)
                .priority(93)
                .tag("dev")
                .tag("serve")
                .tag("browser")
                .tag("dioxus")
                .tag(&platform)
                .estimated_duration(5)
                .build(),
            CommandBuilder::new("dx-serve-release", "dx")
                .label(format!("Dioxus Serve Release ({platform})"))
                .description(format!("Serve in release mode for {platform} platform"))
                .arg("serve")
                .arg("--platform")
                .arg(&platform)
                .arg("--release")
                .category(CommandCategory::Run)
                .priority(85)
                .tag("serve")
                .tag("release")
                .tag("dioxus")
                .tag(&platform)
                .estimated_duration(30)
                .build(),
            CommandBuilder::new("dx-run", "dx")
                .label(format!("Dioxus Run ({platform})"))
                .description(format!("Run without hot-reload for {platform} platform"))
                .arg("run")
                .arg("--platform")
                .arg(&platform)
                .category(CommandCategory::Run)
                .priority(80)
                .tag("run")
                .tag("dioxus")
                .tag(&platform)
                .estimated_duration(5)
                .build(),
        ]
    }

    /// Platform-specific build commands
    fn platform_commands(&self, context: &ProjectContext) -> Vec<Command> {
        let mut commands = vec![
            // Web platform
            CommandBuilder::new("dx-build-web", "dx")
                .label("Build for Web")
                .description("Build Dioxus app for web deployment")
                .arg("build")
                .arg("--platform")
                .arg("web")
                .category(CommandCategory::Build)
                .priority(85)
                .tag("build")
                .tag("web")
                .tag("dioxus")
                .estimated_duration(30)
                .build(),
            // Desktop platform
            CommandBuilder::new("dx-build-desktop", "dx")
                .label("Build for Desktop")
                .description("Build Dioxus desktop application")
                .arg("build")
                .arg("--platform")
                .arg("desktop")
                .category(CommandCategory::Build)
                .priority(80)
                .tag("build")
                .tag("desktop")
                .tag("dioxus")
                .estimated_duration(45)
                .build(),
            // Release builds
            CommandBuilder::new("dx-build-release", "dx")
                .label("Build Release")
                .description("Build optimized release version")
                .arg("build")
                .arg("--release")
                .category(CommandCategory::Build)
                .priority(80)
                .tag("build")
                .tag("release")
                .tag("dioxus")
                .estimated_duration(60)
                .build(),
            // Fullstack build
            CommandBuilder::new("dx-build-fullstack", "dx")
                .label("Build Fullstack")
                .description("Build fullstack app with server and client")
                .arg("build")
                .arg("--fullstack")
                .category(CommandCategory::Build)
                .priority(78)
                .tag("build")
                .tag("fullstack")
                .tag("dioxus")
                .estimated_duration(45)
                .build(),
            // SSG build
            CommandBuilder::new("dx-build-ssg", "dx")
                .label("Build Static Site")
                .description("Generate static site")
                .arg("build")
                .arg("--ssg")
                .category(CommandCategory::Build)
                .priority(77)
                .tag("build")
                .tag("ssg")
                .tag("static")
                .tag("dioxus")
                .estimated_duration(40)
                .build(),
        ];

        // Add mobile commands if we detect mobile dependencies
        if context
            .dependencies
            .iter()
            .any(|d| d.name.contains("mobile"))
        {
            commands.extend(vec![
                CommandBuilder::new("dx-build-ios", "dx")
                    .label("Build for iOS")
                    .description("Build Dioxus app for iOS")
                    .arg("build")
                    .arg("--platform")
                    .arg("ios")
                    .category(CommandCategory::Build)
                    .priority(75)
                    .tag("build")
                    .tag("ios")
                    .tag("mobile")
                    .tag("dioxus")
                    .estimated_duration(90)
                    .build(),
                CommandBuilder::new("dx-build-android", "dx")
                    .label("Build for Android")
                    .description("Build Dioxus app for Android")
                    .arg("build")
                    .arg("--platform")
                    .arg("android")
                    .category(CommandCategory::Build)
                    .priority(75)
                    .tag("build")
                    .tag("android")
                    .tag("mobile")
                    .tag("dioxus")
                    .estimated_duration(90)
                    .build(),
            ]);
        }

        commands
    }

    /// Testing commands
    fn test_commands(&self, _context: &ProjectContext) -> Vec<Command> {
        vec![
            CommandBuilder::new("cargo-test", "cargo")
                .label("Run Tests")
                .description("Run all tests")
                .arg("test")
                .category(CommandCategory::Test)
                .priority(75)
                .tag("test")
                .tag("dioxus")
                .estimated_duration(20)
                .build(),
            CommandBuilder::new("cargo-test-workspace", "cargo")
                .label("Test Workspace")
                .description("Run all workspace tests")
                .arg("test")
                .arg("--workspace")
                .category(CommandCategory::Test)
                .priority(70)
                .tag("test")
                .tag("workspace")
                .tag("dioxus")
                .estimated_duration(30)
                .build(),
            CommandBuilder::new("dx-check", "dx")
                .label("Check Project")
                .description("Check project for issues")
                .arg("check")
                .category(CommandCategory::Lint)
                .priority(72)
                .tag("check")
                .tag("lint")
                .tag("dioxus")
                .estimated_duration(10)
                .build(),
            CommandBuilder::new("dx-fmt", "dx")
                .label("Format RSX")
                .description("Automatically format RSX code")
                .arg("fmt")
                .category(CommandCategory::Format)
                .priority(70)
                .tag("format")
                .tag("rsx")
                .tag("dioxus")
                .estimated_duration(5)
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
                        // Component-related commands for PascalCase functions (Dioxus components)
                        if symbol.name.chars().next().is_some_and(|c| c.is_uppercase()) {
                            commands.push(
                                CommandBuilder::new("dx-component-preview", "dx")
                                    .label("Preview Component")
                                    .description(format!("Preview component: {}", symbol.name))
                                    .arg("serve")
                                    .arg("--component")
                                    .arg(&symbol.name)
                                    .category(CommandCategory::Run)
                                    .priority(85)
                                    .tag("component")
                                    .tag("preview")
                                    .tag("dioxus")
                                    .estimated_duration(10)
                                    .build(),
                            );
                        }

                        // Test function commands
                        if symbol.name.starts_with("test_")
                            || symbol.modifiers.contains(&"test".to_string())
                        {
                            commands.push(
                                CommandBuilder::new("dx-test-current", "dx")
                                    .label("Test Current Function")
                                    .description(format!("Run test: {}", symbol.name))
                                    .arg("test")
                                    .arg(&symbol.name)
                                    .category(CommandCategory::Test)
                                    .priority(90)
                                    .tag("test")
                                    .tag("current")
                                    .tag("dioxus")
                                    .estimated_duration(5)
                                    .build(),
                            );
                        }
                    }
                    SymbolKind::Struct => {
                        // State management related commands
                        if symbol.name.to_lowercase().contains("state") {
                            commands.push(
                                CommandBuilder::new("dx-check-state", "cargo")
                                    .label("Check State Management")
                                    .description(format!("Analyze state struct: {}", symbol.name))
                                    .arg("check")
                                    .category(CommandCategory::Lint)
                                    .priority(70)
                                    .tag("state")
                                    .tag("check")
                                    .tag("dioxus")
                                    .estimated_duration(8)
                                    .build(),
                            );
                        }
                    }
                    _ => {}
                }
            }

            // File-based commands
            if file_context.path.to_string_lossy().contains("component") {
                commands.push(
                    CommandBuilder::new("dx-build-components", "dx")
                        .label("Build Components")
                        .description("Build and validate all components")
                        .arg("build")
                        .arg("--components-only")
                        .category(CommandCategory::Build)
                        .priority(80)
                        .tag("components")
                        .tag("build")
                        .tag("dioxus")
                        .estimated_duration(20)
                        .build(),
                );
            }
        }

        commands
    }

    /// Bundle and deployment commands
    fn deployment_commands(&self, _context: &ProjectContext) -> Vec<Command> {
        vec![
            CommandBuilder::new("dx-bundle", "dx")
                .label("Bundle Application")
                .description("Bundle the Dioxus app into a shippable object")
                .arg("bundle")
                .category(CommandCategory::Build)
                .priority(75)
                .tag("bundle")
                .tag("production")
                .tag("dioxus")
                .estimated_duration(60)
                .build(),
            CommandBuilder::new("dx-bundle-release", "dx")
                .label("Bundle Release")
                .description("Bundle optimized release version")
                .arg("bundle")
                .arg("--release")
                .category(CommandCategory::Deploy)
                .priority(72)
                .tag("bundle")
                .tag("release")
                .tag("deploy")
                .tag("dioxus")
                .estimated_duration(75)
                .build(),
            CommandBuilder::new("dx-clean", "dx")
                .label("Clean Artifacts")
                .description("Clean output artifacts")
                .arg("clean")
                .category(CommandCategory::Clean)
                .priority(65)
                .tag("clean")
                .tag("dioxus")
                .estimated_duration(5)
                .build(),
            CommandBuilder::new("dx-new", "dx")
                .label("New Project")
                .description("Create a new Dioxus project")
                .arg("new")
                .category(CommandCategory::Generate)
                .priority(60)
                .tag("new")
                .tag("create")
                .tag("dioxus")
                .estimated_duration(10)
                .build(),
        ]
    }
}

impl Default for DioxusProvider {
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

    fn create_dioxus_context() -> ProjectContext {
        ProjectContext {
            workspace_root: PathBuf::from("/test"),
            current_file: None,
            cursor_position: None,
            project_type: ProjectType::Dioxus,
            dependencies: vec![Dependency {
                name: "dioxus".to_string(),
                version: "0.4".to_string(),
                features: vec!["web".to_string(), "desktop".to_string()],
                optional: false,
                dev_dependency: false,
            }],
            workspace_members: vec![WorkspaceMember {
                name: "dioxus-app".to_string(),
                path: PathBuf::from("/test"),
                package_type: ProjectType::Dioxus,
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
    async fn test_dioxus_provider_can_handle() {
        let provider = DioxusProvider::new();
        let context = create_dioxus_context();

        assert!(provider.can_handle(&context));
        assert_eq!(provider.name(), "dioxus");
        assert_eq!(provider.priority(), 90);
    }

    #[tokio::test]
    async fn test_dioxus_commands_generation() {
        let provider = DioxusProvider::new();
        let context = create_dioxus_context();

        let commands = provider.commands(&context).await.unwrap();

        assert!(!commands.is_empty());

        // Should have development commands
        assert!(commands.iter().any(|c| c.id == "dx-serve"));
        assert!(commands.iter().any(|c| c.id == "dx-run"));

        // Should have platform-specific builds
        assert!(commands.iter().any(|c| c.id == "dx-build-web"));
        assert!(commands.iter().any(|c| c.id == "dx-build-desktop"));

        // Should have test/check commands
        assert!(commands.iter().any(|c| c.id == "cargo-test"));
        assert!(commands.iter().any(|c| c.id == "dx-check"));

        // Should have bundle commands
        assert!(commands.iter().any(|c| c.id == "dx-bundle"));
    }

    #[tokio::test]
    async fn test_dioxus_mobile_commands() {
        let provider = DioxusProvider::new();
        let mut context = create_dioxus_context();

        // Add mobile dependency
        context.dependencies.push(Dependency {
            name: "dioxus-mobile".to_string(),
            version: "0.4".to_string(),
            features: vec![],
            optional: false,
            dev_dependency: false,
        });

        let commands = provider.commands(&context).await.unwrap();

        // Should have mobile-specific commands
        assert!(commands.iter().any(|c| c.id == "dx-build-ios"));
        assert!(commands.iter().any(|c| c.id == "dx-build-android"));
    }

    #[tokio::test]
    async fn test_dioxus_command_priorities() {
        let provider = DioxusProvider::new();
        let context = create_dioxus_context();

        let commands = provider.commands(&context).await.unwrap();

        // Development server should have highest priority
        let serve_cmd = commands.iter().find(|c| c.id == "dx-serve").unwrap();
        assert_eq!(serve_cmd.priority, 95);

        // All commands should have dioxus tag
        assert!(
            commands
                .iter()
                .all(|c| c.tags.contains(&"dioxus".to_string()))
        );
    }
}
