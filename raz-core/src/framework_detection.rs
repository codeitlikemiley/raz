//! Framework detection with precise pattern matching and scoring
//!
//! This module provides laser-precise framework detection by analyzing:
//! - Dependencies and their combinations
//! - Project structure patterns
//! - File names and locations
//! - Function contexts
//! - Configuration files

use crate::ProjectContext;
use std::collections::HashMap;
use std::path::Path;

/// Framework types that can be detected
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FrameworkType {
    LeptosSSR,
    LeptosCSR,
    DioxusWeb,
    DioxusDesktop,
    DioxusMobile,
    BevyGame,
    TauriDesktop,
    ActixWeb,
    Axum,
    Rocket,
    YewWeb,
    EguiDesktop,
    WasmPack,
    VanillaRust,
}

/// Detection confidence score
#[derive(Debug, Clone)]
pub struct DetectionScore {
    pub framework: FrameworkType,
    pub confidence: f32,
    pub reasons: Vec<String>,
}

/// Framework detection engine with precise pattern matching
pub struct PreciseFrameworkDetector {
    patterns: HashMap<FrameworkType, FrameworkPattern>,
}

/// Pattern definition for framework detection
#[derive(Debug, Clone)]
struct FrameworkPattern {
    /// Required dependencies (all must be present)
    required_deps: Vec<&'static str>,
    /// Optional dependencies that increase confidence
    optional_deps: Vec<&'static str>,
    /// File structure patterns
    file_patterns: Vec<&'static str>,
    /// Configuration file patterns
    config_files: Vec<&'static str>,
    /// Typical entry point patterns
    entry_points: Vec<&'static str>,
    /// Weight for this framework (higher = more specific)
    specificity: f32,
}

