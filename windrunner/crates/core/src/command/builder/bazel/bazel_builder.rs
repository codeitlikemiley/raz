//! Bazel command builder with placeholder support

use crate::{
    bazel::{BazelTargetFinder, BazelTargetKind},
    command::{
        CargoCommand,
        builder::{CommandBuilderImpl, ConfigAccess},
        template::CommandTemplate,
    },
    config::{BazelConfig, BazelFramework, Config},
    error::Result,
    types::{FileType, Runnable, RunnableKind},
};
use std::path::{Path, PathBuf};

/// Bazel command builder with rich placeholder support
pub struct BazelCommandBuilder;

impl ConfigAccess for BazelCommandBuilder {}

impl CommandBuilderImpl for BazelCommandBuilder {
    fn build(
        runnable: &Runnable,
        _package: Option<&str>,
        config: &Config,
        file_type: FileType,
    ) -> Result<CargoCommand> {
        tracing::debug!("BazelCommandBuilder::build called for {:?}", runnable.kind);
        let builder = BazelCommandBuilder;
        let bazel_config = config.bazel.as_ref();

        match &runnable.kind {
            RunnableKind::Test { test_name, .. } => {
                builder.build_test_command(runnable, test_name, bazel_config, config, file_type)
            }
            RunnableKind::ModuleTests { .. } => {
                builder.build_module_tests_command(runnable, bazel_config, config, file_type)
            }
            RunnableKind::Binary { bin_name } => builder.build_binary_command(
                runnable,
                bin_name.as_deref(),
                bazel_config,
                config,
                file_type,
            ),
            RunnableKind::Benchmark { bench_name } => builder.build_benchmark_command(
                runnable,
                bench_name,
                bazel_config,
                config,
                file_type,
            ),
            RunnableKind::DocTest { .. } => {
                builder.build_doc_test_command(runnable, bazel_config, config, file_type)
            }
            _ => Err(crate::error::Error::ParseError(
                "Unsupported runnable type for bazel".to_string(),
            )),
        }
    }
}

impl BazelCommandBuilder {
    fn build_test_command(
        &self,
        runnable: &Runnable,
        test_name: &str,
        bazel_config: Option<&BazelConfig>,
        config: &Config,
        file_type: FileType,
    ) -> Result<CargoCommand> {
        tracing::debug!("build_test_command called for test: {}", test_name);

        // Get the test framework or use defaults
        let mut framework = bazel_config
            .and_then(|bc| bc.test_framework.clone())
            .unwrap_or_else(|| BazelConfig::default_test_framework());

        if !framework
            .test_args
            .as_ref()
            .map(|args| args.iter().any(|arg| arg == "--exact"))
            .unwrap_or(false)
        {
            framework
                .test_args
                .get_or_insert_with(Vec::new)
                .push("--exact".to_string());
        }

        // Determine the target
        let target = self.determine_target(runnable, bazel_config, config, true);

        // Build the test filter
        let test_filter = if runnable.module_path.is_empty() {
            test_name.to_string()
        } else {
            format!("{}::{}", runnable.module_path, test_name)
        };

        // Build the command
        let mut command = self.build_command_from_framework(
            &framework,
            runnable,
            Some(&target),
            Some(&test_filter),
            None,
        );

        // Apply overrides
        self.apply_overrides(&mut command, runnable, config, file_type);

        Ok(command)
    }

