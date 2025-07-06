//! RAZ Command Line Interface
//!
//! Universal command generator for Rust projects with intelligent override parsing

use clap::{Parser, Subcommand};
use colored::*;
use raz_common::output::OutputFormatter;
use raz_core::{Command, Position, RazCore};
use raz_override::{OverrideCli, OverrideMigrator, ParsedOverrides, SmartOverrideParser};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

#[derive(Parser)]
#[command(name = "raz")]
#[command(about = "Universal command generator for Rust projects")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Working directory (defaults to current directory)
    #[arg(short = 'C', long, value_name = "DIR")]
    directory: Option<PathBuf>,

    /// Save override to config file for future use
    #[arg(long)]
    save_override: bool,

    /// Print the command that will be executed without running it
    #[arg(long)]
    dry_run: bool,

    /// Subcommand to run
    #[command(subcommand)]
    command: Option<Commands>,

    /// Direct arguments: file path followed by overrides (when no subcommand)
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    direct_args: Vec<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize configuration for the current project
    Init {
        /// Template to use (default, web, desktop, game, library)
        #[arg(long, default_value = "default")]
        template: String,

        /// Force overwrite existing configuration
        #[arg(long)]
        force: bool,
    },
    /// Override management commands
    Override {
        #[command(subcommand)]
        cmd: OverrideCommands,
    },
    /// Update RAZ to the latest version
    #[command(name = "self-update")]
    SelfUpdate {
        /// Force update even if already on latest version
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum OverrideCommands {
    /// List all overrides
    List {
        /// Show overrides for a specific file
        #[arg(short, long)]
        file: Option<PathBuf>,
    },
    /// Inspect a specific override
    Inspect {
        /// Override key to inspect
        key: String,
    },
    /// Debug resolution at a specific location
    Debug {
        /// File path
        file: PathBuf,
        /// Line number
        line: usize,
        /// Column number (optional)
        column: Option<usize>,
    },
    /// Show override statistics
    Stats,
    /// Export overrides
    Export {
        /// Output file path (prints to stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Import overrides from a file
    Import {
        /// Input file path
        file: PathBuf,
    },
    /// Delete an override
    Delete {
        /// Override key to delete
        key: String,
    },
    /// Clear all overrides
    Clear {
        /// Force clear without confirmation
        #[arg(short, long)]
        force: bool,
        /// Clear overrides for a specific file
        #[arg(long)]
        file: Option<PathBuf>,
    },
    /// Migrate legacy overrides to new format
    Migrate {
        /// Legacy override file to migrate
        #[arg(short, long)]
        file: Option<PathBuf>,
        /// Perform a dry run without making changes
        #[arg(long)]
        dry_run: bool,
        /// Auto-detect and migrate legacy overrides in workspace
        #[arg(long)]
        auto: bool,
    },
    /// Update execution status of an override
    UpdateStatus {
        /// Override key
        key: String,
        /// Execution result (success or failure)
        result: String,
    },
    /// Rollback to the last backup
    Rollback {
        /// Force rollback without confirmation
        #[arg(short, long)]
        force: bool,
    },
    /// List available backups
    ListBackups,
}

// Legacy init command support for template-based project setup

fn parse_file_and_cursor(s: &str) -> Result<(PathBuf, Option<Position>), String> {
    // Check if the string contains a cursor position
    let parts: Vec<&str> = s.rsplitn(3, ':').collect();

    match parts.len() {
        3 => {
            // file:line:column format
            let column = parts[0]
                .parse::<u32>()
                .map_err(|_| "Invalid column number")?;
            let line = parts[1].parse::<u32>().map_err(|_| "Invalid line number")?;
            let file_path = PathBuf::from(parts[2]);

            Ok((file_path, Some(Position { line, column })))
        }
        2 => {
            // Might be file:line or Windows path like C:\file
            if let Ok(line) = parts[0].parse::<u32>() {
                let file_path = PathBuf::from(parts[1]);
                Ok((file_path, Some(Position { line, column: 1 })))
            } else {
                // Not a line number, treat whole string as path
                Ok((PathBuf::from(s), None))
            }
        }
        _ => {
            // Just a file path
            Ok((PathBuf::from(s), None))
        }
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli).await {
        eprintln!("{}", OutputFormatter::error(e));
        process::exit(1);
    }
}

async fn run(cli: Cli) -> anyhow::Result<()> {
    let working_dir = cli
        .directory
        .unwrap_or_else(|| env::current_dir().expect("Failed to get current directory"));

    // Handle subcommands first
    if let Some(command) = cli.command {
        match command {
            Commands::Init { template, force } => {
                return init_config(&working_dir, &template, force);
            }
            Commands::Override { cmd } => {
                return handle_override_command(&working_dir, cmd);
            }
            Commands::SelfUpdate { force } => {
                return handle_self_update(force).await;
            }
        }
    }

    let raz = RazCore::new()?;

    // Handle direct execution syntax
    if !cli.direct_args.is_empty() {
        let file_arg = &cli.direct_args[0];
        let (file_path, cursor) =
            parse_file_and_cursor(file_arg).map_err(|e| anyhow::anyhow!(e))?;

        let override_args = cli.direct_args[1..].join(" ");
        return run_smart_command_simplified(
            &raz,
            &working_dir,
            SimplifiedCommandParams {
                file_path,
                cursor,
                verbose: cli.verbose,
                save_override: cli.save_override,
                dry_run: cli.dry_run,
                override_args,
            },
        )
        .await;
    }

    // No arguments provided - show help
    eprintln!("{}", OutputFormatter::warning("Usage"));
    eprintln!("  raz [OPTIONS] <file>[:<line>:<column>] [overrides...]");
    eprintln!("\n{}", "Examples:".bright_black());
    eprintln!("  raz src/main.rs");
    eprintln!("  raz src/lib.rs:25:1");
    eprintln!("  raz src/main.rs RUST_BACKTRACE=full --release");
    eprintln!("  raz src/lib.rs:25:1 -- --exact --nocapture");
    eprintln!("  raz --save-override src/lib.rs:10:1 --quiet");
    eprintln!("\n{}", "Options:".bright_black());
    eprintln!("  --save-override    Save the override for future use");
    eprintln!("  --dry-run          Show what would be executed");
    eprintln!("  --verbose          Show detailed output");
    eprintln!("\n{}", "For more options, use 'raz --help'".bright_black());

    Ok(())
}

/// Parameters for simplified command execution
struct SimplifiedCommandParams {
    file_path: PathBuf,
    cursor: Option<Position>,
    verbose: bool,
    save_override: bool,
    dry_run: bool,
    override_args: String,
}

async fn run_smart_command_simplified(
    raz: &RazCore,
    working_dir: &Path,
    params: SimplifiedCommandParams,
) -> anyhow::Result<()> {
    let SimplifiedCommandParams {
        file_path,
        cursor,
        verbose,
        save_override,
        dry_run,
        override_args,
    } = params;
    // Detect subcommand from context
    let subcommand = detect_subcommand(&file_path, cursor);

    // Parse overrides using smart parser
    let parser = SmartOverrideParser::new(subcommand);
    let overrides = parser.parse(&override_args);

    if verbose {
        println!("{} Using simplified syntax", OutputFormatter::label("Info"));
        if !overrides.env_vars.is_empty() {
            println!("{} Environment variables:", OutputFormatter::label("Info"));
            for (k, v) in &overrides.env_vars {
                println!("  {} = {}", OutputFormatter::key(k), v);
            }
        }
        if !overrides.options.is_empty() {
            println!(
                "{} Options: {}",
                OutputFormatter::label("Info"),
                overrides.options.join(" ")
            );
        }
        if !overrides.args.is_empty() {
            println!(
                "{} Arguments: {}",
                OutputFormatter::label("Info"),
                overrides.args.join(" ")
            );
        }
    }

    // Run with parsed overrides
    run_smart_command_with_parsed_overrides(
        raz,
        working_dir,
        RunCommandParams {
            file_path,
            cursor,
            verbose,
            save_override,
            dry_run,
            overrides,
        },
    )
    .await
}

// Legacy function removed - only simplified syntax supported

/// Parameters for running a smart command
struct RunCommandParams {
    file_path: PathBuf,
    cursor: Option<Position>,
    verbose: bool,
    save_override: bool,
    dry_run: bool,
    overrides: ParsedOverrides,
}

async fn run_smart_command_with_parsed_overrides(
    raz: &RazCore,
    working_dir: &Path,
    params: RunCommandParams,
) -> anyhow::Result<()> {
    let RunCommandParams {
        file_path,
        cursor,
        verbose,
        save_override,
        dry_run,
        overrides,
    } = params;
    let full_path = if file_path.is_absolute() {
        file_path
    } else {
        working_dir.join(&file_path)
    };

    if !full_path.exists() {
        anyhow::bail!("File not found: {}", full_path.display());
    }

    if verbose {
        println!(
            "{} Analyzing file: {}",
            OutputFormatter::label("Info"),
            full_path.display()
        );
        if let Some(pos) = cursor {
            println!(
                "{} Cursor position: {}:{}",
                OutputFormatter::label("Info"),
                pos.line,
                pos.column
            );
        }
    }

    // Convert 1-based cursor position to 0-based for internal processing
    let internal_cursor = cursor.map(|pos| Position {
        line: pos.line.saturating_sub(1),
        column: pos.column.saturating_sub(1),
    });

    // Platform overrides are now handled through the standard override system

    // Generate base commands - RazCore will automatically load saved overrides from config
    let mut commands = raz
        .generate_universal_commands(&full_path, internal_cursor)
        .await?;

    if commands.is_empty() {
        println!(
            "{}",
            OutputFormatter::warning("No applicable commands found for this file.")
        );
        return Ok(());
    }

    // Apply runtime overrides to the best command (if any were provided)
    let best_command = &mut commands[0];

    if !overrides.is_empty() {
        // Apply environment variables
        for (key, value) in &overrides.env_vars {
            best_command.env.insert(key.clone(), value.clone());
        }

        // Apply options - but avoid duplicates
        if !overrides.options.is_empty() {
            // Find insertion point before any -- separator
            let insert_pos = best_command
                .args
                .iter()
                .position(|arg| arg == "--")
                .unwrap_or(best_command.args.len());

            // Insert options, checking for duplicates
            let mut inserted_count = 0;
            let mut i = 0;
            while i < overrides.options.len() {
                let option = &overrides.options[i];

                // Check if this option already exists
                if option.starts_with("--") {
                    if let Some(existing_pos) =
                        best_command.args.iter().position(|arg| arg == option)
                    {
                        // Option already exists, check if we need to replace its value
                        if i + 1 < overrides.options.len()
                            && !overrides.options[i + 1].starts_with("-")
                        {
                            // There's a value for this option
                            let new_value = &overrides.options[i + 1];
                            if existing_pos + 1 < best_command.args.len()
                                && !best_command.args[existing_pos + 1].starts_with("-")
                            {
                                // Replace existing value
                                best_command.args[existing_pos + 1] = new_value.clone();
                            } else {
                                // Insert value after option
                                best_command
                                    .args
                                    .insert(existing_pos + 1, new_value.clone());
                            }
                            i += 2; // Skip both option and value
                        } else {
                            // Flag option already exists, skip
                            i += 1;
                        }
                        continue;
                    }
                }

                // Option doesn't exist, insert it
                best_command
                    .args
                    .insert(insert_pos + inserted_count, option.clone());
                inserted_count += 1;
                i += 1;
            }
        }

        // Apply args (after --)
        if !overrides.args.is_empty() {
            // Ensure -- separator exists
            if !best_command.args.contains(&"--".to_string()) {
                best_command.args.push("--".to_string());
            }

            // Add args after --
            best_command.args.extend(overrides.args.clone());
        }
    }

    if verbose {
        println!("\n{} Selected command:", OutputFormatter::label("Info"));
        println!(
            "  {} {}",
            OutputFormatter::success(&best_command.label),
            OutputFormatter::dim(format!("(priority: {})", best_command.priority))
        );
    }

    // Prepare override for deferred save (only save after successful execution)
    let pending_override = if save_override && !overrides.is_empty() {
        // We need to determine the workspace root
        let workspace = find_workspace_root(&full_path)?;

        // Use the new override system
        use raz_config::CommandOverride;
        use raz_override::OverrideSystem;

        let mut override_system = OverrideSystem::new(&workspace)?;

        // Create function context from cursor position
        // Use 0-based positions for tree-sitter (internal_cursor is already 0-based)
        let line_number = internal_cursor.map(|c| c.line as usize).unwrap_or(0);
        let column = internal_cursor.map(|c| c.column as usize);

        // Use override system to get proper function context (with auto-detection)
        let function_context =
            override_system.get_function_context(&full_path, line_number, column)?;

        // Generate stable key using the new system
        let override_key = override_system.generate_key(&function_context)?;

        // Create command override
        let mut command_override = CommandOverride::new(best_command.command.clone());

        // Add environment variables
        for (k, v) in &overrides.env_vars {
            command_override.env.insert(k.clone(), v.clone());
        }

        // Categorize options into cargo options and test arguments
        use raz_common::cargo_flags;

        let mut i = 0;
        while i < overrides.options.len() {
            let option = &overrides.options[i];

            if cargo_flags::is_cargo_flag(option) {
                command_override.cargo_options.push(option.clone());

                // Check if this flag takes a value
                if matches!(
                    option.as_str(),
                    "--target"
                        | "--profile"
                        | "--features"
                        | "--package"
                        | "-p"
                        | "--exclude"
                        | "--color"
                        | "--message-format"
                        | "--jobs"
                        | "-j"
                        | "--config"
                        | "-Z"
                        | "--bin"
                        | "--example"
                        | "--test"
                        | "--bench"
                ) {
                    // Next item might be the value for this flag
                    if i + 1 < overrides.options.len() {
                        let next = &overrides.options[i + 1];
                        if !next.starts_with('-') {
                            // It's a value, add it to cargo options
                            command_override.cargo_options.push(next.clone());
                            i += 1; // Skip the value in next iteration
                        }
                    }
                }
            } else {
                // Not a cargo flag, treat as test argument
                command_override.args.push(option.clone());
            }
            i += 1;
        }

        // Add explicit args (after --)
        command_override.args.extend(overrides.args.clone());

        // Validate the override but DON'T save it yet
        // We'll save it only after successful execution

        // Store the override data for later save
        Some((
            workspace,
            override_system,
            override_key,
            command_override,
            function_context,
        ))
    } else {
        None
    };

    // Handle dry run
    if dry_run {
        println!(
            "\n{} {}",
            OutputFormatter::warning("Would execute"),
            best_command.label
        );
        println!(
            "{} {} {}",
            OutputFormatter::dim(">"),
            best_command.command,
            best_command.args.join(" ")
        );

        if !best_command.env.is_empty() {
            println!("{} Environment variables:", OutputFormatter::dim("With:"));
            for (k, v) in &best_command.env {
                println!("  {k}={v}");
            }
        }

        return Ok(());
    }

    // Extract override key for tracking (reuse from pending_override if available)
    let override_key = if let Some((_, _, ref override_key, _, _)) = pending_override {
        Some(override_key.primary.clone())
    } else if !overrides.is_empty() {
        // Generate key only if we don't have one from pending_override
        let workspace = find_workspace_root(&full_path)?;
        let mut override_system = raz_override::OverrideSystem::new(&workspace)?;
        let line_number = internal_cursor.map(|c| c.line as usize).unwrap_or(0);
        let column = internal_cursor.map(|c| c.column as usize);
        let function_context =
            override_system.get_function_context(&full_path, line_number, column)?;
        let key = override_system.generate_key(&function_context)?;
        Some(key.primary)
    } else {
        None
    };

    // Execute the command and track result
    match execute_command(best_command).await {
        Ok(_) => {
            // Command succeeded! Now save the override if requested
            if let Some((
                _workspace,
                override_system,
                override_key,
                command_override,
                function_context,
            )) = pending_override
            {
                // Save the override NOW after successful execution
                match override_system.save_override_with_validation(
                    override_key.clone(),
                    command_override,
                    &function_context,
                    &best_command.command,
                ) {
                    Ok(_) => {
                        println!(
                            "\n{} Override saved after successful execution",
                            OutputFormatter::success("Success")
                        );
                        println!(
                            "{} Key: {}",
                            OutputFormatter::label("Info"),
                            override_key.primary
                        );

                        // Show what function was detected
                        if let Some(func_name) = function_context.function_name {
                            println!("{} Function: {}", OutputFormatter::label("Info"), func_name);
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "\n{} Failed to save override after execution: {}",
                            OutputFormatter::error("Error"),
                            e
                        );
                    }
                }
            }

            // Update execution status if we used an override
            if let Some(key) = override_key {
                let workspace = find_workspace_root(&full_path)?;
                let override_system = raz_override::OverrideSystem::new(&workspace)?;
                let _ = override_system.update_execution_status(&key, true);
            }
            Ok(())
        }
        Err(e) => {
            // Command failed - do NOT save the override
            if let Some((_, _, _, _, _)) = pending_override {
                eprintln!(
                    "\n{} Command failed. Override was NOT saved.",
                    OutputFormatter::warning("Warning")
                );
            }

            // Update execution status and potentially rollback existing overrides
            // Note: With deferred save, failed commands don't create new overrides to rollback,
            // but existing overrides that were applied and failed should still be tracked.
            if let Some(key) = override_key {
                let workspace = find_workspace_root(&full_path)?;
                let override_system = raz_override::OverrideSystem::new(&workspace)?;
                let _ = override_system.update_execution_status(&key, false);

                // Check if we should prompt for rollback
                let config = raz_config::GlobalConfig::load()?;
                let auto_rollback_threshold = config
                    .overrides
                    .as_ref()
                    .map(|o| o.auto_rollback_threshold)
                    .unwrap_or(3);

                if override_system.should_auto_rollback(&key, auto_rollback_threshold)? {
                    eprintln!(
                        "\n{} The override used for this command has failed {} times.",
                        OutputFormatter::warning("Warning"),
                        auto_rollback_threshold
                    );

                    // Prompt for rollback
                    if prompt_for_rollback()? {
                        override_system.rollback_to_last_backup()?;
                        eprintln!(
                            "{} Override reverted to previous state",
                            OutputFormatter::success("Success")
                        );
                    }
                }
            }
            Err(e)
        }
    }
}

async fn execute_command(command: &Command) -> anyhow::Result<()> {
    println!("\n{} {}", OutputFormatter::info("Executing"), command.label);
    println!(
        "{} {} {}",
        OutputFormatter::dim(">"),
        command.command,
        command.args.join(" ")
    );

    // Change to the command's working directory if specified
    let original_dir = env::current_dir()?;
    if let Some(ref cwd) = command.cwd {
        env::set_current_dir(cwd)?;
    }

    // Set environment variables
    for (key, value) in &command.env {
        unsafe {
            env::set_var(key, value);
        }
    }

    // Execute the command
    let status = process::Command::new(&command.command)
        .args(&command.args)
        .status()?;

    // Restore original directory
    env::set_current_dir(original_dir)?;

    if !status.success() {
        let code = status.code().unwrap_or(-1);
        anyhow::bail!("Command failed with exit code: {}", code);
    }

    Ok(())
}

fn prompt_for_rollback() -> anyhow::Result<bool> {
    use std::io::{self, Write};

    println!("\nWould you like to:");
    println!("  1. Revert this override to the previous state");
    println!("  2. Keep the override but mark it as problematic");
    println!("  3. Edit the override");
    println!();
    print!("Choice [1-3] (default: 2): ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let choice = input.trim();

    match choice {
        "1" | "" => Ok(true), // Default to keeping but marking problematic
        "2" => Ok(false),
        "3" => {
            println!("Edit functionality not yet implemented. Keeping current override.");
            Ok(false)
        }
        _ => Ok(false),
    }
}

fn find_workspace_root(path: &Path) -> anyhow::Result<PathBuf> {
    // Start from the file's directory
    let mut current = if path.is_file() {
        path.parent().unwrap_or(path)
    } else {
        path
    };

    // Look for Cargo.toml or .raz directory
    loop {
        if current.join("Cargo.toml").exists() || current.join(".raz").exists() {
            return Ok(current.to_path_buf());
        }

        match current.parent() {
            Some(parent) => current = parent,
            None => {
                // No workspace found, use the file's directory
                return Ok(path.parent().unwrap_or(path).to_path_buf());
            }
        }
    }
}

fn detect_subcommand(file_path: &Path, cursor: Option<Position>) -> String {
    let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    // Simple heuristics - this should be improved with actual file analysis
    if file_name == "main.rs" || file_name.ends_with("_bin.rs") {
        "run".to_string()
    } else if file_name.contains("test") || file_name.contains("spec") || cursor.is_some() {
        "test".to_string()
    } else if file_name.contains("bench") {
        "bench".to_string()
    } else if file_path.starts_with("src") {
        // Source files default to run if they have main, otherwise test
        "run".to_string()
    } else {
        "run".to_string()
    }
}

/// Handle self-update command
async fn handle_self_update(force: bool) -> anyhow::Result<()> {
    use std::process::Stdio;

    println!("{} Checking for updates...", OutputFormatter::info("RAZ"));

    // Get current version
    let current_version = env!("CARGO_PKG_VERSION");
    println!(
        "{} Current version: v{}",
        OutputFormatter::label("Info"),
        current_version
    );

    // Check if running from cargo install or system PATH
    let current_exe = env::current_exe()?;
    let exe_dir = current_exe.parent().unwrap();

    // Check if we're in a cargo bin directory
    let is_cargo_installed = exe_dir.to_string_lossy().contains(".cargo/bin")
        || exe_dir.to_string_lossy().contains(".cargo\\bin");

    if !is_cargo_installed {
        // Check if it's in system PATH but not cargo
        let which_cmd = if cfg!(windows) { "where" } else { "which" };
        let which_result = process::Command::new(which_cmd).arg("raz").output();

        if let Ok(output) = which_result {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() && !path.contains(".cargo") {
                println!(
                    "\n{} RAZ appears to be installed via a package manager or custom installation.",
                    OutputFormatter::warning("Note")
                );
                println!("Please update using the same method you used to install RAZ.");
                return Ok(());
            }
        }
    }

    // Get latest version from GitHub API
    let latest_version = get_latest_version().await?;
    println!(
        "{} Latest version: {}",
        OutputFormatter::label("Info"),
        latest_version
    );

    // Compare versions
    if !force && current_version == latest_version.trim_start_matches('v') {
        println!(
            "\n{} You are already on the latest version!",
            OutputFormatter::success("✓")
        );
        return Ok(());
    }

    // Prompt for confirmation
    if !force {
        print!("\nUpdate RAZ from v{current_version} to {latest_version}? [Y/n] ");
        use std::io::{self, Write};
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let choice = input.trim().to_lowercase();

        if choice != "y" && !choice.is_empty() {
            println!("Update cancelled.");
            return Ok(());
        }
    }

    // Perform update using cargo install
    println!("\n{} Updating RAZ...", OutputFormatter::info("Installing"));
    println!(
        "{} This may take a few minutes...",
        OutputFormatter::dim("Note")
    );

    let mut cmd = process::Command::new("cargo");
    cmd.arg("install")
        .arg("raz-cli")
        .arg("--force")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let status = cmd.status()?;

    if status.success() {
        println!(
            "\n{} RAZ has been successfully updated to {}!",
            OutputFormatter::success("Success"),
            latest_version
        );
        println!(
            "{} Run {} to verify the update.",
            OutputFormatter::info("Tip"),
            OutputFormatter::command("raz --version")
        );
    } else {
        anyhow::bail!("Failed to update RAZ. Please try again or install manually.");
    }

    Ok(())
}

/// Get latest version from GitHub releases
async fn get_latest_version() -> anyhow::Result<String> {
    // Get all releases, not just the "latest" (which might be wrong)
    let url = "https://api.github.com/repos/codeitlikemiley/raz/releases?per_page=20";

    // Use platform-appropriate HTTP client
    let output = if cfg!(windows) {
        // Try PowerShell on Windows
        process::Command::new("powershell")
            .args([
                "-Command",
                &format!(
                    "Invoke-WebRequest -Uri '{url}' -Headers @{{'Accept'='application/vnd.github.v3+json'}} -UseBasicParsing | Select-Object -ExpandProperty Content"
                ),
            ])
            .output()
            .or_else(|_| {
                // Fallback to curl if available on Windows
                process::Command::new("curl")
                    .args(["-s", "-H", "Accept: application/vnd.github.v3+json", url])
                    .output()
            })?
    } else {
        // Use curl on Unix-like systems
        process::Command::new("curl")
            .args(["-s", "-H", "Accept: application/vnd.github.v3+json", url])
            .output()?
    };

    if !output.status.success() {
        anyhow::bail!("Failed to fetch versions from GitHub");
    }

    let response = String::from_utf8(output.stdout)?;

    // Parse JSON to get all releases
    let releases: Vec<serde_json::Value> = serde_json::from_str(&response)?;

    if releases.is_empty() {
        anyhow::bail!("No releases found");
    }

    // Find the highest version that looks like a CLI release (not vscode-specific)
    let mut highest_version = String::new();
    let mut highest_semver = (0, 0, 0);

    for release in releases {
        if let Some(tag_name) = release["tag_name"].as_str() {
            // Skip pre-releases unless there are no stable releases
            if release["prerelease"].as_bool().unwrap_or(false) {
                continue;
            }

            // Skip VS Code specific releases
            if tag_name.contains("vscode") {
                continue;
            }

            // Parse semantic version (handle v prefix)
            let version_str = tag_name.trim_start_matches('v');
            if let Some((major, minor, patch)) = parse_semver(version_str) {
                if (major, minor, patch) > highest_semver {
                    highest_semver = (major, minor, patch);
                    highest_version = tag_name.to_string();
                }
            }
        }
    }

    if highest_version.is_empty() {
        anyhow::bail!("No valid CLI releases found");
    }

    Ok(highest_version)
}

/// Parse semantic version string into (major, minor, patch)
fn parse_semver(version: &str) -> Option<(u32, u32, u32)> {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() >= 3 {
        let major = parts[0].parse().ok()?;
        let minor = parts[1].parse().ok()?;
        // Handle patch versions that might have extra info (e.g., "0-alpha")
        let patch_str = parts[2].split('-').next()?;
        let patch = patch_str.parse().ok()?;
        Some((major, minor, patch))
    } else if parts.len() == 2 {
        // Handle versions like "0.1" as "0.1.0"
        let major = parts[0].parse().ok()?;
        let minor = parts[1].parse().ok()?;
        Some((major, minor, 0))
    } else {
        None
    }
}

/// Initialize configuration for the current project
fn init_config(working_dir: &Path, template: &str, force: bool) -> anyhow::Result<()> {
    // Auto-detect project type if using default template
    let auto_detected_template = if template == "default" {
        detect_project_template(working_dir)
    } else {
        template.to_string()
    };

    if auto_detected_template != template {
        println!(
            "{} Auto-detected project type: {}",
            OutputFormatter::label("Info"),
            OutputFormatter::command(&auto_detected_template)
        );
        println!(
            "{} Use {} to override",
            OutputFormatter::info("Tip"),
            OutputFormatter::command(format!("--template {template}"))
        );
    }
    let config_dir = working_dir.join(".raz");
    let config_file = config_dir.join("config.toml");

    if config_file.exists() && !force {
        return Err(anyhow::anyhow!(
            "Configuration already exists. Use --force to overwrite."
        ));
    }

    fs::create_dir_all(&config_dir)?;

    // Get template content and description
    let (config_content, template_description) = match auto_detected_template.as_str() {
        "web" => (
            include_str!("../templates/web.toml"),
            "Web Development (Leptos, Dioxus, Yew)",
        ),
        "game" => (
            include_str!("../templates/game.toml"),
            "Game Development (Bevy)",
        ),
        "library" => (
            include_str!("../templates/library.toml"),
            "Library Development (focus on docs and testing)",
        ),
        "desktop" => (
            include_str!("../templates/desktop.toml"),
            "Desktop Applications (Tauri, Egui)",
        ),
        "default" => (
            include_str!("../templates/default.toml"),
            "General Rust Development",
        ),
        _ => {
            println!(
                "{} Unknown template '{}', using default template",
                OutputFormatter::warning("Warning"),
                template
            );
            println!(
                "{} Available templates: web, game, library, desktop, default",
                OutputFormatter::label("Info")
            );
            (
                include_str!("../templates/default.toml"),
                "General Rust Development (fallback)",
            )
        }
    };

    fs::write(&config_file, config_content)?;

    println!(
        "{} Configuration initialized at {}",
        OutputFormatter::label("Success"),
        OutputFormatter::path(config_file.display())
    );
    println!(
        "{} Template: {}",
        OutputFormatter::label("Template"),
        template_description
    );

    println!();
    println!("{} Next steps:", OutputFormatter::info("Next"));
    println!(
        "  1. Run {} to see available commands",
        OutputFormatter::command("raz file.rs")
    );
    println!(
        "  2. Edit {} to customize settings",
        OutputFormatter::path(config_file.display())
    );

    Ok(())
}

/// Auto-detect the best template based on project structure and dependencies
fn detect_project_template(working_dir: &Path) -> String {
    let cargo_toml = working_dir.join("Cargo.toml");

    if !cargo_toml.exists() {
        return "default".to_string();
    }

    // Read Cargo.toml to detect dependencies
    if let Ok(content) = fs::read_to_string(&cargo_toml) {
        // Check for web frameworks
        if content.contains("leptos") || content.contains("dioxus") || content.contains("yew") {
            return "web".to_string();
        }

        // Check for game engines
        if content.contains("bevy") {
            return "game".to_string();
        }

        // Check for desktop frameworks
        if content.contains("tauri") || content.contains("egui") || content.contains("iced") {
            return "desktop".to_string();
        }

        // Check if it's a library (no [[bin]] section and lib = true)
        if content.contains("[lib]") && !content.contains("[[bin]]") {
            return "library".to_string();
        }
    }

    "default".to_string()
}

/// Handle override subcommands
fn handle_override_command(working_dir: &Path, cmd: OverrideCommands) -> anyhow::Result<()> {
    // Use config hierarchy to find the appropriate storage location
    let cli = OverrideCli::new(working_dir);

    match cmd {
        OverrideCommands::List { file } => {
            if let Some(file_path) = file {
                cli.list_by_file(&file_path)?;
            } else {
                cli.list_all()?;
            }
        }
        OverrideCommands::Inspect { key } => {
            cli.inspect(&key)?;
        }
        OverrideCommands::Debug { file, line, column } => {
            cli.debug_resolution(&file, line, column)?;
        }
        OverrideCommands::Stats => {
            cli.stats()?;
        }
        OverrideCommands::Export { output } => {
            cli.export(output.as_deref())?;
        }
        OverrideCommands::Import { file } => {
            cli.import(&file)?;
        }
        OverrideCommands::Delete { key } => {
            cli.delete(&key)?;
        }
        OverrideCommands::Clear { force, file } => {
            if let Some(file_path) = file {
                cli.clear_by_file(&file_path, force)?;
            } else {
                cli.clear_all(force)?;
            }
        }
        OverrideCommands::Migrate {
            file,
            dry_run,
            auto,
        } => {
            handle_migration(working_dir, file, dry_run, auto)?;
        }
        OverrideCommands::UpdateStatus { key, result } => {
            let success = match result.as_str() {
                "success" | "true" | "1" => true,
                "failure" | "false" | "0" => false,
                _ => anyhow::bail!("Invalid result: {}. Use 'success' or 'failure'", result),
            };

            let override_system = raz_override::OverrideSystem::new(working_dir)?;
            override_system.update_execution_status(&key, success)?;

            if success {
                println!(
                    "{} Override marked as successful",
                    OutputFormatter::success("Success")
                );
            } else {
                println!(
                    "{} Override marked as failed",
                    OutputFormatter::warning("Warning")
                );

                // Check failure threshold
                let config = raz_config::GlobalConfig::load()?;
                let threshold = config
                    .overrides
                    .as_ref()
                    .map(|o| o.auto_rollback_threshold)
                    .unwrap_or(3);

                if override_system.should_auto_rollback(&key, threshold)? {
                    println!(
                        "{} This override has failed {} times and may need attention",
                        OutputFormatter::warning("Warning"),
                        threshold
                    );
                }
            }
        }
        OverrideCommands::Rollback { force } => {
            if !force {
                print!("Are you sure you want to rollback to the last backup? [y/N] ");
                use std::io::{self, Write};
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;

                if !input.trim().eq_ignore_ascii_case("y") {
                    println!("Rollback cancelled");
                    return Ok(());
                }
            }

            let override_system = raz_override::OverrideSystem::new(working_dir)?;
            override_system.rollback_to_last_backup()?;

            println!(
                "{} Successfully rolled back to last backup",
                OutputFormatter::success("Success")
            );
        }
        OverrideCommands::ListBackups => {
            let _override_system = raz_override::OverrideSystem::new(working_dir)?;
            let storage = raz_override::OverrideStorage::new(working_dir)?;
            let backups = storage.backup_manager().list_backups()?;

            if backups.is_empty() {
                println!("{} No backups found", OutputFormatter::info("Info"));
            } else {
                println!("{} Available backups:", OutputFormatter::label("Backups"));
                for (i, backup) in backups.iter().enumerate() {
                    println!(
                        "  {}. {} - {} overrides ({} bytes)",
                        i + 1,
                        backup.created_at.format("%Y-%m-%d %H:%M:%S"),
                        backup.override_count,
                        backup.size
                    );
                }
            }
        }
    }

    Ok(())
}

/// Handle override migration
fn handle_migration(
    working_dir: &Path,
    file: Option<PathBuf>,
    dry_run: bool,
    auto: bool,
) -> anyhow::Result<()> {
    let mut migrator = OverrideMigrator::new(working_dir)?;

    if auto {
        // Auto-detect and migrate
        match migrator.auto_migrate(working_dir)? {
            Some(report) => {
                println!("{}", OutputFormatter::success("Migration completed!"));
                report.print_summary();
            }
            None => {
                println!(
                    "{}",
                    OutputFormatter::warning("No legacy overrides found or already migrated.")
                );
            }
        }
    } else if let Some(legacy_file) = file {
        // Migrate specific file
        let report = if dry_run {
            println!(
                "{}",
                OutputFormatter::warning("Dry run mode - no changes will be made")
            );
            migrator.dry_run(&legacy_file)?
        } else {
            migrator.migrate_from_file(&legacy_file)?
        };

        report.print_summary();
    } else {
        // Check for legacy overrides in default location
        let legacy_file = working_dir.join(".raz").join("overrides.toml");
        if legacy_file.exists() {
            println!(
                "{}",
                format!("Found legacy overrides at: {}", legacy_file.display()).yellow()
            );
            println!("Run with --auto to migrate automatically, or specify a file with --file");
        } else {
            println!("{}", "No legacy overrides found in workspace.".yellow());
        }
    }

    Ok(())
}
