//! `cargo runner add <crate> [--features f1,f2] [--dev] [--crate-dir <dir>]`
//!
//! Wraps `cargo add` with a Bazel-aware post-sync pipeline:
//!   1. Runs `cargo add <crate> [--features ...] [--dev]` in the target crate dir
//!   2. Runs `cargo update` to refresh Cargo.lock
//!   3. Runs `bazel sync --only=<repo>` to sync crate-universe
//!   4. Regenerates `rust-project.json`
//!
//! The user does not need to know any Bazel commands.

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::bazel_workspace::{crate_repo_name, find_module_bazel};

/// Execute `cargo runner add`.
///
/// * `krate`       — crate name to add (e.g. `tokio`)
/// * `features`    — optional comma-separated feature list (`"full,rt-multi-thread"`)
/// * `dev`         — add as `[dev-dependencies]`
/// * `crate_dir`   — override the target crate directory (defaults to CWD)
/// * `skip_ide`    — skip `gen_rust_project` regeneration
pub fn bazel_add_command(
    krate: &str,
    features: Option<&str>,
    dev: bool,
    crate_dir: Option<&str>,
    skip_ide: bool,
) -> Result<()> {
    let cwd = std::env::current_dir().context("failed to get current directory")?;

    // ── Resolve target crate directory ────────────────────────────────────
    let target_dir = if let Some(dir) = crate_dir {
        PathBuf::from(dir)
            .canonicalize()
            .with_context(|| format!("cannot find crate directory: {dir}"))?
    } else {
        // Walk up to find a directory that has Cargo.toml + BUILD.bazel
        find_local_bazel_crate(&cwd)?
    };

    // ── Derive the crate's repo name for `bazel sync` ─────────────────────
    let cargo_content = std::fs::read_to_string(target_dir.join("Cargo.toml"))
        .context("failed to read Cargo.toml")?;
    let crate_name = extract_crate_name(&cargo_content).unwrap_or_else(|| {
        target_dir
            .file_name()
            .expect("target_dir must have a file name")
            .to_string_lossy()
            .to_string()
    });
    let repo_name = crate_repo_name(&crate_name);

    // ── Locate MODULE.bazel root ──────────────────────────────────────────
    let bazel_root = find_module_bazel(&target_dir).with_context(|| {
        "No MODULE.bazel found — are you inside a Bzlmod Bazel workspace?".to_string()
    })?;

    // ── 1. cargo add ──────────────────────────────────────────────────────
    let mut args = vec!["add", krate];
    let features_flag;
    if let Some(feats) = features {
        features_flag = format!("--features={feats}");
        args.push(&features_flag);
    }
    if dev {
        args.push("--dev");
    }

    println!(
        "📦 cargo add {}{}  in  {}",
        krate,
        features
            .map(|f| format!(" --features {f}"))
            .unwrap_or_default(),
        target_dir.display()
    );

    let status = Command::new("cargo")
        .args(&args)
        .current_dir(&target_dir)
        .status()
        .context("failed to run `cargo add`")?;

    if !status.success() {
        bail!("`cargo add {krate}` failed");
    }

    // ── 2. cargo update ───────────────────────────────────────────────────
    println!("🔄 cargo update …");
    let status = Command::new("cargo")
        .arg("update")
        .current_dir(&target_dir)
        .status()
        .context("failed to run `cargo update`")?;

    if !status.success() {
        bail!("`cargo update` failed in {}", target_dir.display());
    }

    // ── 3. bazel sync ─────────────────────────────────────────────────────
    println!("🔥 bazel sync  →  {repo_name}");
    let only_flag = format!("--only={repo_name}");
    let status = Command::new("bazel")
        .args(["sync", &only_flag])
        .current_dir(&bazel_root)
        .status()
        .context("failed to run `bazel sync`")?;

    if !status.success() {
        bail!("`bazel sync {only_flag}` failed");
    }

    // ── 4. Regenerate rust-project.json ───────────────────────────────────
    if !skip_ide {
        println!("🦀 Regenerating rust-project.json …");
        run_gen_rust_project(&bazel_root);
    }

    println!("\n✅  '{krate}' added to {crate_name}.");
    println!(
        "   Available in BUILD.bazel via all_crate_deps(){}.",
        if dev { " (dev)" } else { "" }
    );

    Ok(())
}

// ── helpers ──────────────────────────────────────────────────────────────────

/// Walk upward from `start` to find the nearest directory with both
/// `Cargo.toml` and `BUILD.bazel`.
fn find_local_bazel_crate(start: &Path) -> Result<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        if current.join("Cargo.toml").exists()
            && (current.join("BUILD.bazel").exists() || current.join("BUILD").exists())
        {
            return Ok(current);
        }
        match current.parent() {
            Some(p) => current = p.to_path_buf(),
            None => bail!(
                "No Cargo.toml + BUILD.bazel found from {}.\n\
                 Use --crate-dir to specify the target crate directory.",
                start.display()
            ),
        }
    }
}

fn extract_crate_name(toml_content: &str) -> Option<String> {
    let mut in_package = false;
    for line in toml_content.lines() {
        let t = line.trim();
        if t == "[package]" {
            in_package = true;
            continue;
        }
        if t.starts_with('[') {
            in_package = false;
        }
        if in_package && t.starts_with("name") {
            if let Some(val) = t.split_once('=').map(|x| x.1) {
                let name = val.trim().trim_matches('"').trim_matches('\'').to_string();
                if !name.is_empty() {
                    return Some(name);
                }
            }
        }
    }
    None
}

fn run_gen_rust_project(bazel_root: &Path) {
    let status = Command::new("bazel")
        .args([
            "run",
            "@rules_rust//tools/rust_analyzer:gen_rust_project",
            "--",
            "//...",
        ])
        .current_dir(bazel_root)
        .status();

    match status {
        Ok(s) if s.success() => {}
        _ => eprintln!(
            "⚠️  gen_rust_project failed. Run manually:\n\
             \tbazel run @rules_rust//tools/rust_analyzer:gen_rust_project -- //..."
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_name_standard() {
        let toml = "[package]\nname = \"my-crate\"\nversion = \"0.1.0\"";
        assert_eq!(extract_crate_name(toml), Some("my-crate".to_string()));
    }

    #[test]
    fn extract_name_single_quotes() {
        let toml = "[package]\nname = 'my-crate'";
        assert_eq!(extract_crate_name(toml), Some("my-crate".to_string()));
    }

    #[test]
    fn extract_name_missing_package() {
        let toml = "[workspace]\nmembers = [\"a\", \"b\"]";
        assert_eq!(extract_crate_name(toml), None);
    }

    #[test]
    fn extract_name_ignores_dependency_name() {
        let toml = "[dependencies]\nname = \"not-this\"\n\n[package]\nname = \"real-name\"";
        assert_eq!(extract_crate_name(toml), Some("real-name".to_string()));
    }

    #[test]
    fn extract_name_with_spaces() {
        let toml = "[package]\nname  =  \"spaced\"";
        assert_eq!(extract_crate_name(toml), Some("spaced".to_string()));
    }

    #[test]
    fn extract_name_empty_value() {
        let toml = "[package]\nname = \"\"";
        assert_eq!(extract_crate_name(toml), None);
    }
}