    fn build_module_tests_command(
        &self,
        runnable: &Runnable,
        bazel_config: Option<&BazelConfig>,
        config: &Config,
        file_type: FileType,
    ) -> Result<CargoCommand> {
        tracing::debug!("build_module_tests_command called");

        // Check if this is a benchmark file - if so, we should run the binary instead
        let is_benchmark_file = runnable
            .file_path
            .components()
            .any(|c| c.as_os_str() == "benches");

        if is_benchmark_file {
            tracing::debug!("Detected benchmark file - redirecting to binary command");
            // For benchmark files, run the binary instead of tests
            return self.build_binary_command(runnable, None, bazel_config, config, file_type);
        }

        // Get the test framework or use defaults
        let mut framework = bazel_config
            .and_then(|bc| bc.test_framework.clone())
            .unwrap_or_else(|| BazelConfig::default_test_framework());

        // Module-level test selection should be broad enough to match the whole
        // module. `--exact` would require the filter to equal a single test name,
        // which filters everything out for module runs like `foo::tests`.
        if let Some(test_args) = &mut framework.test_args {
            test_args.retain(|arg| arg != "--exact");
        }

        // Determine the target
        let target = self.determine_target(runnable, bazel_config, config, true);

        // Build module filter (no exact matching for module tests)
        let test_filter = if !runnable.module_path.is_empty() {
            Some(runnable.module_path.clone())
        } else if let RunnableKind::ModuleTests { module_name } = &runnable.kind {
            // For module tests, use the module name as the filter
            Some(module_name.clone())
        } else {
            None
        };

        // Build the command
        let mut command = self.build_command_from_framework(
            &framework,
            runnable,
            Some(&target),
            test_filter.as_deref(),
            None,
        );

        // Apply overrides
        self.apply_overrides(&mut command, runnable, config, file_type);

        Ok(command)
    }

    fn build_binary_command(
        &self,
        runnable: &Runnable,
        bin_name: Option<&str>,
        bazel_config: Option<&BazelConfig>,
        config: &Config,
        file_type: FileType,
    ) -> Result<CargoCommand> {
        tracing::debug!("build_binary_command called for binary: {:?}", bin_name);

        // Check if this is a build.rs file
        let is_build_script = runnable
            .file_path
            .file_name()
            .map(|f| f == "build.rs")
            .unwrap_or(false);

        // Check if this is a benchmark file
        let is_benchmark_file = runnable
            .file_path
            .components()
            .any(|c| c.as_os_str() == "benches");

        // Get the binary framework or use defaults
        let mut framework = bazel_config
            .and_then(|bc| bc.binary_framework.clone())
            .unwrap_or_else(|| BazelConfig::default_binary_framework());

        // For build scripts, override the subcommand to 'build'
        if is_build_script {
            framework.subcommand = Some("build".to_string());
            tracing::debug!("Using 'bazel build' for build.rs file");
        }

        // For benchmark files, add optimization flag
        if is_benchmark_file {
            if framework.args.is_none() {
                framework.args = Some(vec![]);
            }
            if let Some(ref mut args) = framework.args {
                if !args.contains(&"-c".to_string())
                    && !args.contains(&"--compilation_mode".to_string())
                {
                    args.insert(0, "-c".to_string());
                    args.insert(1, "opt".to_string());
                    tracing::debug!("Added optimization flag for benchmark binary");
                }
            }
        }

        // Determine the target (is_test=false for binaries)
        let target = self.determine_target(runnable, bazel_config, config, false);

        // Build the command
        let mut command =
            self.build_command_from_framework(&framework, runnable, Some(&target), None, bin_name);

        // Apply overrides
        self.apply_overrides(&mut command, runnable, config, file_type);

        Ok(command)
    }

    fn build_benchmark_command(
        &self,
        runnable: &Runnable,
        bench_name: &str,
        bazel_config: Option<&BazelConfig>,
        config: &Config,
        file_type: FileType,
    ) -> Result<CargoCommand> {
        tracing::debug!(
            "build_benchmark_command called for benchmark: {}",
            bench_name
        );

        // Get the benchmark framework or use defaults
        let framework = bazel_config
            .and_then(|bc| bc.benchmark_framework.clone())
            .unwrap_or_else(|| BazelConfig::default_benchmark_framework());

        // Determine the target
        let target = self.determine_target(runnable, bazel_config, config, true);

        // Build the benchmark filter
        let bench_filter = if runnable.module_path.is_empty() {
            bench_name.to_string()
        } else {
            format!("{}::{}", runnable.module_path, bench_name)
        };

        // Build the command
        let mut command = self.build_command_from_framework(
            &framework,
            runnable,
            Some(&target),
            Some(&bench_filter),
            None,
        );

        // Apply overrides
        self.apply_overrides(&mut command, runnable, config, file_type);

        Ok(command)
    }

