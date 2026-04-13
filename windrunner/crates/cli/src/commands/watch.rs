//! `cargo runner watch [FILE]` — Unified file watcher for any project type.
//!
//! Watches `src/` (or the file's directory) for `.rs` changes and automatically
//! triggers a build, test, or run. Auto-detects whether to use Cargo or Bazel.
//!
//! - **Cargo project**: delegates to `cargo watch` if installed, otherwise uses
//!   a notify-based fallback calling `cargo run / test / build`.
//! - **Bazel project**: notify-based watcher calling `bazel run / test / build`.
//!
//! ## Examples
//!
//! ```text
//! cargo runner watch              # watch + build on change (auto-detect)
//! cargo runner watch src/main.rs  # watch the file's src/ directory
//! cargo runner watch --run        # watch + run on change
//! cargo runner watch --test       # watch + test on change
//! cargo runner watch --debounce 500
//! ```

use anyhow::{Context, Result};
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::path::Path;
use std::process::Command;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::config::bazel_workspace::{find_bazel_crates, find_module_bazel};

pub fn watch_command(
    filepath: Option<&str>,
    run_mode: bool,
    test_mode: bool,
    debounce_ms: u64,
) -> Result<()> {
    let cwd = std::env::current_dir().context("Failed to get current directory")?;

    // Determine watch directory from file path or cwd
    let watch_dir = if let Some(fp) = filepath {
        let p = std::path::Path::new(fp);
        if p.is_file() {
            p.parent().unwrap_or(&cwd).to_path_buf()
        } else {
            p.to_path_buf()
        }
    } else if cwd.join("src").is_dir() {
        cwd.join("src")
    } else {
        cwd.clone()
    };

    // Detect project type
    if let Some(bazel_root) = find_module_bazel(&cwd) {
        let target = resolve_bazel_target(&bazel_root, &cwd);
        let mode = mode_str(run_mode, test_mode);
        println!("👀 [Bazel] Watching: {}", watch_dir.display());
        println!("   target: {target}  |  mode: bazel {mode}");
        run_notify_loop(&watch_dir, debounce_ms, move |_changed| {
            bazel_trigger(&bazel_root, &target, mode);
        })
    } else {
        let mode = mode_str(run_mode, test_mode);
        // Try cargo-watch first
        if cargo_watch_available() {
            println!("👀 [Cargo] Watching via cargo-watch …");
            return run_cargo_watch(&cwd, mode);
        }
        println!("👀 [Cargo] Watching: {}", watch_dir.display());
        println!("   mode: cargo {}", cargo_mode_cmd(mode));
        run_notify_loop(&watch_dir, debounce_ms, move |_changed| {
            cargo_trigger(&cwd, mode);
        })
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn mode_str(run_mode: bool, test_mode: bool) -> &'static str {
    if run_mode {
        "run"
    } else if test_mode {
        "test"
    } else {
        "build"
    }
}

fn cargo_mode_cmd(mode: &str) -> &str {
    match mode {
        "run" => "run",
        "test" => "test",
        _ => "build",
    }
}

fn cargo_watch_available() -> bool {
    Command::new("cargo")
        .args(["watch", "--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn run_cargo_watch(root: &Path, mode: &str) -> Result<()> {
    let x_arg = format!("-x{}", cargo_mode_cmd(mode));
    let status = Command::new("cargo")
        .args(["watch", &x_arg])
        .current_dir(root)
        .status()
        .context("Failed to run `cargo watch`")?;
    if !status.success() {
        anyhow::bail!("`cargo watch` exited with error");
    }
    Ok(())
}

fn resolve_bazel_target(bazel_root: &Path, cwd: &Path) -> String {
    if let Ok(all) = find_bazel_crates(bazel_root) {
        if let Some(krate) = all
            .into_iter()
            .find(|c| cwd.starts_with(&c.dir) || c.dir == cwd)
        {
            let rel = krate.dir.strip_prefix(bazel_root).unwrap_or(Path::new(""));
            let s = rel.to_string_lossy();
            return if s.is_empty() {
                "//...".to_string()
            } else {
                format!("//{s}:...")
            };
        }
    }
    "//...".to_string()
}

fn bazel_trigger(root: &Path, target: &str, mode: &str) {
    println!("─────────────────────────────────────────");
    let mut cmd = Command::new("bazel");
    cmd.arg(mode).arg(target).current_dir(root);
    if mode == "test" {
        cmd.arg("--test_output=errors");
    }
    match cmd.status() {
        Ok(s) if s.success() => println!("✅ bazel {mode} succeeded"),
        Ok(_) => println!("❌ bazel {mode} failed — waiting for next change …"),
        Err(e) => println!("⚠️  Failed to run bazel: {e}"),
    }
}

fn cargo_trigger(root: &Path, mode: &str) {
    println!("─────────────────────────────────────────");
    let cmd_name = cargo_mode_cmd(mode);
    match Command::new("cargo")
        .arg(cmd_name)
        .current_dir(root)
        .status()
    {
        Ok(s) if s.success() => println!("✅ cargo {cmd_name} succeeded"),
        Ok(_) => println!("❌ cargo {cmd_name} failed — waiting for next change …"),
        Err(e) => println!("⚠️  Failed to run cargo: {e}"),
    }
}

fn run_notify_loop<F>(watch_dir: &Path, debounce_ms: u64, trigger: F) -> Result<()>
where
    F: Fn(&str) + Send + 'static,
{
    println!("Press Ctrl-C to stop.\n");

    // Trigger once immediately
    trigger("(initial)");

    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
    let mut watcher =
        notify::recommended_watcher(tx).context("Failed to create filesystem watcher")?;
    watcher
        .watch(watch_dir, RecursiveMode::Recursive)
        .with_context(|| format!("Failed to watch {}", watch_dir.display()))?;

    let debounce = Duration::from_millis(debounce_ms);
    let mut last_trigger = Instant::now() - debounce;

    loop {
        match rx.recv() {
            Ok(Ok(event)) => {
                if !is_rust_change(&event) {
                    continue;
                }
                if last_trigger.elapsed() < debounce {
                    continue;
                }
                last_trigger = Instant::now();

                let files: Vec<_> = event
                    .paths
                    .iter()
                    .map(|p| {
                        p.file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string()
                    })
                    .collect();
                println!("📝 Changed: {}", files.join(", "));
                trigger(&files.join(", "));
            }
            Ok(Err(e)) => eprintln!("⚠️  Watcher error: {e}"),
            Err(_) => break,
        }
    }
    Ok(())
}

fn is_rust_change(event: &Event) -> bool {
    matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_))
        && event
            .paths
            .iter()
            .any(|p| p.extension().is_some_and(|e| e == "rs"))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── mode_str ──────────────────────────────────────────────────

    #[test]
    fn mode_str_run() {
        assert_eq!(mode_str(true, false), "run");
    }

    #[test]
    fn mode_str_test() {
        assert_eq!(mode_str(false, true), "test");
    }

    #[test]
    fn mode_str_default_build() {
        assert_eq!(mode_str(false, false), "build");
    }

    #[test]
    fn mode_str_run_takes_priority() {
        // If both are true, run wins (evaluated first)
        assert_eq!(mode_str(true, true), "run");
    }

    // ── cargo_mode_cmd ────────────────────────────────────────────

    #[test]
    fn cargo_mode_cmd_run() {
        assert_eq!(cargo_mode_cmd("run"), "run");
    }

    #[test]
    fn cargo_mode_cmd_test() {
        assert_eq!(cargo_mode_cmd("test"), "test");
    }

    #[test]
    fn cargo_mode_cmd_build_default() {
        assert_eq!(cargo_mode_cmd("build"), "build");
    }

    #[test]
    fn cargo_mode_cmd_unknown_defaults_to_build() {
        assert_eq!(cargo_mode_cmd("anything"), "build");
    }
}
