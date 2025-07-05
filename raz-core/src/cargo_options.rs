//! Cargo command options catalog
//!
//! This module provides a comprehensive catalog of cargo command options
//! to enable intelligent override parsing and validation.

use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};

/// Type of cargo option
#[derive(Debug, Clone, PartialEq)]
pub enum OptionType {
    /// Boolean flag (no value)
    Flag,
    /// Single value option
    Single,
    /// Multiple values (comma-separated)
    Multiple,
    /// Can be used multiple times
    Repeatable,
}

/// Cargo option metadata
#[derive(Debug, Clone)]
pub struct OptionMetadata {
    /// Option type
    pub option_type: OptionType,
    /// Short description
    pub description: &'static str,
    /// Conflicts with these options
    pub conflicts_with: Vec<&'static str>,
}

/// Catalog of all cargo options
pub struct CargoOptions {
    /// Common options for all cargo commands
    pub common: HashMap<&'static str, OptionMetadata>,
    /// cargo run specific options
    pub run: HashMap<&'static str, OptionMetadata>,
    /// cargo test specific options
    pub test: HashMap<&'static str, OptionMetadata>,
    /// cargo build specific options
    pub build: HashMap<&'static str, OptionMetadata>,
    /// cargo check specific options
    pub check: HashMap<&'static str, OptionMetadata>,
    /// cargo bench specific options
    pub bench: HashMap<&'static str, OptionMetadata>,
}

