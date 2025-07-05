//! Complete catalog of all cargo subcommand options
//!
//! This module contains a comprehensive list of all cargo options
//! extracted from cargo help commands for intelligent parsing.

use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Type of option value
#[derive(Debug, Clone, PartialEq)]
pub enum OptionValueType {
    /// No value (boolean flag)
    Flag,
    /// Single value required
    Required(String), // description of expected value
    /// Optional value
    Optional(String), // description of expected value
    /// Multiple values allowed
    Multiple(String), // description of expected value
}

/// Option metadata
#[derive(Debug, Clone)]
pub struct OptionInfo {
    /// Long form (e.g., "--release")
    pub long: &'static str,
    /// Short form if available (e.g., "-r")
    pub short: Option<&'static str>,
    /// Value type
    pub value_type: OptionValueType,
    /// Description
    pub description: &'static str,
    /// Category (Package Selection, Target Selection, etc.)
    pub category: &'static str,
}

/// Complete catalog of cargo options by subcommand
pub static CARGO_OPTIONS_CATALOG: Lazy<HashMap<&'static str, Vec<OptionInfo>>> = Lazy::new(|| {
    let mut catalog = HashMap::new();

    // Common options shared by all subcommands
    let common_options = vec![
        OptionInfo {
            long: "--message-format",
            short: None,
            value_type: OptionValueType::Required("FMT".to_string()),
            description: "Error format",
            category: "Options",
        },
        OptionInfo {
            long: "--verbose",
            short: Some("-v"),
            value_type: OptionValueType::Flag,
            description: "Use verbose output (-vv very verbose/build.rs output)",
            category: "Options",
        },
        OptionInfo {
            long: "--quiet",
            short: Some("-q"),
            value_type: OptionValueType::Flag,
            description: "Do not print cargo log messages",
            category: "Options",
        },
        OptionInfo {
            long: "--color",
            short: None,
            value_type: OptionValueType::Required("WHEN".to_string()),
            description: "Coloring: auto, always, never",
            category: "Options",
        },
        OptionInfo {
            long: "--config",
            short: None,
            value_type: OptionValueType::Required("KEY=VALUE|PATH".to_string()),
            description: "Override a configuration value",
            category: "Options",
        },
        OptionInfo {
            long: "-Z",
            short: None,
            value_type: OptionValueType::Required("FLAG".to_string()),
            description: "Unstable (nightly-only) flags",
            category: "Options",
        },
    ];

    // Package selection options
    let package_options = vec![
        OptionInfo {
            long: "--package",
            short: Some("-p"),
            value_type: OptionValueType::Optional("SPEC".to_string()),
            description: "Package to operate on",
            category: "Package Selection",
        },
        OptionInfo {
            long: "--workspace",
            short: None,
            value_type: OptionValueType::Flag,
            description: "Operate on all packages in the workspace",
            category: "Package Selection",
        },
        OptionInfo {
            long: "--exclude",
            short: None,
            value_type: OptionValueType::Required("SPEC".to_string()),
            description: "Exclude packages from the operation",
            category: "Package Selection",
        },
        OptionInfo {
            long: "--all",
            short: None,
            value_type: OptionValueType::Flag,
            description: "Alias for --workspace (deprecated)",
            category: "Package Selection",
        },
    ];

    // Feature selection options
    let feature_options = vec![
        OptionInfo {
            long: "--features",
            short: Some("-F"),
            value_type: OptionValueType::Required("FEATURES".to_string()),
            description: "Space or comma separated list of features to activate",
            category: "Feature Selection",
        },
        OptionInfo {
            long: "--all-features",
            short: None,
            value_type: OptionValueType::Flag,
            description: "Activate all available features",
            category: "Feature Selection",
        },
        OptionInfo {
            long: "--no-default-features",
            short: None,
            value_type: OptionValueType::Flag,
            description: "Do not activate the default feature",
            category: "Feature Selection",
        },
    ];

    // Compilation options
    let compilation_options = vec![
        OptionInfo {
            long: "--jobs",
            short: Some("-j"),
            value_type: OptionValueType::Required("N".to_string()),
            description: "Number of parallel jobs",
            category: "Compilation Options",
        },
        OptionInfo {
            long: "--release",
            short: Some("-r"),
            value_type: OptionValueType::Flag,
            description: "Build artifacts in release mode",
            category: "Compilation Options",
        },
        OptionInfo {
            long: "--profile",
            short: None,
            value_type: OptionValueType::Required("PROFILE-NAME".to_string()),
            description: "Build artifacts with the specified profile",
            category: "Compilation Options",
        },
        OptionInfo {
            long: "--target",
            short: None,
            value_type: OptionValueType::Optional("TRIPLE".to_string()),
            description: "Build for the target triple",
            category: "Compilation Options",
        },
        OptionInfo {
            long: "--target-dir",
            short: None,
            value_type: OptionValueType::Required("DIRECTORY".to_string()),
            description: "Directory for all generated artifacts",
            category: "Compilation Options",
        },
    ];

    // Manifest options
    let manifest_options = vec![
        OptionInfo {
            long: "--manifest-path",
            short: None,
            value_type: OptionValueType::Required("PATH".to_string()),
            description: "Path to Cargo.toml",
            category: "Manifest Options",
        },
        OptionInfo {
            long: "--locked",
            short: None,
            value_type: OptionValueType::Flag,
            description: "Assert that Cargo.lock will remain unchanged",
            category: "Manifest Options",
        },
        OptionInfo {
            long: "--offline",
            short: None,
            value_type: OptionValueType::Flag,
            description: "Run without accessing the network",
            category: "Manifest Options",
        },
        OptionInfo {
            long: "--frozen",
            short: None,
            value_type: OptionValueType::Flag,
            description: "Equivalent to specifying both --locked and --offline",
            category: "Manifest Options",
        },
    ];

    // cargo run specific options
    let mut run_options = vec![];
    run_options.extend(common_options.clone());
    run_options.extend(package_options.clone());
    run_options.extend(feature_options.clone());
    run_options.extend(compilation_options.clone());
    run_options.extend(manifest_options.clone());
    run_options.extend(vec![
        OptionInfo {
            long: "--bin",
            short: None,
            value_type: OptionValueType::Optional("NAME".to_string()),
            description: "Name of the bin target to run",
            category: "Target Selection",
        },
        OptionInfo {
            long: "--example",
            short: None,
            value_type: OptionValueType::Optional("NAME".to_string()),
            description: "Name of the example target to run",
            category: "Target Selection",
        },
    ]);
    catalog.insert("run", run_options);

    // cargo test specific options
    let mut test_options = vec![];
    test_options.extend(common_options.clone());
    test_options.extend(package_options.clone());
    test_options.extend(feature_options.clone());
    test_options.extend(compilation_options.clone());
    test_options.extend(manifest_options.clone());
    test_options.extend(vec![
        OptionInfo {
            long: "--no-run",
            short: None,
            value_type: OptionValueType::Flag,
            description: "Compile, but don't run tests",
            category: "Options",
        },
        OptionInfo {
            long: "--no-fail-fast",
            short: None,
            value_type: OptionValueType::Flag,
            description: "Run all tests regardless of failure",
            category: "Options",
        },
        OptionInfo {
            long: "--lib",
            short: None,
            value_type: OptionValueType::Flag,
            description: "Test only this package's library",
            category: "Target Selection",
        },
        OptionInfo {
            long: "--bins",
            short: None,
            value_type: OptionValueType::Flag,
            description: "Test all binaries",
            category: "Target Selection",
        },
        OptionInfo {
            long: "--bin",
            short: None,
            value_type: OptionValueType::Optional("NAME".to_string()),
            description: "Test only the specified binary",
            category: "Target Selection",
        },
        OptionInfo {
            long: "--examples",
            short: None,
            value_type: OptionValueType::Flag,
            description: "Test all examples",
            category: "Target Selection",
        },
        OptionInfo {
            long: "--example",
            short: None,
            value_type: OptionValueType::Optional("NAME".to_string()),
            description: "Test only the specified example",
            category: "Target Selection",
        },
        OptionInfo {
            long: "--tests",
            short: None,
            value_type: OptionValueType::Flag,
            description: "Test all test targets",
            category: "Target Selection",
        },
        OptionInfo {
            long: "--test",
            short: None,
            value_type: OptionValueType::Optional("NAME".to_string()),
            description: "Test only the specified test target",
            category: "Target Selection",
        },
        OptionInfo {
            long: "--benches",
            short: None,
            value_type: OptionValueType::Flag,
            description: "Test all bench targets",
            category: "Target Selection",
        },
        OptionInfo {
            long: "--bench",
            short: None,
            value_type: OptionValueType::Optional("NAME".to_string()),
            description: "Test only the specified bench target",
            category: "Target Selection",
        },
        OptionInfo {
            long: "--all-targets",
            short: None,
            value_type: OptionValueType::Flag,
            description: "Test all targets (does not include doctests)",
            category: "Target Selection",
        },
        OptionInfo {
            long: "--doc",
            short: None,
            value_type: OptionValueType::Flag,
            description: "Test only this library's documentation",
            category: "Target Selection",
        },
    ]);
    catalog.insert("test", test_options);

    // cargo build specific options
    let mut build_options = vec![];
    build_options.extend(common_options.clone());
    build_options.extend(package_options.clone());
    build_options.extend(feature_options.clone());
    build_options.extend(compilation_options.clone());
    build_options.extend(manifest_options.clone());
    build_options.extend(vec![
        OptionInfo {
            long: "--lib",
            short: None,
            value_type: OptionValueType::Flag,
            description: "Build only this package's library",
            category: "Target Selection",
        },
        OptionInfo {
            long: "--bins",
            short: None,
            value_type: OptionValueType::Flag,
            description: "Build all binaries",
            category: "Target Selection",
        },
        OptionInfo {
            long: "--bin",
            short: None,
            value_type: OptionValueType::Optional("NAME".to_string()),
            description: "Build only the specified binary",
            category: "Target Selection",
        },
        OptionInfo {
            long: "--examples",
            short: None,
            value_type: OptionValueType::Flag,
            description: "Build all examples",
            category: "Target Selection",
        },
        OptionInfo {
            long: "--example",
            short: None,
            value_type: OptionValueType::Optional("NAME".to_string()),
            description: "Build only the specified example",
            category: "Target Selection",
        },
        OptionInfo {
            long: "--tests",
            short: None,
            value_type: OptionValueType::Flag,
            description: "Build all test targets",
            category: "Target Selection",
        },
        OptionInfo {
            long: "--test",
            short: None,
            value_type: OptionValueType::Optional("NAME".to_string()),
            description: "Build only the specified test target",
            category: "Target Selection",
        },
        OptionInfo {
            long: "--benches",
            short: None,
            value_type: OptionValueType::Flag,
            description: "Build all bench targets",
            category: "Target Selection",
        },
        OptionInfo {
            long: "--bench",
            short: None,
            value_type: OptionValueType::Optional("NAME".to_string()),
            description: "Build only the specified bench target",
            category: "Target Selection",
        },
        OptionInfo {
            long: "--all-targets",
            short: None,
            value_type: OptionValueType::Flag,
            description: "Build all targets",
            category: "Target Selection",
        },
        OptionInfo {
            long: "--keep-going",
            short: None,
            value_type: OptionValueType::Flag,
            description: "Do not abort the build as soon as there is an error",
            category: "Compilation Options",
        },
    ]);
    catalog.insert("build", build_options);

    // cargo bench specific options (similar to test)
    let mut bench_options = vec![];
    bench_options.extend(common_options.clone());
    bench_options.extend(package_options.clone());
    bench_options.extend(feature_options.clone());
    bench_options.extend(compilation_options.clone());
    bench_options.extend(manifest_options.clone());
    bench_options.extend(vec![
        OptionInfo {
            long: "--no-run",
            short: None,
            value_type: OptionValueType::Flag,
            description: "Compile, but don't run benchmarks",
            category: "Options",
        },
        OptionInfo {
            long: "--no-fail-fast",
            short: None,
            value_type: OptionValueType::Flag,
            description: "Run all benchmarks regardless of failure",
            category: "Options",
        },
        // Target selection options same as test
        OptionInfo {
            long: "--lib",
            short: None,
            value_type: OptionValueType::Flag,
            description: "Benchmark only this package's library",
            category: "Target Selection",
        },
        OptionInfo {
            long: "--bins",
            short: None,
            value_type: OptionValueType::Flag,
            description: "Benchmark all binaries",
            category: "Target Selection",
        },
        OptionInfo {
            long: "--bin",
            short: None,
            value_type: OptionValueType::Optional("NAME".to_string()),
            description: "Benchmark only the specified binary",
            category: "Target Selection",
        },
        OptionInfo {
            long: "--examples",
            short: None,
            value_type: OptionValueType::Flag,
            description: "Benchmark all examples",
            category: "Target Selection",
        },
        OptionInfo {
            long: "--example",
            short: None,
            value_type: OptionValueType::Optional("NAME".to_string()),
            description: "Benchmark only the specified example",
            category: "Target Selection",
        },
        OptionInfo {
            long: "--tests",
            short: None,
            value_type: OptionValueType::Flag,
            description: "Benchmark all test targets",
            category: "Target Selection",
        },
        OptionInfo {
            long: "--test",
            short: None,
            value_type: OptionValueType::Optional("NAME".to_string()),
            description: "Benchmark only the specified test target",
            category: "Target Selection",
        },
        OptionInfo {
            long: "--benches",
            short: None,
            value_type: OptionValueType::Flag,
            description: "Benchmark all bench targets",
            category: "Target Selection",
        },
        OptionInfo {
            long: "--bench",
            short: None,
            value_type: OptionValueType::Optional("NAME".to_string()),
            description: "Benchmark only the specified bench target",
            category: "Target Selection",
        },
        OptionInfo {
            long: "--all-targets",
            short: None,
            value_type: OptionValueType::Flag,
            description: "Benchmark all targets",
            category: "Target Selection",
        },
    ]);
    catalog.insert("bench", bench_options);

    // cargo check specific options
    let mut check_options = vec![];
    check_options.extend(common_options.clone());
    check_options.extend(package_options.clone());
    check_options.extend(feature_options.clone());
    check_options.extend(compilation_options.clone());
    check_options.extend(manifest_options);
    check_options.extend(vec![
        OptionInfo {
            long: "--lib",
            short: None,
            value_type: OptionValueType::Flag,
            description: "Check only this package's library",
            category: "Target Selection",
        },
        OptionInfo {
            long: "--bins",
            short: None,
            value_type: OptionValueType::Flag,
            description: "Check all binaries",
            category: "Target Selection",
        },
        OptionInfo {
            long: "--bin",
            short: None,
            value_type: OptionValueType::Optional("NAME".to_string()),
            description: "Check only the specified binary",
            category: "Target Selection",
        },
        OptionInfo {
            long: "--examples",
            short: None,
            value_type: OptionValueType::Flag,
            description: "Check all examples",
            category: "Target Selection",
        },
        OptionInfo {
            long: "--example",
            short: None,
            value_type: OptionValueType::Optional("NAME".to_string()),
            description: "Check only the specified example",
            category: "Target Selection",
        },
        OptionInfo {
            long: "--tests",
            short: None,
            value_type: OptionValueType::Flag,
            description: "Check all test targets",
            category: "Target Selection",
        },
        OptionInfo {
            long: "--test",
            short: None,
            value_type: OptionValueType::Optional("NAME".to_string()),
            description: "Check only the specified test target",
            category: "Target Selection",
        },
        OptionInfo {
            long: "--benches",
            short: None,
            value_type: OptionValueType::Flag,
            description: "Check all bench targets",
            category: "Target Selection",
        },
        OptionInfo {
            long: "--bench",
            short: None,
            value_type: OptionValueType::Optional("NAME".to_string()),
            description: "Check only the specified bench target",
            category: "Target Selection",
        },
        OptionInfo {
            long: "--all-targets",
            short: None,
            value_type: OptionValueType::Flag,
            description: "Check all targets",
            category: "Target Selection",
        },
        OptionInfo {
            long: "--keep-going",
            short: None,
            value_type: OptionValueType::Flag,
            description: "Do not abort the build as soon as there is an error",
            category: "Compilation Options",
        },
    ]);
    catalog.insert("check", check_options);

    catalog
});

