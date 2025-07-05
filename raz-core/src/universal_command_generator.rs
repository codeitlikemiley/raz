//! Universal command generator for all Rust execution patterns
//!
//! This module generates appropriate commands for any Rust file based on
//! stateless detection of project type and file role.

use crate::file_detection::{
    EntryPoint, EntryPointType, ExecutionCapabilities, FileExecutionContext, FileRole,
    RustProjectType, SingleFileType,
};

use crate::framework_detection::{FrameworkType, PreciseFrameworkDetector};
#[cfg(feature = "tree-sitter-support")]
#[allow(unused_imports)] // TODO: Use this when implementing tree-sitter doctest detection
use crate::tree_sitter_test_detector::TreeSitterTestDetector;
use crate::{Command, CommandCategory, Position, ProjectContext, RazResult};
use raz_common::parse::parse_option;
use raz_config::{CommandOverride, OverrideMode};
use raz_override::OptionValue;
use raz_override::{FunctionContext, OverrideSystem};
use std::collections::HashMap;
use std::path::Path;

/// Universal command generator
pub struct UniversalCommandGenerator;

impl UniversalCommandGenerator {
    /// Generate all appropriate commands for the given execution context
    pub fn generate_commands(
        context: &FileExecutionContext,
        cursor: Option<Position>,
    ) -> RazResult<Vec<Command>> {
        Self::generate_commands_with_overrides(context, cursor, None, None)
    }

    /// Generate commands with optional override support
    pub fn generate_commands_with_overrides(
        context: &FileExecutionContext,
        cursor: Option<Position>,
        workspace: Option<&Path>,
        _override_key: Option<&str>,
    ) -> RazResult<Vec<Command>> {
        // First generate base commands
        let mut commands = Self::generate_commands_internal(context, cursor)?;

        // Load and apply saved overrides using the new override system
        if let Some(workspace_path) = workspace {
            // Create function context
            let function_context = if let Some(pos) = cursor {
                // Try to find the test function at the cursor to get its name
                let test_name =
                    Self::find_test_at_cursor(&context.entry_points, pos).map(|ep| ep.name.clone());

                FunctionContext {
                    file_path: context.file_path.clone(),
                    function_name: test_name,
                    line_number: pos.line as usize,
                    context: Some(format!("column:{}", pos.column)),
                }
            } else {
                FunctionContext {
                    file_path: context.file_path.clone(),
                    function_name: None,
                    line_number: 0,
                    context: None,
                }
            };

            // Try to load override using the new system
            let mut override_system = OverrideSystem::new(workspace_path).map_err(|e| {
                crate::error::RazError::Config {
                    message: format!("Failed to create override system: {e}"),
                }
            })?;

            if let Some(override_config) = override_system
                .resolve_override(&function_context)
                .map_err(|e| crate::error::RazError::Config {
                    message: format!("Failed to load override: {e}"),
                })?
            {
                // Apply the saved override to all commands
                for cmd in &mut commands {
                    Self::apply_override_to_command(cmd, &override_config);
                }
            }
            // Note: Legacy ConfigManager fallback removed as the new override system
            // handles all override cases. Use `raz override migrate` for legacy data.
        }

        Ok(commands)
    }

    /// Internal command generation without override loading
    fn generate_commands_internal(
        context: &FileExecutionContext,
        cursor: Option<Position>,
    ) -> RazResult<Vec<Command>> {
        let mut commands = Vec::new();

        // Core Algorithm: Priority order based on content and cursor
        // 1. TEST (highest priority) - mod test, #[test] macros, doc tests
        // 2. RUN - main functions, binaries
        // 3. BUILD - build.rs files
        // 4. BENCH - benchmarks

        // Check if cursor is targeting a specific test or main function
        let (cursor_in_test, cursor_on_main, cursor_in_doc_test) = if let Some(cursor_pos) = cursor
        {
            let test_entry = Self::find_test_at_cursor(&context.entry_points, cursor_pos);
            let main_entry = Self::find_main_at_cursor(&context.entry_points, cursor_pos);
            let in_doc_test = test_entry
                .as_ref()
                .is_some_and(|entry| matches!(entry.entry_type, EntryPointType::DocTest));
            (test_entry.is_some(), main_entry.is_some(), in_doc_test)
        } else {
            (false, false, false)
        };

        // 1. TEST COMMANDS - Highest priority if tests exist AND cursor is not on main
        if context.capabilities.can_test && !cursor_on_main {
            let mut test_commands = Self::generate_test_commands(context, cursor)?;

            // Set base priority for tests (highest)
            for cmd in &mut test_commands {
                cmd.priority = 100; // Base test priority

                // Extra boost if cursor is specifically on a test
                if cursor_in_test {
                    cmd.priority = cmd.priority.saturating_add(50);
                }

                // Lower priority if cursor is on doc test (doc tests should take precedence)
                if cursor_in_doc_test && !cmd.id.contains("doc-test") {
                    cmd.priority = cmd.priority.saturating_sub(25);
                }
            }

            commands.extend(test_commands);
        }

        // 2. DOC TEST COMMANDS - Only generate if doc tests exist or cursor is in doc test
        if context.capabilities.can_doc_test {
            let has_doc_tests = context
                .entry_points
                .iter()
                .any(|ep| ep.entry_type == crate::file_detection::EntryPointType::DocTest);

            // Only generate doc test commands if:
            // 1. No cursor provided (general case), OR
            // 2. Cursor is specifically in a doc test, OR
            // 3. File has doc test entry points
            if cursor.is_none() || cursor_in_doc_test || has_doc_tests {
                let mut doc_test_commands = Self::generate_doc_test_commands(context)?;

                // Set priority just below regular tests
                for cmd in &mut doc_test_commands {
                    cmd.priority = 95;

                    // Boost priority if cursor is on a doc test
                    if cursor_in_doc_test {
                        cmd.priority = cmd.priority.saturating_add(30);
                    }
                }

                commands.extend(doc_test_commands);
            }
        }

        // 3. RUN COMMANDS - High priority if cursor is on main, otherwise lower
        if context.capabilities.can_run {
            let mut run_commands = Self::generate_run_commands(context)?;

            // Set run priority based on cursor position and test existence
            let run_priority = if cursor_on_main {
                150 // Highest priority when cursor is on main function
            } else if context.capabilities.can_test && cursor_in_test {
                50 // Lower priority if tests exist and cursor is in test
            } else if context.capabilities.can_test {
                75 // Medium priority if tests exist but cursor not in test
            } else {
                90 // High priority if no tests exist
            };

            for cmd in &mut run_commands {
                cmd.priority = run_priority;
            }

            commands.extend(run_commands);

            // Framework-specific commands with enhanced priority logic
            let mut framework_commands = Self::generate_advanced_framework_commands(context)?;
            for cmd in &mut framework_commands {
                // Give framework commands higher priority when:
                // 1. Cursor is on main.rs (framework entry point)
                // 2. It's a serve/dev command for web frameworks
                let framework_priority = if cursor_on_main && Self::is_web_framework_command(cmd) {
                    160 // Highest priority for main.rs framework commands (higher than cargo run)
                } else if Self::is_web_framework_command(cmd) {
                    120 // High priority for framework serve/dev commands
                } else {
                    run_priority // Default to run priority for other framework commands
                };
                cmd.priority = framework_priority;
            }
            commands.extend(framework_commands);
        }

        // 4. BUILD SCRIPT COMMANDS - Special handling for build.rs files
        if matches!(
            &context.file_role,
            crate::file_detection::FileRole::BuildScript
        ) {
            let mut build_commands = Self::generate_build_script_commands(context)?;

            for cmd in &mut build_commands {
                cmd.priority = 80; // Medium priority
            }

            commands.extend(build_commands);
        }

        // 5. BENCHMARK COMMANDS - Lower priority
        if context.capabilities.can_bench {
            let mut bench_commands = Self::generate_bench_commands(context)?;

            for cmd in &mut bench_commands {
                cmd.priority = 70; // Lower than run, higher than build
            }

            commands.extend(bench_commands);
        }

        // Sort by priority (highest first)
        commands.sort_by(|a, b| b.priority.cmp(&a.priority));

        Ok(commands)
    }

    /// Generate commands with runtime override input
    pub fn generate_commands_with_runtime_override(
        context: &FileExecutionContext,
        cursor: Option<Position>,
        _workspace: Option<&Path>,
        _override_key: Option<&str>,
        override_input: &str,
    ) -> RazResult<Vec<Command>> {
        // First generate base commands
        let mut commands = Self::generate_commands(context, cursor)?;

        // Parse the runtime override
        let command = if !commands.is_empty() {
            // Detect command from first generated command
            match commands[0].command.as_str() {
                "cargo" => {
                    // Look at first arg to determine subcommand
                    commands[0]
                        .args
                        .first()
                        .map(|s| s.as_str())
                        .unwrap_or("run")
                }
                _ => "run",
            }
        } else {
            "run"
        };

        let parsed_override = raz_override::parse_override_to_command(command, override_input)
            .map_err(|e| crate::error::RazError::Config {
                message: format!("Failed to parse override: {e}"),
            })?;

        // Apply the runtime override to all commands
        for cmd in &mut commands {
            Self::apply_override_to_command(cmd, &parsed_override);
        }

        Ok(commands)
    }

