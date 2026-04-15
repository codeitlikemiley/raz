use anyhow::Result;
use cargo_runner_core::{Runnable, RunnableKind};
use std::path::Path;
use tracing::debug;

use crate::commands::matching::{normalize_query, runnable_matches_query, runnable_symbol_name};
use crate::commands::workspace::{
    find_files_for_module_path, workspace_rs_files, workspace_scan_roots,
};
use crate::config::bazel_workspace::find_cargo_workspace_root;
use crate::display::command_breakdown::print_command_breakdown;
use crate::display::formatter::{determine_file_type, print_runnable_type};
use crate::utils::parser::parse_filepath_with_line;

pub fn runnables_command(
    filepath_arg: Option<&str>,
    filters: RunnableFilters,
    verbose: bool,
    show_config: bool,
) -> Result<()> {
    if let Some(filepath_arg) = filepath_arg {
        return analyze_file_command(filepath_arg, filters, verbose, show_config);
    }

    analyze_workspace_command(filters, verbose, show_config)
}

pub fn analyze_command(filepath_arg: &str, verbose: bool, show_config: bool) -> Result<()> {
    runnables_command(
        Some(filepath_arg),
        RunnableFilters::default(),
        verbose,
        show_config,
    )
}

#[derive(Debug, Clone, Default)]
pub struct RunnableFilters {
    pub bin: bool,
    pub test: bool,
    pub bench: bool,
    pub doc: bool,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub exact: bool,
}

impl RunnableFilters {
    fn active(&self) -> bool {
        self.bin
            || self.test
            || self.bench
            || self.doc
            || self.name.is_some()
            || self.symbol.is_some()
            || self.exact
    }

    fn matches(&self, runnable: &Runnable) -> bool {
        let kind_matches = !self.bin && !self.test && !self.bench && !self.doc
            || matches_runnable_kind(&runnable.kind, self.bin, self.test, self.bench, self.doc);

        kind_matches
            && runnable_matches_query(runnable, self.name.as_deref(), self.exact)
            && matches_symbol_filter(runnable, self.symbol.as_deref(), self.exact)
    }
}

fn analyze_file_command(
    filepath_arg: &str,
    filters: RunnableFilters,
    verbose: bool,
    show_config: bool,
) -> Result<()> {
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
        if filepath.contains("::") {
            return analyze_module_path_command(&filepath, filters, verbose, show_config);
        }
        return Err(anyhow::anyhow!(
            "File not found: {}",
            absolute_path.display()
        ));
    }

    let mut runner = cargo_runner_core::UnifiedRunner::new()?;

    if verbose {
        // Show JSON output for verbose mode
        let mut runnables = if let Some(line_num) = line {
            runner.detect_runnables_at_line(&absolute_path, line_num as u32)?
        } else {
            runner.detect_all_runnables(&absolute_path)?
        };
        runnables.retain(|r| filters.matches(r));
        println!("{}", serde_json::to_string_pretty(&runnables)?);
    } else {
        // Show formatted output
        print_formatted_analysis(&mut runner, &filepath, line, filters, show_config)?;
    }

    Ok(())
}

fn analyze_module_path_command(
    module_path: &str,
    filters: RunnableFilters,
    verbose: bool,
    show_config: bool,
) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let mut runner = cargo_runner_core::UnifiedRunner::new()?;
    let matches = find_files_for_module_path(&runner, module_path, &cwd)?;

    match matches.len() {
        0 => Err(anyhow::anyhow!(
            "No file found for module path: {module_path}"
        )),
        1 => {
            let path = matches.into_iter().next().expect("Checked len == 1");
            print_formatted_analysis(
                &mut runner,
                path.to_str().unwrap_or_default(),
                None,
                filters,
                show_config,
            )?;
            if verbose {
                println!();
            }
            Ok(())
        }
        _ => {
            let paths = matches
                .into_iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            Err(anyhow::anyhow!(
                "Module path is ambiguous: {module_path}. Matches: {paths}"
            ))
        }
    }
}

