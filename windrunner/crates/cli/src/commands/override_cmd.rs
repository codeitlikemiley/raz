use anyhow::{Context, Result};
use serde_json::{Map, Value, json};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

use crate::utils::parser::parse_filepath_with_line;

pub fn override_command(filepath_arg: &str, root: bool, override_args: Vec<String>) -> Result<()> {
    // Parse filepath and line number
    let (filepath, line) = parse_filepath_with_line(filepath_arg);

    println!("🔧 Creating override configuration...");
    println!("   📍 File: {}", filepath);
    if let Some(line_num) = line {
        println!("   📍 Line: {}", line_num + 1); // Convert back to 1-based
    }

    // Create a runner to detect the runnable at the given location
    let mut runner = cargo_runner_core::UnifiedRunner::new()?;

    // Resolve the file path
    let resolved_path = runner.resolve_file_path(&filepath)?;

    // Get the runnable at the specified line
    let runnable = if let Some(line_num) = line {
        runner.get_best_runnable_at_line(&resolved_path, line_num as u32)?
    } else {
        // If no line specified, try to get any runnable from the file
        let all_runnables = runner.detect_all_runnables(&resolved_path)?;
        all_runnables.into_iter().next()
    };

    // If no runnable found, create a file-level override
    if runnable.is_none() {
        println!("   📄 No specific runnable found, creating file-level override");

        // For file-level overrides, we'll use the file path as the match criteria
        return create_file_level_override(&filepath, root, override_args);
    }

    let runnable = runnable.unwrap();

    // Detect file type based on the runnable
    let file_type = runner.detect_file_type(&resolved_path)?;

    // Determine which framework we're targeting
    let framework_type = match &runnable.kind {
        cargo_runner_core::RunnableKind::Test { .. }
        | cargo_runner_core::RunnableKind::ModuleTests { .. } => "test_framework",
        cargo_runner_core::RunnableKind::Benchmark { .. } => "benchmark_framework",
        cargo_runner_core::RunnableKind::Binary { .. }
        | cargo_runner_core::RunnableKind::Standalone { .. } => "binary_framework",
        _ => "unknown",
    };

    println!("   🎯 Found: {:?}", runnable.kind);
    println!("   📝 File type: {:?}", file_type);

    // Load the merged config to detect build system
    println!(
        "   📂 Loading configs for path: {}",
        resolved_path.display()
    );
    let mut merger = cargo_runner_core::config::ConfigMerger::new();
    merger.load_configs_for_path(&resolved_path)?;
    let merged_config = merger.get_merged_config();

    // Determine configuration section based on file type
    let config_section = match file_type {
        cargo_runner_core::FileType::CargoProject => {
            // Check if this is a Bazel project
            let command = merged_config
                .cargo
                .as_ref()
                .and_then(|c| c.command.as_ref());

            if let Some(cmd) = command {
                println!("   🔍 Detected command: {}", cmd);
            }

            let is_bazel = command.map(|cmd| cmd == "bazel").unwrap_or(false);

            if is_bazel { "bazel" } else { "cargo" }
        }
        cargo_runner_core::FileType::Standalone => "rustc",
        cargo_runner_core::FileType::SingleFileScript => "single_file_script",
    };

    println!("   🎨 Config section: {}", config_section);
    if config_section == "rustc" {
        println!("      └─ Framework: {}", framework_type);
    }

    // Create function identity for the override
    let identity = cargo_runner_core::FunctionIdentity {
        package: runner.get_package_name_str(&resolved_path).ok(),
        module_path: if runnable.module_path.is_empty() {
            None
        } else {
            Some(runnable.module_path.clone())
        },
        file_path: Some(resolved_path.clone()),
        function_name: runnable.get_function_name(),
        file_type: Some(file_type),
    };

    println!("   🔑 Function identity:");
    if let Some(pkg) = &identity.package {
        println!("      - package: {}", pkg);
    }
    if let Some(module) = &identity.module_path {
        println!("      - module_path: {}", module);
    }
    if let Some(func) = &identity.function_name {
        println!("      - function_name: {}", func);
    }
    println!("      - file_type: {:?}", file_type);

    // Create the override configuration based on file type
    let mut override_config = Map::new();

    // Add matcher
    let mut matcher = Map::new();
    if let Some(pkg) = &identity.package {
        matcher.insert("package".to_string(), json!(pkg));
    }
    if let Some(module) = &identity.module_path {
        matcher.insert("module_path".to_string(), json!(module));
    }
    if let Some(func) = &identity.function_name {
        matcher.insert("function_name".to_string(), json!(func));
    }
    // Always add file_path for precise matching
    matcher.insert(
        "file_path".to_string(),
        json!(resolved_path.to_str().unwrap()),
    );

    override_config.insert("match".to_string(), Value::Object(matcher));

    // Parse override arguments
    let parsed_args = parse_override_args(&override_args);

    // Check if we should remove the entire override
    if override_args.len() == 1 && override_args[0] == "-" {
        // Load config and remove matching override
        let config_path = if root {
            let root_path = env::var("PROJECT_ROOT")
                .map(PathBuf::from)
                .unwrap_or_else(|_| env::current_dir().unwrap());
            root_path.join(".cargo-runner.json")
        } else {
            runner
                .find_config_path(&resolved_path)?
                .ok_or_else(|| anyhow::anyhow!("No config file found"))?
        };

        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            let mut config: Map<String, Value> = serde_json::from_str(&content)?;

            if let Some(overrides) = config.get_mut("overrides").and_then(|v| v.as_array_mut()) {
                // Find and remove matching override
                overrides.retain(|o| {
                    if let Some(obj) = o.as_object() {
                        if let Some(match_obj) = obj.get("match").and_then(|m| m.as_object()) {
                            // Check if this override matches our identity
                            let matches = match_obj
                                .get("file_path")
                                .and_then(|v| v.as_str())
                                .map(|p| p == resolved_path.to_str().unwrap())
                                .unwrap_or(false);

                            if matches && identity.function_name.is_some() {
                                let func_matches = match_obj
                                    .get("function_name")
                                    .and_then(|v| v.as_str())
                                    .map(|f| Some(f.to_string()) == identity.function_name)
                                    .unwrap_or(false);
                                return !func_matches;
                            }

                            return !matches;
                        }
                    }
                    true
                });

                let json = serde_json::to_string_pretty(&config)?;
                fs::write(&config_path, json)?;

                println!("✅ Override removed successfully!");
                return Ok(());
            }
        }

        println!("❌ No matching override found to remove");
        return Ok(());
    }

    // Add configuration based on file type
    match file_type {
        cargo_runner_core::FileType::CargoProject => {
            // Check if this is a Bazel project
            let is_bazel = merged_config
                .cargo
                .as_ref()
                .and_then(|c| c.command.as_ref())
                .map(|cmd| cmd == "bazel")
                .unwrap_or(false);

            if is_bazel {
                // Create bazel override section
                let mut bazel_config = Map::new();

                // For Bazel, extra_args becomes extra_test_args for tests
                if let Some(extra_args) = parsed_args.get("extra_args") {
                    if matches!(
                        &runnable.kind,
                        cargo_runner_core::RunnableKind::Test { .. }
                            | cargo_runner_core::RunnableKind::ModuleTests { .. }
                    ) {
                        bazel_config.insert("extra_test_args".to_string(), extra_args.clone());
                    } else {
                        bazel_config.insert("extra_run_args".to_string(), extra_args.clone());
                    }
                }

                // Handle extra_test_binary_args as extra_test_args for Bazel
                if let Some(extra_test_binary_args) = parsed_args.get("extra_test_binary_args") {
                    bazel_config.insert(
                        "extra_test_args".to_string(),
                        extra_test_binary_args.clone(),
                    );
                }

                if let Some(extra_env) = parsed_args.get("extra_env") {
                    bazel_config.insert("extra_env".to_string(), extra_env.clone());
                }

                override_config.insert("bazel".to_string(), Value::Object(bazel_config));
            } else {
                // Create cargo override section
                let mut cargo_config = Map::new();

                // Handle command and subcommand
                if let Some(command) = parsed_args.get("command") {
                    cargo_config.insert("command".to_string(), command.clone());
                }
                if let Some(subcommand) = parsed_args.get("subcommand") {
                    cargo_config.insert("subcommand".to_string(), subcommand.clone());
                }
                if let Some(channel) = parsed_args.get("channel") {
                    cargo_config.insert("channel".to_string(), channel.clone());
                }

                if let Some(extra_args) = parsed_args.get("extra_args") {
                    cargo_config.insert("extra_args".to_string(), extra_args.clone());
                }
                if let Some(extra_env) = parsed_args.get("extra_env") {
                    cargo_config.insert("extra_env".to_string(), extra_env.clone());
                }
                if let Some(extra_test_binary_args) = parsed_args.get("extra_test_binary_args") {
                    cargo_config.insert(
                        "extra_test_binary_args".to_string(),
                        extra_test_binary_args.clone(),
                    );
                }
                override_config.insert("cargo".to_string(), Value::Object(cargo_config));
            }
        }
        cargo_runner_core::FileType::Standalone => {
            // For rustc
            let mut rustc_config = Map::new();

            // Determine which framework to configure based on runnable kind
            let framework_key = match &runnable.kind {
                cargo_runner_core::RunnableKind::Test { .. }
                | cargo_runner_core::RunnableKind::ModuleTests { .. } => "test_framework",
                cargo_runner_core::RunnableKind::Benchmark { .. } => "benchmark_framework",
                _ => "binary_framework",
            };

            let mut framework = Map::new();
            let mut build = Map::new();
            let mut exec = Map::new();

            // Handle command for build phase (rustc can have custom command)
            if let Some(command) = parsed_args.get("command") {
                build.insert("command".to_string(), command.clone());
            }

            // Note: channel is not yet supported in rustc configs
            // TODO: Add channel support to RustcPhaseConfig

            // Add parsed arguments to appropriate sections
            if let Some(extra_args) = parsed_args.get("extra_args") {
                build.insert("extra_args".to_string(), extra_args.clone());
            }
            if let Some(extra_env) = parsed_args.get("extra_env") {
                exec.insert("extra_env".to_string(), extra_env.clone());
            }
            if let Some(extra_test_binary_args) = parsed_args.get("extra_test_binary_args") {
                exec.insert(
                    "extra_test_binary_args".to_string(),
                    extra_test_binary_args.clone(),
                );
            }

            if !build.is_empty() {
                framework.insert("build".to_string(), Value::Object(build));
            }
            if !exec.is_empty() {
                framework.insert("exec".to_string(), Value::Object(exec));
            }

            rustc_config.insert(framework_key.to_string(), Value::Object(framework));
            override_config.insert("rustc".to_string(), Value::Object(rustc_config));
        }
        cargo_runner_core::FileType::SingleFileScript => {
            let mut script_config = Map::new();

            // Handle command and subcommand
            if let Some(command) = parsed_args.get("command") {
                script_config.insert("command".to_string(), command.clone());
            }
            if let Some(subcommand) = parsed_args.get("subcommand") {
                script_config.insert("subcommand".to_string(), subcommand.clone());
            }
            if let Some(channel) = parsed_args.get("channel") {
                script_config.insert("channel".to_string(), channel.clone());
            }

            if let Some(extra_args) = parsed_args.get("extra_args") {
                script_config.insert("extra_args".to_string(), extra_args.clone());
            }
            if let Some(extra_env) = parsed_args.get("extra_env") {
                script_config.insert("extra_env".to_string(), extra_env.clone());
            }
            if let Some(extra_test_binary_args) = parsed_args.get("extra_test_binary_args") {
                script_config.insert(
                    "extra_test_binary_args".to_string(),
                    extra_test_binary_args.clone(),
                );
            }
            override_config.insert(
                "single_file_script".to_string(),
                Value::Object(script_config),
            );
        }
    }

    // Load existing config
    let config_path = if root {
        // Use PROJECT_ROOT or current directory for root config
        let root_path = env::var("PROJECT_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| env::current_dir().unwrap());
        root_path.join(".cargo-runner.json")
    } else {
        // Find the closest project config
        runner
            .find_config_path(&resolved_path)?
            .ok_or_else(|| anyhow::anyhow!("No config file found. Run 'cargo runner init' first"))?
    };

    println!("   📄 Config file: {}", config_path.display());

    // Read existing config or create new one
    let mut config: Map<String, Value> = if config_path.exists() {
        let content = fs::read_to_string(&config_path)?;
        serde_json::from_str(&content)?
    } else {
        let mut new_config = Map::new();
        new_config.insert("overrides".to_string(), json!([]));
        new_config
    };

    // Get or create the overrides array
    let overrides = config
        .get_mut("overrides")
        .and_then(|v| v.as_array_mut())
        .ok_or_else(|| {
            anyhow::anyhow!("Invalid config format: missing or invalid 'overrides' array")
        })?;

    // Check if an override already exists for this identity
    let existing_index = overrides.iter().position(|o| {
        if let Some(obj) = o.as_object() {
            if let Some(match_obj) = obj.get("match").and_then(|m| m.as_object()) {
                // Check if this override matches our identity
                let file_matches = match_obj
                    .get("file_path")
                    .and_then(|v| v.as_str())
                    .map(|p| p == resolved_path.to_str().unwrap())
                    .unwrap_or(false);

                if file_matches && identity.function_name.is_some() {
                    let func_matches = match_obj
                        .get("function_name")
                        .and_then(|v| v.as_str())
                        .map(|f| Some(f.to_string()) == identity.function_name)
                        .unwrap_or(false);
                    return func_matches;
                }

                return file_matches;
            }
        }
        false
    });

    // Handle removal operations on existing override
    if let Some(idx) = existing_index {
        let existing = &mut overrides[idx];
        if let Some(existing_obj) = existing.as_object_mut() {
            // Get the appropriate config section
            let config_section = match file_type {
                cargo_runner_core::FileType::CargoProject => {
                    // Check if this is a Bazel project
                    let is_bazel = merged_config
                        .cargo
                        .as_ref()
                        .and_then(|c| c.command.as_ref())
                        .map(|cmd| cmd == "bazel")
                        .unwrap_or(false);

                    if is_bazel {
                        existing_obj.get_mut("bazel")
                    } else {
                        existing_obj.get_mut("cargo")
                    }
                }
                cargo_runner_core::FileType::Standalone => existing_obj.get_mut("rustc"),
                cargo_runner_core::FileType::SingleFileScript => {
                    existing_obj.get_mut("single_file_script")
                }
            };

            if let Some(section) = config_section.and_then(|v| v.as_object_mut()) {
                // Apply removals
                if parsed_args.get("remove_command").is_some() {
                    section.remove("command");
                }
                if parsed_args.get("remove_subcommand").is_some() {
                    section.remove("subcommand");
                }
                if parsed_args.get("remove_channel").is_some() {
                    section.remove("channel");
                }
                if parsed_args.get("remove_args").is_some() {
                    section.remove("extra_args");
                }
                if parsed_args.get("remove_env").is_some() {
                    section.remove("extra_env");
                }
                if parsed_args.get("remove_test_args").is_some() {
                    section.remove("extra_test_binary_args");
                }

                // Remove specific env vars
                if let Some(env_keys) = parsed_args
                    .get("remove_env_keys")
                    .and_then(|v| v.as_array())
                {
                    if let Some(extra_env) =
                        section.get_mut("extra_env").and_then(|v| v.as_object_mut())
                    {
                        for key in env_keys {
                            if let Some(key_str) = key.as_str() {
                                extra_env.remove(key_str);
                            }
                        }
                    }
                }
            }

            // Merge in the new override config
            let section_key = match file_type {
                cargo_runner_core::FileType::CargoProject => {
                    // Check if this is a Bazel project
                    let is_bazel = merged_config
                        .cargo
                        .as_ref()
                        .and_then(|c| c.command.as_ref())
                        .map(|cmd| cmd == "bazel")
                        .unwrap_or(false);

                    if is_bazel { "bazel" } else { "cargo" }
                }
                cargo_runner_core::FileType::Standalone => "rustc",
                cargo_runner_core::FileType::SingleFileScript => "single_file_script",
            };

            if let Some(new_section) = override_config.get(section_key) {
                existing_obj.insert(section_key.to_string(), new_section.clone());
            }

            println!("✅ Override updated successfully!");
        }
    } else {
        // Add new override
        overrides.push(Value::Object(override_config));
        println!("✅ Override added successfully!");
    }

    // Write the updated config
    let json = serde_json::to_string_pretty(&config)?;
    fs::write(&config_path, json)?;

    println!("   • Arguments: {:?}", override_args);

    // Show what was applied
    if !parsed_args.is_empty() {
        println!(
            "\n   📋 Applied changes to {}:",
            match file_type {
                cargo_runner_core::FileType::CargoProject => {
                    // Check if this is a Bazel project
                    let is_bazel = merged_config
                        .cargo
                        .as_ref()
                        .and_then(|c| c.command.as_ref())
                        .map(|cmd| cmd == "bazel")
                        .unwrap_or(false);

                    if is_bazel {
                        "bazel configuration"
                    } else {
                        "cargo configuration"
                    }
                }
                cargo_runner_core::FileType::Standalone => {
                    match &runnable.kind {
                        cargo_runner_core::RunnableKind::Test { .. }
                        | cargo_runner_core::RunnableKind::ModuleTests { .. } => {
                            "rustc.test_framework configuration"
                        }
                        cargo_runner_core::RunnableKind::Benchmark { .. } => {
                            "rustc.benchmark_framework configuration"
                        }
                        _ => "rustc.binary_framework configuration",
                    }
                }
                cargo_runner_core::FileType::SingleFileScript => "single_file_script configuration",
            }
        );
        if parsed_args.contains_key("command") {
            println!("      • command: {}", parsed_args["command"]);
        }
        if parsed_args.contains_key("subcommand") {
            println!("      • subcommand: {}", parsed_args["subcommand"]);
        }
        if parsed_args.contains_key("channel") {
            println!("      • channel: {}", parsed_args["channel"]);
        }
        if parsed_args.contains_key("extra_args") {
            if let Some(args) = parsed_args["extra_args"].as_array() {
                let args_str: Vec<String> = args
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
                if file_type == cargo_runner_core::FileType::Standalone {
                    println!("      • build.extra_args: {:?}", args_str);
                } else {
                    println!("      • extra_args: {:?}", args_str);
                }
            }
        }
        if parsed_args.contains_key("extra_env") {
            if let Some(env) = parsed_args["extra_env"].as_object() {
                for (k, v) in env {
                    if file_type == cargo_runner_core::FileType::Standalone {
                        println!("      • exec.extra_env: {}={}", k, v);
                    } else {
                        println!("      • env: {}={}", k, v);
                    }
                }
            }
        }
        if parsed_args.contains_key("extra_test_binary_args") {
            if let Some(args) = parsed_args["extra_test_binary_args"].as_array() {
                let args_str: Vec<String> = args
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
                if file_type == cargo_runner_core::FileType::Standalone {
                    println!("      • exec.extra_test_binary_args: {:?}", args_str);
                } else {
                    println!("      • extra_test_binary_args: {:?}", args_str);
                }
            }
        }

        // Show removals
        if parsed_args.contains_key("remove_command") {
            println!("      • ❌ removed: command");
        }
        if parsed_args.contains_key("remove_subcommand") {
            println!("      • ❌ removed: subcommand");
        }
        if parsed_args.contains_key("remove_channel") {
            println!("      • ❌ removed: channel");
        }
        if parsed_args.contains_key("remove_args") {
            println!("      • ❌ removed: extra_args");
        }
        if parsed_args.contains_key("remove_env") {
            println!("      • ❌ removed: extra_env");
        }
        if parsed_args.contains_key("remove_test_args") {
            println!("      • ❌ removed: extra_test_binary_args");
        }
        if let Some(env_keys) = parsed_args
            .get("remove_env_keys")
            .and_then(|v| v.as_array())
        {
            for key in env_keys {
                if let Some(key_str) = key.as_str() {
                    println!("      • ❌ removed env: {}", key_str);
                }
            }
        }
    }

    Ok(())
}

