use serde_json::{Map, json};

pub fn create_rustc_config() -> String {
    // Use raw JSON string to preserve exact field order
    r#"{
  "rustc": {
    "benchmark_framework": {
      "build": {
        "command": "rustc",
        "extra_args": [
          "--edition=2024",
          "-O"
        ],
        "args": [
          "{file_path}",
          "--test",
          "-o",
          "{parent_dir}/{file_name}_bench"
        ],
        "extra_env": {
          "CARGO_TARGET_DIR": "target/rust-analyzer"
        }
      },
      "exec": {
        "command": "{parent_dir}/{file_name}_bench",
        "args": [
          "--bench"
        ],
        "extra_env": {
          "RUST_BACKTRACE": "1"
        }
      }
    },
    "binary_framework": {
      "build": {
        "command": "rustc",
        "extra_args": [
          "--edition=2024",
          "-O"
        ],
        "args": [
          "{file_path}",
          "--crate-type",
          "bin",
          "--crate-name",
          "{file_name}",
          "-o",
          "{parent_dir}/{file_name}"
        ],
        "extra_env": {
          "CARGO_TARGET_DIR": "target/rust-analyzer"
        }
      },
      "exec": {
        "command": "{parent_dir}/{file_name}",
        "extra_env": {
          "RUST_LOG": "debug"
        }
      }
    },
    "test_framework": {
      "build": {
        "command": "rustc",
        "extra_args": [
          "--edition=2024",
          "-O"
        ],
        "args": [
          "{file_path}",
          "--test",
          "-o",
          "{parent_dir}/{file_name}_test"
        ],
        "extra_env": {
          "CARGO_TARGET_DIR": "target/rust-analyzer"
        }
      },
      "exec": {
        "command": "{parent_dir}/{file_name}_test",
        "extra_test_binary_args": [
          "--show-output"
        ],
        "extra_env": {
          "RUST_BACKTRACE": "1"
        }
      }
    }
  },
  "overrides": []
}"#
    .to_string()
}

pub fn create_combined_config() -> String {
    // Combine both rustc and single-file-script configs
    r#"{
  "rustc": {
    "benchmark_framework": {
      "build": {
        "command": "rustc",
        "extra_args": [
          "--edition=2024",
          "-O"
        ],
        "args": [
          "{file_path}",
          "--test",
          "-o",
          "{parent_dir}/{file_name}_bench"
        ],
        "extra_env": {
          "CARGO_TARGET_DIR": "target/rust-analyzer"
        }
      },
      "exec": {
        "command": "{parent_dir}/{file_name}_bench",
        "args": [
          "--bench"
        ],
        "extra_env": {
          "RUST_BACKTRACE": "1"
        }
      }
    },
    "binary_framework": {
      "build": {
        "command": "rustc",
        "extra_args": [
          "--edition=2024",
          "-O"
        ],
        "args": [
          "{file_path}",
          "--crate-type",
          "bin",
          "--crate-name",
          "{file_name}",
          "-o",
          "{parent_dir}/{file_name}"
        ],
        "extra_env": {
          "CARGO_TARGET_DIR": "target/rust-analyzer"
        }
      },
      "exec": {
        "command": "{parent_dir}/{file_name}",
        "extra_env": {
          "RUST_LOG": "debug"
        }
      }
    },
    "test_framework": {
      "build": {
        "command": "rustc",
        "extra_args": [
          "--edition=2024",
          "-O"
        ],
        "args": [
          "{file_path}",
          "--test",
          "-o",
          "{parent_dir}/{file_name}_test"
        ],
        "extra_env": {
          "CARGO_TARGET_DIR": "target/rust-analyzer"
        }
      },
      "exec": {
        "command": "{parent_dir}/{file_name}_test",
        "extra_test_binary_args": [
          "--show-output"
        ],
        "extra_env": {
          "RUST_BACKTRACE": "1"
        }
      }
    }
  },
  "single_file_script": {
    "extra_args": [],
    "extra_env": {
      "CARGO_TARGET_DIR": "target/rust-analyzer"
    },
    "extra_test_binary_args": [
      "--show-output"
    ]
  },
  "overrides": []
}"#
    .to_string()
}

pub fn create_single_file_script_config() -> String {
    // Create a config with single_file_script section
    let mut config = Map::new();

    // Create single file script config
    let sfs_config = json!({
        "extra_args": ["--edition=2024"],
        "extra_env": {
            "RUST_BACKTRACE": "1"
        },
        "extra_test_binary_args": ["--show-output"]
    });

    config.insert("single_file_script".to_string(), sfs_config);
    config.insert("overrides".to_string(), json!([]));

    // Pretty print the JSON
    serde_json::to_string_pretty(&config).expect("Failed to serialize Dioxus override template")
}

/// Generate a `.cargo-runner.json` for a Bazel + Rust project.
///
/// Produces a config with all framework defaults populated so the user can see
/// what is available and customise it. The `workspace_name` is used to label
/// the Bazel workspace (visible in tooling but not strictly required by Bazel
/// itself).
///
/// # Example output
/// ```json
/// {
///   "bazel": {
///     "workspace": "my_project",
///     "test_framework": { ... },
///     "binary_framework": { ... },
///     "benchmark_framework": { ... }
///   },
///   "overrides": []
/// }
/// ```
pub fn create_bazel_config(workspace_name: &str) -> String {
    let config = json!({
        "bazel": {
            "workspace": workspace_name,
            "test_framework": {
                "command": "bazel",
                "subcommand": "test",
                "target": "{target}",
                "args": ["--test_output", "streamed"],
                "test_args": ["--nocapture", "{test_filter}"]
            },
            "binary_framework": {
                "command": "bazel",
                "subcommand": "run",
                "target": "{target}"
            },
            "benchmark_framework": {
                "command": "bazel",
                "subcommand": "test",
                "target": "{target}",
                "args": [
                    "--test_output", "streamed",
                    "--test_arg", "--bench"
                ],
                "test_args": ["{bench_filter}"]
            }
        },
        "overrides": []
    });

    serde_json::to_string_pretty(&config).expect("Failed to serialize Leptos override template")
}