impl Default for PreciseFrameworkDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl PreciseFrameworkDetector {
    pub fn new() -> Self {
        let mut patterns = HashMap::new();

        // Leptos SSR pattern (most specific)
        patterns.insert(
            FrameworkType::LeptosSSR,
            FrameworkPattern {
                required_deps: vec!["leptos", "leptos_axum"],
                optional_deps: vec![
                    "leptos_meta",
                    "leptos_router",
                    "axum",
                    "tokio",
                    "tower",
                    "tower-http",
                ],
                file_patterns: vec![
                    "app/src/lib.rs",
                    "frontend/src/lib.rs",
                    "server/src/main.rs",
                    "style/main.scss",
                    "Cargo.toml",
                ],
                config_files: vec!["Cargo.toml", "Trunk.toml", "style.css"],
                entry_points: vec!["server/src/main.rs", "src/main.rs"],
                specificity: 10.0,
            },
        );

        // Leptos CSR pattern
        patterns.insert(
            FrameworkType::LeptosCSR,
            FrameworkPattern {
                required_deps: vec!["leptos"],
                optional_deps: vec!["leptos_meta", "leptos_router", "wasm-bindgen", "web-sys"],
                file_patterns: vec!["src/main.rs", "src/app.rs", "index.html", "Trunk.toml"],
                config_files: vec!["Trunk.toml", "index.html"],
                entry_points: vec!["src/main.rs"],
                specificity: 8.0,
            },
        );

        // Dioxus Web pattern - matches both separate crates and features
        patterns.insert(
            FrameworkType::DioxusWeb,
            FrameworkPattern {
                required_deps: vec!["dioxus"],
                optional_deps: vec![
                    "dioxus-web",
                    "dioxus-router",
                    "dioxus-hooks",
                    "wasm-bindgen",
                ],
                file_patterns: vec!["src/main.rs", "index.html", "Dioxus.toml"],
                config_files: vec!["Dioxus.toml"],
                entry_points: vec!["src/main.rs"],
                specificity: 8.0,
            },
        );

        // Dioxus Desktop pattern - matches both separate crates and features
        patterns.insert(
            FrameworkType::DioxusDesktop,
            FrameworkPattern {
                required_deps: vec!["dioxus"],
                optional_deps: vec!["dioxus-desktop", "dioxus-router", "dioxus-hooks"],
                file_patterns: vec!["src/main.rs", "Dioxus.toml"],
                config_files: vec!["Dioxus.toml"],
                entry_points: vec!["src/main.rs"],
                specificity: 8.0,
            },
        );

        // Dioxus Mobile pattern - matches both separate crates and features
        patterns.insert(
            FrameworkType::DioxusMobile,
            FrameworkPattern {
                required_deps: vec!["dioxus"],
                optional_deps: vec!["dioxus-mobile", "dioxus-router", "dioxus-hooks"],
                file_patterns: vec!["src/main.rs", "Dioxus.toml"],
                config_files: vec!["Dioxus.toml"],
                entry_points: vec!["src/main.rs"],
                specificity: 8.0,
            },
        );

        // Bevy game pattern
        patterns.insert(
            FrameworkType::BevyGame,
            FrameworkPattern {
                required_deps: vec!["bevy"],
                optional_deps: vec!["bevy_egui", "bevy_rapier3d", "bevy_rapier2d"],
                file_patterns: vec!["src/main.rs", "assets/"],
                config_files: vec![],
                entry_points: vec!["src/main.rs"],
                specificity: 7.0,
            },
        );

        // Tauri desktop pattern
        patterns.insert(
            FrameworkType::TauriDesktop,
            FrameworkPattern {
                required_deps: vec!["tauri"],
                optional_deps: vec!["serde", "serde_json", "tokio", "tauri-build"],
                file_patterns: vec![
                    "src-tauri/src/main.rs",
                    "src-tauri/tauri.conf.json",
                    "src/main.rs",     // When running from within src-tauri
                    "tauri.conf.json", // When running from within src-tauri
                    "src/main.js",
                    "index.html",
                ],
                config_files: vec!["src-tauri/tauri.conf.json", "tauri.conf.json"],
                entry_points: vec!["src-tauri/src/main.rs", "src/main.rs"],
                specificity: 9.5, // High specificity for Tauri
            },
        );

        // Actix Web pattern
        patterns.insert(
            FrameworkType::ActixWeb,
            FrameworkPattern {
                required_deps: vec!["actix-web"],
                optional_deps: vec!["actix-rt", "actix-cors", "actix-session"],
                file_patterns: vec!["src/main.rs"],
                config_files: vec![],
                entry_points: vec!["src/main.rs"],
                specificity: 5.0,
            },
        );

        // Axum pattern (without Leptos)
        patterns.insert(
            FrameworkType::Axum,
            FrameworkPattern {
                required_deps: vec!["axum"],
                optional_deps: vec!["tower", "tower-http", "tokio"],
                file_patterns: vec!["src/main.rs"],
                config_files: vec![],
                entry_points: vec!["src/main.rs"],
                specificity: 4.0, // Lower than Leptos SSR which also uses Axum
            },
        );

        // Rocket pattern
        patterns.insert(
            FrameworkType::Rocket,
            FrameworkPattern {
                required_deps: vec!["rocket"],
                optional_deps: vec!["rocket_contrib", "serde", "serde_json"],
                file_patterns: vec!["src/main.rs", "Rocket.toml"],
                config_files: vec!["Rocket.toml"],
                entry_points: vec!["src/main.rs"],
                specificity: 6.0,
            },
        );

        // Yew pattern
        patterns.insert(
            FrameworkType::YewWeb,
            FrameworkPattern {
                required_deps: vec!["yew"],
                optional_deps: vec!["yew-router", "wasm-bindgen", "web-sys", "js-sys"],
                file_patterns: vec![
                    "src/main.rs",
                    "index.html",
                    "Trunk.toml",
                    "index.scss",
                    "style.css",
                ],
                config_files: vec!["Trunk.toml", "index.html"],
                entry_points: vec!["src/main.rs"],
                specificity: 9.0, // High specificity for Yew projects
            },
        );

        // Egui Desktop pattern
        patterns.insert(
            FrameworkType::EguiDesktop,
            FrameworkPattern {
                required_deps: vec!["egui", "eframe"],
                optional_deps: vec!["egui_extras"],
                file_patterns: vec!["src/main.rs"],
                config_files: vec![],
                entry_points: vec!["src/main.rs"],
                specificity: 6.0,
            },
        );

        // WASM Pack pattern
        patterns.insert(
            FrameworkType::WasmPack,
            FrameworkPattern {
                required_deps: vec!["wasm-bindgen"],
                optional_deps: vec!["web-sys", "js-sys", "wasm-bindgen-futures"],
                file_patterns: vec!["src/lib.rs", "Cargo.toml", "pkg/"],
                config_files: vec![],
                entry_points: vec!["src/lib.rs"],
                specificity: 5.0,
            },
        );

        Self { patterns }
    }