    /// Apply command override to a command
    fn apply_override_to_command(command: &mut Command, override_config: &CommandOverride) {
        let is_append = matches!(override_config.mode, OverrideMode::Append);

        // Apply environment variables
        if !override_config.env.is_empty() {
            if is_append {
                // Append mode - merge with existing
                for (key, value) in &override_config.env {
                    command.env.insert(key.clone(), value.clone());
                }
            } else {
                // Replace mode - clear and set new
                command.env.clear();
                for (key, value) in &override_config.env {
                    command.env.insert(key.clone(), value.clone());
                }
            }
        }

        // Apply cargo options
        if !override_config.cargo_options.is_empty() {
            // Collect all cargo option parts first
            let mut all_cargo_parts = Vec::new();
            for option in &override_config.cargo_options {
                match parse_option(option) {
                    Ok(parts) if !parts.is_empty() => {
                        if is_append {
                            // Add if not already present
                            if !command.args.contains(&parts[0]) {
                                all_cargo_parts.extend(parts);
                            }
                        } else {
                            // Just add the options
                            all_cargo_parts.extend(parts);
                        }
                    }
                    Ok(_) => {} // Empty parts, skip
                    Err(e) => {
                        eprintln!("Warning: Failed to parse cargo option '{option}': {e}");
                    }
                }
            }

            // Now insert all cargo options before the -- separator if it exists
            if !all_cargo_parts.is_empty() {
                // Find the separator position
                let separator_pos = command.args.iter().position(|arg| arg == "--");

                if let Some(sep_pos) = separator_pos {
                    // Insert all cargo options before the -- separator
                    for (i, part) in all_cargo_parts.into_iter().enumerate() {
                        command.args.insert(sep_pos + i, part);
                    }
                } else {
                    // No separator, add at the end
                    command.args.extend(all_cargo_parts);
                }
            }
        }

        // Apply rustc options
        if !override_config.rustc_options.is_empty() {
            // Find or add -- separator
            let separator_pos = command.args.iter().position(|arg| arg == "--");
            if separator_pos.is_none()
                && (!override_config.rustc_options.is_empty() || !override_config.args.is_empty())
            {
                command.args.push("--".to_string());
            }

            // Add rustc options after --
            for option in &override_config.rustc_options {
                command.args.push(option.clone());
            }
        }

        // Apply extra arguments
        if !override_config.args.is_empty() {
            // For test commands, handle -- separator specially
            let is_test_command = command.category == CommandCategory::Test
                || command.args.iter().any(|arg| arg == "test");

            if is_test_command {
                // Test commands need special handling for --
                if is_append {
                    // In append mode, intelligently merge test arguments
                    if let Some(separator_pos) = command.args.iter().position(|arg| arg == "--") {
                        // Get existing args after separator
                        let existing_args: Vec<String> = command.args[separator_pos + 1..].to_vec();

                        // Remove duplicates: don't add flags that already exist
                        let mut args_to_add = Vec::new();
                        for arg in &override_config.args {
                            if !existing_args.contains(arg) {
                                args_to_add.push(arg.clone());
                            }
                        }

                        // Insert new args after the separator
                        let insert_pos = separator_pos + 1;
                        command.args.splice(insert_pos..insert_pos, args_to_add);
                    } else {
                        // No separator found, add one and then the args
                        command.args.push("--".to_string());
                        command.args.extend(override_config.args.clone());
                    }
                } else {
                    // In replace mode, find -- position and replace everything after it
                    if let Some(separator_pos) = command.args.iter().position(|arg| arg == "--") {
                        // Remove everything after -- and add new args
                        command.args.truncate(separator_pos + 1);
                        command.args.extend(override_config.args.clone());
                    } else {
                        // No separator found, add one and then the args
                        command.args.push("--".to_string());
                        command.args.extend(override_config.args.clone());
                    }
                }
            } else {
                // For non-test commands, just append the args directly
                if is_append {
                    command.args.extend(override_config.args.clone());
                } else {
                    // In replace mode for non-test commands, we might want to preserve some core args
                    // For now, just extend the args
                    command.args.extend(override_config.args.clone());
                }
            }
        }
    }

    #[allow(dead_code)]
    /// Apply options to a command  
    fn apply_options_to_command(
        command: &mut Command,
        options: &HashMap<String, OptionValue>,
        append_mode: bool,
    ) {
        use crate::rustc_options::translate_cargo_to_rustc;

        // Check if this is a rustc command
        let is_rustc = command.command == "rustc"
            || (command.command == "sh" && command.args.iter().any(|arg| arg.contains("rustc")));

        for (option, value) in options {
            match value {
                OptionValue::Flag(true) => {
                    // For rustc commands, translate the option
                    let actual_option = if is_rustc {
                        if let Some(rustc_opt) = translate_cargo_to_rustc(option) {
                            rustc_opt.to_string()
                        } else {
                            // Skip invalid rustc options
                            continue;
                        }
                    } else {
                        option.clone()
                    };

                    // Add flag if not present
                    if !command.args.contains(&actual_option) {
                        // For rustc commands in sh -c, we need to insert the option in the rustc part
                        if command.command == "sh"
                            && command.args.first() == Some(&"-c".to_string())
                        {
                            if let Some(cmd_string) = command.args.get_mut(1) {
                                // Parse and modify the shell command
                                if cmd_string.contains("rustc")
                                    && !cmd_string.contains(&actual_option)
                                {
                                    // For release builds, add full optimization flags
                                    if option == "--release" && is_rustc {
                                        let opt_flags = "-O -C lto=yes -C codegen-units=1";
                                        *cmd_string = cmd_string
                                            .replace("rustc", &format!("rustc {opt_flags}"));
                                    } else {
                                        *cmd_string = cmd_string
                                            .replace("rustc", &format!("rustc {actual_option}"));
                                    }
                                }
                            }
                        } else {
                            // Normal insertion
                            let insert_pos = command
                                .args
                                .iter()
                                .position(|arg| arg == "--")
                                .unwrap_or(command.args.len());

                            // For release builds, add full optimization flags
                            if option == "--release" && is_rustc {
                                command.args.insert(insert_pos, "-O".to_string());
                                command.args.insert(insert_pos + 1, "-C".to_string());
                                command.args.insert(insert_pos + 2, "lto=yes".to_string());
                                command.args.insert(insert_pos + 3, "-C".to_string());
                                command
                                    .args
                                    .insert(insert_pos + 4, "codegen-units=1".to_string());
                            } else {
                                command.args.insert(insert_pos, actual_option);
                            }
                        }
                    }
                }
                OptionValue::Flag(false) => {
                    // Remove flag if present
                    command.args.retain(|arg| arg != option);
                }
                OptionValue::Single(val) => {
                    if let Some(opt_idx) = command.args.iter().position(|arg| arg == option) {
                        if append_mode && opt_idx + 1 < command.args.len() {
                            // Append mode - keep existing, might need special handling
                            // For now, just replace
                            command.args[opt_idx + 1] = val.clone();
                        } else {
                            // Replace the value
                            if opt_idx + 1 < command.args.len() {
                                command.args[opt_idx + 1] = val.clone();
                            } else {
                                command.args.push(val.clone());
                            }
                        }
                    } else {
                        // Add new option
                        let insert_pos = command
                            .args
                            .iter()
                            .position(|arg| arg == "--")
                            .unwrap_or(command.args.len());
                        command.args.insert(insert_pos, option.clone());
                        command.args.insert(insert_pos + 1, val.clone());
                    }
                }
                OptionValue::Multiple(values) => {
                    let value_str = values.join(",");
                    if let Some(opt_idx) = command.args.iter().position(|arg| arg == option) {
                        if append_mode && opt_idx + 1 < command.args.len() {
                            // Append mode - merge values
                            let existing = &command.args[opt_idx + 1];
                            let mut all_values: Vec<String> =
                                existing.split(',').map(|s| s.to_string()).collect();
                            all_values.extend(values.clone());
                            command.args[opt_idx + 1] = all_values.join(",");
                        } else {
                            // Replace the value
                            if opt_idx + 1 < command.args.len() {
                                command.args[opt_idx + 1] = value_str;
                            } else {
                                command.args.push(value_str);
                            }
                        }
                    } else {
                        // Add new option
                        let insert_pos = command
                            .args
                            .iter()
                            .position(|arg| arg == "--")
                            .unwrap_or(command.args.len());
                        command.args.insert(insert_pos, option.clone());
                        command.args.insert(insert_pos + 1, value_str);
                    }
                }
            }
        }
    }

    /// Generate run commands
    fn generate_run_commands(context: &FileExecutionContext) -> RazResult<Vec<Command>> {
        let mut commands = Vec::new();

        match &context.project_type {
            RustProjectType::CargoWorkspace { root, members } => {
                commands.extend(Self::generate_cargo_workspace_run_commands(
                    root,
                    members,
                    &context.file_role,
                    &context.capabilities,
                )?);
            }
            RustProjectType::CargoPackage { root, package_name } => {
                commands.extend(Self::generate_cargo_package_run_commands(
                    root,
                    package_name,
                    &context.file_role,
                    &context.capabilities,
                )?);
            }
            RustProjectType::CargoScript { file_path, .. } => {
                commands.extend(Self::generate_cargo_script_run_commands(file_path)?);
            }
            RustProjectType::SingleFile {
                file_path,
                file_type,
            } => {
                commands.extend(Self::generate_single_file_run_commands(
                    file_path, file_type,
                )?);
            }
        }

        Ok(commands)
    }

    /// Generate test commands
    fn generate_test_commands(
        context: &FileExecutionContext,
        cursor: Option<Position>,
    ) -> RazResult<Vec<Command>> {
        let mut commands = Vec::new();

        // If cursor is on a specific test, prioritize that test
        if let Some(cursor_pos) = cursor {
            if let Some(test_entry) = Self::find_test_at_cursor(&context.entry_points, cursor_pos) {
                // Generate specific test commands for the test at cursor
                commands.extend(Self::generate_specific_test_commands(context, test_entry)?);
                // Boost priority for cursor-specific commands
                for cmd in &mut commands {
                    if matches!(test_entry.entry_type, EntryPointType::DocTest) {
                        cmd.priority = cmd.priority.saturating_add(50);
                    }
                }
            }
        }

        // General test commands
        match &context.project_type {
            RustProjectType::CargoWorkspace { root, members } => {
                commands.extend(Self::generate_cargo_workspace_test_commands(
                    root,
                    members,
                    &context.file_role,
                )?);
            }
            RustProjectType::CargoPackage { root, package_name } => {
                commands.extend(Self::generate_cargo_package_test_commands(
                    root,
                    package_name,
                    &context.file_role,
                )?);
            }
            RustProjectType::CargoScript { file_path, .. } => {
                commands.extend(Self::generate_cargo_script_test_commands(file_path)?);
            }
            RustProjectType::SingleFile {
                file_path,
                file_type,
            } => {
                commands.extend(Self::generate_single_file_test_commands(
                    file_path, file_type,
                )?);
            }
        }

        Ok(commands)
    }