/// Static instance of cargo options catalog
pub static CARGO_OPTIONS: Lazy<CargoOptions> = Lazy::new(|| {
    let mut options = CargoOptions {
        common: HashMap::new(),
        run: HashMap::new(),
        test: HashMap::new(),
        build: HashMap::new(),
        check: HashMap::new(),
        bench: HashMap::new(),
    };

    // Common options for all cargo commands
    options.common.insert(
        "--package",
        OptionMetadata {
            option_type: OptionType::Single,
            description: "Package to run",
            conflicts_with: vec!["--workspace"],
        },
    );

    options.common.insert(
        "-p",
        OptionMetadata {
            option_type: OptionType::Single,
            description: "Alias for --package",
            conflicts_with: vec!["--workspace"],
        },
    );

    options.common.insert(
        "--workspace",
        OptionMetadata {
            option_type: OptionType::Flag,
            description: "Run for all workspace members",
            conflicts_with: vec!["--package", "-p"],
        },
    );

    options.common.insert(
        "--exclude",
        OptionMetadata {
            option_type: OptionType::Multiple,
            description: "Exclude packages from the operation",
            conflicts_with: vec![],
        },
    );

    options.common.insert(
        "--features",
        OptionMetadata {
            option_type: OptionType::Multiple,
            description: "Space or comma separated list of features to activate",
            conflicts_with: vec!["--all-features"],
        },
    );

    options.common.insert(
        "--all-features",
        OptionMetadata {
            option_type: OptionType::Flag,
            description: "Activate all available features",
            conflicts_with: vec!["--features", "--no-default-features"],
        },
    );

    options.common.insert(
        "--no-default-features",
        OptionMetadata {
            option_type: OptionType::Flag,
            description: "Do not activate the default feature",
            conflicts_with: vec![],
        },
    );

    options.common.insert(
        "--release",
        OptionMetadata {
            option_type: OptionType::Flag,
            description: "Build in release mode",
            conflicts_with: vec!["--debug"],
        },
    );

    options.common.insert(
        "--debug",
        OptionMetadata {
            option_type: OptionType::Flag,
            description: "Build in debug mode (default)",
            conflicts_with: vec!["--release"],
        },
    );

    options.common.insert(
        "--profile",
        OptionMetadata {
            option_type: OptionType::Single,
            description: "Build with the specified profile",
            conflicts_with: vec![],
        },
    );

    options.common.insert(
        "--target",
        OptionMetadata {
            option_type: OptionType::Single,
            description: "Build for the target triple",
            conflicts_with: vec![],
        },
    );

    options.common.insert(
        "--target-dir",
        OptionMetadata {
            option_type: OptionType::Single,
            description: "Directory for all generated artifacts",
            conflicts_with: vec![],
        },
    );

    options.common.insert(
        "--verbose",
        OptionMetadata {
            option_type: OptionType::Flag,
            description: "Use verbose output",
            conflicts_with: vec!["--quiet"],
        },
    );

    options.common.insert(
        "-v",
        OptionMetadata {
            option_type: OptionType::Flag,
            description: "Alias for --verbose",
            conflicts_with: vec!["--quiet", "-q"],
        },
    );

    options.common.insert(
        "--quiet",
        OptionMetadata {
            option_type: OptionType::Flag,
            description: "No output printed to stdout",
            conflicts_with: vec!["--verbose", "-v"],
        },
    );

    options.common.insert(
        "-q",
        OptionMetadata {
            option_type: OptionType::Flag,
            description: "Alias for --quiet",
            conflicts_with: vec!["--verbose", "-v"],
        },
    );

    options.common.insert(
        "--color",
        OptionMetadata {
            option_type: OptionType::Single,
            description: "Coloring: auto, always, never",
            conflicts_with: vec![],
        },
    );

    options.common.insert(
        "--message-format",
        OptionMetadata {
            option_type: OptionType::Single,
            description: "Error format",
            conflicts_with: vec![],
        },
    );

    options.common.insert(
        "--manifest-path",
        OptionMetadata {
            option_type: OptionType::Single,
            description: "Path to Cargo.toml",
            conflicts_with: vec![],
        },
    );

    options.common.insert(
        "--frozen",
        OptionMetadata {
            option_type: OptionType::Flag,
            description: "Require Cargo.lock and cache are up to date",
            conflicts_with: vec!["--locked"],
        },
    );

    options.common.insert(
        "--locked",
        OptionMetadata {
            option_type: OptionType::Flag,
            description: "Require Cargo.lock is up to date",
            conflicts_with: vec!["--frozen"],
        },
    );

    options.common.insert(
        "--offline",
        OptionMetadata {
            option_type: OptionType::Flag,
            description: "Run without accessing the network",
            conflicts_with: vec![],
        },
    );

    options.common.insert(
        "-Z",
        OptionMetadata {
            option_type: OptionType::Repeatable,
            description: "Unstable (nightly-only) flags to Cargo",
            conflicts_with: vec![],
        },
    );

    // cargo run specific options
    options.run.insert(
        "--bin",
        OptionMetadata {
            option_type: OptionType::Single,
            description: "Name of the bin target to run",
            conflicts_with: vec!["--example"],
        },
    );

    options.run.insert(
        "--example",
        OptionMetadata {
            option_type: OptionType::Single,
            description: "Name of the example target to run",
            conflicts_with: vec!["--bin"],
        },
    );

    // Framework-specific options (for Dioxus, etc.)
    // Framework-specific options for Dioxus serve
    options.run.insert(
        "--platform",
        OptionMetadata {
            option_type: OptionType::Single,
            description: "Target platform (web, desktop, mobile)",
            conflicts_with: vec![],
        },
    );

    options.run.insert(
        "--device",
        OptionMetadata {
            option_type: OptionType::Single,
            description: "Build for device (true) or simulator (false)",
            conflicts_with: vec![],
        },
    );

    options.run.insert(
        "--hot-reload",
        OptionMetadata {
            option_type: OptionType::Flag,
            description: "Enable hot reloading",
            conflicts_with: vec![],
        },
    );

    options.run.insert(
        "--open",
        OptionMetadata {
            option_type: OptionType::Flag,
            description: "Open the browser on startup",
            conflicts_with: vec![],
        },
    );

    options.run.insert(
        "--port",
        OptionMetadata {
            option_type: OptionType::Single,
            description: "Port to serve on",
            conflicts_with: vec![],
        },
    );

    options.run.insert(
        "--host",
        OptionMetadata {
            option_type: OptionType::Single,
            description: "Host to serve on",
            conflicts_with: vec![],
        },
    );

    // cargo test specific options
    options.test.insert(
        "--lib",
        OptionMetadata {
            option_type: OptionType::Flag,
            description: "Test only this package's library",
            conflicts_with: vec!["--bin", "--bins"],
        },
    );

    options.test.insert(
        "--bin",
        OptionMetadata {
            option_type: OptionType::Single,
            description: "Test only the specified binary",
            conflicts_with: vec!["--lib", "--bins"],
        },
    );

    options.test.insert(
        "--bins",
        OptionMetadata {
            option_type: OptionType::Flag,
            description: "Test all binaries",
            conflicts_with: vec!["--lib", "--bin"],
        },
    );

    options.test.insert(
        "--example",
        OptionMetadata {
            option_type: OptionType::Single,
            description: "Test only the specified example",
            conflicts_with: vec!["--examples"],
        },
    );

    options.test.insert(
        "--examples",
        OptionMetadata {
            option_type: OptionType::Flag,
            description: "Test all examples",
            conflicts_with: vec!["--example"],
        },
    );

    options.test.insert(
        "--test",
        OptionMetadata {
            option_type: OptionType::Single,
            description: "Test only the specified test target",
            conflicts_with: vec!["--tests"],
        },
    );

    options.test.insert(
        "--tests",
        OptionMetadata {
            option_type: OptionType::Flag,
            description: "Test all tests",
            conflicts_with: vec!["--test"],
        },
    );

    options.test.insert(
        "--bench",
        OptionMetadata {
            option_type: OptionType::Single,
            description: "Test only the specified bench target",
            conflicts_with: vec!["--benches"],
        },
    );

    options.test.insert(
        "--benches",
        OptionMetadata {
            option_type: OptionType::Flag,
            description: "Test all benches",
            conflicts_with: vec!["--bench"],
        },
    );

    options.test.insert(
        "--all-targets",
        OptionMetadata {
            option_type: OptionType::Flag,
            description: "Test all targets",
            conflicts_with: vec![],
        },
    );

    options.test.insert(
        "--doc",
        OptionMetadata {
            option_type: OptionType::Flag,
            description: "Test only this library's documentation",
            conflicts_with: vec![],
        },
    );

    options.test.insert(
        "--no-run",
        OptionMetadata {
            option_type: OptionType::Flag,
            description: "Compile, but don't run tests",
            conflicts_with: vec![],
        },
    );

    options.test.insert(
        "--no-fail-fast",
        OptionMetadata {
            option_type: OptionType::Flag,
            description: "Run all tests regardless of failure",
            conflicts_with: vec![],
        },
    );

    options.test.insert(
        "-j",
        OptionMetadata {
            option_type: OptionType::Single,
            description: "Number of parallel jobs",
            conflicts_with: vec![],
        },
    );

    options.test.insert(
        "--jobs",
        OptionMetadata {
            option_type: OptionType::Single,
            description: "Number of parallel jobs",
            conflicts_with: vec![],
        },
    );

    // Framework-specific options (for Dioxus, etc.)
    options.test.insert(
        "--platform",
        OptionMetadata {
            option_type: OptionType::Single,
            description: "Target platform (web, desktop, mobile)",
            conflicts_with: vec![],
        },
    );

    // cargo build specific options
    options.build.insert(
        "--lib",
        OptionMetadata {
            option_type: OptionType::Flag,
            description: "Build only this package's library",
            conflicts_with: vec!["--bin", "--bins"],
        },
    );

    options.build.insert(
        "--bin",
        OptionMetadata {
            option_type: OptionType::Single,
            description: "Build only the specified binary",
            conflicts_with: vec!["--lib", "--bins"],
        },
    );

    options.build.insert(
        "--bins",
        OptionMetadata {
            option_type: OptionType::Flag,
            description: "Build all binaries",
            conflicts_with: vec!["--lib", "--bin"],
        },
    );

    options.build.insert(
        "--example",
        OptionMetadata {
            option_type: OptionType::Single,
            description: "Build only the specified example",
            conflicts_with: vec!["--examples"],
        },
    );

    options.build.insert(
        "--examples",
        OptionMetadata {
            option_type: OptionType::Flag,
            description: "Build all examples",
            conflicts_with: vec!["--example"],
        },
    );

    options.build.insert(
        "--test",
        OptionMetadata {
            option_type: OptionType::Single,
            description: "Build only the specified test target",
            conflicts_with: vec!["--tests"],
        },
    );

    options.build.insert(
        "--tests",
        OptionMetadata {
            option_type: OptionType::Flag,
            description: "Build all tests",
            conflicts_with: vec!["--test"],
        },
    );

    options.build.insert(
        "--bench",
        OptionMetadata {
            option_type: OptionType::Single,
            description: "Build only the specified bench target",
            conflicts_with: vec!["--benches"],
        },
    );

    options.build.insert(
        "--benches",
        OptionMetadata {
            option_type: OptionType::Flag,
            description: "Build all benches",
            conflicts_with: vec!["--bench"],
        },
    );

    options.build.insert(
        "--all-targets",
        OptionMetadata {
            option_type: OptionType::Flag,
            description: "Build all targets",
            conflicts_with: vec![],
        },
    );

    // cargo check specific options (shares most with build)
    options.check.insert(
        "--lib",
        OptionMetadata {
            option_type: OptionType::Flag,
            description: "Check only this package's library",
            conflicts_with: vec!["--bin", "--bins"],
        },
    );

    options.check.insert(
        "--bin",
        OptionMetadata {
            option_type: OptionType::Single,
            description: "Check only the specified binary",
            conflicts_with: vec!["--lib", "--bins"],
        },
    );

    options.check.insert(
        "--bins",
        OptionMetadata {
            option_type: OptionType::Flag,
            description: "Check all binaries",
            conflicts_with: vec!["--lib", "--bin"],
        },
    );

    options.check.insert(
        "--example",
        OptionMetadata {
            option_type: OptionType::Single,
            description: "Check only the specified example",
            conflicts_with: vec!["--examples"],
        },
    );

    options.check.insert(
        "--examples",
        OptionMetadata {
            option_type: OptionType::Flag,
            description: "Check all examples",
            conflicts_with: vec!["--example"],
        },
    );

    options.check.insert(
        "--test",
        OptionMetadata {
            option_type: OptionType::Single,
            description: "Check only the specified test target",
            conflicts_with: vec!["--tests"],
        },
    );

    options.check.insert(
        "--tests",
        OptionMetadata {
            option_type: OptionType::Flag,
            description: "Check all tests",
            conflicts_with: vec!["--test"],
        },
    );

    options.check.insert(
        "--bench",
        OptionMetadata {
            option_type: OptionType::Single,
            description: "Check only the specified bench target",
            conflicts_with: vec!["--benches"],
        },
    );

    options.check.insert(
        "--benches",
        OptionMetadata {
            option_type: OptionType::Flag,
            description: "Check all benches",
            conflicts_with: vec!["--bench"],
        },
    );

    options.check.insert(
        "--all-targets",
        OptionMetadata {
            option_type: OptionType::Flag,
            description: "Check all targets",
            conflicts_with: vec![],
        },
    );

    // cargo bench specific options
    options.bench.insert(
        "--lib",
        OptionMetadata {
            option_type: OptionType::Flag,
            description: "Benchmark only this package's library",
            conflicts_with: vec!["--bin", "--bins"],
        },
    );

    options.bench.insert(
        "--bin",
        OptionMetadata {
            option_type: OptionType::Single,
            description: "Benchmark only the specified binary",
            conflicts_with: vec!["--lib", "--bins"],
        },
    );

    options.bench.insert(
        "--bins",
        OptionMetadata {
            option_type: OptionType::Flag,
            description: "Benchmark all binaries",
            conflicts_with: vec!["--lib", "--bin"],
        },
    );

    options.bench.insert(
        "--example",
        OptionMetadata {
            option_type: OptionType::Single,
            description: "Benchmark only the specified example",
            conflicts_with: vec!["--examples"],
        },
    );

    options.bench.insert(
        "--examples",
        OptionMetadata {
            option_type: OptionType::Flag,
            description: "Benchmark all examples",
            conflicts_with: vec!["--example"],
        },
    );

    options.bench.insert(
        "--test",
        OptionMetadata {
            option_type: OptionType::Single,
            description: "Benchmark only the specified test target",
            conflicts_with: vec!["--tests"],
        },
    );

    options.bench.insert(
        "--tests",
        OptionMetadata {
            option_type: OptionType::Flag,
            description: "Benchmark all tests",
            conflicts_with: vec!["--test"],
        },
    );

    options.bench.insert(
        "--bench",
        OptionMetadata {
            option_type: OptionType::Single,
            description: "Benchmark only the specified bench target",
            conflicts_with: vec!["--benches"],
        },
    );

    options.bench.insert(
        "--benches",
        OptionMetadata {
            option_type: OptionType::Flag,
            description: "Benchmark all benches",
            conflicts_with: vec!["--bench"],
        },
    );

    options.bench.insert(
        "--all-targets",
        OptionMetadata {
            option_type: OptionType::Flag,
            description: "Benchmark all targets",
            conflicts_with: vec![],
        },
    );

    options.bench.insert(
        "--no-run",
        OptionMetadata {
            option_type: OptionType::Flag,
            description: "Compile, but don't run benchmarks",
            conflicts_with: vec![],
        },
    );

    options
});