    /// Detect framework with scoring algorithm
    pub fn detect(
        &self,
        context: &ProjectContext,
        file_path: Option<&Path>,
    ) -> Vec<DetectionScore> {
        let mut scores = Vec::new();

        for (framework, pattern) in &self.patterns {
            let mut score = 0.0;
            let mut reasons = Vec::new();

            // Check required dependencies (must have all)
            let has_all_required = pattern
                .required_deps
                .iter()
                .all(|dep| context.dependencies.iter().any(|d| d.name == *dep));

            if !has_all_required {
                continue; // Skip this framework if missing required deps
            }

            score += pattern.specificity;
            reasons.push(format!(
                "Has all required dependencies: {:?}",
                pattern.required_deps
            ));

            // Check optional dependencies (bonus points)
            let optional_count = pattern
                .optional_deps
                .iter()
                .filter(|dep| context.dependencies.iter().any(|d| d.name == **dep))
                .count();

            if optional_count > 0 {
                score += optional_count as f32 * 0.5;
                reasons.push(format!("Has {optional_count} optional dependencies"));
            }

            // Check file structure patterns
            let file_pattern_matches = pattern
                .file_patterns
                .iter()
                .filter(|pattern| {
                    let path = context.workspace_root.join(pattern);
                    path.exists()
                })
                .count();

            if file_pattern_matches > 0 {
                score += file_pattern_matches as f32 * 1.0;
                reasons.push(format!(
                    "Matches {file_pattern_matches} file structure patterns"
                ));
            }

            // Check config files
            let config_matches = pattern
                .config_files
                .iter()
                .filter(|config| {
                    let path = context.workspace_root.join(config);
                    path.exists()
                })
                .count();

            if config_matches > 0 {
                score += config_matches as f32 * 2.0;
                reasons.push(format!("Has {config_matches} config files"));
            }

            // Check if current file matches entry point
            if let Some(current_file) = file_path {
                for entry_point in &pattern.entry_points {
                    let entry_path = context.workspace_root.join(entry_point);
                    if current_file == entry_path {
                        score += 3.0;
                        reasons.push(format!("Current file is entry point: {entry_point}"));
                        break;
                    }
                }
            }

            // Special case: Leptos SSR structure detection
            if framework == &FrameworkType::LeptosSSR {
                // Check for typical Leptos SSR workspace structure
                let has_app_crate = context.workspace_members.iter().any(|m| m.name == "app");
                let has_frontend_crate = context
                    .workspace_members
                    .iter()
                    .any(|m| m.name == "frontend");
                let has_server_crate = context.workspace_members.iter().any(|m| m.name == "server");

                if has_app_crate || has_frontend_crate {
                    score += 2.0;
                    reasons
                        .push("Has Leptos workspace structure (app/frontend crates)".to_string());
                }

                if has_server_crate {
                    score += 2.0;
                    reasons.push("Has server crate typical of Leptos SSR".to_string());
                }
            }

            // Special case: Dioxus platform detection
            if framework == &FrameworkType::DioxusDesktop
                || framework == &FrameworkType::DioxusWeb
                || framework == &FrameworkType::DioxusMobile
            {
                score += self.detect_dioxus_platform_specifics(context, framework, &mut reasons);
            }

            scores.push(DetectionScore {
                framework: framework.clone(),
                confidence: score,
                reasons,
            });
        }

        // Sort by confidence (highest first)
        scores.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

        scores
    }

