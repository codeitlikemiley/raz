//! Integration test for Yew framework support

use raz_core::{
    BuildTarget, Dependency, ProjectContext, ProjectType, RazCore, TargetType, WorkspaceMember,
};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[tokio::test]
async fn test_yew_project_detection_and_commands() {
    // Create a temporary directory structure that looks like a Yew project
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create Cargo.toml with Yew dependency
    let cargo_toml = project_root.join("Cargo.toml");
    fs::write(
        &cargo_toml,
        r#"
[package]
name = "yew-test-app"
version = "0.1.0"
edition = "2021"

[dependencies]
yew = { version = "0.21", features = ["csr"] }
web-sys = { version = "0.3", features = ["console", "Document", "Element", "HtmlElement", "Window"] }
wasm-bindgen = "0.2"
"#,
    )
    .unwrap();

    // Create Trunk.toml
    let trunk_toml = project_root.join("Trunk.toml");
    fs::write(
        &trunk_toml,
        r#"
[build]
target = "index.html"

[watch]
ignore = ["dist/"]
"#,
    )
    .unwrap();

    // Create index.html
    let index_html = project_root.join("index.html");
    fs::write(
        &index_html,
        r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8" />
    <title>Yew Test App</title>
</head>
<body></body>
</html>
"#,
    )
    .unwrap();

    // Create src directory and main.rs
    let src_dir = project_root.join("src");
    fs::create_dir_all(&src_dir).unwrap();

    let main_rs = src_dir.join("main.rs");
    fs::write(
        &main_rs,
        r#"
use yew::prelude::*;

#[function_component]
fn App() -> Html {
    html! {
        <div>
            <h1>{"Hello, Yew!"}</h1>
        </div>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_component() {
        // Test component creation
        assert!(true);
    }
}
"#,
    )
    .unwrap();

    // Create RazCore instance and analyze the workspace
    let raz = RazCore::new().unwrap();
    let context = raz.analyze_workspace(project_root).await.unwrap();

    // Verify that the project type is detected as Yew
    match &context.project_type {
        ProjectType::Yew => {
            // Perfect! Yew was detected
        }
        other => panic!("Expected ProjectType::Yew, but got: {other:?}"),
    }

    // Verify that yew dependency was detected
    assert!(
        context.dependencies.iter().any(|dep| dep.name == "yew"),
        "Yew dependency should be detected"
    );

    // Generate commands and verify Yew-specific commands are present
    let commands = raz.generate_commands(&context).await.unwrap();

    // Check for essential Yew commands
    let command_ids: Vec<&str> = commands.iter().map(|c| c.id.as_str()).collect();

    assert!(
        command_ids.contains(&"yew-serve"),
        "Should have yew-serve command. Available commands: {command_ids:?}"
    );

    assert!(
        command_ids.contains(&"yew-build"),
        "Should have yew-build command. Available commands: {command_ids:?}"
    );

    assert!(
        command_ids.contains(&"yew-build-release"),
        "Should have yew-build-release command. Available commands: {command_ids:?}"
    );

    // Verify that the commands use 'trunk' as the executor
    let yew_serve_cmd = commands.iter().find(|c| c.id == "yew-serve").unwrap();
    assert_eq!(
        yew_serve_cmd.command, "trunk",
        "Yew serve command should use trunk"
    );
    assert!(yew_serve_cmd.args.contains(&"serve".to_string()));

    let yew_build_cmd = commands.iter().find(|c| c.id == "yew-build").unwrap();
    assert_eq!(
        yew_build_cmd.command, "trunk",
        "Yew build command should use trunk"
    );
    assert!(yew_build_cmd.args.contains(&"build".to_string()));

    // Verify tags are applied correctly
    assert!(yew_serve_cmd.tags.contains(&"yew".to_string()));
    assert!(yew_build_cmd.tags.contains(&"yew".to_string()));

    println!("✅ Yew integration test passed!");
    println!("   - Project type correctly detected as Yew");
    println!("   - Generated {} commands", commands.len());
    println!("   - Essential Yew commands present (serve, build, etc.)");
}

#[tokio::test]
async fn test_yew_dependency_only_detection() {
    // Test that Yew can be detected even without Trunk.toml (fallback detection)
    let context = ProjectContext {
        workspace_root: PathBuf::from("/test/yew-project"),
        current_file: None,
        cursor_position: None,
        project_type: ProjectType::Binary, // Will be overridden by detection
        dependencies: vec![
            Dependency {
                name: "yew".to_string(),
                version: "0.21.0".to_string(),
                features: vec!["csr".to_string()],
                optional: false,
                dev_dependency: false,
            },
            Dependency {
                name: "yew-router".to_string(),
                version: "0.18.0".to_string(),
                features: vec![],
                optional: false,
                dev_dependency: false,
            },
        ],
        workspace_members: vec![WorkspaceMember {
            name: "yew-app".to_string(),
            path: PathBuf::from("/test/yew-project"),
            package_type: ProjectType::Yew,
        }],
        build_targets: vec![BuildTarget {
            name: "main".to_string(),
            target_type: TargetType::Binary,
            path: PathBuf::from("/test/yew-project/src/main.rs"),
        }],
        active_features: vec!["csr".to_string()],
        env_vars: HashMap::new(),
    };

    let raz = RazCore::new().unwrap();
    let commands = raz.generate_commands(&context).await.unwrap();

    // Verify Yew commands are generated even without explicit project type
    let yew_commands: Vec<_> = commands
        .iter()
        .filter(|c| c.tags.contains(&"yew".to_string()))
        .collect();

    assert!(
        !yew_commands.is_empty(),
        "Should generate Yew commands when Yew dependencies are detected"
    );

    println!("✅ Yew dependency detection test passed!");
    println!(
        "   - Generated {} Yew-specific commands",
        yew_commands.len()
    );
}
