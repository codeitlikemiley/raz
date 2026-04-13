//! Bazel workspace detection helpers for the CLI layer.
//!
//! These utilities locate `Cargo.toml` + `BUILD.bazel` pairs in a directory,
//! find the `MODULE.bazel` root, and derive the Bazel crate-repo name used in
//! `MODULE.bazel` / `BUILD.bazel` `load()` statements.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// A Rust crate that has both a `Cargo.toml` and a `BUILD.bazel`.
#[derive(Debug, Clone)]
pub struct BazelCrate {
    /// Absolute path to the crate directory.
    pub dir: PathBuf,
    /// Crate name from `Cargo.toml` (package.name), or the directory basename
    /// as a fallback.
    pub name: String,
    /// True if this crate's `Cargo.toml` contains a `[workspace]` section —
    /// i.e. it is a workspace root rather than a plain member.
    pub is_workspace_root: bool,
    /// The name of the crate-universe repo used by Bazel, e.g. `server` or
    /// `complex_bazel_setup`.
    pub repo_name: String,
}

/// Walk `root` and return every subdirectory that contains both a `Cargo.toml`
/// and a `BUILD.bazel` (or `BUILD`).
pub fn find_bazel_crates(root: &Path) -> Result<Vec<BazelCrate>> {
    let mut crates = Vec::new();

    for entry in walkdir::WalkDir::new(root)
        .min_depth(0)
        .into_iter()
        .filter_entry(|e| {
            // Skip hidden dirs and Bazel output dirs
            let name = e.file_name().to_string_lossy();
            !name.starts_with('.') && name != "bazel-out" && name != "target"
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_dir())
    {
        let dir = entry.path();
        let has_cargo = dir.join("Cargo.toml").exists();
        let has_build = dir.join("BUILD.bazel").exists() || dir.join("BUILD").exists();

        if has_cargo && has_build {
            let cargo_content = std::fs::read_to_string(dir.join("Cargo.toml"))
                .with_context(|| format!("reading Cargo.toml in {}", dir.display()))?;

            let name = extract_package_name(&cargo_content)
                .unwrap_or_else(|| dir_basename(dir).to_string());

            let is_workspace_root = cargo_content.contains("[workspace]");
            let repo_name =
                cargo_workspace_repo_name_for_path(dir).unwrap_or_else(|| crate_repo_name(&name));

            crates.push(BazelCrate {
                dir: dir.to_path_buf(),
                name,
                is_workspace_root,
                repo_name,
            });
        }
    }

    Ok(crates)
}

/// Derive the Bazel crate-universe repo name from the Cargo workspace root
/// that contains `start`.
pub fn cargo_workspace_repo_name_for_path(start: &Path) -> Option<String> {
    let workspace_root = find_cargo_workspace_root(start)?;
    let root_name = workspace_root.file_name()?.to_str()?;
    Some(crate_repo_name(root_name))
}

/// Walk upward from `start` to find the directory containing `MODULE.bazel`.
/// Returns `None` if not found (i.e. this is not a Bzlmod workspace).
pub fn find_module_bazel(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        if current.join("MODULE.bazel").exists() {
            return Some(current);
        }
        match current.parent() {
            Some(p) => current = p.to_path_buf(),
            None => return None,
        }
    }
}

/// Walk upward from `start` to find the directory that owns a `Cargo.toml`
/// with a `[workspace]` section (the Cargo workspace root).
pub fn find_cargo_workspace_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        let cargo = current.join("Cargo.toml");
        if cargo.exists() {
            if let Ok(content) = std::fs::read_to_string(&cargo) {
                if content.contains("[workspace]") {
                    return Some(current);
                }
            }
        }
        match current.parent() {
            Some(p) => current = p.to_path_buf(),
            None => return None,
        }
    }
}

/// Derive the Bazel crate-universe repo name from a crate name.
///
/// Convention: `<crate_name>_crates`, with `-` replaced by `_`.
///
/// # Examples
/// ```
/// use cargo_runner::config::crate_repo_name;
/// assert_eq!(crate_repo_name("server"), "server_crates");
/// assert_eq!(crate_repo_name("my-service"), "my_service_crates");
/// ```
pub fn crate_repo_name(crate_name: &str) -> String {
    format!("{}_crates", crate_name.replace('-', "_"))
}

/// Extract `package.name` from raw Cargo.toml text using simple string
/// scanning (avoids pulling in `toml` as a dependency of the CLI).
fn extract_package_name(toml_content: &str) -> Option<String> {
    let mut in_package = false;
    for line in toml_content.lines() {
        let trimmed = line.trim();
        if trimmed == "[package]" {
            in_package = true;
            continue;
        }
        if trimmed.starts_with('[') {
            in_package = false;
        }
        if in_package && trimmed.starts_with("name") {
            // name = "server"
            if let Some(val) = trimmed.split_once('=').map(|x| x.1) {
                let name = val.trim().trim_matches('"').trim_matches('\'').to_string();
                if !name.is_empty() {
                    return Some(name);
                }
            }
        }
    }
    None
}

fn dir_basename(path: &Path) -> &str {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_crate_repo_name() {
        assert_eq!(crate_repo_name("server"), "server_crates");
        assert_eq!(crate_repo_name("my-service"), "my_service_crates");
        assert_eq!(crate_repo_name("corex"), "corex_crates");
    }

    #[test]
    fn test_extract_package_name() {
        let toml = r#"
[workspace]

[package]
name = "server"
version = "0.1.0"
"#;
        assert_eq!(extract_package_name(toml), Some("server".to_string()));
    }

    #[test]
    fn test_extract_package_name_none() {
        let toml = "[workspace]\nresolver = \"2\"\n";
        assert_eq!(extract_package_name(toml), None);
    }

    #[test]
    fn test_cargo_workspace_repo_name_for_path() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        let server = root.join("server");
        fs::create_dir(&server).unwrap();
        fs::write(
            server.join("Cargo.toml"),
            r#"[package]
name = "server"
version = "0.1.0"

[workspace]
"#,
        )
        .unwrap();

        assert_eq!(
            cargo_workspace_repo_name_for_path(&server.join("Cargo.toml")),
            Some("server_crates".to_string())
        );
    }
}