fn create_file_level_override(
    filepath: &str,
    root: bool,
    override_args: Vec<String>,
) -> Result<()> {
    // Parse the override arguments - this returns a Map with the parsed configuration
    let override_config = parse_override_args(&override_args);

    // Create the match criteria for file-level override
    let mut match_criteria = Map::new();
    match_criteria.insert("file_path".to_string(), json!(filepath));

    // Create the final override entry
    let mut override_entry = Map::new();
    override_entry.insert("match".to_string(), Value::Object(match_criteria));

    // Separate cargo-specific fields from other fields
    let cargo_fields = vec![
        "command",
        "subcommand",
        "channel",
        "extra_args",
        "extra_env",
        "extra_test_binary_args",
        "package",
        "remove_command",
        "remove_subcommand",
        "remove_channel",
        "remove_args",
        "remove_env",
        "remove_test_args",
        "remove_env_keys",
    ];

    let mut cargo_config = Map::new();
    let mut has_cargo_fields = false;

    // Add all the override configurations, separating cargo fields
    for (key, value) in override_config {
        if cargo_fields.contains(&key.as_str()) {
            cargo_config.insert(key, value);
            has_cargo_fields = true;
        } else {
            // Non-cargo fields go at the root level
            override_entry.insert(key, value);
        }
    }

    // Only add cargo section if we have cargo fields
    if has_cargo_fields {
        override_entry.insert("cargo".to_string(), Value::Object(cargo_config));
    }

    // Debug: verify structure
    if has_cargo_fields {
        println!("   📝 Cargo fields moved to cargo section");
    }

    // Determine config file location
    let config_path = if root {
        // Get PROJECT_ROOT from environment
        let project_root = env::var("PROJECT_ROOT")
            .map(PathBuf::from)
            .or_else(|_| env::current_dir())
            .context("Failed to determine project root")?;
        project_root.join(".cargo-runner.json")
    } else {
        // Find the nearest .cargo-runner.json file
        let path = Path::new(filepath);
        let mut current = path.parent();

        while let Some(dir) = current {
            let config_path = dir.join(".cargo-runner.json");
            if config_path.exists() {
                println!("   📂 Found config at: {}", config_path.display());
                add_override_to_existing_config(&config_path, override_entry)?;

                println!("\n✅ File-level override created successfully!");
                println!("   📍 Config: {}", config_path.display());
                println!("   📄 File: {}", filepath);

                return Ok(());
            }
            current = dir.parent();
        }

        // If no config found, create one in the file's directory
        let parent_dir = path.parent().unwrap_or(Path::new("."));
        parent_dir.join(".cargo-runner.json")
    };

    // Add the override to the config
    add_override_to_existing_config(&config_path, override_entry.clone())?;

    println!("\n✅ File-level override created successfully!");
    println!("   📍 Config: {}", config_path.display());
    println!("   📄 File: {}", filepath);

    // Show what was configured
    if let Some(cargo_config) = override_entry.get("cargo") {
        if let Some(obj) = cargo_config.as_object() {
            if let Some(subcommand) = obj.get("subcommand") {
                println!(
                    "   🚀 Subcommand: cargo {}",
                    subcommand.as_str().unwrap_or("")
                );
            }
            if let Some(extra_args) = obj.get("extra_args") {
                if let Some(args) = extra_args.as_array() {
                    let args_str: Vec<String> = args
                        .iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect();
                    if !args_str.is_empty() {
                        println!("   📝 Extra args: {}", args_str.join(" "));
                    }
                }
            }
        }
    }

    if let Some(bazel_config) = override_entry.get("bazel") {
        if let Some(obj) = bazel_config.as_object() {
            if let Some(extra_test_args) = obj.get("extra_test_args") {
                if let Some(args) = extra_test_args.as_array() {
                    let args_str: Vec<String> = args
                        .iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect();
                    if !args_str.is_empty() {
                        println!("   📝 Extra test args (Bazel): {}", args_str.join(" "));
                    }
                }
            }
            if let Some(extra_run_args) = obj.get("extra_run_args") {
                if let Some(args) = extra_run_args.as_array() {
                    let args_str: Vec<String> = args
                        .iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect();
                    if !args_str.is_empty() {
                        println!("   📝 Extra run args (Bazel): {}", args_str.join(" "));
                    }
                }
            }
        }
    }

    Ok(())
}

