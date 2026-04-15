use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Main Bazel configuration structure.
/// Used for project-level config under the `bazel` top-level key.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct BazelConfig {
    /// Test framework configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_framework: Option<BazelFramework>,

    /// Binary framework configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary_framework: Option<BazelFramework>,

    /// Benchmark framework configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub benchmark_framework: Option<BazelFramework>,

    /// Doc test framework configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc_test_framework: Option<BazelFramework>,

    /// Bazel workspace name (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,

    /// Default test target template (default: ":test")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_test_target: Option<String>,

    /// Default binary target template (default: "//:server")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_binary_target: Option<String>,

    // Legacy fields for backward compatibility
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary_target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_test_args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_run_args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_test_binary_args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_env: Option<HashMap<String, String>>,
}

/// Flat per-function Bazel override.
///
/// Used inside `overrides[].bazel` in `.cargo-runner.json`.
/// Instead of nesting `test_framework.test_args`, you write the fields
/// directly — the override always targets the active framework for that
/// runnable (test → test_framework, binary → binary_framework, etc.).
///
/// Example:
/// ```json
/// { "match": { "function_name": "my_test" }, "bazel": { "test_args": ["--nocapture"] } }
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct BazelOverride {
    /// Override the Bazel command (rarely needed, e.g. "bazelisk").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,

    /// Override the subcommand (e.g. "test", "run").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subcommand: Option<String>,

    /// Override the target template.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,

    /// Override base args (passed directly to bazel).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,

    /// Extra args appended after base args (no expansion).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_args: Option<Vec<String>>,

    /// Override test args (passed via `--test_arg`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_args: Option<Vec<String>>,

    /// Override exec args (passed after `--` for `bazel run`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exec_args: Option<Vec<String>>,

    /// Extra environment variables.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_env: Option<HashMap<String, String>>,
}

/// Bazel framework configuration for a specific runnable type
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct BazelFramework {
    /// The bazel command (default: "bazel")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,

    /// The subcommand (e.g., "test", "run", "build")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subcommand: Option<String>,

    /// Target template with placeholders (e.g., "{target}", "//:{file_name}")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,

    /// Base arguments with placeholder support
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,

    /// Additional arguments appended after base args
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_args: Option<Vec<String>>,

    /// Arguments passed via --test_arg (for test subcommand)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_args: Option<Vec<String>>,

    /// Arguments passed after -- (for run subcommand)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exec_args: Option<Vec<String>>,

    /// Environment variables
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_env: Option<HashMap<String, String>>,
}

impl BazelConfig {
    pub fn merge_with(&mut self, other: BazelConfig) {
        // Merge framework configurations
        if other.test_framework.is_some() {
            self.test_framework = other.test_framework;
        }
        if other.binary_framework.is_some() {
            self.binary_framework = other.binary_framework;
        }
        if other.benchmark_framework.is_some() {
            self.benchmark_framework = other.benchmark_framework;
        }
        if other.doc_test_framework.is_some() {
            self.doc_test_framework = other.doc_test_framework;
        }

        // Merge global settings
        if other.workspace.is_some() {
            self.workspace = other.workspace;
        }
        if other.default_test_target.is_some() {
            self.default_test_target = other.default_test_target;
        }
        if other.default_binary_target.is_some() {
            self.default_binary_target = other.default_binary_target;
        }

        // Merge legacy fields for backward compatibility
        if other.test_target.is_some() {
            self.test_target = other.test_target;
        }
        if other.binary_target.is_some() {
            self.binary_target = other.binary_target;
        }
        if let Some(ref args) = other.extra_test_args {
            if !args.is_empty() {
                self.extra_test_args = other.extra_test_args;
            }
        }
        if let Some(ref args) = other.extra_run_args {
            if !args.is_empty() {
                self.extra_run_args = other.extra_run_args;
            }
        }
        if let Some(ref args) = other.extra_test_binary_args {
            if !args.is_empty() {
                self.extra_test_binary_args = other.extra_test_binary_args;
            }
        }
        if let Some(ref env) = other.extra_env {
            if !env.is_empty() {
                self.extra_env = other.extra_env;
            }
        }
    }

    /// Get the default test framework configuration
    pub fn default_test_framework() -> BazelFramework {
        BazelFramework {
            command: Some("bazel".to_string()),
            subcommand: Some("test".to_string()),
            target: Some("{target}".to_string()),
            args: Some(vec!["--test_output".to_string(), "streamed".to_string()]),
            test_args: Some(vec!["--nocapture".to_string(), "{test_filter}".to_string()]),
            ..Default::default()
        }
    }

    /// Get the default binary framework configuration
    pub fn default_binary_framework() -> BazelFramework {
        BazelFramework {
            command: Some("bazel".to_string()),
            subcommand: Some("run".to_string()),
            target: Some("{target}".to_string()),
            args: None,
            test_args: None,
            exec_args: None,
            ..Default::default()
        }
    }