    /// Generate benchmark commands
    fn generate_bench_commands(context: &FileExecutionContext) -> RazResult<Vec<Command>> {
        let mut commands = Vec::new();

        match &context.project_type {
            RustProjectType::CargoWorkspace { root, members } => {
                commands.extend(Self::generate_cargo_workspace_bench_commands(
                    root,
                    members,
                    &context.file_role,
                )?);
            }
            RustProjectType::CargoPackage { root, package_name } => {
                commands.extend(Self::generate_cargo_package_bench_commands(
                    root,
                    package_name,
                    &context.file_role,
                )?);
            }
            RustProjectType::SingleFile { file_path, .. } => {
                commands.extend(Self::generate_single_file_bench_commands(file_path)?);
            }
            _ => {} // Cargo scripts don't typically have benchmarks
        }

        Ok(commands)
    }

    /// Generate doc-test commands
    fn generate_doc_test_commands(context: &FileExecutionContext) -> RazResult<Vec<Command>> {
        let mut commands = Vec::new();

        match &context.project_type {
            RustProjectType::CargoWorkspace { root, members } => {
                commands.extend(Self::generate_cargo_workspace_doc_test_commands(
                    root,
                    members,
                    &context.file_role,
                )?);
            }
            RustProjectType::CargoPackage { root, package_name } => {
                commands.extend(Self::generate_cargo_package_doc_test_commands(
                    root,
                    package_name,
                    &context.file_role,
                )?);
            }
            RustProjectType::SingleFile { file_path, .. } => {
                // Only generate doc test commands if the file actually has doc test entry points
                let has_doc_tests = context
                    .entry_points
                    .iter()
                    .any(|ep| ep.entry_type == EntryPointType::DocTest);
                if has_doc_tests {
                    commands.extend(Self::generate_single_file_doc_test_commands(file_path)?);
                }
            }
            _ => {} // Cargo scripts don't typically have doc tests
        }

        Ok(commands)
    }

    /// Generate build script specific commands
    fn generate_build_script_commands(context: &FileExecutionContext) -> RazResult<Vec<Command>> {
        let mut commands = Vec::new();

        match &context.project_type {
            RustProjectType::CargoWorkspace { root, .. }
            | RustProjectType::CargoPackage { root, .. } => {
                // Primary: Trigger build script via cargo build
                commands.push(Command {
                    id: "build-script-trigger".to_string(),
                    label: "Run build script".to_string(),
                    description: Some("Trigger build script execution via cargo build".to_string()),
                    command: "cargo".to_string(),
                    args: vec!["build".to_string()],
                    env: HashMap::new(),
                    cwd: Some(root.to_path_buf()),
                    category: CommandCategory::Build,
                    priority: 80,
                    conditions: Vec::new(),
                    tags: vec!["build".to_string(), "script".to_string()],
                    requires_input: false,
                    estimated_duration: Some(10),
                });

                // Alternative: Run build script directly (only if it has a main function)
                if context.capabilities.can_run {
                    commands.push(Command {
                        id: "build-script-direct".to_string(),
                        label: "Execute build script directly".to_string(),
                        description: Some(
                            "Compile and run build script as standalone executable".to_string(),
                        ),
                        command: "sh".to_string(),
                        args: vec![
                            "-c".to_string(),
                            format!("rustc {} && ./build", root.join("build.rs").display()),
                        ],
                        env: HashMap::new(),
                        cwd: Some(root.to_path_buf()),
                        category: CommandCategory::Build,
                        priority: 60,
                        conditions: Vec::new(),
                        tags: vec![
                            "build".to_string(),
                            "script".to_string(),
                            "direct".to_string(),
                        ],
                        requires_input: false,
                        estimated_duration: Some(5),
                    });
                }
            }
            RustProjectType::SingleFile { file_path, .. } => {
                // Handle standalone build.rs files
                commands.extend(Self::generate_standalone_build_script_commands(file_path)?);
            }
            _ => {} // Build scripts in cargo scripts don't make sense
        }

        Ok(commands)
    }

    // Cargo Workspace Commands

    fn generate_cargo_workspace_run_commands(
        root: &Path,
        members: &[crate::file_detection::WorkspaceMember],
        file_role: &FileRole,
        _capabilities: &ExecutionCapabilities,
    ) -> RazResult<Vec<Command>> {
        let mut commands = Vec::new();

        match file_role {
            FileRole::MainBinary { binary_name } | FileRole::AdditionalBinary { binary_name } => {
                // Find the package this file belongs to
                if let Some(package_name) = Self::find_package_for_file(members, root) {
                    commands.push(Self::create_cargo_run_command(
                        root,
                        Some(&package_name),
                        Some(binary_name),
                        100,
                    ));
                }
            }
            FileRole::FrontendLibrary { framework } => {
                commands.extend(Self::generate_framework_commands(root, framework)?);
            }
            FileRole::Example { example_name } => {
                if let Some(package_name) = Self::find_package_for_file(members, root) {
                    commands.push(Self::create_cargo_example_command(
                        root,
                        Some(&package_name),
                        example_name,
                        90,
                    ));
                }
            }
            _ => {}
        }

        Ok(commands)
    }

    fn generate_cargo_workspace_test_commands(
        root: &Path,
        members: &[crate::file_detection::WorkspaceMember],
        file_role: &FileRole,
    ) -> RazResult<Vec<Command>> {
        let mut commands = Vec::new();

        if let Some(package_name) = Self::find_package_for_file(members, root) {
            match file_role {
                FileRole::IntegrationTest { test_name } => {
                    commands.push(Self::create_cargo_test_command(
                        root,
                        Some(&package_name),
                        Some(&format!("--test {test_name}")),
                        "integration test",
                        95,
                    ));
                }
                FileRole::LibraryRoot => {
                    commands.push(Self::create_cargo_test_command(
                        root,
                        Some(&package_name),
                        Some("--lib"),
                        "library tests",
                        90,
                    ));
                }
                FileRole::Module => {
                    // Module files should use --lib for library tests
                    commands.push(Self::create_cargo_test_command(
                        root,
                        Some(&package_name),
                        Some("--lib"),
                        "module tests",
                        90,
                    ));
                }
                _ => {
                    commands.push(Self::create_cargo_test_command(
                        root,
                        Some(&package_name),
                        None,
                        "all tests",
                        85,
                    ));
                }
            }
        }

        Ok(commands)
    }

    fn generate_cargo_workspace_bench_commands(
        root: &Path,
        members: &[crate::file_detection::WorkspaceMember],
        file_role: &FileRole,
    ) -> RazResult<Vec<Command>> {
        let mut commands = Vec::new();

        if let Some(package_name) = Self::find_package_for_file(members, root) {
            match file_role {
                FileRole::Benchmark { bench_name } => {
                    commands.push(Self::create_cargo_bench_command(
                        root,
                        Some(&package_name),
                        Some(&format!("--bench {bench_name}")),
                        "specific benchmark",
                        95,
                    ));
                }
                _ => {
                    commands.push(Self::create_cargo_bench_command(
                        root,
                        Some(&package_name),
                        None,
                        "all benchmarks",
                        85,
                    ));
                }
            }
        }

        Ok(commands)
    }

    fn generate_cargo_workspace_doc_test_commands(
        root: &Path,
        members: &[crate::file_detection::WorkspaceMember],
        _file_role: &FileRole,
    ) -> RazResult<Vec<Command>> {
        let mut commands = Vec::new();

        if let Some(package_name) = Self::find_package_for_file(members, root) {
            commands.push(Self::create_cargo_doc_test_command(
                root,
                Some(&package_name),
                "doc tests",
                85,
            ));
        }

        Ok(commands)
    }

    // Cargo Package Commands

    fn generate_cargo_package_run_commands(
        root: &Path,
        package_name: &str,
        file_role: &FileRole,
        _capabilities: &ExecutionCapabilities,
    ) -> RazResult<Vec<Command>> {
        let mut commands = Vec::new();

        match file_role {
            FileRole::MainBinary { binary_name } | FileRole::AdditionalBinary { binary_name } => {
                commands.push(Self::create_cargo_run_command(
                    root,
                    Some(package_name),
                    Some(binary_name),
                    100,
                ));
            }
            FileRole::FrontendLibrary { framework } => {
                commands.extend(Self::generate_framework_commands(root, framework)?);
            }
            FileRole::Example { example_name } => {
                commands.push(Self::create_cargo_example_command(
                    root,
                    Some(package_name),
                    example_name,
                    90,
                ));
            }
            FileRole::BuildScript => {
                // Build scripts are now handled separately in generate_build_script_commands
                // to ensure they always generate commands regardless of can_run capability
            }
            FileRole::Standalone => {
                // Standalone files should not be handled here - they will be handled
                // as single files through the detection logic
            }
            _ => {}
        }

        Ok(commands)
    }

    fn generate_cargo_package_test_commands(
        root: &Path,
        package_name: &str,
        file_role: &FileRole,
    ) -> RazResult<Vec<Command>> {
        let mut commands = Vec::new();

        match file_role {
            FileRole::IntegrationTest { test_name } => {
                commands.push(Self::create_cargo_test_command(
                    root,
                    Some(package_name),
                    Some(&format!("--test {test_name}")),
                    "integration test",
                    95,
                ));
            }
            FileRole::LibraryRoot => {
                commands.push(Self::create_cargo_test_command(
                    root,
                    Some(package_name),
                    Some("--lib"),
                    "library tests",
                    90,
                ));
            }
            FileRole::Module => {
                // Module files should use --lib for library tests
                commands.push(Self::create_cargo_test_command(
                    root,
                    Some(package_name),
                    Some("--lib"),
                    "module tests",
                    90,
                ));
            }
            _ => {
                commands.push(Self::create_cargo_test_command(
                    root,
                    Some(package_name),
                    None,
                    "all tests",
                    85,
                ));
            }
        }

        Ok(commands)
    }