/// Check if a string is a valid cargo option for the given subcommand
pub fn is_cargo_option(subcommand: &str, option: &str) -> bool {
    if let Some(options) = CARGO_OPTIONS_CATALOG.get(subcommand) {
        options
            .iter()
            .any(|opt| opt.long == option || (opt.short == Some(option)))
    } else {
        false
    }
}

/// Get option info for a given subcommand and option
pub fn get_option_info(subcommand: &str, option: &str) -> Option<&'static OptionInfo> {
    CARGO_OPTIONS_CATALOG
        .get(subcommand)?
        .iter()
        .find(|opt| opt.long == option || (opt.short == Some(option)))
}

/// Parse an option and its value from tokens
pub fn parse_option_value(
    option: &str,
    tokens: &[String],
    index: usize,
) -> (Option<String>, usize) {
    if let Some(info) = CARGO_OPTIONS_CATALOG
        .values()
        .flatten()
        .find(|opt| opt.long == option || (opt.short == Some(option)))
    {
        match &info.value_type {
            OptionValueType::Flag => (None, 0),
            OptionValueType::Required(_) => {
                if index + 1 < tokens.len() && !tokens[index + 1].starts_with('-') {
                    (Some(tokens[index + 1].clone()), 1)
                } else {
                    // Error: required value missing
                    (None, 0)
                }
            }
            OptionValueType::Optional(_) => {
                if index + 1 < tokens.len() && !tokens[index + 1].starts_with('-') {
                    (Some(tokens[index + 1].clone()), 1)
                } else {
                    (None, 0)
                }
            }
            OptionValueType::Multiple(_) => {
                // For multiple values, take all non-option tokens
                let mut values = vec![];
                let mut consumed = 0;
                for token in tokens.iter().skip(index + 1) {
                    if token.starts_with('-') {
                        break;
                    }
                    values.push(token.clone());
                    consumed += 1;
                }
                if values.is_empty() {
                    (None, 0)
                } else {
                    (Some(values.join(" ")), consumed)
                }
            }
        }
    } else {
        (None, 0)
    }
}