    /// Get the default benchmark framework configuration
    pub fn default_benchmark_framework() -> BazelFramework {
        BazelFramework {
            command: Some("bazel".to_string()),
            subcommand: Some("test".to_string()),
            target: Some("{target}".to_string()),
            args: Some(vec![
                "--test_output".to_string(),
                "streamed".to_string(),
                "--test_arg".to_string(),
                "--bench".to_string(),
            ]),
            test_args: Some(vec!["{bench_filter}".to_string()]),
            ..Default::default()
        }
    }

    /// Get the default doc test framework configuration
    pub fn default_doc_test_framework() -> BazelFramework {
        // Doc tests in Bazel are included in the main test target
        // No separate doc test target exists
        BazelFramework {
            command: Some("bazel".to_string()),
            subcommand: Some("test".to_string()),
            target: Some("{target}".to_string()),
            args: Some(vec!["--test_output".to_string(), "streamed".to_string()]),
            ..Default::default()
        }
    }
}

impl BazelConfig {
    /// Normalizes the configuration by migrating legacy fields into their respective frameworks.
    pub fn normalize(&mut self) {
        if self.test_target.is_some()
            || self.extra_test_args.is_some()
            || self.extra_test_binary_args.is_some()
            || self.extra_env.is_some()
        {
            let mut fw = self.test_framework.take().unwrap_or_default();
            if let Some(target) = self.test_target.take() {
                if fw.target.is_none() {
                    fw.target = Some(target);
                }
            }
            if let Some(args) = self.extra_test_args.take() {
                if fw.extra_args.is_none() {
                    fw.extra_args = Some(args);
                }
            }
            if let Some(tb_args) = self.extra_test_binary_args.take() {
                if fw.test_args.is_none() {
                    fw.test_args = Some(tb_args);
                }
            }
            if let Some(env) = self.extra_env.clone() {
                if fw.extra_env.is_none() {
                    fw.extra_env = Some(env);
                }
            }
            self.test_framework = Some(fw);
        }

        if self.binary_target.is_some() || self.extra_run_args.is_some() || self.extra_env.is_some()
        {
            let mut fw = self.binary_framework.take().unwrap_or_default();
            if let Some(target) = self.binary_target.take() {
                if fw.target.is_none() {
                    fw.target = Some(target);
                }
            }
            if let Some(args) = self.extra_run_args.take() {
                if fw.extra_args.is_none() {
                    fw.extra_args = Some(args);
                }
            }
            if let Some(env) = self.extra_env.take() {
                if fw.extra_env.is_none() {
                    fw.extra_env = Some(env);
                }
            }
            self.binary_framework = Some(fw);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bazel_config_serialization() {
        let config = BazelConfig {
            workspace: Some("my_workspace".to_string()),
            default_test_target: Some("//:my_test".to_string()),
            test_framework: Some(BazelFramework {
                command: Some("bazel".to_string()),
                subcommand: Some("test".to_string()),
                target: Some("{target}".to_string()),
                args: Some(vec!["--test_output".to_string(), "all".to_string()]),
                test_args: Some(vec![
                    "--nocapture".to_string(),
                    "--exact".to_string(),
                    "{test_filter}".to_string(),
                ]),
                ..Default::default()
            }),
            ..Default::default()
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        println!("Serialized Bazel config:\n{json}");

        let parsed: BazelConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.workspace, Some("my_workspace".to_string()));
        assert_eq!(parsed.default_test_target, Some("//:my_test".to_string()));
        assert!(parsed.test_framework.is_some());
    }

    #[test]
    fn test_default_frameworks() {
        let test_fw = BazelConfig::default_test_framework();
        assert_eq!(test_fw.subcommand, Some("test".to_string()));
        assert!(test_fw.args.is_some());

        let binary_fw = BazelConfig::default_binary_framework();
        assert_eq!(binary_fw.subcommand, Some("run".to_string()));

        let bench_fw = BazelConfig::default_benchmark_framework();
        assert_eq!(bench_fw.subcommand, Some("test".to_string()));
        assert!(bench_fw.args.unwrap().contains(&"--test_arg".to_string()));
    }

    #[test]
    fn test_framework_merge() {
        let mut base = BazelConfig {
            workspace: Some("base_workspace".to_string()),
            test_framework: Some(BazelFramework {
                command: Some("bazel".to_string()),
                subcommand: Some("test".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };

        let override_config = BazelConfig {
            workspace: Some("override_workspace".to_string()),
            binary_framework: Some(BazelFramework {
                command: Some("bazelisk".to_string()),
                subcommand: Some("run".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };

        base.merge_with(override_config);

        assert_eq!(base.workspace, Some("override_workspace".to_string()));
        assert!(base.test_framework.is_some());
        assert!(base.binary_framework.is_some());
        assert_eq!(
            base.binary_framework.as_ref().unwrap().command,
            Some("bazelisk".to_string())
        );
    }
}
