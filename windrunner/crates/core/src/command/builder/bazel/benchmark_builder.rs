use super::*;
use crate::command::Command;
use crate::config::{BazelConfig, Config};
use crate::error::Result;
use crate::types::{FileType, Runnable};

impl BazelCommandBuilder {
    pub(crate) fn build_benchmark_command(
        &self,
        runnable: &Runnable,
        bench_name: &str,
        bazel_config: Option<&BazelConfig>,
        config: &Config,
        file_type: FileType,
    ) -> Result<Command> {
        tracing::debug!(
            "build_benchmark_command called for benchmark: {}",
            bench_name
        );

        // Get the benchmark framework or use defaults
        let framework = bazel_config
            .and_then(|bc| bc.benchmark_framework.clone())
            .unwrap_or_else(BazelConfig::default_benchmark_framework);

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
}
