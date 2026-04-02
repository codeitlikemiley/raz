//! `cargo runner bazel-query [EXPR] [OPTIONS]`
//!
//! A thin, friendly wrapper around `bazel query` for discovering targets
//! without needing to remember Bazel query syntax.
//!
//! Defaults to `//...` (all targets) when no expression is provided.
//!
//! ## Examples
//!
//! ```text
//! cargo runner bazel-query               # list all targets
//! cargo runner bazel-query //:example2   # inspect a specific target
//! cargo runner bazel-query --tests       # list only test targets
//! cargo runner bazel-query --bins        # list only binary targets
//! cargo runner bazel-query 'deps(//:my_bin)' --output label_kind
//! ```

use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

use crate::config::bazel_workspace::find_module_bazel;

pub fn bazel_query_command(
    expr: Option<&str>,
    output: &str,
    tests_only: bool,
    bins_only: bool,
) -> Result<()> {
    let cwd = std::env::current_dir().context("Failed to get current directory")?;
    let bazel_root = find_module_bazel(&cwd).with_context(|| {
        format!(
            "No MODULE.bazel found from {}.\n\
             Make sure you are inside a Bzlmod Bazel workspace.",
            cwd.display()
        )
    })?;

    // Build the query expression
    let query_expr = build_query_expr(expr, tests_only, bins_only);

    println!("🔍 bazel query  →  {}", query_expr);
    println!();

    run_bazel_query(&bazel_root, &query_expr, output)
}

fn build_query_expr(expr: Option<&str>, tests_only: bool, bins_only: bool) -> String {
    // If user supplied an explicit expression, use it as-is
    if let Some(e) = expr {
        return e.to_string();
    }

    // Filter shortcuts
    if tests_only {
        // All rust_test targets in the workspace
        return "kind('rust_test', //...)".to_string();
    }
    if bins_only {
        // All rust_binary targets in the workspace
        return "kind('rust_binary', //...)".to_string();
    }

    // Default: all targets
    "//...".to_string()
}

fn run_bazel_query(root: &Path, expr: &str, output: &str) -> Result<()> {
    let mut cmd = Command::new("bazel");
    cmd.arg("query")
        .arg(expr)
        .arg("--output")
        .arg(output)
        .arg("--noshow_progress") // cleaner output for listing
        .current_dir(root);

    let status = cmd
        .status()
        .context("Failed to run `bazel query` — is Bazel installed?")?;

    if !status.success() {
        anyhow::bail!("`bazel query {}` failed", expr);
    }
    Ok(())
}