fn analyze_workspace_command(
    filters: RunnableFilters,
    verbose: bool,
    show_config: bool,
) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let workspace_root = find_cargo_workspace_root(&cwd).unwrap_or(cwd.clone());
    let scan_roots = workspace_scan_roots(&workspace_root)?;

    println!("🔍 Scanning workspace: {}", workspace_root.display());
    println!("{}", "=".repeat(80));

    let runner = cargo_runner_core::UnifiedRunner::new()?;
    let mut files = workspace_rs_files(&scan_roots);
    files.sort();

    if files.is_empty() {
        println!("No Rust files found under {}", workspace_root.display());
        return Ok(());
    }

    let mut found_any = false;
    for path in files {
        let mut runnables = match runner.detect_runnables(&path) {
            Ok(runnables) if !runnables.is_empty() => runnables,
            _ => continue,
        };

        runnables.retain(|r| filters.matches(r));
        if runnables.is_empty() {
            continue;
        }

        found_any = true;
        println!();
        println!("📄 {}", path.display());
        println!("✅ Found {} runnable(s):\n", runnables.len());

        for (i, runnable) in runnables.iter().enumerate() {
            println!("{}. {}", i + 1, runnable.label);
            if verbose {
                println!(
                    "   📏 Scope: lines {}-{}",
                    runnable.scope.start.line + 1,
                    runnable.scope.end.line + 1
                );
            }
            if !runnable.module_path.is_empty() {
                println!("   📍 Module path: {}", runnable.module_path);
            }
            if show_config {
                println!("   📦 Type: {}", describe_runnable_kind(&runnable.kind));
            }
            if i < runnables.len() - 1 {
                println!();
            }
        }

        if verbose || show_config {
            println!();
        }
    }

    if !found_any {
        if filters.active() {
            println!(
                "No runnable items matched the filters in {}",
                workspace_root.display()
            );
        } else {
            println!("No runnable items found in {}", workspace_root.display());
        }
    }

    Ok(())
}

fn matches_runnable_kind(
    kind: &RunnableKind,
    bin: bool,
    test: bool,
    bench: bool,
    doc: bool,
) -> bool {
    match kind {
        RunnableKind::Binary { .. } => bin,
        RunnableKind::Test { .. } | RunnableKind::ModuleTests { .. } => test,
        RunnableKind::Benchmark { .. } => bench,
        RunnableKind::DocTest { .. } => doc,
        _ => false,
    }
}

fn matches_symbol_filter(runnable: &Runnable, query: Option<&str>, exact: bool) -> bool {
    if query.is_none() {
        return true;
    }

    let symbol_name = runnable_symbol_name(runnable);
    let Some(symbol_name) = symbol_name else {
        return false;
    };

    let query = normalize_query(query.expect("Query is verified is_some"));
    let candidate = normalize_query(&symbol_name);
    if exact {
        candidate == query
    } else {
        candidate.contains(&query)
    }
}

fn describe_runnable_kind(kind: &cargo_runner_core::RunnableKind) -> &'static str {
    match kind {
        cargo_runner_core::RunnableKind::Test { .. } => "test",
        cargo_runner_core::RunnableKind::DocTest { .. } => "doc test",
        cargo_runner_core::RunnableKind::Benchmark { .. } => "benchmark",
        cargo_runner_core::RunnableKind::Binary { .. } => "binary",
        cargo_runner_core::RunnableKind::ModuleTests { .. } => "module tests",
        cargo_runner_core::RunnableKind::Standalone { .. } => "standalone",
        cargo_runner_core::RunnableKind::SingleFileScript { .. } => "single-file script",
    }
}