    fn build_doc_test_command(
        &self,
        runnable: &Runnable,
        bazel_config: Option<&BazelConfig>,
        config: &Config,
        file_type: FileType,
    ) -> Result<CargoCommand> {
        tracing::debug!("build_doc_test_command called");

        // First, try to find a rust_doc_test target for this file
        let abs_file_path = if runnable.file_path.is_absolute() {
            runnable.file_path.clone()
        } else {
            std::env::current_dir()
                .ok()
                .map(|cwd| cwd.join(&runnable.file_path))
                .unwrap_or_else(|| runnable.file_path.clone())
        };

        // Find the workspace root
        let workspace_root = abs_file_path
            .ancestors()
            .find(|p| p.join("MODULE.bazel").exists() || p.join("WORKSPACE").exists());

        if let Some(workspace_root) = workspace_root {
            let mut finder = BazelTargetFinder::new()?;
            if let Some(doc_test_target) =
                finder.find_doc_test_target(&abs_file_path, workspace_root)?
            {
                // Found a rust_doc_test target!
                tracing::debug!("Found rust_doc_test target: {}", doc_test_target.label);

                // Build command to run the doc test target
                let mut args = vec!["test".to_string(), doc_test_target.label];

                // Add standard test output streaming
                args.push("--test_output".to_string());
                args.push("streamed".to_string());

                // Get the doc test framework or use defaults
                let framework = bazel_config
                    .and_then(|bc| bc.doc_test_framework.clone())
                    .unwrap_or_else(|| BazelConfig::default_doc_test_framework());

                // Add extra args from framework
                if let Some(extra_args) = &framework.extra_args {
                    args.extend(extra_args.clone());
                }

                let mut command = CargoCommand::new_bazel(args);

                // Apply environment variables
                if let Some(env) = &framework.extra_env {
                    for (key, value) in env {
                        command.env.push((key.clone(), value.clone()));
                    }
                }

                // Apply overrides
                self.apply_overrides(&mut command, runnable, config, file_type);

                // Note: Bazel doesn't support running individual doc tests
                // If this is a specific doc test (not file-level), we should inform the user
                if let RunnableKind::DocTest {
                    method_name: Some(_),
                    ..
                } = &runnable.kind
                {
                    // Add a comment in the environment that can be checked by the CLI
                    command.env.push((
                        "_BAZEL_DOC_TEST_LIMITATION".to_string(),
                        "Bazel runs all doc tests together, not individual ones".to_string(),
                    ));
                }

                return Ok(command);
            }
        }

        // No rust_doc_test target found
        Err(crate::error::Error::ParseError(
            "No rust_doc_test target found in BUILD file. To run doc tests in Bazel, add a rust_doc_test target.".to_string()
        ))
    }

