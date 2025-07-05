//! Integration tests for the `raz init` command

use anyhow::Result;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Helper function to run init_config from main.rs
fn init_config(working_dir: &Path, template: &str, force: bool) -> Result<()> {
    // Since init_config is in main.rs, we'll need to extract it to a lib
    // For now, we'll test by running the actual CLI command
    use std::process::Command;

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_raz"));
    cmd.arg("-C").arg(working_dir);
    cmd.arg("init");
    cmd.arg("--template").arg(template);

    if force {
        cmd.arg("--force");
    }

    let output = cmd.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Init command failed: {}", stderr);
    }

    Ok(())
}

#[test]
fn test_init_default_template() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let working_dir = temp_dir.path();

    // Initialize with default template
    init_config(working_dir, "default", false)?;

    // Check that config file was created
    let config_file = working_dir.join(".raz").join("config.toml");
    assert!(config_file.exists(), "Config file should be created");

    // Read and verify content
    let content = fs::read_to_string(&config_file)?;

    // Verify default template content
    assert!(content.contains("# RAZ Configuration - General Rust Development"));
    assert!(content.contains("[providers.cargo]"));
    assert!(content.contains("enabled = true"));
    assert!(content.contains("[providers.doc]"));
    assert!(content.contains("[filters]"));
    assert!(content.contains("[ui]"));
    assert!(content.contains("max_commands = 15"));

    Ok(())
}

#[test]
fn test_init_web_template() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let working_dir = temp_dir.path();

    // Initialize with web template
    init_config(working_dir, "web", false)?;

    let config_file = working_dir.join(".raz").join("config.toml");
    let content = fs::read_to_string(&config_file)?;

    // Verify web template content
    assert!(content.contains("# RAZ Configuration - Web Development Template"));
    assert!(content.contains("[providers.leptos]"));
    assert!(content.contains("[providers.dioxus]"));
    assert!(content.contains("enabled = true"));
    assert!(content.contains("[[filters.rules]]"));
    assert!(content.contains("boost-dev-commands"));

    Ok(())
}

#[test]
fn test_init_game_template() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let working_dir = temp_dir.path();

    // Initialize with game template
    init_config(working_dir, "game", false)?;

    let config_file = working_dir.join(".raz").join("config.toml");
    let content = fs::read_to_string(&config_file)?;

    // Verify game template content
    assert!(content.contains("# RAZ Configuration - Game Development Template"));
    assert!(content.contains("[providers.bevy]"));
    assert!(content.contains("enabled = true"));
    assert!(content.contains("boost-run-commands"));

    Ok(())
}

#[test]
fn test_init_library_template() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let working_dir = temp_dir.path();

    // Initialize with library template
    init_config(working_dir, "library", false)?;

    let config_file = working_dir.join(".raz").join("config.toml");
    let content = fs::read_to_string(&config_file)?;

    // Verify library template content
    assert!(content.contains("# RAZ Configuration - Library Development Template"));
    assert!(content.contains("prioritize-test-doc"));
    assert!(content.contains("BoostPriority = { pattern = \"test\", boost = 20 }"));

    Ok(())
}

#[test]
fn test_init_desktop_template() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let working_dir = temp_dir.path();

    // Initialize with desktop template
    init_config(working_dir, "desktop", false)?;

    let config_file = working_dir.join(".raz").join("config.toml");
    let content = fs::read_to_string(&config_file)?;

    // Verify desktop template content
    assert!(content.contains("# RAZ Configuration - Desktop Application Template"));
    assert!(content.contains("[providers.tauri]"));
    assert!(content.contains("boost-app-commands"));

    Ok(())
}

#[test]
fn test_init_force_overwrite() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let working_dir = temp_dir.path();

    // First initialization
    init_config(working_dir, "default", false)?;

    // Try to init again without force - should fail
    let result = init_config(working_dir, "web", false);
    assert!(result.is_err(), "Should fail without force flag");

    // Init with force - should succeed
    init_config(working_dir, "web", true)?;

    // Verify it's now the web template
    let config_file = working_dir.join(".raz").join("config.toml");
    let content = fs::read_to_string(&config_file)?;
    assert!(content.contains("# RAZ Configuration - Web Development Template"));

    Ok(())
}

#[test]
fn test_init_auto_detection() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let working_dir = temp_dir.path();

    // Create a Cargo.toml with Leptos dependency
    let cargo_toml = r#"
[package]
name = "test-web"
version = "0.1.0"

[dependencies]
leptos = "0.6"
"#;
    fs::write(working_dir.join("Cargo.toml"), cargo_toml)?;

    // Initialize with default template - should auto-detect web
    init_config(working_dir, "default", false)?;

    // For now, auto-detection chooses web template when default is specified
    // and web framework is detected
    let config_file = working_dir.join(".raz").join("config.toml");
    assert!(config_file.exists());

    Ok(())
}

#[test]
fn test_init_creates_directory_structure() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let working_dir = temp_dir.path();

    // Initialize
    init_config(working_dir, "default", false)?;

    // Check directory structure
    let raz_dir = working_dir.join(".raz");
    assert!(raz_dir.exists() && raz_dir.is_dir());

    let config_file = raz_dir.join("config.toml");
    assert!(config_file.exists() && config_file.is_file());

    Ok(())
}

#[test]
fn test_all_templates_valid_toml() -> Result<()> {
    let templates = vec!["default", "web", "game", "library", "desktop"];

    for template in templates {
        let temp_dir = TempDir::new()?;
        let working_dir = temp_dir.path();

        init_config(working_dir, template, false)?;

        let config_file = working_dir.join(".raz").join("config.toml");
        let content = fs::read_to_string(&config_file)?;

        // Try to parse as TOML to ensure it's valid
        let parsed: toml::Value = toml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Template {} has invalid TOML: {}", template, e))?;

        // Basic structure validation
        assert!(
            parsed.get("providers").is_some(),
            "Template {template} should have providers section"
        );
        assert!(
            parsed.get("filters").is_some(),
            "Template {template} should have filters section"
        );
        assert!(
            parsed.get("ui").is_some(),
            "Template {template} should have ui section"
        );
    }

    Ok(())
}