pub fn print_formatted_analysis(
    runner: &mut cargo_runner_core::UnifiedRunner,
    filepath: &str,
    line: Option<usize>,
    filters: RunnableFilters,
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
            println!("   📦 Type: {file_type}");

            // Get file scope info
            if let Ok(source) = std::fs::read_to_string(path) {
                let line_count = source.lines().count();
                println!("   📏 Scope: lines 1-{line_count}");
            }
        }
        Ok(None) => {
            println!("\n📄 File-level command: None");
        }
        Err(e) => {
            println!("\n📄 File-level command: Error - {e}");
        }
    }

    // Get runnables based on line number
    let mut runnables = if let Some(line_num) = line {
        runner.detect_runnables_at_line(path, line_num as u32)?
    } else {
        runner.detect_all_runnables(path)?
    };

    runnables.retain(|r| filters.matches(r));

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
                "\n❌ No runnable items matched the filters at line {} (but file-level command above can be used).",
                line_num + 1
            );
        } else {
            println!(
                "\n❌ No runnable items matched the filters in this file (but file-level command above can be used)."
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
                                    println!("        - features: {selected:?}");
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
        println!("   {cmd}");
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
            println!("      • command: {command}");
        }
        if let Some(subcommand) = &cargo_config.subcommand {
            println!("      • subcommand: {subcommand}");
        }
        if let Some(channel) = &cargo_config.channel {
            println!("      • channel: {channel}");
        }
        if let Some(features) = &cargo_config.features {
            match features {
                cargo_runner_core::config::Features::All(s) if s == "all" => {
                    println!("      • features: all");
                }
                cargo_runner_core::config::Features::Selected(selected) => {
                    println!("      • features: {selected:?}");
                }
                _ => {}
            }
        }
        if let Some(extra_args) = &cargo_config.extra_args {
            if !extra_args.is_empty() {
                println!("      • extra_args: {extra_args:?}");
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
                println!("         - extra_args: {extra_args:?}");
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

#[cfg(test)]
mod tests {
    use super::*;
    use cargo_runner_core::types::{Position, Scope, ScopeKind};
    use std::path::PathBuf;

    fn sample_runnable(
        kind: RunnableKind,
        label: &str,
        module_path: &str,
        scope_name: Option<&str>,
    ) -> Runnable {
        Runnable {
            label: label.to_string(),
            scope: Scope {
                start: Position::new(0, 0),
                end: Position::new(1, 0),
                kind: ScopeKind::Function,
                name: scope_name.map(str::to_string),
            },
            kind,
            module_path: module_path.to_string(),
            file_path: PathBuf::from("src/lib.rs"),
            extended_scope: None,
        }
    }

    #[test]
    fn name_filter_is_case_and_separator_insensitive() {
        let filters = RunnableFilters {
            name: Some("My Function".to_string()),
            ..Default::default()
        };
        let runnable = sample_runnable(
            RunnableKind::Test {
                test_name: "my_function".to_string(),
                is_async: false,
            },
            "Run test 'my_function'",
            "crate::tests",
            Some("my_function"),
        );

        assert!(filters.matches(&runnable));
    }

    #[test]
    fn exact_name_filter_requires_full_normalized_match() {
        let fuzzy = RunnableFilters {
            name: Some("my".to_string()),
            exact: true,
            ..Default::default()
        };
        let exact = RunnableFilters {
            name: Some("my function".to_string()),
            exact: true,
            ..Default::default()
        };
        let runnable = sample_runnable(
            RunnableKind::Test {
                test_name: "my_function".to_string(),
                is_async: false,
            },
            "Run test 'my_function'",
            "crate::tests",
            Some("my_function"),
        );

        assert!(!fuzzy.matches(&runnable));
        assert!(exact.matches(&runnable));
    }

    #[test]
    fn symbol_filter_targets_doc_test_symbols_only() {
        let filters = RunnableFilters {
            symbol: Some("Users".to_string()),
            ..Default::default()
        };
        let doc_symbol = sample_runnable(
            RunnableKind::DocTest {
                struct_or_module_name: "Users".to_string(),
                method_name: None,
            },
            "Run doc test for 'Users'",
            "crate::models",
            Some("Users"),
        );
        let doc_method = sample_runnable(
            RunnableKind::DocTest {
                struct_or_module_name: "Users".to_string(),
                method_name: Some("new".to_string()),
            },
            "Run doc test for 'Users::new'",
            "crate::models",
            Some("Users"),
        );

        assert!(filters.matches(&doc_symbol));
        assert!(!filters.matches(&doc_method));
    }

    #[test]
    fn kind_filters_can_be_combined_with_name_and_symbol_filters() {
        let filters = RunnableFilters {
            bin: true,
            test: true,
            name: Some("app".to_string()),
            symbol: Some("Users".to_string()),
            ..Default::default()
        };
        let test_runnable = sample_runnable(
            RunnableKind::Test {
                test_name: "test_add".to_string(),
                is_async: false,
            },
            "Run test 'test_add'",
            "crate::tests",
            Some("test_add"),
        );
        let binary_runnable = sample_runnable(
            RunnableKind::Binary {
                bin_name: Some("app".to_string()),
            },
            "Run binary 'app'",
            "crate::main",
            Some("app"),
        );
        let symbol_runnable = sample_runnable(
            RunnableKind::DocTest {
                struct_or_module_name: "Users".to_string(),
                method_name: None,
            },
            "Run doc test for 'Users'",
            "crate::models",
            Some("Users"),
        );

        assert!(!filters.matches(&test_runnable));
        assert!(!filters.matches(&binary_runnable));
        assert!(!filters.matches(&symbol_runnable));
    }
}
