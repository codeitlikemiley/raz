//! `cargo runner bazel-clean [OPTIONS]`
//!
//! Clean Bazel build outputs and optionally the shared disk / repository caches.
//!
//! ## Modes
//!
//! | Flag            | What it removes                                          |
//! |-----------------|----------------------------------------------------------|
//! | *(none)*        | `bazel clean` — removes output dirs for this workspace   |
//! | `--expunge`     | `bazel clean --expunge` — removes ALL Bazel state        |
//! | `--disk-cache`  | Deletes `~/.cache/bazel-disk` (the shared build cache)   |
//! | `--repo-cache`  | Deletes `~/.cache/bazel-repo` (the shared fetch cache)   |
//! | `--all-caches`  | Clears both disk and repo caches                         |

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::bazel_workspace::find_module_bazel;

pub fn bazel_clean_command(
    expunge: bool,
    disk_cache: bool,
    repo_cache: bool,
    all_caches: bool,
) -> Result<()> {
    let cwd = std::env::current_dir().context("Failed to get current directory")?;
    let bazel_root = find_module_bazel(&cwd).with_context(|| {
        format!(
            "No MODULE.bazel found from {}.\n\
             Make sure you are inside a Bzlmod Bazel workspace.",
            cwd.display()
        )
    })?;

    // ── 1. bazel clean ───────────────────────────────────────────────────────
    run_bazel_clean(&bazel_root, expunge)?;

    // ── 2. Optional cache purges ──────────────────────────────────────────────
    let clear_disk = disk_cache || all_caches;
    let clear_repo = repo_cache || all_caches;

    if clear_disk {
        remove_cache_dir("disk", "~/.cache/bazel-disk")?;
    }
    if clear_repo {
        remove_cache_dir("repo", "~/.cache/bazel-repo")?;
    }

    println!();
    println!("✅ Clean complete.");
    Ok(())
}

fn run_bazel_clean(root: &Path, expunge: bool) -> Result<()> {
    let mut args = vec!["clean"];
    if expunge {
        args.push("--expunge");
        println!("🧹 bazel clean --expunge  (removes all Bazel state for this workspace)");
    } else {
        println!("🧹 bazel clean  (removes build outputs for this workspace)");
    }

    let status = Command::new("bazel")
        .args(&args)
        .current_dir(root)
        .status()
        .context("Failed to run `bazel clean` — is Bazel installed?")?;

    if !status.success() {
        anyhow::bail!("`bazel clean` failed");
    }
    Ok(())
}

fn remove_cache_dir(label: &str, raw_path: &str) -> Result<()> {
    let expanded = expand_tilde(raw_path);
    if expanded.exists() {
        println!("🗑️  Removing {} cache: {}", label, expanded.display());
        std::fs::remove_dir_all(&expanded)
            .with_context(|| format!("Failed to remove {}", expanded.display()))?;
        println!("   ✅ {} cache cleared", label);
    } else {
        println!("   ~ {} cache not found — skipping", label);
    }
    Ok(())
}

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs_home() {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}
