//! Single file script builder for cargo script files

use crate::{
    command::{
        CargoCommand,
        builder::{CommandBuilderImpl, ConfigAccess},
    },
    config::Config,
    error::Result,
    types::{FileType, FunctionIdentity, Runnable, RunnableKind},
};

/// Single file script builder for cargo script files
pub struct SingleFileScriptBuilder;

impl ConfigAccess for SingleFileScriptBuilder {}

impl CommandBuilderImpl for SingleFileScriptBuilder {
    fn build(
        runnable: &Runnable,
        _package: Option<&str>,
        config: &Config,
        file_type: FileType,
    ) -> Result<CargoCommand> {
        let builder = SingleFileScriptBuilder;

        match &runnable.kind {
            RunnableKind::SingleFileScript { .. } => {
                // Extract shebang from file
                let shebang = builder.extract_shebang(&runnable.file_path)?;

                // Build command for running the script
                let mut args = builder.parse_shebang_args(&shebang);

                // Add the script file path
                args.push(runnable.file_path.to_str().unwrap_or("").to_string());

                // Check if the file contains benchmarks
                if let Ok(content) = std::fs::read_to_string(&runnable.file_path) {
                    let has_benchmarks = content.contains("#[bench]")
                        || content.contains("criterion_group!")
                        || content.contains("criterion_main!");

                    if has_benchmarks {
                        // Add --bench flag for running benchmarks
                        args.push("--bench".to_string());
                    }
                }

                // Apply extra args
                builder.apply_args(&mut args, runnable, config, file_type);

                let mut command = CargoCommand::new(args);

                // Apply env vars
                builder.apply_common_config(&mut command, config, file_type);
                builder.apply_env(&mut command, runnable, config, file_type);

                Ok(command)
            }
            RunnableKind::Test { test_name, .. } => {
                // Build command for running a test in a cargo script
                let mut args = vec!["+nightly".to_string(), "-Zscript".to_string()];

                // Add test subcommand
                args.push("test".to_string());

                // Add --manifest-path with the script file
                args.push("--manifest-path".to_string());
                args.push(runnable.file_path.to_str().unwrap_or("").to_string());

                // Apply extra args
                builder.apply_args(&mut args, runnable, config, file_type);

                // Add test filter with module path
                args.push("--".to_string());

                // Build the full test path including module
                let full_test_path = if !runnable.module_path.is_empty() {
                    format!("{}::{}", runnable.module_path, test_name)
                } else {
                    test_name.clone()
                };
                args.push(full_test_path);

                // Add --exact flag for individual test
                args.push("--exact".to_string());

                // Apply test binary args
                builder.apply_test_binary_args(&mut args, runnable, config, file_type);

                let mut command = CargoCommand::new(args);

                // Apply env vars
                builder.apply_common_config(&mut command, config, file_type);
                builder.apply_env(&mut command, runnable, config, file_type);

                Ok(command)
            }
            RunnableKind::ModuleTests { module_name } => {
                // Build command for running all tests in a cargo script
                let mut args = vec!["+nightly".to_string(), "-Zscript".to_string()];

                // Add test subcommand
                args.push("test".to_string());

                // Add --manifest-path with the script file
                args.push("--manifest-path".to_string());
                args.push(runnable.file_path.to_str().unwrap_or("").to_string());

                // Apply extra args
                builder.apply_args(&mut args, runnable, config, file_type);

                // Add module filter
                args.push("--".to_string());

                // Use the full module path if available, otherwise just the module name
                if !runnable.module_path.is_empty() {
                    args.push(runnable.module_path.clone());
                } else {
                    args.push(module_name.clone());
                }

                // Apply test binary args (but NOT --exact for module tests)
                builder.apply_test_binary_args(&mut args, runnable, config, file_type);

                let mut command = CargoCommand::new(args);

                // Apply env vars
                builder.apply_common_config(&mut command, config, file_type);
                builder.apply_env(&mut command, runnable, config, file_type);

                Ok(command)
            }
            RunnableKind::Binary { .. } | RunnableKind::Standalone { .. } => {
                // For binary/main function in cargo script, just run the script
                // Extract shebang from file
                let shebang = builder.extract_shebang(&runnable.file_path)?;

                // Build command for running the script
                let mut args = builder.parse_shebang_args(&shebang);

                // Add the script file path
                args.push(runnable.file_path.to_str().unwrap_or("").to_string());

                // Apply extra args
                builder.apply_args(&mut args, runnable, config, file_type);

                let mut command = CargoCommand::new(args);

                // Apply env vars
                builder.apply_common_config(&mut command, config, file_type);
                builder.apply_env(&mut command, runnable, config, file_type);

                Ok(command)
            }
            RunnableKind::Benchmark { bench_name } => {
                // Build command for running a benchmark in a cargo script
                // Extract shebang from file
                let shebang = builder.extract_shebang(&runnable.file_path)?;

                // Build command for running the script
                let mut args = builder.parse_shebang_args(&shebang);

                // Add the script file path
                args.push(runnable.file_path.to_str().unwrap_or("").to_string());

                // Add --bench flag to run benchmarks
                args.push("--bench".to_string());

                // Apply extra args
                builder.apply_args(&mut args, runnable, config, file_type);

                // Add benchmark filter if specific benchmark
                args.push("--".to_string());
                args.push(bench_name.clone());

                let mut command = CargoCommand::new(args);

                // Apply env vars
                builder.apply_common_config(&mut command, config, file_type);
                builder.apply_env(&mut command, runnable, config, file_type);

                Ok(command)
            }
            _ => Err(crate::error::Error::ParseError(
                "Unsupported runnable type for single file script".to_string(),
            )),
        }
    }
}

