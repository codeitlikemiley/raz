use anyhow::Result;
use std::path::Path;
use tracing::debug;

use crate::display::command_breakdown::print_command_breakdown;
use crate::display::formatter::{determine_file_type, print_runnable_type};
use crate::utils::parser::parse_filepath_with_line;

pub fn analyze_command(filepath_arg: &str, verbose: bool, show_config: bool) -> Result<()> {
    debug!("Analyzing file: {}", filepath_arg);

    // Parse filepath and line number first
    let (filepath, line) = parse_filepath_with_line(filepath_arg);
    
    // Check if file exists - resolve to absolute path
    let filepath_path = Path::new(&filepath);
    let absolute_path = if filepath_path.is_absolute() {
        filepath_path.to_path_buf()
    } else {
        std::env::current_dir()?.join(filepath_path)
    };
    
    if !absolute_path.exists() {
        return Err(anyhow::anyhow!("File not found: {}", absolute_path.display()));
    }

    let mut runner = cargo_runner_core::UnifiedRunner::new()?;

    if verbose {
        // Show JSON output for verbose mode
        if let Some(line_num) = line {
            let runnables = runner.analyze_at_line(&filepath, line_num)?;
            println!("{runnables}");
        } else {
            let runnables = runner.analyze(&filepath)?;
            println!("{runnables}");
        }
    } else {
        // Show formatted output
        print_formatted_analysis(&mut runner, &filepath, line, show_config)?;
    }

    Ok(())
}

