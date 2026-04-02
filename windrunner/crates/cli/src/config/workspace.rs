use anyhow::{Context, Result};
use std::{fs, path::Path};

pub fn is_workspace_only(cargo_toml: &Path) -> Result<bool> {
    let contents = fs::read_to_string(cargo_toml)
        .with_context(|| format!("Failed to read {}", cargo_toml.display()))?;

    // Check if it has [workspace] but no [package]
    let has_workspace = contents.contains("[workspace]");
    let has_package = contents.contains("[package]");

    Ok(has_workspace && !has_package)
}

pub fn get_package_name(cargo_toml: &Path) -> Result<String> {
    let contents = fs::read_to_string(cargo_toml)
        .with_context(|| format!("Failed to read {}", cargo_toml.display()))?;

    // Simple TOML parsing for package name
    for line in contents.lines() {
        if let Some(name) = line.strip_prefix("name = ") {
            let name = name.trim().trim_matches('"');
            return Ok(name.to_string());
        }
    }

    // Fallback to directory name
    Ok(cargo_toml
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string())
}