    /// Build a command from a framework configuration
    fn build_command_from_framework(
        &self,
        framework: &BazelFramework,
        runnable: &Runnable,
        target: Option<&str>,
        test_filter: Option<&str>,
        binary_name: Option<&str>,
    ) -> CargoCommand {
        let command_name = framework.command.as_deref().unwrap_or("bazel");
        let subcommand = framework.subcommand.as_deref().unwrap_or("test");

        let mut args = vec![subcommand.to_string()];

        // Add the target
        if let Some(target_template) = &framework.target {
            let expanded_target = self.expand_template(
                target_template,
                &runnable.file_path,
                target.unwrap_or(":test"),
                test_filter,
                binary_name,
                &runnable.module_path,
            );
            args.push(expanded_target);
        } else if let Some(target) = target {
            args.push(target.to_string());
        }

        // Add base args with placeholder expansion
        if let Some(base_args) = &framework.args {
            for arg in base_args {
                let expanded = self.expand_template(
                    arg,
                    &runnable.file_path,
                    target.unwrap_or(":test"),
                    test_filter,
                    binary_name,
                    &runnable.module_path,
                );
                args.push(expanded);
            }
        }

        // Add extra args (no expansion needed)
        if let Some(extra_args) = &framework.extra_args {
            args.extend(extra_args.clone());
        }

        // Add test args (for test subcommand)
        if subcommand == "test" && test_filter.is_some() {
            if let Some(test_args) = &framework.test_args {
                for arg in test_args {
                    let expanded = self.expand_template(
                        arg,
                        &runnable.file_path,
                        target.unwrap_or(":test"),
                        test_filter,
                        binary_name,
                        &runnable.module_path,
                    );
                    if !expanded.is_empty() {
                        // Add --test_arg before each test argument
                        args.push("--test_arg".to_string());
                        args.push(expanded);
                    }
                }
            }
        }

        // Add exec args (for run subcommand)
        if subcommand == "run" {
            if let Some(exec_args) = &framework.exec_args {
                if !exec_args.is_empty() {
                    args.push("--".to_string());
                    for arg in exec_args {
                        let expanded = self.expand_template(
                            arg,
                            &runnable.file_path,
                            target.unwrap_or("//:server"),
                            test_filter,
                            binary_name,
                            &runnable.module_path,
                        );
                        args.push(expanded);
                    }
                }
            }
        }

        let mut command = if command_name == "bazel" {
            CargoCommand::new_bazel(args)
        } else {
            // Support custom commands (like bazelisk)
            CargoCommand::new_shell(command_name.to_string(), args)
        };

        // Set working directory to workspace root for Bazel
        let abs_path = if runnable.file_path.is_absolute() {
            runnable.file_path.clone()
        } else {
            std::env::current_dir()
                .ok()
                .map(|cwd| cwd.join(&runnable.file_path))
                .unwrap_or_else(|| runnable.file_path.clone())
        };

        if let Some(workspace_root) = abs_path
            .ancestors()
            .find(|p| p.join("MODULE.bazel").exists() || p.join("WORKSPACE").exists())
        {
            command.working_dir = Some(workspace_root.to_string_lossy().to_string());
            tracing::debug!(
                "Set working directory for Bazel command: {:?}",
                workspace_root
            );
        }

        // Apply environment variables
        if let Some(env) = &framework.extra_env {
            for (key, value) in env {
                command.env.push((key.clone(), value.clone()));
            }
        }

        command
    }

