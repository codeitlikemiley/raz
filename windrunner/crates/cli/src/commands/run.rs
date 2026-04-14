use anyhow::{Context, Result};
use cargo_runner_core::{Runnable, RunnableWithScore};
use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

use crate::commands::matching::selector_match_rank;
use crate::commands::workspace::resolve_module_path_to_file;
use crate::commands::workspace::{workspace_rs_files, workspace_scan_roots};
use crate::config::bazel_workspace::find_cargo_workspace_root;
use crate::utils::parse_filepath_with_line;

pub fn run_command(filepath_arg: &str, dry_run: bool) -> Result<()> {
    // Parse filepath and line number first
    let (filepath, line) = parse_filepath_with_line(filepath_arg);
    let cwd = std::env::current_dir()?;

    let mut runner = cargo_runner_core::UnifiedRunner::new()?;

    let command = match resolve_run_target(&mut runner, &cwd, &filepath)? {
        RunTarget::File(resolved_path) => {
            debug!(
                "Running file: {} at line: {:?}",
                resolved_path.display(),
                line
            );
            if line.is_none() {
                // For file-level commands (no line specified), use get_file_command
                // which has special logic to prefer test commands over doc tests
                runner
                    .get_file_command(&resolved_path)?
                    .ok_or_else(|| anyhow::anyhow!("No runnable found in file"))?
            } else {
                // For line-specific commands, use the regular method
                runner.get_command_at_position_with_dir(&resolved_path, line.map(|l| l as u32))?
            }
        }
        RunTarget::Runnable(runnable) => {
            debug!(
                "Running runnable selector: {:?} -> {:?}",
                filepath, runnable.label
            );
            runner
                .build_command_for_runnable(&runnable)?
                .ok_or_else(|| anyhow::anyhow!("No runnable found for selector"))?
        }
    };

    if dry_run {
        println!("{}", command.to_shell_command());
        if let Some(ref dir) = command.working_dir {
            println!("Working directory: {}", dir.display());
        }
        if !command.env.is_empty() {
            println!("Environment variables:");
            for (key, value) in &command.env {
                println!("  {key}={value}");
            }
        }
    } else {
        // Check for Bazel doc test limitation
        if let Some((_, msg)) = command
            .env
            .iter()
            .find(|(k, _)| k.as_str() == "_BAZEL_DOC_TEST_LIMITATION")
        {
            eprintln!("Note: {msg}");
            eprintln!("Running all doc tests for the crate instead.");
        }

        let shell_cmd = command.to_shell_command();
        info!("Running: {}", shell_cmd);
        if let Some(ref dir) = command.working_dir {
            info!("Working directory: {}", dir.display());
        }

        // Execute using the Command's execute method which handles working_dir
        let status = command
            .execute()
            .with_context(|| format!("Failed to execute: {shell_cmd}"))?;

        if !status.success() {
            std::process::exit(status.code().unwrap_or(1));
        }
    }

    Ok(())
}

#[derive(Debug)]
enum RunTarget {
    File(PathBuf),
    Runnable(Box<Runnable>),
}

#[derive(Debug)]
struct SelectorMatch {
    path: PathBuf,
    runnable: Runnable,
    rank: cargo_runner_core::RunnableWithScore,
    selector_rank: crate::commands::matching::SelectorMatchRank,
}

fn resolve_run_target(
    runner: &mut cargo_runner_core::UnifiedRunner,
    cwd: &Path,
    filepath: &str,
) -> Result<RunTarget> {
    let filepath_path = Path::new(filepath);
    let absolute_path = if filepath_path.is_absolute() {
        filepath_path.to_path_buf()
    } else {
        cwd.join(filepath_path)
    };

    if absolute_path.exists() {
        return Ok(RunTarget::File(absolute_path));
    }

    if filepath.contains("::") {
        if let Ok(module_path) = resolve_module_path_to_file(runner, filepath, cwd) {
            debug!(
                "Resolved module path '{}' to file '{}'",
                filepath,
                module_path.display()
            );
            return Ok(RunTarget::File(module_path));
        }
    }

    let matches = find_selector_matches(runner, cwd, filepath)?;
    match matches.as_slice() {
        [] => Err(anyhow::anyhow!(
            "No runnable found for selector: {filepath}"
        )),
        [only] => Ok(RunTarget::Runnable(Box::new(only.runnable.clone()))),
        [first, second, ..] if first.selector_rank == second.selector_rank => Err(anyhow::anyhow!(
            "Selector is ambiguous: {}. Matches: {}",
            filepath,
            format_selector_matches(&matches)
        )),
        [first, ..] => Ok(RunTarget::Runnable(Box::new(first.runnable.clone()))),
    }
}

fn find_selector_matches(
    runner: &mut cargo_runner_core::UnifiedRunner,
    cwd: &Path,
    selector: &str,
) -> Result<Vec<SelectorMatch>> {
    let workspace_root = find_cargo_workspace_root(cwd).unwrap_or_else(|| cwd.to_path_buf());
    let scan_roots = workspace_scan_roots(&workspace_root)?;

    let mut matches = Vec::new();
    for path in workspace_rs_files(&scan_roots) {
        let Ok(runnables) = runner.detect_runnables(&path) else {
            continue;
        };

        for runnable in runnables {
            if let Some(selector_rank) = selector_match_rank(selector, &runnable) {
                matches.push(SelectorMatch {
                    path: path.clone(),
                    rank: RunnableWithScore::new(runnable.clone()),
                    runnable,
                    selector_rank,
                });
            }
        }
    }

    matches.sort_by(compare_selector_matches);
    Ok(matches)
}

fn compare_selector_matches(a: &SelectorMatch, b: &SelectorMatch) -> Ordering {
    a.selector_rank
        .cmp(&b.selector_rank)
        .then_with(|| a.rank.cmp(&b.rank))
        .then_with(|| a.path.cmp(&b.path))
        .then_with(|| a.runnable.label.cmp(&b.runnable.label))
}

fn format_selector_matches(matches: &[SelectorMatch]) -> String {
    matches
        .iter()
        .map(|m| format!("{} ({})", m.runnable.label, m.path.display()))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_not_found() {
        let result = run_command("nonexistent.rs", true);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No runnable found for selector")
        );
    }
}
