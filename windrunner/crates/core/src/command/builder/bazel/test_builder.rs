use super::*;
use crate::RunnableKind;
use crate::command::Command;
use crate::config::{BazelConfig, Config};
use crate::error::Result;
use crate::types::{FileType, Runnable};

impl BazelCommandBuilder {
    pub(crate) fn build_test_command(
        &self,
        runnable: &Runnable,
        test_name: &str,
        bazel_config: Option<&BazelConfig>,
        config: &Config,
        file_type: FileType,
    ) -> Result<Command> {
        tracing::debug!("build_test_command called for test: {}", test_name);

        // Get the test framework or use defaults
        let mut framework = bazel_config
            .and_then(|bc| bc.test_framework.clone())
            .unwrap_or_else(BazelConfig::default_test_framework);

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
        let target = if let Some(t) = bazel_config.and_then(|c| c.test_target.as_ref()) {
            t.to_string()
        } else {
            self.determine_target(runnable, bazel_config, config, true)
        };

        if target.is_empty() {
            return Err(crate::error::Error::MissingBazelTarget {
                file: runnable.file_path.clone(),
                hint: "Make sure it is declared in a BUILD.bazel rule",
            });
        }

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

    pub(crate) fn build_module_tests_command(
        &self,
        runnable: &Runnable,
        bazel_config: Option<&BazelConfig>,
        config: &Config,
        file_type: FileType,
    ) -> Result<Command> {
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
            .unwrap_or_else(BazelConfig::default_test_framework);

        // Module-level test selection should be broad enough to match the whole
        // module. `--exact` would require the filter to equal a single test name,
        // which filters everything out for module runs like `foo::tests`.
        if let Some(test_args) = &mut framework.test_args {
            test_args.retain(|arg| arg != "--exact");
        }

        // Determine the target
        let target = if let Some(t) = bazel_config.and_then(|c| c.test_target.as_ref()) {
            t.to_string()
        } else {
            self.determine_target(runnable, bazel_config, config, true)
        };

        if target.is_empty() {
            return Err(crate::error::Error::MissingBazelTarget {
                file: runnable.file_path.clone(),
                hint: "Make sure it is declared in a BUILD.bazel rule",
            });
        }

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
}