    /// Determine the Bazel target based on the runnable and configuration
    fn determine_target(
        &self,
        runnable: &Runnable,
        bazel_config: Option<&BazelConfig>,
        config: &Config,
        is_test: bool,
    ) -> String {
        // Try to use the new target finder first
        let abs_file_path = if runnable.file_path.is_absolute() {
            runnable.file_path.clone()
        } else {
            std::env::current_dir()
                .ok()
                .map(|cwd| cwd.join(&runnable.file_path))
                .unwrap_or_else(|| runnable.file_path.clone())
        };

        tracing::debug!(
            "determine_target: file_path={:?}, abs_file_path={:?}, is_test={}",
            runnable.file_path,
            abs_file_path,
            is_test
        );

        // Find the workspace root
        let workspace_root = abs_file_path
            .ancestors()
            .find(|p| p.join("MODULE.bazel").exists() || p.join("WORKSPACE").exists());

        if let Some(workspace_root) = workspace_root {
            tracing::debug!("Found workspace root: {:?}", workspace_root);

            match BazelTargetFinder::new() {
                Ok(mut finder) => {
                    tracing::debug!("Successfully created BazelTargetFinder");

                    // Check if this is a build.rs file
                    if !is_test
                        && runnable
                            .file_path
                            .file_name()
                            .map(|f| f == "build.rs")
                            .unwrap_or(false)
                    {
                        tracing::debug!("Looking for cargo_build_script target for build.rs");

                        // Find all targets for this file
                        if let Ok(targets) =
                            finder.find_targets_for_file(&abs_file_path, workspace_root)
                        {
                            // Look for a cargo_build_script target
                            for target in targets {
                                if matches!(target.kind, BazelTargetKind::BuildScript) {
                                    tracing::info!(
                                        "Found cargo_build_script target: {}",
                                        target.label
                                    );
                                    return target.label;
                                }
                            }
                        }

                        tracing::warn!("No cargo_build_script target found for build.rs");
                        tracing::warn!(
                            "Make sure your BUILD.bazel contains a cargo_build_script rule"
                        );
                    }

                    // First, check if this is an integration test (in tests/ directory)
                    let has_tests_component = runnable
                        .file_path
                        .components()
                        .any(|c| c.as_os_str() == "tests");
                    tracing::debug!("Has tests component: {}", has_tests_component);

                    if is_test && has_tests_component {
                        tracing::info!(
                            "Looking for integration test target for file: {:?}",
                            abs_file_path
                        );
                        match finder.find_integration_test_target(&abs_file_path, workspace_root) {
                            Ok(Some(target)) => {
                                tracing::info!(
                                    "Found integration test target from BUILD file: {}",
                                    target.label
                                );
                                return target.label;
                            }
                            Ok(None) => {
                                tracing::warn!(
                                    "No integration test target found for file: {:?}",
                                    abs_file_path
                                );
                                tracing::warn!(
                                    "Make sure BUILD.bazel contains rust_test_suite with appropriate glob pattern"
                                );

                                // Try to list what targets were found
                                if let Ok(all_targets) =
                                    finder.find_targets_for_file(&abs_file_path, workspace_root)
                                {
                                    if all_targets.is_empty() {
                                        tracing::warn!("No targets found that include this file");
                                    } else {
                                        tracing::warn!(
                                            "Found {} targets but none are rust_test_suite:",
                                            all_targets.len()
                                        );
                                        for target in all_targets {
                                            tracing::warn!(
                                                "  - {} ({:?})",
                                                target.label,
                                                target.kind
                                            );
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::error!("Error finding integration test target: {}", e);
                            }
                        }
                    }

                    // Try to find any runnable target for this file
                    let kind_filter = if is_test {
                        Some(BazelTargetKind::Test)
                    } else {
                        Some(BazelTargetKind::Binary)
                    };

                    if let Ok(Some(target)) =
                        finder.find_runnable_target(&abs_file_path, workspace_root, kind_filter)
                    {
                        tracing::debug!("Found target from BUILD file: {}", target.label);
                        return target.label;
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to create BazelTargetFinder: {}", e);
                }
            }
        }
        // Check for legacy configuration first
        if let Some(config) = bazel_config {
            if is_test && config.test_target.is_some() {
                return config.test_target.clone().unwrap();
            } else if !is_test && config.binary_target.is_some() {
                return config.binary_target.clone().unwrap();
            }
        }

        // Use configured defaults if available
        if let Some(config) = bazel_config {
            if is_test && config.default_test_target.is_some() {
                return config.default_test_target.clone().unwrap();
            } else if !is_test && config.default_binary_target.is_some() {
                return config.default_binary_target.clone().unwrap();
            }
        }

        // Check if using linked projects
        if let Some(cargo_config) = &config.cargo {
            if let Some(linked_projects) = &cargo_config.linked_projects {
                // Try to find which linked project contains this file
                if let Some(target) = self.find_bazel_target_via_linked_projects(
                    &abs_file_path,
                    linked_projects,
                    is_test,
                ) {
                    return target;
                }
            }
        }

        // Fall back to simple inference
        let file_path = &runnable.file_path;
        let file_name = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        // Check common patterns
        if file_path.to_string_lossy().contains("src/bin/") {
            // Binary in src/bin
            format!("//src/bin:{}", file_name)
        } else if file_name == "main" && !is_test {
            // Main binary
            "//:main".to_string()
        } else if is_test && file_path.to_string_lossy().contains("/tests/") {
            // For integration tests, we should have found a rust_test_suite target
            // If we're here, it means no target was found
            tracing::error!(
                "No rust_test_suite target found for integration test: {:?}",
                file_path
            );
            tracing::error!("Make sure your BUILD.bazel file contains a rust_test_suite rule");
            tracing::error!("Looking for file: {:?}", abs_file_path);
            tracing::error!("Workspace root used: {:?}", workspace_root);

            // Try to provide more helpful error information
            if let Some(ws_root) = workspace_root {
                // Find the BUILD file that should contain the target
                let mut current = abs_file_path.parent();
                while let Some(dir) = current {
                    if dir == ws_root {
                        break;
                    }
                    let build_bazel = dir.join("BUILD.bazel");
                    let build = dir.join("BUILD");
                    if build_bazel.exists() || build.exists() {
                        tracing::error!("Found BUILD file at: {:?}", dir);
                        tracing::error!(
                            "This BUILD file should contain a rust_test_suite rule with glob pattern matching {:?}",
                            file_path.file_name()
                        );
                        break;
                    }
                    current = dir.parent();
                }
            }

            // Return a generic target that will likely fail with a clear error
            ":integration_tests_not_found".to_string()
        } else if is_test {
            // Default test target
            ":test".to_string()
        } else {
            // Default binary target
            "//:server".to_string()
        }
    }

    /// Expand template placeholders using the `CommandTemplate` engine.
    ///
    /// Supported placeholders:
    /// - `{target}` — full Bazel target label (e.g. `//server:server`)
    /// - `{target_name}` — label after `:` (e.g. `server`)
    /// - `{package}` — label before `:` (e.g. `//server`)
    /// - `{file_path}` — absolute path to the source file
    /// - `{file_name}` — stem of the file name (no extension)
    /// - `{parent_dir}` — parent directory of the source file
    /// - `{test_filter}` / `{test_name}` / `{bench_filter}` — test/bench filter string
    /// - `{module_path}` — Rust module path
    /// - `{binary_name}` — binary name (falls back to `{file_name}`)
    fn expand_template(
        &self,
        template: &str,
        file_path: &Path,
        target: &str,
        test_filter: Option<&str>,
        binary_name: Option<&str>,
        module_path: &str,
    ) -> String {
        let file_name = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let parent_dir = file_path.parent().and_then(|p| p.to_str()).unwrap_or(".");

        // Pre-compute target components.
        let (package, target_name) = if target.contains(':') {
            let mut parts = target.splitn(2, ':');
            (parts.next().unwrap_or(""), parts.next().unwrap_or(target))
        } else {
            ("", target)
        };

        let file_name_owned = file_name.to_string();
        let parent_dir_owned = parent_dir.to_string();
        let target_owned = target.to_string();
        let target_name_owned = target_name.to_string();
        let package_owned = package.to_string();
        let module_path_owned = module_path.to_string();
        let file_path_str = file_path.to_str().unwrap_or("").to_string();
        let test_filter_owned = test_filter.unwrap_or("").to_string();
        let binary_owned = binary_name.unwrap_or(file_name).to_string();

        // Try the template engine first; fall back to legacy string-replace on parse error.
        match CommandTemplate::parse(template) {
            Ok(tmpl) => {
                match tmpl.render(|ph| match ph {
                    "target" => Some(target_owned.clone()),
                    "target_name" => Some(target_name_owned.clone()),
                    "package" => Some(package_owned.clone()),
                    "file_path" => Some(file_path_str.clone()),
                    "file_name" => Some(file_name_owned.clone()),
                    "parent_dir" => Some(parent_dir_owned.clone()),
                    "test_filter" | "test_name" | "bench_filter" => {
                        if test_filter_owned.is_empty() {
                            None
                        } else {
                            Some(test_filter_owned.clone())
                        }
                    }
                    "module_path" => Some(module_path_owned.clone()),
                    "binary_name" => Some(binary_owned.clone()),
                    _ => None,
                }) {
                    Ok(rendered) => rendered,
                    Err(e) => {
                        tracing::warn!(
                            "BazelCommandBuilder: template render failed ({e}), using legacy expand"
                        );
                        Self::legacy_expand(
                            template,
                            &target_owned,
                            &target_name_owned,
                            &package_owned,
                            &file_path_str,
                            &file_name_owned,
                            &parent_dir_owned,
                            &test_filter_owned,
                            &module_path_owned,
                            &binary_owned,
                        )
                    }
                }
            }
            Err(e) => {
                tracing::warn!(
                    "BazelCommandBuilder: template parse failed ({e}), using legacy expand"
                );
                Self::legacy_expand(
                    template,
                    &target_owned,
                    &target_name_owned,
                    &package_owned,
                    &file_path_str,
                    &file_name_owned,
                    &parent_dir_owned,
                    &test_filter_owned,
                    &module_path_owned,
                    &binary_owned,
                )
            }
        }
    }

    /// Legacy fallback: the original chained `.replace()` implementation.
    #[inline]
    fn legacy_expand(
        template: &str,
        target: &str,
        target_name: &str,
        package: &str,
        file_path: &str,
        file_name: &str,
        parent_dir: &str,
        test_filter: &str,
        module_path: &str,
        binary_name: &str,
    ) -> String {
        template
            .replace("{target}", target)
            .replace("{target_name}", target_name)
            .replace("{package}", package)
            .replace("{file_path}", file_path)
            .replace("{file_name}", file_name)
            .replace("{parent_dir}", parent_dir)
            .replace("{test_filter}", test_filter)
            .replace("{bench_filter}", test_filter)
            .replace("{test_name}", test_filter)
            .replace("{module_path}", module_path)
            .replace("{binary_name}", binary_name)
    }

    /// Find Bazel target using linked projects configuration (simplified approach)
    fn find_bazel_target_via_linked_projects(
        &self,
        abs_file_path: &Path,
        linked_projects: &[String],
        is_test: bool,
    ) -> Option<String> {
        // Find which linked project contains this file
        for linked_project_str in linked_projects {
            let linked_project = PathBuf::from(linked_project_str);
            // Get the directory of the linked project (parent of Cargo.toml)
            let project_dir = linked_project.parent()?;
            // Check if our file is under this project directory
            if abs_file_path.starts_with(project_dir) {
                // Get Bazel package path from linked project path
                // e.g. /Users/uriah/Code/yoyo/combos/frontend/Cargo.toml -> //combos/frontend
                let _bazel_package = self.get_bazel_package_from_linked_project(&linked_project)?;
                // Check if this directory has a BUILD.bazel file
                let build_file = project_dir.join("BUILD.bazel");
                if !build_file.exists() {
                    let build_file = project_dir.join("BUILD");
                    if !build_file.exists() {
                        continue;
                    }
                }

                // Use the new target finder
                if let Ok(mut finder) = BazelTargetFinder::new() {
                    // Find the workspace root
                    let workspace_root = project_dir.ancestors().find(|p| {
                        p.join("MODULE.bazel").exists() || p.join("WORKSPACE").exists()
                    })?;

                    let kind_filter = if is_test {
                        Some(BazelTargetKind::Test)
                    } else {
                        Some(BazelTargetKind::Binary)
                    };

                    if let Ok(Some(target)) =
                        finder.find_runnable_target(abs_file_path, workspace_root, kind_filter)
                    {
                        return Some(target.label);
                    }
                }
            }
        }

        None
    }

    /// Get Bazel package path from linked project path
    /// e.g. /Users/uriah/Code/yoyo/combos/frontend/Cargo.toml -> //combos/frontend
    fn get_bazel_package_from_linked_project(&self, linked_project: &Path) -> Option<String> {
        // Find PROJECT_ROOT to determine the base path
        let project_root = if let Ok(root) = std::env::var("PROJECT_ROOT") {
            PathBuf::from(root)
        } else {
            // Try to find MODULE.bazel by walking up
            linked_project
                .ancestors()
                .find(|p| p.join("MODULE.bazel").exists())?
                .to_path_buf()
        };

        // Get the parent directory of the Cargo.toml
        let project_dir = linked_project.parent()?;

        // Get relative path from PROJECT_ROOT to project directory
        let relative_path = project_dir.strip_prefix(&project_root).ok()?;

        // Convert to Bazel package format
        if relative_path.as_os_str().is_empty() {
            Some("//".to_string())
        } else {
            Some(format!(
                "//{}",
                relative_path.display().to_string().replace('\\', "/")
            ))
        }
    }

    /// Get override configuration for a runnable
    fn get_override<'a>(
        &self,
        runnable: &Runnable,
        config: &'a Config,
        file_type: FileType,
    ) -> Option<&'a crate::config::Override> {
        let identity = crate::types::FunctionIdentity {
            package: config
                .bazel
                .as_ref()
                .and_then(|b| b.workspace.clone())
                .or_else(|| config.cargo.as_ref().and_then(|c| c.package.clone())),
            module_path: if runnable.module_path.is_empty() {
                None
            } else {
                Some(runnable.module_path.clone())
            },
            file_path: Some(runnable.file_path.clone()),
            function_name: runnable.get_function_name(),
            file_type: Some(file_type),
        };

        tracing::debug!("Looking for override for identity: {:?}", identity);
        let result = config.get_override_for(&identity);
        if result.is_some() {
            tracing::debug!("Found matching override!");
        } else {
            tracing::debug!("No matching override found");
        }
        result
    }

    /// Apply overrides to the command.
    ///
    /// Reads from the flat `BazelOverride` shape — all fields are optional
    /// and applied incrementally on top of the already-built command.
    fn apply_overrides(
        &self,
        command: &mut CargoCommand,
        runnable: &Runnable,
        config: &Config,
        file_type: FileType,
    ) {
        let override_config = self.get_override(runnable, config, file_type);

        if let Some(override_) = override_config {
            if let Some(ov) = &override_.bazel {
                // Note: `ov.command` (e.g. "bazelisk") is stored for tooling/display
                // but cannot be mutated here — the binary is encoded in CommandType::Bazel.
                // Future: convert to CommandType::Shell when command != "bazel".

                // Override subcommand (first arg is always the subcommand)
                if let Some(subcmd) = &ov.subcommand {
                    if !command.args.is_empty() {
                        command.args[0] = subcmd.clone();
                    }
                }

                // Override target (second arg = target label)
                if let Some(target) = &ov.target {
                    if command.args.len() >= 2 {
                        command.args[1] = target.clone();
                    }
                }

                // Replace the base arg block (everything after subcommand+target)
                if let Some(args) = &ov.args {
                    let keep = command.args.len().min(2); // keep subcmd + target
                    command.args.truncate(keep);
                    command.args.extend(args.clone());
                }

                // Append extra args verbatim
                if let Some(extra) = &ov.extra_args {
                    command.args.extend(extra.clone());
                }

                // Inject test_args as `--test_arg <value>` pairs
                if let Some(test_args) = &ov.test_args {
                    for arg in test_args {
                        command.args.push("--test_arg".to_string());
                        command.args.push(arg.clone());
                    }
                }

                // Append exec_args after `--` separator (for `bazel run`)
                if let Some(exec_args) = &ov.exec_args {
                    if !exec_args.is_empty() {
                        command.args.push("--".to_string());
                        command.args.extend(exec_args.clone());
                    }
                }

                // Merge environment variables
                if let Some(env) = &ov.extra_env {
                    for (key, value) in env {
                        command.env.push((key.clone(), value.clone()));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "bazel_builder_test.rs"]
mod bazel_builder_test;
