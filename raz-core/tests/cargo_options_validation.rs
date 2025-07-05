//! Comprehensive tests for cargo command option validation
//!
//! This test suite validates that all cargo options from `cargo [command] --help`
//! are properly recognized and categorized by our option validation system.

use raz_core::cargo_options::{CARGO_OPTIONS, OptionType};

/// Test all cargo run options from `cargo run --help`
#[cfg(test)]
mod cargo_run_tests {
    use super::*;

    #[test]
    fn test_cargo_run_common_options() {
        // Test common options that work with cargo run
        assert!(CARGO_OPTIONS.is_valid_option("run", "--release"));
        assert!(CARGO_OPTIONS.is_valid_option("run", "--debug"));
        assert!(CARGO_OPTIONS.is_valid_option("run", "--target"));
        assert!(CARGO_OPTIONS.is_valid_option("run", "--features"));
        assert!(CARGO_OPTIONS.is_valid_option("run", "--all-features"));
        assert!(CARGO_OPTIONS.is_valid_option("run", "--no-default-features"));
        assert!(CARGO_OPTIONS.is_valid_option("run", "--profile"));
        assert!(CARGO_OPTIONS.is_valid_option("run", "--package"));
        assert!(CARGO_OPTIONS.is_valid_option("run", "--workspace"));
        assert!(CARGO_OPTIONS.is_valid_option("run", "--exclude"));
        assert!(CARGO_OPTIONS.is_valid_option("run", "--verbose"));
        assert!(CARGO_OPTIONS.is_valid_option("run", "--quiet"));
        assert!(CARGO_OPTIONS.is_valid_option("run", "--color"));
        assert!(CARGO_OPTIONS.is_valid_option("run", "--message-format"));
        assert!(CARGO_OPTIONS.is_valid_option("run", "--manifest-path"));
        assert!(CARGO_OPTIONS.is_valid_option("run", "--frozen"));
        assert!(CARGO_OPTIONS.is_valid_option("run", "--locked"));
        assert!(CARGO_OPTIONS.is_valid_option("run", "--offline"));
    }

    #[test]
    fn test_cargo_run_specific_options() {
        // Test cargo run specific options
        assert!(CARGO_OPTIONS.is_valid_option("run", "--bin"));
        assert!(CARGO_OPTIONS.is_valid_option("run", "--example"));
    }

    #[test]
    fn test_cargo_run_option_types() {
        // Test that options have correct types
        let options = &CARGO_OPTIONS;

        // Flag options (no value required)
        assert_eq!(
            options.get_option("run", "--release").unwrap().option_type,
            OptionType::Flag
        );
        assert_eq!(
            options
                .get_option("run", "--all-features")
                .unwrap()
                .option_type,
            OptionType::Flag
        );
        assert_eq!(
            options
                .get_option("run", "--no-default-features")
                .unwrap()
                .option_type,
            OptionType::Flag
        );
        assert_eq!(
            options
                .get_option("run", "--workspace")
                .unwrap()
                .option_type,
            OptionType::Flag
        );
        assert_eq!(
            options.get_option("run", "--verbose").unwrap().option_type,
            OptionType::Flag
        );
        assert_eq!(
            options.get_option("run", "--quiet").unwrap().option_type,
            OptionType::Flag
        );
        assert_eq!(
            options.get_option("run", "--frozen").unwrap().option_type,
            OptionType::Flag
        );
        assert_eq!(
            options.get_option("run", "--locked").unwrap().option_type,
            OptionType::Flag
        );
        assert_eq!(
            options.get_option("run", "--offline").unwrap().option_type,
            OptionType::Flag
        );

        // Single value options
        assert_eq!(
            options.get_option("run", "--target").unwrap().option_type,
            OptionType::Single
        );
        assert_eq!(
            options.get_option("run", "--profile").unwrap().option_type,
            OptionType::Single
        );
        assert_eq!(
            options.get_option("run", "--package").unwrap().option_type,
            OptionType::Single
        );
        assert_eq!(
            options.get_option("run", "--color").unwrap().option_type,
            OptionType::Single
        );
        assert_eq!(
            options
                .get_option("run", "--message-format")
                .unwrap()
                .option_type,
            OptionType::Single
        );
        assert_eq!(
            options
                .get_option("run", "--manifest-path")
                .unwrap()
                .option_type,
            OptionType::Single
        );
        assert_eq!(
            options.get_option("run", "--bin").unwrap().option_type,
            OptionType::Single
        );
        assert_eq!(
            options.get_option("run", "--example").unwrap().option_type,
            OptionType::Single
        );

        // Multiple value options
        assert_eq!(
            options.get_option("run", "--features").unwrap().option_type,
            OptionType::Multiple
        );
        assert_eq!(
            options.get_option("run", "--exclude").unwrap().option_type,
            OptionType::Multiple
        );
    }

