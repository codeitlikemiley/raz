use anyhow::{Context, Result};
use tracing::{debug, info};

use crate::utils::parse_filepath_with_line;

pub fn run_command(filepath_arg: &str, dry_run: bool) -> Result<()> {
    // Parse filepath and line number first
    let (filepath, line) = parse_filepath_with_line(filepath_arg);
    
    // Check if file exists - resolve to absolute path
    let filepath_path = std::path::Path::new(&filepath);
    let absolute_path = if filepath_path.is_absolute() {
        filepath_path.to_path_buf()
    } else {
        std::env::current_dir()?.join(filepath_path)
    };
    
    if !absolute_path.exists() {
        return Err(anyhow::anyhow!("File not found: {}", absolute_path.display()));
    }

    debug!("Running file: {} at line: {:?}", filepath, line);

    let mut runner = cargo_runner_core::UnifiedRunner::new()?;
    let filepath_path = std::path::Path::new(&filepath);
    let command = if line.is_none() {
        // For file-level commands (no line specified), use get_file_command
        // which has special logic to prefer test commands over doc tests
        runner
            .get_file_command(filepath_path)?
            .ok_or_else(|| anyhow::anyhow!("No runnable found in file"))?
    } else {
        // For line-specific commands, use the regular method
        runner.get_command_at_position_with_dir(filepath_path, line.map(|l| l as u32))?
    };

    if dry_run {
        println!("{}", command.to_shell_command());
        if let Some(ref dir) = command.working_dir {
            println!("Working directory: {}", dir);
        }
        if !command.env.is_empty() {
            println!("Environment variables:");
            for (key, value) in &command.env {
                println!("  {}={}", key, value);
            }
        }
    } else {
        // Check for Bazel doc test limitation
        if let Some((_, msg)) = command.env.iter().find(|(k, _)| k == "_BAZEL_DOC_TEST_LIMITATION") {
            eprintln!("Note: {}", msg);
            eprintln!("Running all doc tests for the crate instead.");
        }
        
        let shell_cmd = command.to_shell_command();
        info!("Running: {}", shell_cmd);
        if let Some(ref dir) = command.working_dir {
            info!("Working directory: {}", dir);
        }

        // Execute using the CargoCommand's execute method which handles working_dir
        let status = command
            .execute()
            .with_context(|| format!("Failed to execute: {}", shell_cmd))?;

        if !status.success() {
            std::process::exit(status.code().unwrap_or(1));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_not_found() {
        let result = run_command("nonexistent.rs", true);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("File not found"));
    }
}