impl SingleFileScriptBuilder {
    fn extract_shebang(&self, file_path: &std::path::Path) -> Result<String> {
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| crate::error::Error::ParseError(format!("Failed to read file: {}", e)))?;

        if let Some(first_line) = content.lines().next() {
            if first_line.starts_with("#!") {
                return Ok(first_line.to_string());
            }
        }

        // Default shebang if not found
        Ok("#!/usr/bin/env -S cargo +nightly -Zscript".to_string())
    }

    fn parse_shebang_args(&self, shebang: &str) -> Vec<String> {
        // Parse shebang line to extract cargo command and args
        // Example: #!/usr/bin/env -S cargo +nightly -Zscript
        let mut args = Vec::new();

        if let Some(cmd_part) = shebang.strip_prefix("#!") {
            let parts: Vec<&str> = cmd_part.split_whitespace().collect();

            // Skip /usr/bin/env and -S if present
            let start_idx = if parts.get(0) == Some(&"/usr/bin/env") {
                if parts.get(1) == Some(&"-S") { 2 } else { 1 }
            } else {
                0
            };

            // Collect remaining args, skipping "cargo" since CargoCommand adds it
            for (i, part) in parts[start_idx..].iter().enumerate() {
                if i == 0 && *part == "cargo" {
                    // Skip "cargo" as it's added by CargoCommand
                    continue;
                }
                args.push(part.to_string());
            }
        }

        args
    }

    fn apply_args(
        &self,
        args: &mut Vec<String>,
        runnable: &Runnable,
        config: &Config,
        file_type: FileType,
    ) {
        // Apply override args
        if let Some(override_config) = self.get_override(runnable, config, file_type) {
            if let Some(override_sfs) = &override_config.single_file_script {
                if let Some(extra_args) = &override_sfs.extra_args {
                    args.extend(extra_args.clone());
                }
            }
        }

        // Apply global args
        if let Some(extra_args) = self.get_extra_args(config, file_type) {
            args.extend(extra_args.clone());
        }
    }

    fn apply_env(
        &self,
        command: &mut CargoCommand,
        runnable: &Runnable,
        config: &Config,
        file_type: FileType,
    ) {
        // Apply override env vars
        if let Some(override_config) = self.get_override(runnable, config, file_type) {
            if let Some(override_sfs) = &override_config.single_file_script {
                if let Some(extra_env) = &override_sfs.extra_env {
                    for (key, value) in extra_env {
                        command.env.push((key.clone(), value.clone()));
                    }
                }
            }
        }
    }

    fn get_override<'a>(
        &self,
        runnable: &Runnable,
        config: &'a Config,
        file_type: FileType,
    ) -> Option<&'a crate::config::Override> {
        let identity = self.create_identity(runnable, config, file_type);
        config.get_override_for(&identity)
    }

    fn create_identity(
        &self,
        runnable: &Runnable,
        _config: &Config,
        file_type: FileType,
    ) -> FunctionIdentity {
        FunctionIdentity {
            package: None, // Single file scripts don't have packages
            module_path: None,
            file_path: Some(runnable.file_path.clone()),
            function_name: runnable.get_function_name(),
            file_type: Some(file_type),
        }
    }

    fn apply_test_binary_args(
        &self,
        args: &mut Vec<String>,
        runnable: &Runnable,
        config: &Config,
        file_type: FileType,
    ) {
        // Apply override test binary args
        if let Some(override_config) = self.get_override(runnable, config, file_type) {
            if let Some(override_sfs) = &override_config.single_file_script {
                if let Some(extra_args) = &override_sfs.extra_test_binary_args {
                    args.extend(extra_args.clone());
                }
            }
        }

        // Apply global test binary args
        if let Some(extra_args) = self.get_extra_test_binary_args(config, file_type) {
            args.extend(extra_args.clone());
        }
    }

    fn apply_common_config(
        &self,
        command: &mut CargoCommand,
        config: &Config,
        file_type: FileType,
    ) {
        // Apply environment variables based on file type
        if let Some(extra_env) = self.get_extra_env(config, file_type) {
            for (key, value) in extra_env {
                command.env.push((key.clone(), value.clone()));
            }
        }
    }
}