    #[test]
    fn test_cargo_run_framework_options() {
        // Test framework-specific options (Dioxus)
        assert!(CARGO_OPTIONS.is_valid_option("run", "--platform"));
        assert!(CARGO_OPTIONS.is_valid_option("run", "--device"));
        assert!(CARGO_OPTIONS.is_valid_option("run", "--hot-reload"));
        assert!(CARGO_OPTIONS.is_valid_option("run", "--open"));
        assert!(CARGO_OPTIONS.is_valid_option("run", "--port"));
        assert!(CARGO_OPTIONS.is_valid_option("run", "--host"));

        // Check types
        let options = &CARGO_OPTIONS;
        assert_eq!(
            options.get_option("run", "--platform").unwrap().option_type,
            OptionType::Single
        );
        assert_eq!(
            options.get_option("run", "--device").unwrap().option_type,
            OptionType::Single
        );
        assert_eq!(
            options
                .get_option("run", "--hot-reload")
                .unwrap()
                .option_type,
            OptionType::Flag
        );
        assert_eq!(
            options.get_option("run", "--open").unwrap().option_type,
            OptionType::Flag
        );
        assert_eq!(
            options.get_option("run", "--port").unwrap().option_type,
            OptionType::Single
        );
        assert_eq!(
            options.get_option("run", "--host").unwrap().option_type,
            OptionType::Single
        );
    }
}

/// Test all cargo test options from `cargo test --help`
#[cfg(test)]
mod cargo_test_tests {
    use super::*;

    #[test]
    fn test_cargo_test_common_options() {
        // Common options that work with cargo test
        assert!(CARGO_OPTIONS.is_valid_option("test", "--release"));
        assert!(CARGO_OPTIONS.is_valid_option("test", "--debug"));
        assert!(CARGO_OPTIONS.is_valid_option("test", "--target"));
        assert!(CARGO_OPTIONS.is_valid_option("test", "--features"));
        assert!(CARGO_OPTIONS.is_valid_option("test", "--all-features"));
        assert!(CARGO_OPTIONS.is_valid_option("test", "--no-default-features"));
        assert!(CARGO_OPTIONS.is_valid_option("test", "--profile"));
        assert!(CARGO_OPTIONS.is_valid_option("test", "--package"));
        assert!(CARGO_OPTIONS.is_valid_option("test", "--workspace"));
        assert!(CARGO_OPTIONS.is_valid_option("test", "--exclude"));
        assert!(CARGO_OPTIONS.is_valid_option("test", "--verbose"));
        assert!(CARGO_OPTIONS.is_valid_option("test", "--quiet"));
        assert!(CARGO_OPTIONS.is_valid_option("test", "--color"));
        assert!(CARGO_OPTIONS.is_valid_option("test", "--message-format"));
        assert!(CARGO_OPTIONS.is_valid_option("test", "--manifest-path"));
        assert!(CARGO_OPTIONS.is_valid_option("test", "--frozen"));
        assert!(CARGO_OPTIONS.is_valid_option("test", "--locked"));
        assert!(CARGO_OPTIONS.is_valid_option("test", "--offline"));
    }