    fn generate_cargo_package_bench_commands(
        root: &Path,
        package_name: &str,
        file_role: &FileRole,
    ) -> RazResult<Vec<Command>> {
        let mut commands = Vec::new();

        match file_role {
            FileRole::Benchmark { bench_name } => {
                commands.push(Self::create_cargo_bench_command(
                    root,
                    Some(package_name),
                    Some(&format!("--bench {bench_name}")),
                    "specific benchmark",
                    95,
                ));
            }
            _ => {
                commands.push(Self::create_cargo_bench_command(
                    root,
                    Some(package_name),
                    None,
                    "all benchmarks",
                    85,
                ));
            }
        }

        Ok(commands)
    }

    fn generate_cargo_package_doc_test_commands(
        root: &Path,
        package_name: &str,
        _file_role: &FileRole,
    ) -> RazResult<Vec<Command>> {
        Ok(vec![Self::create_cargo_doc_test_command(
            root,
            Some(package_name),
            "doc tests",
            85,
        )])
    }

    // Cargo Script Commands

    fn generate_cargo_script_run_commands(file_path: &Path) -> RazResult<Vec<Command>> {
        let mut commands = Vec::new();

        let file_name = file_path.to_string_lossy();

        commands.push(Command {
            id: "cargo-script-run".to_string(),
            label: "Run cargo script".to_string(),
            description: Some("Execute the cargo script".to_string()),
            command: "cargo".to_string(),
            args: vec![
                "+nightly".to_string(),
                "-Zscript".to_string(),
                file_name.to_string(),
            ],
            env: HashMap::new(),
            cwd: file_path.parent().map(|p| p.to_path_buf()),
            category: CommandCategory::Run,
            priority: 100,
            conditions: Vec::new(),
            tags: vec!["script".to_string(), "nightly".to_string()],
            requires_input: false,
            estimated_duration: Some(3),
        });

        Ok(commands)
    }

    fn generate_cargo_script_test_commands(file_path: &Path) -> RazResult<Vec<Command>> {
        let mut commands = Vec::new();

        let file_name = file_path.to_string_lossy();

        commands.push(Command {
            id: "cargo-script-test".to_string(),
            label: "Test cargo script".to_string(),
            description: Some("Run tests in the cargo script".to_string()),
            command: "cargo".to_string(),
            args: vec![
                "+nightly".to_string(),
                "-Zscript".to_string(),
                file_name.to_string(),
                "--".to_string(),
                "--test".to_string(),
            ],
            env: HashMap::new(),
            cwd: file_path.parent().map(|p| p.to_path_buf()),
            category: CommandCategory::Test,
            priority: 90,
            conditions: Vec::new(),
            tags: vec![
                "script".to_string(),
                "test".to_string(),
                "nightly".to_string(),
            ],
            requires_input: false,
            estimated_duration: Some(5),
        });

        Ok(commands)
    }

    // Single File Commands

    /// Generate commands for standalone build.rs files
    fn generate_standalone_build_script_commands(file_path: &Path) -> RazResult<Vec<Command>> {
        let mut commands = Vec::new();

        let file_name = file_path.to_string_lossy();
        let parent_dir = file_path.parent().unwrap_or_else(|| Path::new("."));

        // Check if this is in a cargo project directory
        if parent_dir.join("Cargo.toml").exists() {
            // In a cargo project - trigger via cargo build
            commands.push(Command {
                id: "build-script-cargo".to_string(),
                label: "Run build script".to_string(),
                description: Some("Trigger build script execution via cargo build".to_string()),
                command: "cargo".to_string(),
                args: vec!["build".to_string()],
                env: HashMap::new(),
                cwd: Some(parent_dir.to_path_buf()),
                category: CommandCategory::Build,
                priority: 90,
                conditions: Vec::new(),
                tags: vec!["build".to_string(), "script".to_string()],
                requires_input: false,
                estimated_duration: Some(10),
            });
        }

        // Always offer direct execution option
        let executable_name = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("build");

        commands.push(Command {
            id: "build-script-direct".to_string(),
            label: "Execute build script directly".to_string(),
            description: Some("Compile and run build script as standalone executable".to_string()),
            command: "sh".to_string(),
            args: vec![
                "-c".to_string(),
                format!("rustc {} && ./{}", file_name, executable_name),
            ],
            env: HashMap::new(),
            cwd: Some(parent_dir.to_path_buf()),
            category: CommandCategory::Build,
            priority: 80,
            conditions: Vec::new(),
            tags: vec![
                "build".to_string(),
                "script".to_string(),
                "direct".to_string(),
            ],
            requires_input: false,
            estimated_duration: Some(5),
        });

        Ok(commands)
    }

    fn generate_single_file_run_commands(
        file_path: &Path,
        file_type: &SingleFileType,
    ) -> RazResult<Vec<Command>> {
        let mut commands = Vec::new();

        if let SingleFileType::Executable = file_type {
            let file_name = file_path.to_string_lossy();
            let executable_name = file_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("program");

            commands.push(Command {
                id: "rustc-run".to_string(),
                label: format!("Compile and run {executable_name}"),
                description: Some("Compile with rustc and execute".to_string()),
                command: "sh".to_string(),
                args: vec![
                    "-c".to_string(),
                    format!("rustc '{}' && './{}'", file_name, executable_name),
                ],
                env: HashMap::new(),
                cwd: file_path.parent().map(|p| p.to_path_buf()),
                category: CommandCategory::Run,
                priority: 100,
                conditions: Vec::new(),
                tags: vec!["rustc".to_string(), "standalone".to_string()],
                requires_input: false,
                estimated_duration: Some(10),
            });

            // Optimized version
            commands.push(Command {
                id: "rustc-run-optimized".to_string(),
                label: format!("Compile and run {executable_name} (optimized)"),
                description: Some("Compile with optimizations and execute".to_string()),
                command: "sh".to_string(),
                args: vec![
                    "-c".to_string(),
                    format!("rustc -O '{}' && './{}'", file_name, executable_name),
                ],
                env: HashMap::new(),
                cwd: file_path.parent().map(|p| p.to_path_buf()),
                category: CommandCategory::Run,
                priority: 80,
                conditions: Vec::new(),
                tags: vec![
                    "rustc".to_string(),
                    "optimized".to_string(),
                    "standalone".to_string(),
                ],
                requires_input: false,
                estimated_duration: Some(15),
            });
        }
        // Library/Test/Module single files don't run directly

        Ok(commands)
    }

    fn generate_single_file_test_commands(
        file_path: &Path,
        _file_type: &SingleFileType,
    ) -> RazResult<Vec<Command>> {
        let mut commands = Vec::new();

        let file_name = file_path.to_string_lossy();
        let test_name = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("test");

        commands.push(Command {
            id: "rustc-test".to_string(),
            label: format!("Test all ({test_name})"),
            description: Some("Compile and run all tests with rustc".to_string()),
            command: "sh".to_string(),
            args: vec![
                "-c".to_string(),
                format!(
                    "rustc --test '{}' -o './{}_test' && './{}_test'",
                    file_name, test_name, test_name
                ),
            ],
            env: HashMap::new(),
            cwd: file_path.parent().map(|p| p.to_path_buf()),
            category: CommandCategory::Test,
            priority: 90,
            conditions: Vec::new(),
            tags: vec![
                "rustc".to_string(),
                "test".to_string(),
                "standalone".to_string(),
            ],
            requires_input: false,
            estimated_duration: Some(8),
        });

        Ok(commands)
    }

    fn generate_single_file_bench_commands(file_path: &Path) -> RazResult<Vec<Command>> {
        let mut commands = Vec::new();

        let file_name = file_path.to_string_lossy();
        let bench_name = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("bench");

        commands.push(Command {
            id: "rustc-bench".to_string(),
            label: format!("Benchmark {bench_name}"),
            description: Some("Compile and run benchmarks with rustc".to_string()),
            command: "sh".to_string(),
            args: vec![
                "-c".to_string(),
                format!("rustc --test '{}' && './{}'", file_name, bench_name),
            ],
            env: HashMap::new(),
            cwd: file_path.parent().map(|p| p.to_path_buf()),
            category: CommandCategory::Custom("benchmark".to_string()),
            priority: 85,
            conditions: Vec::new(),
            tags: vec![
                "rustc".to_string(),
                "bench".to_string(),
                "standalone".to_string(),
            ],
            requires_input: false,
            estimated_duration: Some(15),
        });

        Ok(commands)
    }

    fn generate_single_file_doc_test_commands(file_path: &Path) -> RazResult<Vec<Command>> {
        let mut commands = Vec::new();

        let file_name = file_path.to_string_lossy();

        commands.push(Command {
            id: "rustdoc-test".to_string(),
            label: "Test documentation examples".to_string(),
            description: Some("Run documentation tests with rustdoc".to_string()),
            command: "rustdoc".to_string(),
            args: vec!["--test".to_string(), file_name.to_string()],
            env: HashMap::new(),
            cwd: file_path.parent().map(|p| p.to_path_buf()),
            category: CommandCategory::Test,
            priority: 80,
            conditions: Vec::new(),
            tags: vec![
                "rustdoc".to_string(),
                "doctest".to_string(),
                "standalone".to_string(),
            ],
            requires_input: false,
            estimated_duration: Some(5),
        });

        Ok(commands)
    }

    // Framework-specific commands