impl CargoOptions {
    /// Get option metadata for a specific command
    pub fn get_option(&self, command: &str, option: &str) -> Option<&OptionMetadata> {
        // Check common options first
        if let Some(metadata) = self.common.get(option) {
            return Some(metadata);
        }

        // Check command-specific options
        match command {
            "run" => self.run.get(option),
            "test" => self.test.get(option),
            "build" => self.build.get(option),
            "check" => self.check.get(option),
            "bench" => self.bench.get(option),
            _ => None,
        }
    }

    /// Check if an option is valid for a command
    pub fn is_valid_option(&self, command: &str, option: &str) -> bool {
        // First validate that the command is known
        match command {
            "run" | "test" | "build" | "check" | "bench" => {
                self.get_option(command, option).is_some()
            }
            _ => false, // Unknown command
        }
    }

    /// Get all valid options for a command
    pub fn get_all_options(&self, command: &str) -> HashSet<&'static str> {
        let mut options: HashSet<&'static str> = self.common.keys().copied().collect();

        match command {
            "run" => options.extend(self.run.keys().copied()),
            "test" => options.extend(self.test.keys().copied()),
            "build" => options.extend(self.build.keys().copied()),
            "check" => options.extend(self.check.keys().copied()),
            "bench" => options.extend(self.bench.keys().copied()),
            _ => {}
        }