    #[test]
    fn test_cargo_test_specific_options() {
        // Test cargo test specific options
        assert!(CARGO_OPTIONS.is_valid_option("test", "--lib"));
        assert!(CARGO_OPTIONS.is_valid_option("test", "--bin"));
        assert!(CARGO_OPTIONS.is_valid_option("test", "--bins"));
        assert!(CARGO_OPTIONS.is_valid_option("test", "--example"));
        assert!(CARGO_OPTIONS.is_valid_option("test", "--examples"));
        assert!(CARGO_OPTIONS.is_valid_option("test", "--test"));
        assert!(CARGO_OPTIONS.is_valid_option("test", "--tests"));
        assert!(CARGO_OPTIONS.is_valid_option("test", "--bench"));
        assert!(CARGO_OPTIONS.is_valid_option("test", "--benches"));
        assert!(CARGO_OPTIONS.is_valid_option("test", "--all-targets"));
        assert!(CARGO_OPTIONS.is_valid_option("test", "--doc"));
        assert!(CARGO_OPTIONS.is_valid_option("test", "--no-run"));
        assert!(CARGO_OPTIONS.is_valid_option("test", "--no-fail-fast"));
    }

    #[test]
    fn test_cargo_test_option_types() {
        let options = &CARGO_OPTIONS;

        // Flag options
        assert_eq!(
            options.get_option("test", "--lib").unwrap().option_type,
            OptionType::Flag
        );
        assert_eq!(
            options.get_option("test", "--bins").unwrap().option_type,
            OptionType::Flag
        );
        assert_eq!(
            options
                .get_option("test", "--examples")
                .unwrap()
                .option_type,
            OptionType::Flag
        );
        assert_eq!(
            options.get_option("test", "--tests").unwrap().option_type,
            OptionType::Flag
        );
        assert_eq!(
            options.get_option("test", "--benches").unwrap().option_type,
            OptionType::Flag
        );
        assert_eq!(
            options
                .get_option("test", "--all-targets")
                .unwrap()
                .option_type,
            OptionType::Flag
        );
        assert_eq!(
            options.get_option("test", "--doc").unwrap().option_type,
            OptionType::Flag
        );
        assert_eq!(
            options.get_option("test", "--no-run").unwrap().option_type,
            OptionType::Flag
        );
        assert_eq!(
            options
                .get_option("test", "--no-fail-fast")
                .unwrap()
                .option_type,
            OptionType::Flag
        );

        // Single value options
        assert_eq!(
            options.get_option("test", "--bin").unwrap().option_type,
            OptionType::Single
        );
        assert_eq!(
            options.get_option("test", "--example").unwrap().option_type,
            OptionType::Single
        );
        assert_eq!(
            options.get_option("test", "--test").unwrap().option_type,
            OptionType::Single
        );
        assert_eq!(
            options.get_option("test", "--bench").unwrap().option_type,
            OptionType::Single
        );
    }

    #[test]
    fn test_cargo_test_conflicts() {
        let options = &CARGO_OPTIONS;

        // Test known conflicts
        let conflicts = options.check_conflicts("test", &["--lib", "--bin"]);
        assert!(!conflicts.is_empty());

        let conflicts = options.check_conflicts("test", &["--bins", "--lib"]);
        assert!(!conflicts.is_empty());
    }
}

/// Test all cargo build options from `cargo build --help`
#[cfg(test)]
mod cargo_build_tests {
    use super::*;

    #[test]
    fn test_cargo_build_common_options() {
        // Common options that work with cargo build
        assert!(CARGO_OPTIONS.is_valid_option("build", "--release"));
        assert!(CARGO_OPTIONS.is_valid_option("build", "--debug"));
        assert!(CARGO_OPTIONS.is_valid_option("build", "--target"));
        assert!(CARGO_OPTIONS.is_valid_option("build", "--features"));
        assert!(CARGO_OPTIONS.is_valid_option("build", "--all-features"));
        assert!(CARGO_OPTIONS.is_valid_option("build", "--no-default-features"));
        assert!(CARGO_OPTIONS.is_valid_option("build", "--profile"));
        assert!(CARGO_OPTIONS.is_valid_option("build", "--package"));
        assert!(CARGO_OPTIONS.is_valid_option("build", "--workspace"));
        assert!(CARGO_OPTIONS.is_valid_option("build", "--exclude"));
        assert!(CARGO_OPTIONS.is_valid_option("build", "--verbose"));
        assert!(CARGO_OPTIONS.is_valid_option("build", "--quiet"));
        assert!(CARGO_OPTIONS.is_valid_option("build", "--color"));
        assert!(CARGO_OPTIONS.is_valid_option("build", "--message-format"));
        assert!(CARGO_OPTIONS.is_valid_option("build", "--manifest-path"));
        assert!(CARGO_OPTIONS.is_valid_option("build", "--frozen"));
        assert!(CARGO_OPTIONS.is_valid_option("build", "--locked"));
        assert!(CARGO_OPTIONS.is_valid_option("build", "--offline"));
    }

