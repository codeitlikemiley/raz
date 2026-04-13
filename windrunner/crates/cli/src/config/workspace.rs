use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::{fs, path::Path};
use toml::Value;

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

pub fn find_cargo_package_root(start: &Path) -> Option<std::path::PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        let cargo = current.join("Cargo.toml");
        if cargo.exists()
            && fs::read_to_string(&cargo)
                .ok()
                .map(|contents| contents.contains("[package]"))
                .unwrap_or(false)
        {
            return Some(current);
        }

        match current.parent() {
            Some(p) => current = p.to_path_buf(),
            None => return None,
        }
    }
}

pub fn resolve_default_run_for_package(package_root: &Path) -> Result<Option<std::path::PathBuf>> {
    let cargo_toml = package_root.join("Cargo.toml");
    let contents = fs::read_to_string(&cargo_toml)
        .with_context(|| format!("Failed to read {}", cargo_toml.display()))?;
    let manifest: Value = contents
        .parse()
        .with_context(|| format!("Failed to parse {}", cargo_toml.display()))?;

    let Some(default_run) = manifest
        .get("package")
        .and_then(|pkg| pkg.get("default-run"))
        .and_then(|v| v.as_str())
    else {
        return Ok(None);
    };

    let package_name = manifest
        .get("package")
        .and_then(|pkg| pkg.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or_default();

    if let Some(bin_target) = manifest.get("bin").and_then(|v| v.as_array()) {
        for bin in bin_target {
            let matches_name = bin
                .get("name")
                .and_then(|v| v.as_str())
                .map(|name| name == default_run)
                .unwrap_or(false);
            if !matches_name {
                continue;
            }

            if let Some(path) = bin.get("path").and_then(|v| v.as_str()) {
                return Ok(Some(package_root.join(path)));
            }

            let default_path = package_root
                .join("src/bin")
                .join(format!("{default_run}.rs"));
            if default_path.exists() {
                return Ok(Some(default_path));
            }

            return Ok(Some(package_root.join("src/main.rs")));
        }
    }

    if default_run == package_name {
        let main_rs = package_root.join("src/main.rs");
        if main_rs.exists() {
            return Ok(Some(main_rs));
        }
    }

    let bin_rs = package_root
        .join("src/bin")
        .join(format!("{default_run}.rs"));
    if bin_rs.exists() {
        return Ok(Some(bin_rs));
    }

    let bin_main = package_root
        .join("src/bin")
        .join(default_run)
        .join("main.rs");
    if bin_main.exists() {
        return Ok(Some(bin_main));
    }

    Err(anyhow::anyhow!(
        "default-run '{}' in {} does not resolve to a binary entry point",
        default_run,
        cargo_toml.display()
    ))
}

pub fn local_dependency_labels(crate_dir: &Path) -> Result<Vec<String>> {
    let cargo_toml = crate_dir.join("Cargo.toml");
    let contents = fs::read_to_string(&cargo_toml)
        .with_context(|| format!("Failed to read {}", cargo_toml.display()))?;

    let workspace_root = fs::canonicalize(
        find_cargo_workspace_root(crate_dir).unwrap_or_else(|| crate_dir.to_path_buf()),
    )
    .with_context(|| {
        format!(
            "Failed to resolve workspace root for {}",
            crate_dir.display()
        )
    })?;
    let crate_dir = fs::canonicalize(crate_dir)
        .with_context(|| format!("Failed to resolve {}", crate_dir.display()))?;
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
        if !matches!(
            section_name,
            "dependencies" | "dev-dependencies" | "build-dependencies"
        ) {
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
        let dep_dir = fs::canonicalize(&dep_dir)
            .with_context(|| format!("Failed to resolve {}", dep_dir.display()))?;
        let label = match dep_dir.strip_prefix(&workspace_root) {
            Ok(rel) if rel.as_os_str().is_empty() => format!("//:{package_name}_lib"),
            Ok(rel) => {
                let rel = rel.to_string_lossy().replace('\\', "/");
                format!("//{rel}:{package_name}_lib")
            }
            Err(_) => format!("//:{package_name}_lib"),
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
    if value.is_empty() { None } else { Some(value) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_local_dependency_labels_normalizes_relative_paths() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        fs::write(
            root.join("Cargo.toml"),
            r#"[workspace]
members = ["crates/cli", "crates/core"]
"#,
        )
        .unwrap();

        let cli = root.join("crates/cli");
        let core = root.join("crates/core");
        fs::create_dir_all(&cli).unwrap();
        fs::create_dir_all(&core).unwrap();
        fs::write(
            cli.join("Cargo.toml"),
            r#"[package]
name = "cargo-runner"
version = "0.1.0"

[dependencies]
cargo-runner-core = { path = "../core" }
"#,
        )
        .unwrap();
        fs::write(
            core.join("Cargo.toml"),
            r#"[package]
name = "cargo-runner-core"
version = "0.1.0"
"#,
        )
        .unwrap();

        let labels = local_dependency_labels(&cli).unwrap();
        assert_eq!(
            labels,
            vec!["//crates/core:cargo-runner-core_lib".to_string()]
        );
    }

    #[test]
    fn test_resolve_default_run_for_package_prefers_named_bin() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        fs::create_dir_all(root.join("src/bin")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            r#"[package]
name = "demo"
version = "0.1.0"
default-run = "server"
"#,
        )
        .unwrap();
        fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
        fs::write(root.join("src/bin/server.rs"), "fn main() {}\n").unwrap();

        let resolved = resolve_default_run_for_package(root).unwrap();
        assert_eq!(resolved, Some(root.join("src/bin/server.rs")));
    }
}