    fn generate_framework_commands(root: &Path, framework: &str) -> RazResult<Vec<Command>> {
        let mut commands = Vec::new();

        match framework {
            "leptos" => {
                commands.push(Command {
                    id: "leptos-watch".to_string(),
                    label: "Leptos Dev Server".to_string(),
                    description: Some("Run Leptos development server with hot reload".to_string()),
                    command: "cargo".to_string(),
                    args: vec!["leptos".to_string(), "watch".to_string()],
                    env: HashMap::new(),
                    cwd: Some(root.to_path_buf()),
                    category: CommandCategory::Run,
                    priority: 100,
                    conditions: Vec::new(),
                    tags: vec!["leptos".to_string(), "dev".to_string(), "watch".to_string()],
                    requires_input: false,
                    estimated_duration: Some(5),
                });
            }
            "dioxus" => {
                commands.push(Command {
                    id: "dioxus-serve".to_string(),
                    label: "Dioxus Dev Server".to_string(),
                    description: Some("Run Dioxus development server".to_string()),
                    command: "dx".to_string(),
                    args: vec!["serve".to_string()],
                    env: HashMap::new(),
                    cwd: Some(root.to_path_buf()),
                    category: CommandCategory::Run,
                    priority: 100,
                    conditions: Vec::new(),
                    tags: vec!["dioxus".to_string(), "dev".to_string(), "serve".to_string()],
                    requires_input: false,
                    estimated_duration: Some(5),
                });
            }
            "yew" => {
                commands.push(Command {
                    id: "trunk-serve".to_string(),
                    label: "Trunk Dev Server".to_string(),
                    description: Some("Run Yew app with Trunk dev server".to_string()),
                    command: "trunk".to_string(),
                    args: vec!["serve".to_string()],
                    env: HashMap::new(),
                    cwd: Some(root.to_path_buf()),
                    category: CommandCategory::Run,
                    priority: 100,
                    conditions: Vec::new(),
                    tags: vec!["yew".to_string(), "trunk".to_string(), "serve".to_string()],
                    requires_input: false,
                    estimated_duration: Some(5),
                });
            }
            _ => {}
        }

        Ok(commands)
    }

    // Helper methods for creating commands

    fn create_cargo_run_command(
        root: &Path,
        package: Option<&str>,
        binary: Option<&str>,
        priority: u8,
    ) -> Command {
        let mut args = vec!["run".to_string()];

        if let Some(pkg) = package {
            args.extend(vec!["--package".to_string(), pkg.to_string()]);
        }

        if let Some(bin) = binary {
            args.extend(vec!["--bin".to_string(), bin.to_string()]);
        }

        let label = match (package, binary) {
            (Some(pkg), Some(bin)) => format!("Run {bin} ({pkg})"),
            (Some(pkg), None) => format!("Run {pkg}"),
            (None, Some(bin)) => format!("Run {bin}"),
            (None, None) => "Run".to_string(),
        };

        Command {
            id: "cargo-run".to_string(),
            label,
            description: Some("Execute the binary with cargo run".to_string()),
            command: "cargo".to_string(),
            args,
            env: HashMap::new(),
            cwd: Some(root.to_path_buf()),
            category: CommandCategory::Run,
            priority,
            conditions: Vec::new(),
            tags: vec!["cargo".to_string(), "run".to_string()],
            requires_input: false,
            estimated_duration: Some(5),
        }
    }

    fn create_cargo_test_command(
        root: &Path,
        package: Option<&str>,
        extra_args: Option<&str>,
        description_suffix: &str,
        priority: u8,
    ) -> Command {
        let mut args = vec!["test".to_string()];

        if let Some(pkg) = package {
            args.extend(vec!["--package".to_string(), pkg.to_string()]);
        }

        if let Some(extra) = extra_args {
            match parse_option(extra) {
                Ok(parts) => args.extend(parts),
                Err(e) => eprintln!("Warning: Failed to parse extra args '{extra}': {e}"),
            }
        }

        Command {
            id: "cargo-test".to_string(),
            label: format!("Test {description_suffix}"),
            description: Some(format!("Run {description_suffix} with cargo test")),
            command: "cargo".to_string(),
            args,
            env: HashMap::new(),
            cwd: Some(root.to_path_buf()),
            category: CommandCategory::Test,
            priority,
            conditions: Vec::new(),
            tags: vec!["cargo".to_string(), "test".to_string()],
            requires_input: false,
            estimated_duration: Some(10),
        }
    }

    fn create_cargo_binary_test_command(
        root: &Path,
        package: Option<&str>,
        binary_name: &str,
        test_filter: Option<&str>,
        description_suffix: &str,
        priority: u8,
    ) -> Command {
        let mut args = vec!["test".to_string()];

        if let Some(pkg) = package {
            args.extend(vec!["--package".to_string(), pkg.to_string()]);
        }

        // Add --bin flag for binary targets
        args.extend(vec!["--bin".to_string(), binary_name.to_string()]);

        // Add test filter after --
        if let Some(filter) = test_filter {
            args.push("--".to_string());
            match parse_option(filter) {
                Ok(parts) => args.extend(parts),
                Err(e) => eprintln!("Warning: Failed to parse test filter '{filter}': {e}"),
            }
        }

        Command {
            id: "cargo-test-binary".to_string(),
            label: format!("Test {description_suffix}"),
            description: Some(format!("Run {description_suffix} in binary {binary_name}")),
            command: "cargo".to_string(),
            args,
            env: HashMap::new(),
            cwd: Some(root.to_path_buf()),
            category: CommandCategory::Test,
            priority,
            conditions: Vec::new(),
            tags: vec![
                "cargo".to_string(),
                "test".to_string(),
                "binary".to_string(),
            ],
            requires_input: false,
            estimated_duration: Some(10),
        }
    }

    fn create_cargo_bench_command(
        root: &Path,
        package: Option<&str>,
        extra_args: Option<&str>,
        description_suffix: &str,
        priority: u8,
    ) -> Command {
        let mut args = vec!["bench".to_string()];

        if let Some(pkg) = package {
            args.extend(vec!["--package".to_string(), pkg.to_string()]);
        }

        if let Some(extra) = extra_args {
            match parse_option(extra) {
                Ok(parts) => args.extend(parts),
                Err(e) => eprintln!("Warning: Failed to parse extra args '{extra}': {e}"),
            }
        }

        Command {
            id: "cargo-bench".to_string(),
            label: format!("Benchmark {description_suffix}"),
            description: Some(format!("Run {description_suffix} with cargo bench")),
            command: "cargo".to_string(),
            args,
            env: HashMap::new(),
            cwd: Some(root.to_path_buf()),
            category: CommandCategory::Custom("benchmark".to_string()),
            priority,
            conditions: Vec::new(),
            tags: vec!["cargo".to_string(), "bench".to_string()],
            requires_input: false,
            estimated_duration: Some(30),
        }
    }

    fn create_cargo_doc_test_command(
        root: &Path,
        package: Option<&str>,
        description_suffix: &str,
        priority: u8,
    ) -> Command {
        let mut args = vec!["test".to_string(), "--doc".to_string()];

        if let Some(pkg) = package {
            args.extend(vec!["--package".to_string(), pkg.to_string()]);
        }

        Command {
            id: "cargo-doc-test".to_string(),
            label: format!("Doc test {description_suffix}"),
            description: Some("Run documentation tests".to_string()),
            command: "cargo".to_string(),
            args,
            env: HashMap::new(),
            cwd: Some(root.to_path_buf()),
            category: CommandCategory::Test,
            priority,
            conditions: Vec::new(),
            tags: vec!["cargo".to_string(), "doctest".to_string()],
            requires_input: false,
            estimated_duration: Some(8),
        }
    }

    fn create_cargo_doc_test_command_with_filter(
        root: &Path,
        package: Option<&str>,
        function_name: &str,
        description_suffix: &str,
        priority: u8,
    ) -> Command {
        let mut args = vec!["test".to_string(), "--doc".to_string()];

        if let Some(pkg) = package {
            args.extend(vec!["--package".to_string(), pkg.to_string()]);
        }

        // Add function name filter and show output
        args.extend(vec![
            "--".to_string(),
            function_name.to_string(),
            "--show-output".to_string(),
        ]);

        Command {
            id: "cargo-doc-test-specific".to_string(),
            label: format!("Doc test {description_suffix}"),
            description: Some(format!(
                "Run documentation test for function {function_name}"
            )),
            command: "cargo".to_string(),
            args,
            env: HashMap::new(),
            cwd: Some(root.to_path_buf()),
            category: CommandCategory::Test,
            priority,
            conditions: Vec::new(),
            tags: vec![
                "cargo".to_string(),
                "doctest".to_string(),
                "specific".to_string(),
            ],
            requires_input: false,
            estimated_duration: Some(8),
        }
    }

    fn create_cargo_example_command(
        root: &Path,
        package: Option<&str>,
        example_name: &str,
        priority: u8,
    ) -> Command {
        let mut args = vec!["run".to_string()];

        if let Some(pkg) = package {
            args.extend(vec!["--package".to_string(), pkg.to_string()]);
        }

        args.extend(vec!["--example".to_string(), example_name.to_string()]);

        Command {
            id: "cargo-example".to_string(),
            label: format!("Run example {example_name}"),
            description: Some("Execute the example".to_string()),
            command: "cargo".to_string(),
            args,
            env: HashMap::new(),
            cwd: Some(root.to_path_buf()),
            category: CommandCategory::Run,
            priority,
            conditions: Vec::new(),
            tags: vec!["cargo".to_string(), "example".to_string()],
            requires_input: false,
            estimated_duration: Some(5),
        }
    }

    // Helper methods

    fn find_package_for_file(
        members: &[crate::file_detection::WorkspaceMember],
        _root: &Path,
    ) -> Option<String> {
        // In a real implementation, we'd determine which workspace member
        // contains the current file. For now, return the first member.
        members.first().map(|m| m.name.clone())
    }

    fn find_main_at_cursor(entry_points: &[EntryPoint], cursor: Position) -> Option<&EntryPoint> {
        let cursor_line = cursor.line + 1; // Convert to 1-based

        // Check if cursor is on or near main function
        for ep in entry_points {
            if ep.entry_type == EntryPointType::Main {
                // Check if cursor is on the main function line or within 2 lines
                if cursor_line >= ep.line.saturating_sub(1) && cursor_line <= ep.line + 2 {
                    return Some(ep);
                }
            }
        }

        None
    }