pub fn print_formatted_analysis(
    runner: &mut cargo_runner_core::UnifiedRunner,
    filepath: &str,
    line: Option<usize>,
    show_config: bool,
) -> Result<()> {
    println!(
        "🔍 Analyzing: {}{}",
        filepath,
        if let Some(l) = line {
            format!(":{}", l + 1)
        } else {
            String::new()
        }
    );
    println!("{}", "=".repeat(80));

    let mut final_command: Option<String> = None;

    // Show config details if requested
    if show_config {
        print_config_details(runner, filepath)?;
    }

    let path = Path::new(filepath);

    // Always show file-level command as it represents the entire file scope
    // Get file-level command
    match runner.get_file_command(path) {
        Ok(Some(cmd)) => {
            println!("\n📄 File-level command:");
            print_command_breakdown(&cmd);

            // Determine file type
            let file_type = determine_file_type(path);
            println!("   📦 Type: {}", file_type);

            // Get file scope info
            if let Ok(source) = std::fs::read_to_string(path) {
                let line_count = source.lines().count();
                println!("   📏 Scope: lines 1-{}", line_count);
            }
        }
        Ok(None) => {
            println!("\n📄 File-level command: None");
        }
        Err(e) => {
            println!("\n📄 File-level command: Error - {}", e);
        }
    }

    // Get runnables based on line number
    let mut runnables = if let Some(line_num) = line {
        runner.detect_runnables_at_line(path, line_num as u32)?
    } else {
        runner.detect_all_runnables(path)?
    };

    // When analyzing a specific line, filter to the most specific runnable
    if line.is_some() && runnables.len() > 1 {
        // For doc tests, prefer more specific ones (e.g., User::new over User)
        runnables.sort_by(|a, b| {
            // For doc tests, use extended scope size if available
            let a_size = if matches!(a.kind, cargo_runner_core::RunnableKind::DocTest { .. }) {
                if let Some(ref extended) = a.extended_scope {
                    extended.scope.end.line - extended.scope.start.line
                } else {
                    a.scope.end.line - a.scope.start.line
                }
            } else {
                a.scope.end.line - a.scope.start.line
            };

            let b_size = if matches!(b.kind, cargo_runner_core::RunnableKind::DocTest { .. }) {
                if let Some(ref extended) = b.extended_scope {
                    extended.scope.end.line - extended.scope.start.line
                } else {
                    b.scope.end.line - b.scope.start.line
                }
            } else {
                b.scope.end.line - b.scope.start.line
            };

            a_size.cmp(&b_size)
        });

        // Keep only the most specific runnable
        if let Some(most_specific) = runnables.first().cloned() {
            runnables = vec![most_specific];
        }
    }

    if runnables.is_empty() {
        if let Some(line_num) = line {
            println!(
                "\n❌ No specific runnables found at line {} (but file-level command above can be used).",
                line_num + 1
            );
        } else {
            println!(
                "\n❌ No specific runnables found in this file (but file-level command above can be used)."
            );
        }
    } else {
        println!("\n✅ Found {} runnable(s):\n", runnables.len());

        for (i, runnable) in runnables.iter().enumerate() {
            println!("{}. {}", i + 1, runnable.label);

            // Show scope with 1-based line numbers
            // For doc tests, show the extended scope if available
            if matches!(
                runnable.kind,
                cargo_runner_core::RunnableKind::DocTest { .. }
            ) {
                if let Some(ref extended) = runnable.extended_scope {
                    println!(
                        "   📏 Scope: lines {}-{}",
                        extended.scope.start.line + 1,
                        extended.scope.end.line + 1
                    );
                } else {
                    println!(
                        "   📏 Scope: lines {}-{}",
                        runnable.scope.start.line + 1,
                        runnable.scope.end.line + 1
                    );
                }
            } else {
                println!(
                    "   📏 Scope: lines {}-{}",
                    runnable.scope.start.line + 1,
                    runnable.scope.end.line + 1
                );
            }

            // Debug: show if this runnable contains the requested line
            if let Some(line_num) = line {
                let contains = runnable.scope.contains_line(line_num as u32);
                debug!(
                    "Runnable '{}' contains line {}? {}",
                    runnable.label,
                    line_num + 1,
                    contains
                );
            }

            // Debug: show module path
            if !runnable.module_path.is_empty() {
                println!("   📍 Module path: {}", runnable.module_path);
            }

            // Show attributes if present
            if let Some(ref extended) = runnable.extended_scope {
                if extended.attribute_lines > 0 {
                    println!("   🏷️  Attributes: {} lines", extended.attribute_lines);
                }
                if extended.has_doc_tests {
                    println!("   🧪 Contains doc tests");
                }
            }

            // Build and show command
            if let Some(command) = runner.build_command_for_runnable(runnable)? {
                print_command_breakdown(&command);
                // Store the final command
                final_command = Some(command.to_shell_command());
            }

            // Show matching override if config details requested
            if show_config {
                if let Some(override_config) = runner.get_override_for_runnable(runnable) {
                    println!("   🔀 Matched override:");
                    println!("      • match: {:?}", override_config.identity);

                    // Show cargo config if present
                    if let Some(cargo) = &override_config.cargo {
                        println!("      • cargo config:");
                        if cargo.command.is_some() {
                            println!("        - command: {:?}", cargo.command);
                        }
                        if cargo.subcommand.is_some() {
                            println!("        - subcommand: {:?}", cargo.subcommand);
                        }
                        if let Some(features) = &cargo.features {
                            match features {
                                cargo_runner_core::config::Features::All(s) if s == "all" => {
                                    println!("        - features: all");
                                }
                                cargo_runner_core::config::Features::Selected(selected) => {
                                    println!("        - features: {:?}", selected);
                                }
                                _ => {}
                            }
                        }
                        if cargo.extra_args.is_some() {
                            println!("        - extra_args: {:?}", cargo.extra_args);
                        }
                        if cargo.extra_test_binary_args.is_some() {
                            println!(
                                "        - extra_test_binary_args: {:?}",
                                cargo.extra_test_binary_args
                            );
                        }
                        if cargo.extra_env.is_some() {
                            println!("        - extra_env: {:?}", cargo.extra_env);
                        }
                    }

                    // Show rustc config if present
                    if let Some(rustc) = &override_config.rustc {
                        println!("      • rustc config:");
                        if rustc.test_framework.is_some() {
                            println!("        - test_framework: present");
                        }
                        if rustc.binary_framework.is_some() {
                            println!("        - binary_framework: present");
                        }
                        if rustc.benchmark_framework.is_some() {
                            println!("        - benchmark_framework: present");
                        }
                    }

                    // Show single_file_script config if present
                    if let Some(sfs) = &override_config.single_file_script {
                        println!("      • single_file_script config:");
                        if sfs.extra_args.is_some() {
                            println!("        - extra_args: {:?}", sfs.extra_args);
                        }
                        if sfs.extra_test_binary_args.is_some() {
                            println!(
                                "        - extra_test_binary_args: {:?}",
                                sfs.extra_test_binary_args
                            );
                        }
                        if sfs.extra_env.is_some() {
                            println!("        - extra_env: {:?}", sfs.extra_env);
                        }
                    }
                }
            }

            // Show type
            print!("   📦 Type: ");
            print_runnable_type(&runnable.kind);

            // Show module path
            if !runnable.module_path.is_empty() {
                println!("   📁 Module path: {}", runnable.module_path);
            }

            if i < runnables.len() - 1 {
                println!();
            }
        }
    }

    // Display final command at the end
    if let Some(cmd) = final_command {
        println!("\n🎯 Command to run:");
        println!("   {}", cmd);
    }

    println!("\n{}", "=".repeat(80));
    Ok(())
}

