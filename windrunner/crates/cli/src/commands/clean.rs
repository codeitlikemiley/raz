//! `cargo runner clean` — Unified clean for any project type.
//!
//! Auto-detects whether the current project uses Bazel or Cargo:
//!
//!   - **Cargo project**: runs `cargo clean`
//!   - **Bazel project**: runs `bazel clean` (+ optional cache flags)
//!
//! ## Examples
//!
//! ```text
//! cargo runner clean              # clean (auto-detect)
//! cargo runner clean --cache      # Bazel: also clear shared disk + repo caches
//! cargo runner clean --expunge    # Bazel: bazel clean --expunge
//! ```

use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

use crate::config::bazel_workspace::find_module_bazel;

pub fn clean_command(expunge: bool, cache: bool) -> Result<()> {
    let cwd = std::env::current_dir().context("Failed to get current directory")?;

    // Detect project type
    if find_module_bazel(&cwd).is_some() {
        clean_bazel(&cwd, expunge, cache)
    } else {
        if expunge || cache {
            eprintln!(
                "⚠️  --expunge / --cache are Bazel-only flags and are ignored for Cargo projects."
            );
        }
        clean_cargo(&cwd)
    }
}

fn clean_cargo(root: &Path) -> Result<()> {
    println!("🧹 cargo clean");
    let status = Command::new("cargo")
        .arg("clean")
        .current_dir(root)
        .status()
        .context("Failed to run `cargo clean`")?;

    if !status.success() {
        anyhow::bail!("`cargo clean` failed");
    }
    println!("✅ Clean complete.");
    Ok(())
}

fn clean_bazel(root: &Path, expunge: bool, cache: bool) -> Result<()> {
    let mut args = vec!["clean"];
    if expunge {
        args.push("--expunge");
        println!("🧹 bazel clean --expunge");
    } else {
        println!("🧹 bazel clean");
    }

    let status = Command::new("bazel")
        .args(&args)
        .current_dir(root)
        .status()
        .context("Failed to run `bazel clean` — is Bazel installed?")?;

    if !status.success() {
        anyhow::bail!("`bazel clean` failed");
    }

    if cache {
        remove_cache_dir("disk cache", "~/.cache/bazel-disk")?;
        remove_cache_dir("repo cache", "~/.cache/bazel-repo")?;
    }

    println!("✅ Clean complete.");
    Ok(())
}

fn remove_cache_dir(label: &str, raw_path: &str) -> Result<()> {
    let expanded = expand_tilde(raw_path);
    if expanded.exists() {
        println!("🗑️  Removing {}: {}", label, expanded.display());
        std::fs::remove_dir_all(&expanded)
            .with_context(|| format!("Failed to remove {}", expanded.display()))?;
        println!("   ✅ {label} cleared");
    } else {
        println!("   ~ {label} not found — skipping");
    }
    Ok(())
}

fn expand_tilde(path: &str) -> std::path::PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return std::path::PathBuf::from(home).join(rest);
        }
    }
    std::path::PathBuf::from(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_tilde_home_prefix() {
        let result = expand_tilde("~/.cache/bazel-disk");
        let home = std::env::var("HOME").unwrap();
        assert_eq!(
            result,
            std::path::PathBuf::from(format!("{home}/.cache/bazel-disk"))
        );
    }

    #[test]
    fn expand_tilde_no_tilde() {
        let result = expand_tilde("/usr/local/bin");
        assert_eq!(result, std::path::PathBuf::from("/usr/local/bin"));
    }

    #[test]
    fn expand_tilde_bare_tilde_no_slash() {
        // "~something" is NOT expanded (only "~/" is)
        let result = expand_tilde("~something");
        assert_eq!(result, std::path::PathBuf::from("~something"));
    }

    #[test]
    fn expand_tilde_nested_path() {
        let result = expand_tilde("~/a/b/c/d");
        let home = std::env::var("HOME").unwrap();
        assert_eq!(result, std::path::PathBuf::from(format!("{home}/a/b/c/d")));
    }
}
