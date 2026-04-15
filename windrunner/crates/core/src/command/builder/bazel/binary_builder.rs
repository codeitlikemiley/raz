use super::*;
use crate::command::Command;
use crate::config::{BazelConfig, Config};
use crate::error::Result;
use crate::types::{FileType, Runnable};

impl BazelCommandBuilder {
    pub(crate) fn build_binary_command(
        &self,
        runnable: &Runnable,
        bin_name: Option<&str>,
        bazel_config: Option<&BazelConfig>,
        config: &Config,
        file_type: FileType,
    ) -> Result<Command> {
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
            .unwrap_or_else(BazelConfig::default_binary_framework);

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

        let target = if let Some(t) = bazel_config.and_then(|c| c.binary_target.as_ref()) {
            t.to_string()
        } else {
            self.determine_target(runnable, bazel_config, config, false)
        };

        if target.is_empty() {
            return Err(crate::error::Error::MissingBazelTarget {
                file: runnable.file_path.clone(),
                hint: "Make sure it is declared in a BUILD.bazel rule",
            });
        }

        // Build the command
        let mut command =
            self.build_command_from_framework(&framework, runnable, Some(&target), None, bin_name);

        // Apply overrides
        self.apply_overrides(&mut command, runnable, config, file_type);

        Ok(command)
    }
}
