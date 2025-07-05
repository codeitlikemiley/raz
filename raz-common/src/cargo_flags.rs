//! Common cargo flags and their categorization

use once_cell::sync::Lazy;
use std::collections::HashSet;

/// Set of common cargo flags that should be treated as cargo options
/// These flags are commonly used with cargo commands and should not be
/// passed after the -- separator
static CARGO_FLAGS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        // Build configuration
        "--release",
        "--debug",
        "--profile",
        // Target selection
        "--target",
        "--target-dir",
        // Feature flags
        "--features",
        "--all-features",
        "--no-default-features",
        // Package selection
        "--package",
        "-p",
        "--workspace",
        "--all",
        "--exclude",
        // Output options
        "--verbose",
        "-v",
        "--quiet",
        "-q",
        "--color",
        "--message-format",
        // Build options
        "--jobs",
        "-j",
        "--frozen",
        "--locked",
        "--offline",
        // Test-specific cargo flags
        "--lib",
        "--bin",
        "--bins",
        "--example",
        "--examples",
        "--test",
        "--tests",
        "--bench",
        "--benches",
        "--all-targets",
        "--doc",
        "--no-run",
        "--no-fail-fast",
        // Environment
        "--config",
        "-Z", // Unstable flags
    ]
    .into_iter()
    .collect()
});

/// Check if a flag is a known cargo flag
pub fn is_cargo_flag(flag: &str) -> bool {
    CARGO_FLAGS.contains(flag)
}

/// Parse a command line string and categorize arguments into cargo options and test arguments
///
/// # Example
/// ```
/// use raz_common::cargo_flags::categorize_override_args;
///
/// let (cargo_opts, test_args) = categorize_override_args("--release --nocapture");
/// assert_eq!(cargo_opts, vec!["--release"]);
/// assert_eq!(test_args, vec!["--nocapture"]);
/// ```
pub fn categorize_override_args(args: &str) -> (Vec<String>, Vec<String>) {
    let mut cargo_options = Vec::new();
    let mut test_args = Vec::new();

    // Parse the arguments
    let parts = match shell_words::split(args) {
        Ok(parts) => parts,
        Err(_) => return (cargo_options, test_args),
    };

    let mut i = 0;
    while i < parts.len() {
        let arg = &parts[i];

        // Check if this is a cargo flag
        if is_cargo_flag(arg) {
            cargo_options.push(arg.clone());

            // Some flags take values, check if next arg is a value
            if matches!(
                arg.as_str(),
                "--target"
                    | "--profile"
                    | "--features"
                    | "--package"
                    | "-p"
                    | "--exclude"
                    | "--color"
                    | "--message-format"
                    | "--jobs"
                    | "-j"
                    | "--config"
                    | "-Z"
                    | "--bin"
                    | "--example"
                    | "--test"
                    | "--bench"
            ) {
                // Next argument is the value for this flag
                if i + 1 < parts.len() && !parts[i + 1].starts_with('-') {
                    i += 1;
                    cargo_options.push(parts[i].clone());
                }
            }
        } else {
            // Not a cargo flag, must be a test argument
            test_args.push(arg.clone());
        }

        i += 1;
    }

    (cargo_options, test_args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_cargo_flag() {
        assert!(is_cargo_flag("--release"));
        assert!(is_cargo_flag("--features"));
        assert!(is_cargo_flag("-p"));
        assert!(!is_cargo_flag("--nocapture"));
        assert!(!is_cargo_flag("--show-output"));
    }

    #[test]
    fn test_categorize_override_args() {
        let (cargo, test) = categorize_override_args("--release --nocapture");
        assert_eq!(cargo, vec!["--release"]);
        assert_eq!(test, vec!["--nocapture"]);

        let (cargo, test) = categorize_override_args("--features foo --test-threads=1");
        assert_eq!(cargo, vec!["--features", "foo"]);
        assert_eq!(test, vec!["--test-threads=1"]);

        let (cargo, test) = categorize_override_args("-p my-crate --show-output");
        assert_eq!(cargo, vec!["-p", "my-crate"]);
        assert_eq!(test, vec!["--show-output"]);
    }
}