    #[test]
    fn test_cargo_build_specific_options() {
        // Test cargo build specific options
        assert!(CARGO_OPTIONS.is_valid_option("build", "--lib"));
        assert!(CARGO_OPTIONS.is_valid_option("build", "--bin"));
        assert!(CARGO_OPTIONS.is_valid_option("build", "--bins"));
        assert!(CARGO_OPTIONS.is_valid_option("build", "--example"));
        assert!(CARGO_OPTIONS.is_valid_option("build", "--examples"));
        assert!(CARGO_OPTIONS.is_valid_option("build", "--test"));
        assert!(CARGO_OPTIONS.is_valid_option("build", "--tests"));
        assert!(CARGO_OPTIONS.is_valid_option("build", "--bench"));
        assert!(CARGO_OPTIONS.is_valid_option("build", "--benches"));
        assert!(CARGO_OPTIONS.is_valid_option("build", "--all-targets"));
    }
}

/// Test all cargo check options from `cargo check --help`
#[cfg(test)]
mod cargo_check_tests {
    use super::*;

    #[test]
    fn test_cargo_check_options() {
        // cargo check shares most options with build
        assert!(CARGO_OPTIONS.is_valid_option("check", "--release"));
        assert!(CARGO_OPTIONS.is_valid_option("check", "--features"));
        assert!(CARGO_OPTIONS.is_valid_option("check", "--all-features"));
        assert!(CARGO_OPTIONS.is_valid_option("check", "--no-default-features"));
        assert!(CARGO_OPTIONS.is_valid_option("check", "--target"));
        assert!(CARGO_OPTIONS.is_valid_option("check", "--lib"));
        assert!(CARGO_OPTIONS.is_valid_option("check", "--bin"));
        assert!(CARGO_OPTIONS.is_valid_option("check", "--bins"));
        assert!(CARGO_OPTIONS.is_valid_option("check", "--example"));
        assert!(CARGO_OPTIONS.is_valid_option("check", "--examples"));
        assert!(CARGO_OPTIONS.is_valid_option("check", "--test"));
        assert!(CARGO_OPTIONS.is_valid_option("check", "--tests"));
        assert!(CARGO_OPTIONS.is_valid_option("check", "--bench"));
        assert!(CARGO_OPTIONS.is_valid_option("check", "--benches"));
        assert!(CARGO_OPTIONS.is_valid_option("check", "--all-targets"));
        assert!(CARGO_OPTIONS.is_valid_option("check", "--profile"));
        assert!(CARGO_OPTIONS.is_valid_option("check", "--package"));
        assert!(CARGO_OPTIONS.is_valid_option("check", "--workspace"));
        assert!(CARGO_OPTIONS.is_valid_option("check", "--exclude"));
    }
}

/// Test all cargo bench options from `cargo bench --help`
#[cfg(test)]
mod cargo_bench_tests {
    use super::*;

    #[test]
    fn test_cargo_bench_options() {
        // cargo bench specific options
        assert!(CARGO_OPTIONS.is_valid_option("bench", "--bench"));
        assert!(CARGO_OPTIONS.is_valid_option("bench", "--benches"));
        assert!(CARGO_OPTIONS.is_valid_option("bench", "--lib"));
        assert!(CARGO_OPTIONS.is_valid_option("bench", "--bin"));
        assert!(CARGO_OPTIONS.is_valid_option("bench", "--bins"));
        assert!(CARGO_OPTIONS.is_valid_option("bench", "--example"));
        assert!(CARGO_OPTIONS.is_valid_option("bench", "--examples"));
        assert!(CARGO_OPTIONS.is_valid_option("bench", "--test"));
        assert!(CARGO_OPTIONS.is_valid_option("bench", "--tests"));
        assert!(CARGO_OPTIONS.is_valid_option("bench", "--all-targets"));
        assert!(CARGO_OPTIONS.is_valid_option("bench", "--no-run"));

        // Common options
        assert!(CARGO_OPTIONS.is_valid_option("bench", "--features"));
        assert!(CARGO_OPTIONS.is_valid_option("bench", "--all-features"));
        assert!(CARGO_OPTIONS.is_valid_option("bench", "--no-default-features"));
        assert!(CARGO_OPTIONS.is_valid_option("bench", "--target"));
        assert!(CARGO_OPTIONS.is_valid_option("bench", "--package"));
        assert!(CARGO_OPTIONS.is_valid_option("bench", "--workspace"));
        assert!(CARGO_OPTIONS.is_valid_option("bench", "--exclude"));
    }
}

