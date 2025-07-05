//! Rustc option validation and cargo-to-rustc translation

/// Translate cargo options to rustc equivalents
pub fn translate_cargo_to_rustc(option: &str) -> Option<&'static str> {
    match option {
        "--release" => Some("-O"),
        "--debug" => Some("-g"),
        "--target" => Some("--target"), // Same in both
        "--verbose" | "-v" => Some("-v"),
        "--quiet" | "-q" => None,        // No rustc equivalent
        "--features" => None,            // No direct rustc equivalent
        "--all-features" => None,        // No direct rustc equivalent
        "--no-default-features" => None, // No direct rustc equivalent
        "--bin" => None,                 // Handled differently in rustc
        "--lib" => Some("--crate-type=lib"),
        "--example" => None, // No direct rustc equivalent
        "--test" => Some("--test"),
        "--bench" => None, // No direct rustc equivalent
        _ => None,
    }
}

/// Check if an option is valid for rustc
pub fn is_valid_rustc_option(option: &str) -> bool {
    // Short options
    if option.len() == 2 && option.starts_with('-') {
        matches!(
            option,
            "-h" | "-g"
                | "-O"
                | "-o"
                | "-A"
                | "-W"
                | "-D"
                | "-F"
                | "-C"
                | "-V"
                | "-v"
                | "-L"
                | "-l"
        )
    } else if option.starts_with("--") {
        // Long options
        matches!(
            option,
            "--help"
                | "--cfg"
                | "--check-cfg"
                | "--crate-type"
                | "--crate-name"
                | "--edition"
                | "--emit"
                | "--print"
                | "--out-dir"
                | "--explain"
                | "--test"
                | "--target"
                | "--allow"
                | "--warn"
                | "--force-warn"
                | "--deny"
                | "--forbid"
                | "--cap-lints"
                | "--codegen"
                | "--version"
                | "--verbose"
                | "--extern"
                | "--sysroot"
                | "--error-format"
                | "--json"
                | "--color"
                | "--diagnostic-width"
                | "--remap-path-prefix"
                | "--env-set"
        )
    } else if option.starts_with("-C") || option.starts_with("--codegen") {
        // Codegen options
        true
    } else if option.starts_with("-L") || option.starts_with("-l") {
        // Library options
        true
    } else {
        false
    }
}

/// Get rustc optimization flags based on profile
pub fn get_rustc_optimization_flags(profile: &str) -> Vec<&'static str> {
    match profile {
        "release" => vec!["-O", "-C", "lto=yes", "-C", "codegen-units=1"],
        "debug" => vec!["-g", "-C", "opt-level=0"],
        "test" => vec!["--test", "-g"],
        _ => vec![],
    }
}

/// Get help text for cargo-to-rustc option translation
pub fn get_rustc_translation_help() -> &'static str {
    r#"Cargo to Rustc Option Translation:

Common Cargo options and their Rustc equivalents:
  --release           →  -O (optimize) + -C lto=yes + -C codegen-units=1
  --debug             →  -g (debug info)
  --test              →  --test (build test harness)
  --lib               →  --crate-type=lib
  --target <TARGET>   →  --target <TARGET>
  --verbose/-v        →  -v

Options with no Rustc equivalent (will be ignored):
  --features, --all-features, --no-default-features
  --bin, --example, --bench
  --quiet/-q

For rustc scripts, you can use native rustc options:
  -O              Optimize (equivalent to -C opt-level=3)
  -g              Include debug info
  -C <OPT>=<VAL>  Set codegen options (e.g., -C lto=yes)
  --edition       Set Rust edition (2015|2018|2021|2024)
  --crate-type    Set output type (bin|lib|rlib|dylib|cdylib|staticlib)
  
Example overrides for rustc:
  -O                        # Optimize for release
  -g                        # Include debug symbols
  --edition 2021            # Use Rust 2021 edition
  -C target-cpu=native      # Optimize for native CPU
"#
}

/// Validate and translate a set of options for rustc
pub fn validate_and_translate_options(options: &[String], is_rustc: bool) -> Vec<String> {
    if !is_rustc {
        return options.to_vec();
    }

    let mut translated = Vec::new();
    let mut skip_next = false;

    for (i, option) in options.iter().enumerate() {
        if skip_next {
            skip_next = false;
            continue;
        }

        // First try to translate cargo option to rustc
        if let Some(rustc_opt) = translate_cargo_to_rustc(option) {
            translated.push(rustc_opt.to_string());

            // Check if this option takes a value
            if matches!(option.as_str(), "--target") && i + 1 < options.len() {
                translated.push(options[i + 1].clone());
                skip_next = true;
            }
        } else if is_valid_rustc_option(option) {
            // It's already a valid rustc option
            translated.push(option.clone());

            // Check if this option takes a value
            if needs_value(option) && i + 1 < options.len() && !options[i + 1].starts_with('-') {
                translated.push(options[i + 1].clone());
                skip_next = true;
            }
        } else if option == "--profile" && i + 1 < options.len() {
            // Special handling for --profile
            let profile = &options[i + 1];
            translated.extend(
                get_rustc_optimization_flags(profile)
                    .iter()
                    .map(|s| s.to_string()),
            );
            skip_next = true;
        }
        // Skip invalid options silently
    }

    translated
}

/// Check if a rustc option needs a value
fn needs_value(option: &str) -> bool {
    matches!(
        option,
        "--cfg"
            | "--check-cfg"
            | "--crate-type"
            | "--crate-name"
            | "--edition"
            | "--emit"
            | "--print"
            | "--out-dir"
            | "--explain"
            | "--target"
            | "--allow"
            | "--warn"
            | "--force-warn"
            | "--deny"
            | "--forbid"
            | "--cap-lints"
            | "--codegen"
            | "-o"
            | "-L"
            | "-l"
            | "--extern"
            | "--sysroot"
            | "--error-format"
            | "--json"
            | "--color"
            | "--diagnostic-width"
            | "--remap-path-prefix"
            | "--env-set"
    ) || option == "-C"
        || option == "-A"
        || option == "-W"
        || option == "-D"
        || option == "-F"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translate_cargo_to_rustc() {
        assert_eq!(translate_cargo_to_rustc("--release"), Some("-O"));
        assert_eq!(translate_cargo_to_rustc("--debug"), Some("-g"));
        assert_eq!(translate_cargo_to_rustc("--test"), Some("--test"));
        assert_eq!(translate_cargo_to_rustc("--lib"), Some("--crate-type=lib"));
        assert_eq!(translate_cargo_to_rustc("--features"), None);
    }

    #[test]
    fn test_is_valid_rustc_option() {
        assert!(is_valid_rustc_option("-O"));
        assert!(is_valid_rustc_option("-g"));
        assert!(is_valid_rustc_option("--test"));
        assert!(is_valid_rustc_option("--target"));
        assert!(is_valid_rustc_option("-C"));
        assert!(!is_valid_rustc_option("--release"));
        assert!(!is_valid_rustc_option("--features"));
    }

    #[test]
    fn test_validate_and_translate_options() {
        let options = vec![
            "--release".to_string(),
            "--target".to_string(),
            "wasm32-unknown-unknown".to_string(),
            "--features".to_string(),
            "foo,bar".to_string(),
        ];

        let translated = validate_and_translate_options(&options, true);
        assert_eq!(
            translated,
            vec![
                "-O".to_string(),
                "--target".to_string(),
                "wasm32-unknown-unknown".to_string(),
            ]
        );
    }
}
