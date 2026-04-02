//! `cargo runner test [TARGET] [OPTIONS]`
//!
//! Run `bazel test` for a target discovered from the current crate or
//! the whole workspace, with optional test-name filtering.
//!
//! ## Examples
//!
//! ```text
//! cargo runner test                           # test all targets (//...)
//! cargo runner test //:my_crate_test          # specific target
//! cargo runner test --filter my_function_name # run tests matching a name
//! cargo runner test --crate server            # test a named crate only
//! cargo runner test --streamed               # show all test output live
//! ```

use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

use crate::config::bazel_workspace::{find_bazel_crates, find_module_bazel};

pub fn bazel_test_command(
    target: Option<&str>,
    filter: Option<&str>,
    crate_name: Option<&str>,
    streamed: bool,
) -> Result<()> {
    let cwd = std::env::current_dir().context("Failed to get current directory")?;
    let bazel_root = find_module_bazel(&cwd).with_context(|| {
        format!(
            "No MODULE.bazel found from {}.\n\
             Make sure you are inside a Bzlmod Bazel workspace.",
            cwd.display()
        )
    })?;

    // Resolve target
    let resolved_target = if let Some(t) = target {
        t.to_string()
    } else if let Some(krate) = crate_name {
        // Find the crate and derive its test target
        let all = find_bazel_crates(&bazel_root)
            .context("failed to scan workspace for Bazel crates")?;
        let found = all
            .iter()
            .find(|c| c.name == krate || c.dir.file_name().map_or(false, |n| n == krate))
            .with_context(|| {
                let names: Vec<_> = all.iter().map(|c| c.name.as_str()).collect();
                format!(
                    "No crate '{}' found. Available: {}",
                    krate,
                    names.join(", ")
                )
            })?;
        format!("//{}:...", found.dir.strip_prefix(&bazel_root).map_or(
            found.name.as_str(),
            |p| p.to_str().unwrap_or(found.name.as_str()),
        ))
    } else {
        // Default: test everything
        "//...".to_string()
    };

    let output_mode = if streamed { "streamed" } else { "errors" };

    println!("🧪 bazel test  →  {}", resolved_target);
    if let Some(f) = filter {
        println!("   filter: {}", f);
    }
    println!();

    run_bazel_test(&bazel_root, &resolved_target, filter, output_mode)
}

fn run_bazel_test(
    root: &Path,
    target: &str,
    filter: Option<&str>,
    output_mode: &str,
) -> Result<()> {
    let mut cmd = Command::new("bazel");
    cmd.arg("test")
        .arg(target)
        .arg("--test_output")
        .arg(output_mode)
        .arg("--keep_going")
        .current_dir(root);

    // Rust test filter maps to --test_arg=--exact --test_arg=<name>
    if let Some(f) = filter {
        cmd.arg("--test_arg=--exact");
        cmd.arg(format!("--test_arg={}", f));
    }

    let status = cmd
        .status()
        .context("Failed to run `bazel test` — is Bazel installed?")?;

    if !status.success() {
        anyhow::bail!("`bazel test {}` failed", target);
    }
    Ok(())
}