    /// Find a doctest entry that belongs to a function the cursor is currently on
    /// Uses tree-sitter AST for accurate detection when available
    fn find_doctest_for_function_at_cursor(
        entry_points: &[EntryPoint],
        cursor: Position,
    ) -> Option<&EntryPoint> {
        #[cfg(feature = "tree-sitter-support")]
        {
            // Try tree-sitter approach first for more accurate detection
            if let Some(_context) = entry_points.first().and({
                // We need access to the source code to use tree-sitter
                // For now, we'll use the file path from the entry point context
                None::<()> // TODO: Get source content here
            }) {
                // Tree-sitter implementation would go here
                // For now, fall back to regex approach
            }
        }

        // Fallback to the previous line-based approach (which is flawed but still better than nothing)
        let cursor_line = cursor.line + 1; // Convert to 1-based

        // First check: if cursor is inside a test module, don't look for doctests
        let in_test_module = entry_points.iter().any(|ep| {
            ep.entry_type == EntryPointType::TestModule
                && cursor_line >= ep.line
                && cursor_line <= ep.line_range.1
        });

        if in_test_module {
            return None; // Don't associate doctests when cursor is in test modules
        }

        // Also check for #[cfg(test)] modules by looking for test functions nearby
        let near_test_function = entry_points.iter().any(|ep| {
            ep.entry_type == EntryPointType::Test && ep.line.abs_diff(cursor_line) <= 10 // Within 10 lines of a test function
        });

        if near_test_function {
            return None; // Don't associate doctests when cursor is near test functions
        }

        // Find the closest preceding doctest
        let mut closest_doctest: Option<&EntryPoint> = None;
        let mut closest_distance = u32::MAX;

        for doctest_ep in entry_points {
            if doctest_ep.entry_type == EntryPointType::DocTest {
                let doctest_end = doctest_ep.line_range.1;

                // Check if cursor is after this doctest and within reasonable distance
                // Reduced distance to be more conservative
                if cursor_line > doctest_end && cursor_line <= doctest_end + 20 {
                    let distance = cursor_line - doctest_end;
                    if distance < closest_distance {
                        closest_distance = distance;
                        closest_doctest = Some(doctest_ep);
                    }
                }
            }
        }

        closest_doctest
    }

    pub fn find_test_at_cursor(
        entry_points: &[EntryPoint],
        cursor: Position,
    ) -> Option<&EntryPoint> {
        let cursor_line = cursor.line + 1; // Convert to 1-based

        // First priority: Check if cursor is within any test function's line range
        for ep in entry_points {
            match ep.entry_type {
                EntryPointType::Test | EntryPointType::DocTest => {
                    // Check if cursor is within the test function's line range
                    if cursor_line >= ep.line_range.0 && cursor_line <= ep.line_range.1 {
                        return Some(ep);
                    }
                }
                _ => {}
            }
        }

        // Second priority: Check if cursor is on a function that has an associated doctest
        if let Some(doctest_for_function) =
            Self::find_doctest_for_function_at_cursor(entry_points, cursor)
        {
            return Some(doctest_for_function);
        }

        // Second priority: Check if cursor is INSIDE a test module scope
        // Find the most specific (deepest) module that contains the cursor
        let mut best_module: Option<&EntryPoint> = None;
        let mut smallest_range = u32::MAX;

        for ep in entry_points {
            if ep.entry_type == EntryPointType::TestModule {
                // Check if cursor is within the test module scope
                if cursor_line >= ep.line && cursor_line <= ep.line_range.1 {
                    let range_size = ep.line_range.1 - ep.line;
                    if range_size < smallest_range {
                        smallest_range = range_size;
                        best_module = Some(ep);
                    }
                }
            }
        }

        if let Some(module) = best_module {
            return Some(module);
        }

        // Third priority: Find closest test within 5 lines
        // But ONLY if we're not inside a test module (to avoid overriding module selection)
        let in_test_module = entry_points.iter().any(|ep| {
            ep.entry_type == EntryPointType::TestModule
                && cursor_line >= ep.line
                && cursor_line <= ep.line_range.1
        });

        if !in_test_module {
            let mut closest_test: Option<&EntryPoint> = None;
            let mut closest_distance = u32::MAX;

            for ep in entry_points {
                match ep.entry_type {
                    EntryPointType::Test | EntryPointType::DocTest => {
                        let distance = ep.line.abs_diff(cursor_line);

                        // Only consider tests within 5 lines of the cursor
                        if distance <= 5 && distance < closest_distance {
                            closest_distance = distance;
                            closest_test = Some(ep);
                        }
                    }
                    _ => {}
                }
            }

            return closest_test;
        }

        None
    }

    fn generate_specific_test_commands(
        context: &FileExecutionContext,
        test_entry: &EntryPoint,
    ) -> RazResult<Vec<Command>> {
        let mut commands = Vec::new();

        match test_entry.entry_type {
            EntryPointType::DocTest => {
                // For doc tests, generate doc test command with function name filter
                let function_name = test_entry.full_path.as_ref().unwrap_or(&test_entry.name);

                match &context.project_type {
                    RustProjectType::CargoWorkspace { root, members } => {
                        if let Some(package_name) = Self::find_package_for_file(members, root) {
                            commands.push(Self::create_cargo_doc_test_command_with_filter(
                                root,
                                Some(&package_name),
                                function_name,
                                &format!("doc test for {function_name}"),
                                150, // High priority for specific doc test
                            ));
                        }
                    }
                    RustProjectType::CargoPackage { root, package_name } => {
                        commands.push(Self::create_cargo_doc_test_command_with_filter(
                            root,
                            Some(package_name),
                            function_name,
                            &format!("doc test for {function_name}"),
                            150, // High priority for specific doc test
                        ));
                    }
                    RustProjectType::SingleFile { file_path, .. } => {
                        // For single files, use rustdoc directly
                        let file_name = file_path.to_string_lossy();
                        commands.push(Command {
                            id: "rustdoc-test-specific".to_string(),
                            label: format!("Doc test for {function_name}"),
                            description: Some(format!(
                                "Run documentation test for function {function_name}"
                            )),
                            command: "rustdoc".to_string(),
                            args: vec!["--test".to_string(), file_name.to_string()],
                            env: HashMap::new(),
                            cwd: file_path.parent().map(|p| p.to_path_buf()),
                            category: CommandCategory::Test,
                            priority: 150,
                            conditions: Vec::new(),
                            tags: vec![
                                "rustdoc".to_string(),
                                "doctest".to_string(),
                                "specific".to_string(),
                            ],
                            requires_input: false,
                            estimated_duration: Some(5),
                        });
                    }
                    _ => {}
                }
            }
            EntryPointType::TestModule => {
                // For test modules, run all tests in the module
                let module_name = test_entry.full_path.as_ref().unwrap_or(&test_entry.name);

                match &context.project_type {
                    RustProjectType::CargoWorkspace { root, members } => {
                        if let Some(package_name) = Self::find_package_for_file(members, root) {
                            // Check if this is a binary file
                            if let FileRole::AdditionalBinary { binary_name }
                            | FileRole::MainBinary { binary_name } = &context.file_role
                            {
                                // For binaries, use --bin flag and simpler test path
                                let test_path = if module_name.contains("::") {
                                    // Extract just the module path after the binary module prefix
                                    module_name
                                        .split("::")
                                        .skip(2)
                                        .collect::<Vec<_>>()
                                        .join("::")
                                } else {
                                    module_name.clone()
                                };

                                commands.push(Self::create_cargo_binary_test_command(
                                    root,
                                    Some(&package_name),
                                    binary_name,
                                    Some(&format!("{test_path}::")),
                                    &format!("all tests in module: {test_path}"),
                                    100,
                                ));
                            } else {
                                // Use standard module filtering for libraries
                                commands.push(Self::create_cargo_test_command(
                                    root,
                                    Some(&package_name),
                                    Some(&format!("-- {module_name}::")),
                                    &format!("all tests in module: {module_name}"),
                                    100,
                                ));
                            }
                        }
                    }
                    RustProjectType::CargoPackage { root, package_name } => {
                        // Check if this is a binary file
                        if let FileRole::AdditionalBinary { binary_name }
                        | FileRole::MainBinary { binary_name } = &context.file_role
                        {
                            // For binaries, use --bin flag and simpler test path
                            let test_path = if module_name.contains("::") {
                                // Extract just the module path after the binary module prefix
                                module_name
                                    .split("::")
                                    .skip(2)
                                    .collect::<Vec<_>>()
                                    .join("::")
                            } else {
                                module_name.clone()
                            };

                            commands.push(Self::create_cargo_binary_test_command(
                                root,
                                Some(package_name),
                                binary_name,
                                Some(&format!("{test_path}::")),
                                &format!("all tests in module: {test_path}"),
                                100,
                            ));
                        } else {
                            // Use standard module filtering for libraries
                            commands.push(Self::create_cargo_test_command(
                                root,
                                Some(package_name),
                                Some(&format!("-- {module_name}::")),
                                &format!("all tests in module: {module_name}"),
                                100,
                            ));
                        }
                    }
                    RustProjectType::SingleFile { file_path, .. } => {
                        let file_name = file_path.to_string_lossy();
                        let test_binary = file_path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("test");

                        commands.push(Command {
                            id: "rustc-test-module".to_string(),
                            label: format!("Test module: {module_name}"),
                            description: Some(format!(
                                "Compile and run all tests in module: {module_name}"
                            )),
                            command: "sh".to_string(),
                            args: vec![
                                "-c".to_string(),
                                format!("rustc --test '{}' && './{}'", file_name, test_binary),
                            ],
                            env: HashMap::new(),
                            cwd: file_path.parent().map(|p| p.to_path_buf()),
                            category: CommandCategory::Test,
                            priority: 100,
                            conditions: Vec::new(),
                            tags: vec![
                                "rustc".to_string(),
                                "test".to_string(),
                                "module".to_string(),
                            ],
                            requires_input: false,
                            estimated_duration: Some(8),
                        });
                    }
                    _ => {}
                }
            }
            _ => {
                // For specific tests, run the exact test
                let test_name = test_entry.full_path.as_ref().unwrap_or(&test_entry.name);

                match &context.project_type {
                    RustProjectType::CargoWorkspace { root, members } => {
                        if let Some(package_name) = Self::find_package_for_file(members, root) {
                            // Check if this is a binary file
                            if let FileRole::AdditionalBinary { binary_name }
                            | FileRole::MainBinary { binary_name } = &context.file_role
                            {
                                // For binaries, use --bin flag and strip the binary module prefix from test path
                                let test_path = if test_name.starts_with("bin::") {
                                    // Remove "bin::binary_name::" prefix
                                    test_name.split("::").skip(2).collect::<Vec<_>>().join("::")
                                } else {
                                    test_name.clone()
                                };

                                commands.push(Self::create_cargo_binary_test_command(
                                    root,
                                    Some(&package_name),
                                    binary_name,
                                    Some(&Self::build_test_args_with_defaults(
                                        &test_path, true, true,
                                    )),
                                    &format!("specific test: {test_path}"),
                                    100,
                                ));
                            } else {
                                commands.push(Self::create_cargo_test_command(
                                    root,
                                    Some(&package_name),
                                    Some(&format!(
                                        "-- {}",
                                        Self::build_test_args_with_defaults(test_name, true, false)
                                    )),
                                    &format!("specific test: {test_name}"),
                                    100,
                                ));
                            }
                        }
                    }
                    RustProjectType::CargoPackage { root, package_name } => {
                        // Check if this is a binary file
                        if let FileRole::AdditionalBinary { binary_name }
                        | FileRole::MainBinary { binary_name } = &context.file_role
                        {
                            // For binaries, use --bin flag and strip the binary module prefix from test path
                            let test_path = if test_name.starts_with("bin::") {
                                // Remove "bin::binary_name::" prefix
                                test_name.split("::").skip(2).collect::<Vec<_>>().join("::")
                            } else {
                                test_name.clone()
                            };

                            commands.push(Self::create_cargo_binary_test_command(
                                root,
                                Some(package_name),
                                binary_name,
                                Some(&Self::build_test_args_with_defaults(&test_path, true, true)),
                                &format!("specific test: {test_path}"),
                                100,
                            ));
                        } else {
                            commands.push(Self::create_cargo_test_command(
                                root,
                                Some(package_name),
                                Some(&format!(
                                    "-- {}",
                                    Self::build_test_args_with_defaults(test_name, true, false)
                                )),
                                &format!("specific test: {test_name}"),
                                100,
                            ));
                        }
                    }
                    RustProjectType::SingleFile { file_path, .. } => {
                        // For single files, we can't run a specific test with rustc,
                        // but we can provide a focused test command
                        let file_name = file_path.to_string_lossy();
                        let test_binary = file_path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("test");

                        commands.push(Command {
                            id: "rustc-test-specific".to_string(),
                            label: format!("Test {test_name}"),
                            description: Some(format!(
                                "Compile and run specific test: {test_name}"
                            )),
                            command: "sh".to_string(),
                            args: vec![
                                "-c".to_string(),
                                format!(
                                    "rustc --test '{}' -o './{}_test' && './{}_test' '{}'",
                                    file_name, test_binary, test_binary, test_name
                                ),
                            ],
                            env: HashMap::new(),
                            cwd: file_path.parent().map(|p| p.to_path_buf()),
                            category: CommandCategory::Test,
                            priority: 140, // Higher priority for specific test
                            conditions: Vec::new(),
                            tags: vec![
                                "rustc".to_string(),
                                "test".to_string(),
                                "specific".to_string(),
                            ],
                            requires_input: false,
                            estimated_duration: Some(8),
                        });
                    }
                    _ => {}
                }
            }
        }

        Ok(commands)
    }