fn print_config_details(_runner: &cargo_runner_core::UnifiedRunner, filepath: &str) -> Result<()> {
    use cargo_runner_core::config::ConfigMerger;
    use std::path::Path;

    println!("\n📁 Configuration Details:");
    println!("   {}", "-".repeat(75));

    // Get the merged configs
    let path = Path::new(filepath);
    let mut merger = ConfigMerger::new();
    merger.load_configs_for_path(path)?;

    // Show which configs were loaded
    let config_info = merger.get_config_info();

    if let Some(root_path) = &config_info.root_config_path {
        println!("   🏯 Root config: {}", root_path.display());
    } else {
        println!("   🏯 Root config: None");
    }

    if let Some(workspace_path) = &config_info.workspace_config_path {
        println!("   📦 Workspace config: {}", workspace_path.display());
    } else {
        println!("   📦 Workspace config: None");
    }

    if let Some(package_path) = &config_info.package_config_path {
        println!("   📦 Package config: {}", package_path.display());
    } else {
        println!("   📦 Package config: None");
    }

    // Show the merged config summary
    let merged_config = merger.get_merged_config();
    println!("\n   🔀 Merged configuration:");

    // Show cargo configuration if present
    if let Some(cargo_config) = &merged_config.cargo {
        if let Some(command) = &cargo_config.command {
            println!("      • command: {}", command);
        }
        if let Some(subcommand) = &cargo_config.subcommand {
            println!("      • subcommand: {}", subcommand);
        }
        if let Some(channel) = &cargo_config.channel {
            println!("      • channel: {}", channel);
        }
        if let Some(features) = &cargo_config.features {
            match features {
                cargo_runner_core::config::Features::All(s) if s == "all" => {
                    println!("      • features: all");
                }
                cargo_runner_core::config::Features::Selected(selected) => {
                    println!("      • features: {:?}", selected);
                }
                _ => {}
            }
        }
        if let Some(extra_args) = &cargo_config.extra_args {
            if !extra_args.is_empty() {
                println!("      • extra_args: {:?}", extra_args);
            }
        }
        if let Some(extra_env) = &cargo_config.extra_env {
            if !extra_env.is_empty() {
                println!("      • extra_env: {} variables", extra_env.len());
            }
        }
        if let Some(linked_projects) = &cargo_config.linked_projects {
            println!(
                "      • linked_projects: {} projects",
                linked_projects.len()
            );
        }
    }

    // Show rustc configuration if present
    if let Some(rustc_config) = &merged_config.rustc {
        println!("      • rustc config:");
        if rustc_config.test_framework.is_some() {
            println!("         - test_framework: configured");
        }
        if rustc_config.binary_framework.is_some() {
            println!("         - binary_framework: configured");
        }
        if rustc_config.benchmark_framework.is_some() {
            println!("         - benchmark_framework: configured");
        }
    }

    // Show single file script configuration if present
    if let Some(sfs_config) = &merged_config.single_file_script {
        println!("      • single_file_script config:");
        if let Some(extra_args) = &sfs_config.extra_args {
            if !extra_args.is_empty() {
                println!("         - extra_args: {:?}", extra_args);
            }
        }
        if let Some(extra_env) = &sfs_config.extra_env {
            if !extra_env.is_empty() {
                println!("         - extra_env: {} variables", extra_env.len());
            }
        }
    }
    if !merged_config.overrides.is_empty() {
        println!(
            "      • overrides: {} configured",
            merged_config.overrides.len()
        );
    }

    println!();
    Ok(())
}
