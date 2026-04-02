use anyhow::Result;
use serde_json::{Map, Value, json};
use std::path::{Path, PathBuf};

use crate::config::workspace::{get_package_name, is_workspace_only};

pub fn create_workspace_config() -> String {
    // Create a config with new nested structure for workspace
    let mut config = Map::new();

    // Create cargo config section without package name
    let mut cargo_config = Map::new();
    cargo_config.insert("extra_args".to_string(), json!([]));
    cargo_config.insert("extra_env".to_string(), json!({}));
    cargo_config.insert("extra_test_binary_args".to_string(), json!([]));

    // Add cargo config to main config
    config.insert("cargo".to_string(), Value::Object(cargo_config));

    // Add empty overrides array
    config.insert("overrides".to_string(), json!([]));

    // Pretty print the JSON
    serde_json::to_string_pretty(&config).unwrap()
}

pub fn create_default_config(package_name: &str) -> String {
    // Create a config with new nested structure
    let mut config = Map::new();

    // Create cargo config section
    let mut cargo_config = Map::new();
    cargo_config.insert("package".to_string(), json!(package_name));
    cargo_config.insert("extra_args".to_string(), json!([]));
    cargo_config.insert("extra_env".to_string(), json!({}));
    cargo_config.insert("extra_test_binary_args".to_string(), json!([]));

    // Add cargo config to main config
    config.insert("cargo".to_string(), Value::Object(cargo_config));

    // Add empty overrides array
    config.insert("overrides".to_string(), json!([]));

    // Example test_frameworks configuration (commented out by default)
    // Uncomment and modify as needed:
    /*
    config.insert("test_frameworks".to_string(), json!({
        "command": "cargo",
        "subcommand": "nextest run",
        "args": ["-j10"],
        "extra_env": {
            "RUST_BACKTRACE": "full"
        }
    }));
    */

    let config_value = Value::Object(config);
    serde_json::to_string_pretty(&config_value).unwrap()
}

pub fn create_root_config(project_root: &Path, cargo_tomls: &[PathBuf]) -> Result<String> {
    // Get the root package name if available
    let root_cargo_toml = project_root.join("Cargo.toml");
    let package_name = if root_cargo_toml.exists() {
        // Check if this is a workspace-only Cargo.toml
        if is_workspace_only(&root_cargo_toml)? {
            None // Workspaces don't have package names
        } else {
            Some(get_package_name(&root_cargo_toml)?)
        }
    } else {
        None
    };

    // Check if this is a Bazel project
    let is_bazel = project_root.join("BUILD.bazel").exists()
        || project_root.join("BUILD").exists()
        || project_root.join("MODULE.bazel").exists()
        || project_root.join("WORKSPACE").exists()
        || project_root.join("WORKSPACE.bazel").exists();

    // Always use Cargo.toml files for linkedProjects
    // rust-project.json is for rust-analyzer, not for cargo-runner
    let linked_projects: Vec<String> = cargo_tomls
        .iter()
        .map(|p| p.display().to_string())
        .collect();

    // Create root configuration with new nested structure
    let mut config = Map::new();

    // Create cargo config section
    let mut cargo_config = Map::new();

    // For Bazel projects, set command to "bazel"
    if is_bazel {
        cargo_config.insert("command".to_string(), json!("bazel"));
    }

    // Only add package if we have one
    if let Some(pkg) = package_name {
        cargo_config.insert("package".to_string(), json!(pkg));
    }

    // Add linked projects to cargo config
    cargo_config.insert("linked_projects".to_string(), json!(linked_projects));

    // Add empty defaults for cargo config
    cargo_config.insert("extra_args".to_string(), json!([]));
    cargo_config.insert("extra_env".to_string(), json!({}));
    cargo_config.insert("extra_test_binary_args".to_string(), json!([]));

    // Add cargo config to main config
    config.insert("cargo".to_string(), Value::Object(cargo_config));

    // Add empty overrides array
    config.insert("overrides".to_string(), json!([]));

    // Example test_frameworks configuration with miri and nextest
    // Uncomment and modify as needed:
    /*
    config.insert("test_frameworks".to_string(), json!({
        "command": "cargo",
        "subcommand": "miri nextest run",
        "channel": "nightly",
        "args": ["-j10"],
        "extra_env": {
            "MIRIFLAGS": "-Zmiri-disable-isolation",
            "RUST_BACKTRACE": "full"
        }
    }));
    */

    let config_value = Value::Object(config);
    Ok(serde_json::to_string_pretty(&config_value).unwrap())
}