/// Test validation edge cases and error conditions
#[cfg(test)]
mod validation_edge_cases {
    use super::*;

    #[test]
    fn test_invalid_options() {
        // Test that invalid options are rejected
        assert!(!CARGO_OPTIONS.is_valid_option("run", "--invalid-option"));
        assert!(!CARGO_OPTIONS.is_valid_option("test", "--made-up-flag"));
        assert!(!CARGO_OPTIONS.is_valid_option("build", "--nonexistent"));
    }

    #[test]
    fn test_invalid_commands() {
        // Test that invalid commands are handled gracefully
        assert!(!CARGO_OPTIONS.is_valid_option("invalid-command", "--release"));
    }

    #[test]
    fn test_command_specific_validation() {
        // Test that options are specific to the right commands
        assert!(CARGO_OPTIONS.is_valid_option("run", "--bin"));
        assert!(CARGO_OPTIONS.is_valid_option("test", "--doc"));
        assert!(!CARGO_OPTIONS.is_valid_option("run", "--doc")); // doc is test-specific
    }

    #[test]
    fn test_option_metadata_completeness() {
        let options = &CARGO_OPTIONS;

        // Ensure all options have proper metadata
        for option in [
            "--release",
            "--debug",
            "--features",
            "--target",
            "--package",
        ] {
            let meta = options.get_option("run", option);
            assert!(meta.is_some(), "Option {option} should have metadata");
            assert!(
                !meta.unwrap().description.is_empty(),
                "Option {option} should have description"
            );
        }
    }

    #[test]
    fn test_conflicts_comprehensive() {
        let options = &CARGO_OPTIONS;

        // Test all known conflicts are properly defined
        let known_conflicts = [
            ("test", &["--lib", "--bin"][..]),
            ("test", &["--lib", "--bins"]),
            ("test", &["--bin", "--bins"]),
            ("build", &["--lib", "--bin"]),
            ("build", &["--lib", "--bins"]),
            ("build", &["--bin", "--bins"]),
        ];

        for (command, conflicting_options) in known_conflicts {
            let conflicts = options.check_conflicts(command, conflicting_options);
            assert!(
                !conflicts.is_empty(),
                "Should detect conflicts between options {conflicting_options:?} for command {command}"
            );
        }
    }
}

/// Integration tests with the override parser
#[cfg(test)]
mod parser_integration_tests {

    use raz_override::{OptionValue, OverrideParser};

    #[test]
    fn test_parser_validates_against_cargo_options() {
        let parser = OverrideParser::new("run");

        // Valid options should parse successfully
        let result = parser.parse("--release --features web --bin mybin");
        assert!(result.is_ok());

        let parsed = result.unwrap();
        assert!(parsed.options.contains_key("--release"));
        assert!(parsed.options.contains_key("--features"));
        assert!(parsed.options.contains_key("--bin"));
    }

    #[test]
    fn test_parser_handles_unknown_options() {
        let parser = OverrideParser::new("run");

        // Unknown options should still parse (treated as flags)
        let result = parser.parse("--unknown-option");
        assert!(result.is_ok());

        let parsed = result.unwrap();
        assert!(parsed.options.contains_key("--unknown-option"));
    }

    #[test]
    fn test_parser_respects_option_types() {
        let parser = OverrideParser::new("run");

        // Single value option should require a value
        let result = parser.parse("--target x86_64-unknown-linux-gnu");
        assert!(result.is_ok());

        let parsed = result.unwrap();
        assert_eq!(
            parsed.options.get("--target").unwrap(),
            &OptionValue::Single("x86_64-unknown-linux-gnu".to_string())
        );
    }
}
