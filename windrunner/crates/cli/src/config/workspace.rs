use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::{fs, path::Path};

use crate::config::bazel_workspace::find_cargo_workspace_root;

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

pub fn rust_crate_name(package_name: &str) -> String {
    package_name.replace('-', "_")
}

pub fn local_dependency_labels(crate_dir: &Path) -> Result<Vec<String>> {
    let cargo_toml = crate_dir.join("Cargo.toml");
    let contents = fs::read_to_string(&cargo_toml)
        .with_context(|| format!("Failed to read {}", cargo_toml.display()))?;

    let workspace_root = find_cargo_workspace_root(crate_dir).unwrap_or_else(|| crate_dir.to_path_buf());
    let mut labels = BTreeSet::new();
    let mut section: Option<String> = None;

    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            section = match trimmed {
                "[dependencies]" | "[dev-dependencies]" | "[build-dependencies]" => {
                    Some(trimmed.trim_matches(['[', ']']).to_string())
                }
                _ => None,
            };
            continue;
        }

        let Some(section_name) = section.as_deref() else {
            continue;
        };
        if !matches!(section_name, "dependencies" | "dev-dependencies" | "build-dependencies") {
            continue;
        }

        let Some(path_value) = extract_inline_path_value(trimmed) else {
            continue;
        };

        let dep_dir = crate_dir.join(path_value);
        let dep_cargo = dep_dir.join("Cargo.toml");
        if !dep_cargo.exists() {
            continue;
        }

        let package_name = get_package_name(&dep_cargo)?;
        let label = match dep_dir.strip_prefix(&workspace_root) {
            Ok(rel) if rel.as_os_str().is_empty() => format!("//:{}_lib", package_name),
            Ok(rel) => {
                let rel = rel.to_string_lossy().replace('\\', "/");
                format!("//{}:{}_lib", rel, package_name)
            }
            Err(_) => format!("//:{}_lib", package_name),
        };
        labels.insert(label);
    }

    Ok(labels.into_iter().collect())
}

fn extract_inline_path_value(line: &str) -> Option<String> {
    let path_idx = line.find("path")?;
    let rhs = &line[path_idx..];
    let eq_idx = rhs.find('=')?;
    let value = rhs[eq_idx + 1..].trim_start();
    let quote = value.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let value = value[1..].split(quote).next()?.trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}