fn add_override_to_existing_config(
    config_path: &Path,
    override_entry: Map<String, Value>,
) -> Result<()> {
    println!("   🔧 Adding override to config...");
    println!("   📝 Override entry: {:?}", override_entry);

    // Read existing config or create new one
    let mut config: Map<String, Value> = if config_path.exists() {
        let content = fs::read_to_string(config_path)
            .with_context(|| format!("Failed to read config from {}", config_path.display()))?;
        serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse config from {}", config_path.display()))?
    } else {
        let mut new_config = Map::new();
        new_config.insert(
            "cargo".to_string(),
            json!({
                "extra_args": [],
                "extra_env": {},
                "extra_test_binary_args": []
            }),
        );
        new_config.insert("overrides".to_string(), json!([]));
        new_config
    };

    // Get or create overrides array
    let overrides = config
        .entry("overrides".to_string())
        .or_insert(json!([]))
        .as_array_mut()
        .ok_or_else(|| anyhow::anyhow!("overrides is not an array"))?;

    // Add the new override
    overrides.push(Value::Object(override_entry));

    // Write back the config
    let json_string = serde_json::to_string_pretty(&config)?;
    fs::write(config_path, json_string)
        .with_context(|| format!("Failed to write config to {}", config_path.display()))?;

    Ok(())
}

