//! `cargo runner watch [OPTIONS]`
//!
//! Watch a crate's `src/` directory for changes and automatically trigger
//! a Bazel build, test, or run on every save.
//!
//! ## Examples
//!
//! ```text
//! cargo runner watch             # watch + bazel build on change
//! cargo runner watch --test      # watch + bazel test on change
//! cargo runner watch --run       # watch + bazel run on change
//! cargo runner watch --target //:my_bin --run
//! cargo runner watch --debounce 500   # wait 500ms before re-triggering
//! ```

use anyhow::{Context, Result};
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::path::Path;
use std::process::Command;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::config::bazel_workspace::{find_bazel_crates, find_module_bazel};

pub fn watch_command(
    target: Option<&str>,
    run_mode: bool,
    test_mode: bool,
    debounce_ms: u64,
) -> Result<()> {
    let cwd = std::env::current_dir().context("Failed to get current directory")?;
    let bazel_root = find_module_bazel(&cwd).with_context(|| {
        format!(
            "No MODULE.bazel found from {}.\n\
             Make sure you are inside a Bzlmod Bazel workspace.",
            cwd.display()
        )
    })?;

    // Resolve Bazel target for the current crate
    let resolved_target = if let Some(t) = target {
        t.to_string()
    } else {
        resolve_local_target(&bazel_root, &cwd)?
    };

    // Determine the watch directory (prefer src/ of the current crate, fall back to cwd)
    let watch_dir = if cwd.join("src").is_dir() {
        cwd.join("src")
    } else {
        cwd.clone()
    };

    let mode = if run_mode {
        "run"
    } else if test_mode {
        "test"
    } else {
        "build"
    };

    println!("👀 Watching: {}", watch_dir.display());
    println!("   target:  {}", resolved_target);
    println!("   mode:    bazel {}", mode);
    println!("   debounce: {}ms", debounce_ms);
    println!();
    println!("Press Ctrl-C to stop.");
    println!();

    // Run once immediately on start
    trigger(&bazel_root, &resolved_target, mode);

    // Set up file watcher
    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
    let mut watcher = notify::recommended_watcher(tx)
        .context("Failed to create filesystem watcher")?;
    watcher
        .watch(&watch_dir, RecursiveMode::Recursive)
        .with_context(|| format!("Failed to watch {}", watch_dir.display()))?;

    let debounce = Duration::from_millis(debounce_ms);
    let mut last_trigger = Instant::now() - debounce; // allow immediate first event

    loop {
        match rx.recv() {
            Ok(Ok(event)) => {
                if !is_rust_change(&event) {
                    continue;
                }
                // Debounce: skip if we fired recently
                if last_trigger.elapsed() < debounce {
                    continue;
                }
                last_trigger = Instant::now();

                let changed: Vec<_> = event
                    .paths
                    .iter()
                    .map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string())
                    .collect();
                println!("📝 Changed: {}  →  bazel {} {}", changed.join(", "), mode, resolved_target);
                trigger(&bazel_root, &resolved_target, mode);
            }
            Ok(Err(e)) => eprintln!("⚠️  Watcher error: {}", e),
            Err(_) => break, // channel closed
        }
    }

    Ok(())
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Returns true if the event is a write/create to a `.rs` file.
fn is_rust_change(event: &Event) -> bool {
    matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(_)
    ) && event.paths.iter().any(|p| {
        p.extension().map_or(false, |ext| ext == "rs")
    })
}

/// Trigger a bazel build/test/run and print a result banner.
fn trigger(root: &Path, target: &str, mode: &str) {
    println!("─────────────────────────────────────────");
    let mut cmd = Command::new("bazel");
    cmd.arg(mode).arg(target).current_dir(root);

    // For test mode, default to streaming output so failures are visible
    if mode == "test" {
        cmd.arg("--test_output=errors");
    }

    match cmd.status() {
        Ok(s) if s.success() => println!("✅ bazel {} succeeded", mode),
        Ok(_) => println!("❌ bazel {} failed — waiting for next change…", mode),
        Err(e) => println!("⚠️  Failed to run bazel: {}", e),
    }
}

/// Find the Bazel target that corresponds to the current working directory.
fn resolve_local_target(bazel_root: &Path, cwd: &Path) -> Result<String> {
    let all = find_bazel_crates(bazel_root)
        .context("Failed to scan workspace for Bazel crates")?;

    // Find the crate whose dir matches or is a parent of cwd
    let local = all
        .into_iter()
        .find(|c| cwd.starts_with(&c.dir) || c.dir == cwd);

    if let Some(krate) = local {
        // Construct target path relative to bazel_root  e.g. //server:...
        let rel = krate
            .dir
            .strip_prefix(bazel_root)
            .unwrap_or(Path::new(""));
        let rel_str = rel.to_string_lossy();
        if rel_str.is_empty() {
            Ok("//...".to_string())
        } else {
            Ok(format!("//{}:...", rel_str))
        }
    } else {
        // Fallback: watch the whole workspace
        Ok("//...".to_string())
    }
}