    /// Detect Dioxus platform-specific features and configuration
    fn detect_dioxus_platform_specifics(
        &self,
        context: &ProjectContext,
        framework: &FrameworkType,
        reasons: &mut Vec<String>,
    ) -> f32 {
        let mut bonus_score = 0.0;

        // Check workspace structure for platform-specific folders
        for member in &context.workspace_members {
            let member_name = member.name.to_lowercase();
            let path_name = member
                .path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("")
                .to_lowercase();

            match framework {
                FrameworkType::DioxusWeb => {
                    if member_name.contains("web") || path_name.contains("web") {
                        bonus_score += 5.0;
                        reasons.push(format!("Found web workspace member: {}", member.name));
                    }
                }
                FrameworkType::DioxusDesktop => {
                    if member_name.contains("desktop") || path_name.contains("desktop") {
                        bonus_score += 5.0;
                        reasons.push(format!("Found desktop workspace member: {}", member.name));
                    }
                }
                FrameworkType::DioxusMobile => {
                    if member_name.contains("mobile") || path_name.contains("mobile") {
                        bonus_score += 5.0;
                        reasons.push(format!("Found mobile workspace member: {}", member.name));
                    }
                }
                _ => {}
            }
        }

        // Check for Dioxus.toml configuration
        let dioxus_toml_path = context.workspace_root.join("Dioxus.toml");
        if dioxus_toml_path.exists() {
            bonus_score += 2.0;
            reasons.push("Has Dioxus.toml configuration file".to_string());

            // Parse Dioxus.toml to check default_platform
            if let Ok(content) = std::fs::read_to_string(&dioxus_toml_path) {
                if let Ok(config) = content.parse::<toml::Value>() {
                    if let Some(app_config) = config.get("application") {
                        if let Some(default_platform) = app_config.get("default_platform") {
                            if let Some(platform_str) = default_platform.as_str() {
                                match (framework, platform_str) {
                                    (FrameworkType::DioxusDesktop, "desktop") => {
                                        bonus_score += 3.0;
                                        reasons.push(
                                            "Dioxus.toml specifies desktop platform".to_string(),
                                        );
                                    }
                                    (FrameworkType::DioxusWeb, "web") => {
                                        bonus_score += 3.0;
                                        reasons
                                            .push("Dioxus.toml specifies web platform".to_string());
                                    }
                                    (FrameworkType::DioxusMobile, "mobile") => {
                                        bonus_score += 3.0;
                                        reasons.push(
                                            "Dioxus.toml specifies mobile platform".to_string(),
                                        );
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }

        // Check for Dioxus features in dependencies
        if let Some(dioxus_dep) = context.dependencies.iter().find(|d| d.name == "dioxus") {
            match framework {
                FrameworkType::DioxusDesktop => {
                    if dioxus_dep.features.contains(&"desktop".to_string()) {
                        bonus_score += 3.0;
                        reasons.push("Dioxus dependency has 'desktop' feature".to_string());
                    }
                }
                FrameworkType::DioxusWeb => {
                    if dioxus_dep.features.contains(&"web".to_string()) {
                        bonus_score += 3.0;
                        reasons.push("Dioxus dependency has 'web' feature".to_string());
                    }
                }
                FrameworkType::DioxusMobile => {
                    if dioxus_dep.features.contains(&"mobile".to_string()) {
                        bonus_score += 3.0;
                        reasons.push("Dioxus dependency has 'mobile' feature".to_string());
                    }
                }
                _ => {}
            }

            // Check for fullstack feature (indicates complex setup)
            if dioxus_dep.features.contains(&"fullstack".to_string()) {
                bonus_score += 1.0;
                reasons.push("Dioxus dependency has 'fullstack' feature".to_string());
            }
        }

        // Check default features in Cargo.toml
        // Parse Cargo.toml to check if mobile is in default features
        let cargo_toml_path = context.workspace_root.join("Cargo.toml");
        if cargo_toml_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&cargo_toml_path) {
                if let Ok(config) = content.parse::<toml::Value>() {
                    if let Some(features) = config.get("features") {
                        if let Some(default_features) = features.get("default") {
                            if let Some(default_array) = default_features.as_array() {
                                let default_feature_names: Vec<String> = default_array
                                    .iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect();

                                match framework {
                                    FrameworkType::DioxusMobile => {
                                        if default_feature_names.contains(&"mobile".to_string()) {
                                            bonus_score += 4.0;
                                            reasons
                                                .push("Mobile is in default features".to_string());
                                        }
                                    }
                                    FrameworkType::DioxusDesktop => {
                                        if default_feature_names.contains(&"desktop".to_string()) {
                                            bonus_score += 4.0;
                                            reasons
                                                .push("Desktop is in default features".to_string());
                                        }
                                        // Desktop is common default, give small bonus
                                        if default_feature_names.is_empty() {
                                            bonus_score += 1.0;
                                        }
                                    }
                                    FrameworkType::DioxusWeb => {
                                        if default_feature_names.contains(&"web".to_string()) {
                                            bonus_score += 4.0;
                                            reasons.push("Web is in default features".to_string());
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }

        bonus_score
    }

    /// Get the best match with minimum confidence threshold
    pub fn best_match(
        &self,
        context: &ProjectContext,
        file_path: Option<&Path>,
        min_confidence: f32,
    ) -> Option<FrameworkType> {
        let scores = self.detect(context, file_path);

        scores
            .first()
            .filter(|s| s.confidence >= min_confidence)
            .map(|s| s.framework.clone())
    }
}

impl FrameworkType {
    /// Check if this is a web framework
    pub fn is_web_framework(&self) -> bool {
        matches!(
            self,
            FrameworkType::LeptosSSR
                | FrameworkType::LeptosCSR
                | FrameworkType::DioxusWeb
                | FrameworkType::YewWeb
                | FrameworkType::ActixWeb
                | FrameworkType::Axum
                | FrameworkType::Rocket
        )
    }

    /// Get the primary run command for this framework
    pub fn primary_run_command(&self) -> (&'static str, Vec<&'static str>) {
        match self {
            FrameworkType::LeptosSSR => ("cargo", vec!["leptos", "watch"]),
            FrameworkType::LeptosCSR => ("trunk", vec!["serve"]),
            FrameworkType::DioxusWeb => ("dx", vec!["serve"]),
            FrameworkType::DioxusDesktop => ("dx", vec!["serve", "--platform", "desktop"]),
            FrameworkType::DioxusMobile => ("dx", vec!["serve", "--platform", "mobile"]),
            FrameworkType::BevyGame => ("cargo", vec!["run"]),
            FrameworkType::TauriDesktop => ("cargo", vec!["tauri", "dev"]),
            FrameworkType::ActixWeb => ("cargo", vec!["run"]),
            FrameworkType::Axum => ("cargo", vec!["run"]),
            FrameworkType::Rocket => ("cargo", vec!["run"]),
            FrameworkType::YewWeb => ("trunk", vec!["serve"]),
            FrameworkType::EguiDesktop => ("cargo", vec!["run"]),
            FrameworkType::WasmPack => ("wasm-pack", vec!["build"]),
            FrameworkType::VanillaRust => ("cargo", vec!["run"]),
        }
    }

    /// Get the build command for this framework
    pub fn build_command(&self) -> (&'static str, Vec<&'static str>) {
        match self {
            FrameworkType::LeptosSSR => ("cargo", vec!["leptos", "build", "--release"]),
            FrameworkType::LeptosCSR => ("trunk", vec!["build", "--release"]),
            FrameworkType::DioxusWeb => ("dx", vec!["build", "--release"]),
            FrameworkType::DioxusDesktop => ("dx", vec!["build", "--release"]),
            FrameworkType::DioxusMobile => {
                ("dx", vec!["build", "--release", "--platform", "mobile"])
            }
            FrameworkType::BevyGame => ("cargo", vec!["build", "--release"]),
            FrameworkType::TauriDesktop => ("cargo", vec!["tauri", "build"]),
            FrameworkType::ActixWeb => ("cargo", vec!["build", "--release"]),
            FrameworkType::Axum => ("cargo", vec!["build", "--release"]),
            FrameworkType::Rocket => ("cargo", vec!["build", "--release"]),
            FrameworkType::YewWeb => ("trunk", vec!["build", "--release"]),
            FrameworkType::EguiDesktop => ("cargo", vec!["build", "--release"]),
            FrameworkType::WasmPack => ("wasm-pack", vec!["build", "--release"]),
            FrameworkType::VanillaRust => ("cargo", vec!["build", "--release"]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Dependency, ProjectType, WorkspaceMember};
    use std::path::PathBuf;

    #[test]
    fn test_leptos_ssr_detection() {
        let detector = PreciseFrameworkDetector::new();

        let context = ProjectContext {
            workspace_root: PathBuf::from("/test/project"),
            current_file: None,
            cursor_position: None,
            project_type: ProjectType::Binary,
            dependencies: vec![
                Dependency {
                    name: "leptos".to_string(),
                    version: "0.5.0".to_string(),
                    features: vec![],
                    optional: false,
                    dev_dependency: false,
                },
                Dependency {
                    name: "leptos_axum".to_string(),
                    version: "0.5.0".to_string(),
                    features: vec![],
                    optional: false,
                    dev_dependency: false,
                },
                Dependency {
                    name: "axum".to_string(),
                    version: "0.6.0".to_string(),
                    features: vec![],
                    optional: false,
                    dev_dependency: false,
                },
            ],
            workspace_members: vec![
                WorkspaceMember {
                    name: "app".to_string(),
                    path: PathBuf::from("app"),
                    package_type: ProjectType::Library,
                },
                WorkspaceMember {
                    name: "server".to_string(),
                    path: PathBuf::from("server"),
                    package_type: ProjectType::Binary,
                },
            ],
            build_targets: vec![],
            active_features: vec![],
            env_vars: std::collections::HashMap::new(),
        };

        let scores = detector.detect(&context, None);
        assert!(!scores.is_empty());
        assert_eq!(scores[0].framework, FrameworkType::LeptosSSR);
        assert!(scores[0].confidence > 10.0);
    }
}