fn parse_override_args(args: &[String]) -> Map<String, Value> {
    let mut result = Map::new();
    let mut extra_args = Vec::new();
    let mut extra_env = Map::new();
    let mut extra_test_binary_args = Vec::new();
    let mut command = None;
    let mut subcommand = None;
    let mut channel = None;

    // Fields to remove
    let mut remove_command = false;
    let mut remove_subcommand = false;
    let mut remove_channel = false;
    let mut remove_args = false;
    let mut remove_env = false;
    let mut remove_test_args = false;
    let mut env_to_remove = Vec::new();

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];

        // Check for removal tokens
        if arg.starts_with('-') && !arg.starts_with("--") {
            match arg.as_str() {
                "-command" | "-cmd" => remove_command = true,
                "-subcommand" | "-sub" => remove_subcommand = true,
                "-channel" | "-ch" => remove_channel = true,
                "-arg" => remove_args = true,
                "-env" => remove_env = true,
                "-test" | "-/" => remove_test_args = true,
                _ => {
                    // Check if it's an env var removal like -RUST_LOG
                    let env_name = &arg[1..];
                    if env_name.chars().all(|c| c.is_uppercase() || c == '_')
                        && !env_name.is_empty()
                    {
                        env_to_remove.push(env_name.to_string());
                    }
                }
            }
        }
        // Check for command token @command.subcommand
        else if arg.starts_with('@') {
            let token = &arg[1..];
            let parts: Vec<&str> = token.split('.').collect();

            if !parts.is_empty() {
                let cmd = parts[0];

                // Special handling for @cargo.subcommand format
                if cmd == "cargo" && parts.len() > 1 {
                    // Don't set command to cargo (it's the default)
                    // Just set the subcommand
                    subcommand = Some(parts[1..].join(" "));
                } else {
                    // For other commands like @dx, @trunk, @bazel, etc.
                    command = Some(cmd.to_string());
                    if parts.len() > 1 {
                        subcommand = Some(parts[1..].join(" "));
                    }
                }
            }
        }
        // Check for channel token +channel
        else if arg.starts_with('+') && arg.len() > 1 {
            channel = Some(arg[1..].to_string());
        }
        // Check for test binary args starting with /
        else if arg == "/" || arg.starts_with('/') {
            // The / acts like -- in cargo test, everything after goes to test binary

            // If there's content immediately after / (like /--show-output), add it
            if arg.len() > 1 {
                let arg_content = &arg[1..];
                extra_test_binary_args.push(arg_content.to_string());
            }

            // Collect ALL remaining args as test binary args
            while i + 1 < args.len() {
                i += 1;
                extra_test_binary_args.push(args[i].clone());
            }
        }
        // Check for environment variables (SCREAMING_CASE=value)
        else if arg
            .chars()
            .take_while(|&c| c != '=')
            .all(|c| c.is_uppercase() || c == '_')
            && arg.contains('=')
        {
            let parts: Vec<&str> = arg.splitn(2, '=').collect();
            if parts.len() == 2 && !parts[0].is_empty() {
                extra_env.insert(parts[0].to_string(), json!(parts[1]));
            }
        }
        // Everything else goes to extra_args
        else {
            extra_args.push(arg.clone());
        }

        i += 1;
    }

    // Build result based on what was parsed and removal flags
    if let Some(cmd) = command {
        if !remove_command {
            result.insert("command".to_string(), json!(cmd));
        }
    } else if remove_command {
        result.insert("remove_command".to_string(), json!(true));
    }

    if let Some(sub) = subcommand {
        if !remove_subcommand {
            result.insert("subcommand".to_string(), json!(sub));
        }
    } else if remove_subcommand {
        result.insert("remove_subcommand".to_string(), json!(true));
    }

    if let Some(ch) = channel {
        if !remove_channel {
            result.insert("channel".to_string(), json!(ch));
        }
    } else if remove_channel {
        result.insert("remove_channel".to_string(), json!(true));
    }

    if !extra_args.is_empty() && !remove_args {
        result.insert("extra_args".to_string(), json!(extra_args));
    } else if remove_args {
        result.insert("remove_args".to_string(), json!(true));
    }

    if !extra_env.is_empty() && !remove_env {
        result.insert("extra_env".to_string(), Value::Object(extra_env));
    } else if remove_env {
        result.insert("remove_env".to_string(), json!(true));
    }

    if !env_to_remove.is_empty() {
        result.insert("remove_env_keys".to_string(), json!(env_to_remove));
    }

    if !extra_test_binary_args.is_empty() && !remove_test_args {
        result.insert(
            "extra_test_binary_args".to_string(),
            json!(extra_test_binary_args),
        );
    } else if remove_test_args {
        result.insert("remove_test_args".to_string(), json!(true));
    }

    result
}