    /// Generate framework-specific commands using lightweight detection
    fn generate_advanced_framework_commands(
        context: &FileExecutionContext,
    ) -> RazResult<Vec<Command>> {
        let mut commands = Vec::new();

        // Convert FileExecutionContext to ProjectContext for framework detection
        if let Some(project_context) = Self::convert_to_project_context(context) {
            let detector = PreciseFrameworkDetector::new();
            let framework_scores = detector.detect(&project_context, Some(&context.file_path));

            // Check all framework scores to find the best match
            for score in framework_scores.iter() {
                let confidence_threshold = match score.framework {
                    FrameworkType::YewWeb => 6.0, // Even lower threshold for Yew
                    _ => 8.0,                     // Higher threshold for other frameworks
                };
                if score.confidence >= confidence_threshold {
                    match &score.framework {
                        FrameworkType::DioxusDesktop
                        | FrameworkType::DioxusWeb
                        | FrameworkType::DioxusMobile => {
                            commands
                                .extend(Self::generate_dioxus_commands(context, &score.framework)?);
                        }
                        FrameworkType::LeptosSSR | FrameworkType::LeptosCSR => {
                            commands
                                .extend(Self::generate_leptos_commands(context, &score.framework)?);
                        }
                        FrameworkType::YewWeb => {
                            commands
                                .extend(Self::generate_yew_commands(context, &score.framework)?);
                        }
                        FrameworkType::TauriDesktop => {
                            commands
                                .extend(Self::generate_tauri_commands(context, &score.framework)?);
                        }
                        _ => continue, // Skip unknown frameworks
                    }
                    break; // Only process the first matching framework
                }
            }
        }

        Ok(commands)
    }

