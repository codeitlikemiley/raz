use super::*;
use crate::RunnableKind;
use crate::bazel::BazelTargetFinder;
use crate::command::Command;
use crate::config::{BazelConfig, Config};
use crate::error::Result;
use crate::types::{FileType, Runnable};

impl BazelCommandBuilder {
    pub(crate) fn build_doc_test_command(
        &self,
        runnable: &Runnable,
        bazel_config: Option<&BazelConfig>,
        config: &Config,
        file_type: FileType,
    ) -> Result<Command> {
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
                    .unwrap_or_else(BazelConfig::default_doc_test_framework);

                // Add extra args from framework
                if let Some(extra_args) = &framework.extra_args {
                    args.extend(extra_args.clone());
                }

                let mut command = Command::bazel(args);

                // Apply environment variables
                if let Some(env) = &framework.extra_env {
                    for (key, value) in env {
                        command.env.insert(key.clone(), value.clone());
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
                    command.env.insert(
                        "_BAZEL_DOC_TEST_LIMITATION".to_string(),
                        "Bazel runs all doc tests together, not individual ones".to_string(),
                    );
                }

                return Ok(command);
            }
        }

        // No rust_doc_test target found
        Err(crate::error::Error::MissingBazelTarget {
            file: runnable.file_path.clone(),
            hint: "To run doc tests in Bazel, add a rust_doc_test target",
        })
    }
}