        options
    }

    /// Check for option conflicts
    pub fn check_conflicts<'a>(
        &self,
        command: &str,
        options: &[&'a str],
    ) -> Vec<(&'a str, &'a str)> {
        let mut conflicts = Vec::new();

        for (i, &opt1) in options.iter().enumerate() {
            if let Some(metadata) = self.get_option(command, opt1) {
                for &opt2 in &options[i + 1..] {
                    if metadata.conflicts_with.contains(&opt2) {
                        conflicts.push((opt1, opt2));
                    }
                }
            }
        }

        conflicts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_common_options() {
        let options = &CARGO_OPTIONS;
        assert!(options.is_valid_option("run", "--features"));
        assert!(options.is_valid_option("test", "--release"));
        assert!(options.is_valid_option("bench", "--package"));
    }

    #[test]
    fn test_command_specific_options() {
        let options = &CARGO_OPTIONS;
        assert!(options.is_valid_option("run", "--bin"));
        assert!(options.is_valid_option("test", "--doc"));
        assert!(options.is_valid_option("bench", "--benches"));

        // Cross-command validation
        assert!(!options.is_valid_option("run", "--doc")); // doc is test-specific
        assert!(!options.is_valid_option("run", "--no-run")); // no-run is test/bench-specific
    }

    #[test]
    fn test_option_conflicts() {
        let options = &CARGO_OPTIONS;
        let conflicts = options.check_conflicts("test", &["--lib", "--bin", "--bins"]);
        assert!(!conflicts.is_empty());
        assert!(conflicts.contains(&("--lib", "--bin")));
        assert!(conflicts.contains(&("--lib", "--bins")));
        assert!(conflicts.contains(&("--bin", "--bins")));
    }
}