    /// Convert FileExecutionContext to ProjectContext for framework detection
    fn convert_to_project_context(context: &FileExecutionContext) -> Option<ProjectContext> {
        match &context.project_type {
            RustProjectType::CargoWorkspace { root, .. }
            | RustProjectType::CargoPackage { root, .. } => {
                // Try to analyze the project at the root
                // For now, create a minimal context for detection
                // This is a simplified conversion - in a full implementation,
                // we'd parse Cargo.toml to get dependencies
                let cargo_toml = root.join("Cargo.toml");
                if cargo_toml.exists() {
                    if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                        let dependencies = Self::parse_dependencies_from_toml(&content);
                        return Some(ProjectContext {
                            workspace_root: root.clone(),
                            current_file: None,
                            cursor_position: None,
                            project_type: crate::ProjectType::Binary, // Simplified
                            dependencies,
                            workspace_members: Vec::new(),
                            build_targets: Vec::new(),
                            active_features: Vec::new(),
                            env_vars: std::collections::HashMap::new(),
                        });
                    }
                }
            }
            _ => {}
        }
        None
    }

    /// Parse dependencies from Cargo.toml content (simplified)
    fn parse_dependencies_from_toml(content: &str) -> Vec<crate::Dependency> {
        let mut dependencies = Vec::new();

        if let Ok(value) = content.parse::<toml::Value>() {
            if let Some(deps) = value.get("dependencies").and_then(|d| d.as_table()) {
                for (name, dep_value) in deps {
                    let mut features = Vec::new();

                    // Extract features if they exist
                    if let Some(dep_table) = dep_value.as_table() {
                        if let Some(feat_array) =
                            dep_table.get("features").and_then(|f| f.as_array())
                        {
                            features = feat_array
                                .iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect();
                        }
                    }

                    dependencies.push(crate::Dependency {
                        name: name.clone(),
                        version: "unknown".to_string(), // Simplified
                        features,
                        optional: false,
                        dev_dependency: false,
                    });
                }
            }
        }

        dependencies
    }

    /// Generate Dioxus-specific commands
    fn generate_dioxus_commands(
        context: &FileExecutionContext,
        framework: &FrameworkType,
    ) -> RazResult<Vec<Command>> {
        let mut commands = Vec::new();

        let (root, platform) = match &context.project_type {
            RustProjectType::CargoWorkspace { root, .. }
            | RustProjectType::CargoPackage { root, .. } => {
                // Detect platform preference
                let platform = Self::detect_dioxus_platform(root, framework);
                (root.clone(), platform)
            }
            _ => return Ok(commands),
        };

        // Primary Dioxus serve command
        commands.push(Command {
            id: "dx-serve".to_string(),
            label: format!("Dioxus Serve ({platform})"),
            description: Some(format!("Start Dioxus dev server for {platform} platform")),
            command: "dx".to_string(),
            args: vec![
                "serve".to_string(),
                "--platform".to_string(),
                platform.clone(),
            ],
            env: std::collections::HashMap::new(),
            cwd: Some(root.clone()),
            category: CommandCategory::Run,
            priority: 110, // Higher than cargo run
            conditions: Vec::new(),
            tags: vec!["dioxus".to_string(), "serve".to_string(), platform.clone()],
            requires_input: false,
            estimated_duration: Some(5),
        });

        Ok(commands)
    }

    /// Generate Leptos-specific commands
    fn generate_leptos_commands(
        context: &FileExecutionContext,
        _framework: &FrameworkType,
    ) -> RazResult<Vec<Command>> {
        let mut commands = Vec::new();

        let root = match &context.project_type {
            RustProjectType::CargoWorkspace { root, .. }
            | RustProjectType::CargoPackage { root, .. } => root.clone(),
            _ => return Ok(commands),
        };

        // Primary Leptos watch command
        commands.push(Command {
            id: "leptos-watch".to_string(),
            label: "Leptos Watch".to_string(),
            description: Some("Start Leptos development server with hot reload".to_string()),
            command: "cargo".to_string(),
            args: vec!["leptos".to_string(), "watch".to_string()],
            env: std::collections::HashMap::new(),
            cwd: Some(root),
            category: CommandCategory::Run,
            priority: 110, // Higher than cargo run
            conditions: Vec::new(),
            tags: vec!["leptos".to_string(), "watch".to_string()],
            requires_input: false,
            estimated_duration: Some(5),
        });

        Ok(commands)
    }

    /// Generate Yew framework commands
    fn generate_yew_commands(
        context: &FileExecutionContext,
        _framework: &FrameworkType,
    ) -> RazResult<Vec<Command>> {
        let mut commands = Vec::new();

        let root = match &context.project_type {
            RustProjectType::CargoWorkspace { root, .. }
            | RustProjectType::CargoPackage { root, .. } => root.clone(),
            _ => return Ok(commands),
        };

        // Primary Yew serve command using trunk
        commands.push(Command {
            id: "yew-serve".to_string(),
            label: "Yew Dev Server".to_string(),
            description: Some("Start Yew development server with hot reload".to_string()),
            command: "trunk".to_string(),
            args: vec!["serve".to_string()],
            env: std::collections::HashMap::new(),
            cwd: Some(root.clone()),
            category: CommandCategory::Run,
            priority: 110, // Higher than cargo run
            conditions: Vec::new(),
            tags: vec!["yew".to_string(), "serve".to_string(), "trunk".to_string()],
            requires_input: false,
            estimated_duration: Some(5),
        });

        // Yew serve with auto-open browser
        commands.push(Command {
            id: "yew-serve-open".to_string(),
            label: "Yew Dev Server + Open".to_string(),
            description: Some("Start Yew development server and open browser".to_string()),
            command: "trunk".to_string(),
            args: vec!["serve".to_string(), "--open".to_string()],
            env: std::collections::HashMap::new(),
            cwd: Some(root.clone()),
            category: CommandCategory::Run,
            priority: 105, // Slightly lower than basic serve
            conditions: Vec::new(),
            tags: vec![
                "yew".to_string(),
                "serve".to_string(),
                "open".to_string(),
                "trunk".to_string(),
            ],
            requires_input: false,
            estimated_duration: Some(5),
        });

        // Yew build command
        commands.push(Command {
            id: "yew-build".to_string(),
            label: "Yew Build".to_string(),
            description: Some("Build Yew application for production".to_string()),
            command: "trunk".to_string(),
            args: vec!["build".to_string(), "--release".to_string()],
            env: std::collections::HashMap::new(),
            cwd: Some(root),
            category: CommandCategory::Build,
            priority: 85,
            conditions: Vec::new(),
            tags: vec!["yew".to_string(), "build".to_string(), "trunk".to_string()],
            requires_input: false,
            estimated_duration: Some(30),
        });

        Ok(commands)
    }

    /// Generate Tauri framework commands
    fn generate_tauri_commands(
        context: &FileExecutionContext,
        _framework: &FrameworkType,
    ) -> RazResult<Vec<Command>> {
        let mut commands = Vec::new();

        let root = match &context.project_type {
            RustProjectType::CargoWorkspace { root, .. }
            | RustProjectType::CargoPackage { root, .. } => root.clone(),
            _ => return Ok(commands),
        };

        // Determine the correct working directory
        // If we're in src-tauri, use current directory
        // Otherwise, check if src-tauri exists as subdirectory
        let tauri_dir = if root.ends_with("src-tauri") {
            root.clone()
        } else if root.join("src-tauri").exists() {
            root.join("src-tauri")
        } else {
            root.clone()
        };

        // Primary Tauri dev command
        commands.push(Command {
            id: "tauri-dev".to_string(),
            label: "Tauri Dev".to_string(),
            description: Some("Start Tauri development server with hot reload".to_string()),
            command: "cargo".to_string(),
            args: vec!["tauri".to_string(), "dev".to_string()],
            env: std::collections::HashMap::new(),
            cwd: Some(tauri_dir.clone()),
            category: CommandCategory::Run,
            priority: 110, // Higher than cargo run
            conditions: Vec::new(),
            tags: vec!["tauri".to_string(), "dev".to_string()],
            requires_input: false,
            estimated_duration: Some(10),
        });

        // Tauri build command
        commands.push(Command {
            id: "tauri-build".to_string(),
            label: "Tauri Build".to_string(),
            description: Some("Build Tauri application for production".to_string()),
            command: "cargo".to_string(),
            args: vec!["tauri".to_string(), "build".to_string()],
            env: std::collections::HashMap::new(),
            cwd: Some(tauri_dir.clone()),
            category: CommandCategory::Build,
            priority: 85,
            conditions: Vec::new(),
            tags: vec!["tauri".to_string(), "build".to_string()],
            requires_input: false,
            estimated_duration: Some(60),
        });

        // Tauri build with debug info
        commands.push(Command {
            id: "tauri-build-debug".to_string(),
            label: "Tauri Build (Debug)".to_string(),
            description: Some("Build Tauri application with debug information".to_string()),
            command: "cargo".to_string(),
            args: vec![
                "tauri".to_string(),
                "build".to_string(),
                "--debug".to_string(),
            ],
            env: std::collections::HashMap::new(),
            cwd: Some(tauri_dir),
            category: CommandCategory::Build,
            priority: 80,
            conditions: Vec::new(),
            tags: vec![
                "tauri".to_string(),
                "build".to_string(),
                "debug".to_string(),
            ],
            requires_input: false,
            estimated_duration: Some(45),
        });

        Ok(commands)
    }

    /// Detect Dioxus platform preference
    fn detect_dioxus_platform(root: &Path, _framework: &FrameworkType) -> String {
        // 1. Check for environment variable override (highest priority)
        if let Ok(platform_override) = std::env::var("RAZ_PLATFORM_OVERRIDE") {
            return platform_override;
        }

        // 2. Check Dioxus.toml
        let dioxus_toml = root.join("Dioxus.toml");
        if dioxus_toml.exists() {
            if let Ok(content) = std::fs::read_to_string(&dioxus_toml) {
                if let Ok(config) = content.parse::<toml::Value>() {
                    if let Some(app_config) = config.get("application") {
                        if let Some(default_platform) = app_config.get("default_platform") {
                            if let Some(platform_str) = default_platform.as_str() {
                                return platform_str.to_string();
                            }
                        }
                    }
                }
            }
        }

        // 3. Default to desktop for all Dioxus projects (user can override)
        "desktop".to_string()
    }

    /// Check if a command is a web framework serve/dev command
    fn is_web_framework_command(cmd: &Command) -> bool {
        // Check for framework-specific serve/dev commands
        let is_serve_dev = cmd.tags.contains(&"serve".to_string())
            || cmd.tags.contains(&"dev".to_string())
            || cmd.tags.contains(&"watch".to_string());

        // Check for framework-specific commands
        let is_framework_cmd = cmd
            .args
            .iter()
            .any(|arg| arg == "leptos" || arg == "dioxus" || arg == "tauri" || arg == "trunk")
            || cmd.tags.contains(&"yew".to_string())
            || cmd.tags.contains(&"tauri".to_string());

        is_serve_dev || is_framework_cmd
    }

    /// Helper to intelligently add test arguments with defaults
    /// This ensures we don't duplicate flags like --exact or --show-output
    fn build_test_args_with_defaults(
        test_name: &str,
        add_exact: bool,
        add_show_output: bool,
    ) -> String {
        let mut args = vec![test_name.to_string()];

        // Add default flags only if requested
        if add_exact {
            args.push("--exact".to_string());
        }
        if add_show_output {
            args.push("--show-output".to_string());
        }

        args.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_detection::{
        ExecutionCapabilities, FileExecutionContext, FileRole, RustProjectType, SingleFileType,
    };
    use std::path::PathBuf;

    #[test]
    fn test_single_file_executable_commands() {
        let context = FileExecutionContext {
            project_type: RustProjectType::SingleFile {
                file_path: PathBuf::from("/tmp/hello.rs"),
                file_type: SingleFileType::Executable,
            },
            file_role: FileRole::Standalone,
            entry_points: vec![],
            capabilities: ExecutionCapabilities {
                can_run: true,
                can_test: false,
                can_bench: false,
                can_doc_test: false,
                requires_framework: None,
            },
            file_path: PathBuf::from("/tmp/hello.rs"),
        };

        let commands = UniversalCommandGenerator::generate_commands(&context, None).unwrap();
        assert!(!commands.is_empty());
        assert!(commands.iter().any(|c| c.id == "rustc-run"));
        assert!(commands.iter().any(|c| c.command == "sh"));
    }

    #[test]
    fn test_cargo_script_commands() {
        let context = FileExecutionContext {
            project_type: RustProjectType::CargoScript {
                file_path: PathBuf::from("/tmp/script.rs"),
                manifest: None,
            },
            file_role: FileRole::MainBinary {
                binary_name: "script".to_string(),
            },
            entry_points: vec![],
            capabilities: ExecutionCapabilities {
                can_run: true,
                can_test: true,
                can_bench: false,
                can_doc_test: false,
                requires_framework: None,
            },
            file_path: PathBuf::from("/tmp/script.rs"),
        };

        let commands = UniversalCommandGenerator::generate_commands(&context, None).unwrap();
        assert!(!commands.is_empty());
        assert!(commands.iter().any(|c| c.id == "cargo-script-run"));
        assert!(
            commands
                .iter()
                .any(|c| c.args.contains(&"+nightly".to_string()))
        );
    }

    #[test]
    fn test_frontend_library_commands() {
        let context = FileExecutionContext {
            project_type: RustProjectType::CargoPackage {
                root: PathBuf::from("/tmp/project"),
                package_name: "frontend".to_string(),
            },
            file_role: FileRole::FrontendLibrary {
                framework: "leptos".to_string(),
            },
            entry_points: vec![],
            capabilities: ExecutionCapabilities {
                can_run: true,
                can_test: false,
                can_bench: false,
                can_doc_test: true,
                requires_framework: Some("leptos".to_string()),
            },
            file_path: PathBuf::from("/tmp/project/src/lib.rs"),
        };

        let commands = UniversalCommandGenerator::generate_commands(&context, None).unwrap();
        assert!(!commands.is_empty());
        assert!(commands.iter().any(|c| c.id == "leptos-watch"));
        assert!(
            commands
                .iter()
                .any(|c| c.command == "cargo" && c.args.contains(&"leptos".to_string()))
        );
    }
}
