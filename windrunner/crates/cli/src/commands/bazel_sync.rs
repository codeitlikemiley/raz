//! `cargo runner sync` — Synchronise Bazel crate-universe after `cargo add`.
//!
//! Workflow:
//!   1. Find every crate in the workspace that has both `Cargo.toml` + `BUILD.bazel`
//!   2. Run `cargo update` in each crate directory (or only the specified one)
//!   3. Run `bazel sync --only=<repo,...>` from the MODULE.bazel root
//!   4. Regenerate `rust-project.json` via `gen_rust_project`
//!
//! The user sees only a compact progress story and never needs to know what
//! Bazel commands are running underneath.

use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::Command;

use crate::config::bazel_workspace::{find_bazel_crates, find_module_bazel};

/// Run the full sync pipeline from `cwd`.
///
/// * `crate_filter` — if `Some`, only sync that specific crate (by name or
///   directory basename). Pass `None` to sync all crates.
/// * `skip_ide` — skip the `gen_rust_project` step (useful in CI).
pub fn bazel_sync_command(crate_filter: Option<&str>, skip_ide: bool) -> Result<()> {
    let cwd = std::env::current_dir().context("failed to get current directory")?;

    // ── 1. Locate the Bazel workspace root ──────────────────────────────────
    let bazel_root = find_module_bazel(&cwd).with_context(|| {
        format!(
            "No MODULE.bazel found from {}.\n\
             Make sure you are inside a Bzlmod Bazel workspace.",
            cwd.display()
        )
    })?;

    println!("🔥 Bazel workspace root: {}", bazel_root.display());

    // ── 2. Discover crates ────────────────────────────────────────────────
    let all_crates =
        find_bazel_crates(&bazel_root).context("failed to scan workspace for Bazel crates")?;

    if all_crates.is_empty() {
        bail!(
            "No crates with both Cargo.toml + BUILD.bazel found under {}",
            bazel_root.display()
        );
    }

    let crates_to_sync: Vec<_> = if let Some(filter) = crate_filter {
        let filtered: Vec<_> = all_crates
            .iter()
            .filter(|c| c.name == filter || c.dir.file_name().is_some_and(|n| n == filter))
            .collect();
        if filtered.is_empty() {
            bail!(
                "No crate matching '{}' found. Available: {}",
                filter,
                all_crates
                    .iter()
                    .map(|c| c.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        filtered
    } else {
        all_crates.iter().collect()
    };

    // ── 3. cargo update in each crate dir ─────────────────────────────────
    for krate in &crates_to_sync {
        println!("📦 cargo update  →  {}", krate.dir.display());
        run_cargo_update(&krate.dir)?;
    }

    // ── 4. bazel sync ──────────────────────────────────────────────────────
    let repo_names: Vec<&str> = crates_to_sync
        .iter()
        .map(|c| c.repo_name.as_str())
        .collect();
    println!("🔄 bazel sync  →  {}", repo_names.join(", "));
    run_bazel_sync(&bazel_root, &repo_names)?;

    // ── 5. Regenerate rust-project.json ────────────────────────────────────
    if !skip_ide {
        println!("🦀 Regenerating rust-project.json for rust-analyzer …");
        run_gen_rust_project(&bazel_root)?;
    }

    println!("\n✅ Sync complete.");
    if !skip_ide {
        println!("   rust-project.json updated — restart rust-analyzer if needed.");
    }

    Ok(())
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn run_cargo_update(crate_dir: &Path) -> Result<()> {
    let status = Command::new("cargo")
        .arg("update")
        .current_dir(crate_dir)
        .status()
        .with_context(|| format!("failed to run `cargo update` in {}", crate_dir.display()))?;

    if !status.success() {
        bail!("`cargo update` failed in {}", crate_dir.display());
    }
    Ok(())
}

fn run_bazel_sync(bazel_root: &Path, repos: &[&str]) -> Result<()> {
    let only_flag = format!("--only={}", repos.join(","));
    let status = Command::new("bazel")
        .args(["sync", &only_flag])
        .current_dir(bazel_root)
        .status()
        .context("failed to run `bazel sync`")?;

    if !status.success() {
        bail!("`bazel sync {only_flag}` failed");
    }
    Ok(())
}

fn run_gen_rust_project(bazel_root: &Path) -> Result<()> {
    let status = Command::new("bazel")
        .args([
            "run",
            "@rules_rust//tools/rust_analyzer:gen_rust_project",
            "--",
            "//...",
        ])
        .current_dir(bazel_root)
        .status()
        .context("failed to run gen_rust_project")?;

    if !status.success() {
        // Non-fatal: warn but don't bail — the dep sync already succeeded.
        eprintln!(
            "⚠️  gen_rust_project exited with error. Run manually:\n\
             \tbazel run @rules_rust//tools/rust_analyzer:gen_rust_project -- //..."
        );
    }
    Ok(())
}
