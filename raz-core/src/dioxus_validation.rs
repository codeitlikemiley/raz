//! Comprehensive Framework CLI validation system
//!
//! Provides validation for Dioxus (`dx serve`) and Leptos (`cargo-leptos watch`)
//! options with proper type checking, enum validation, and intelligent suggestions for typos.

use regex::Regex;
use std::collections::HashMap;

/// Dioxus option value types
#[derive(Debug, Clone, PartialEq)]
pub enum DioxusOptionType {
    /// Flag option (no value required)
    Flag,
    /// Boolean option with explicit true/false values
    Bool { default: Option<bool> },
    /// String value
    String { default: Option<String> },
    /// Port number (1-65535)
    Port { default: Option<u16> },
    /// Network address
    Address { default: Option<String> },
    /// Platform enum
    Platform { default: Option<String> },
    /// Architecture enum
    Architecture,
    /// Profile name
    Profile { default: Option<String> },
    /// Features list (space-separated)
    Features,
    /// Multiple string values
    Multiple,
    /// Interval in seconds
    Interval { default: Option<u64> },
}

/// Dioxus option metadata
#[derive(Debug, Clone)]
pub struct DioxusOptionInfo {
    pub long: &'static str,
    pub short: Option<&'static str>,
    pub option_type: DioxusOptionType,
    pub description: &'static str,
    pub possible_values: Option<Vec<&'static str>>,
}

/// Validation result
#[derive(Debug, Clone)]
pub enum ValidationResult {
    Valid,
    InvalidOption {
        suggestion: Option<String>,
    },
    InvalidValue {
        expected: String,
        got: String,
        suggestions: Vec<String>,
    },
    MissingValue {
        expected: String,
    },
}

/// Framework CLI validator for Dioxus and Leptos
pub struct FrameworkValidator {
    dioxus_serve_options: HashMap<&'static str, DioxusOptionInfo>,
    leptos_watch_options: HashMap<&'static str, DioxusOptionInfo>,
    platform_values: Vec<&'static str>,
    arch_values: Vec<&'static str>,
    bool_values: Vec<&'static str>,
}

/// Legacy alias for backward compatibility
pub type DioxusValidator = FrameworkValidator;

impl FrameworkValidator {
    pub fn new() -> Self {
        let mut dioxus_serve_options = HashMap::new();
        let mut leptos_watch_options = HashMap::new();

        // ===== DIOXUS DX SERVE OPTIONS =====

        // Port option
        dioxus_serve_options.insert(
            "--port",
            DioxusOptionInfo {
                long: "--port",
                short: None,
                option_type: DioxusOptionType::Port { default: None },
                description: "The port the server will run on",
                possible_values: None,
            },
        );

        // Address option
        dioxus_serve_options.insert(
            "--addr",
            DioxusOptionInfo {
                long: "--addr",
                short: None,
                option_type: DioxusOptionType::Address {
                    default: Some("localhost".to_string()),
                },
                description: "The address the server will run on",
                possible_values: None,
            },
        );

        // Open option - boolean with default true
        dioxus_serve_options.insert(
            "--open",
            DioxusOptionInfo {
                long: "--open",
                short: None,
                option_type: DioxusOptionType::Bool {
                    default: Some(true),
                },
                description: "Open the app in the default browser",
                possible_values: Some(vec!["true", "false"]),
            },
        );

        // Hot reload option - boolean with default true
        dioxus_serve_options.insert(
            "--hot-reload",
            DioxusOptionInfo {
                long: "--hot-reload",
                short: None,
                option_type: DioxusOptionType::Bool {
                    default: Some(true),
                },
                description: "Enable full hot reloading for the app",
                possible_values: Some(vec!["true", "false"]),
            },
        );

        // Always on top option - boolean with default true
        dioxus_serve_options.insert(
            "--always-on-top",
            DioxusOptionInfo {
                long: "--always-on-top",
                short: None,
                option_type: DioxusOptionType::Bool {
                    default: Some(true),
                },
                description: "Configure always-on-top for desktop apps",
                possible_values: Some(vec!["true", "false"]),
            },
        );

        // Cross origin policy - flag
        dioxus_serve_options.insert(
            "--cross-origin-policy",
            DioxusOptionInfo {
                long: "--cross-origin-policy",
                short: None,
                option_type: DioxusOptionType::Flag,
                description: "Set cross-origin-policy to same-origin",
                possible_values: None,
            },
        );

        // Args option
        dioxus_serve_options.insert(
            "--args",
            DioxusOptionInfo {
                long: "--args",
                short: None,
                option_type: DioxusOptionType::String { default: None },
                description: "Additional arguments to pass to the executable",
                possible_values: None,
            },
        );

        // WSL file poll interval
        dioxus_serve_options.insert(
            "--wsl-file-poll-interval",
            DioxusOptionInfo {
                long: "--wsl-file-poll-interval",
                short: None,
                option_type: DioxusOptionType::Interval { default: None },
                description:
                    "Sets the interval in seconds that the CLI will poll for file changes on WSL",
                possible_values: None,
            },
        );

        // Interactive option - boolean with optional value
        dioxus_serve_options.insert(
            "--interactive",
            DioxusOptionInfo {
                long: "--interactive",
                short: Some("-i"),
                option_type: DioxusOptionType::Bool {
                    default: Some(false),
                },
                description: "Run the server in interactive mode",
                possible_values: Some(vec!["true", "false"]),
            },
        );

        // Release flag
        dioxus_serve_options.insert(
            "--release",
            DioxusOptionInfo {
                long: "--release",
                short: Some("-r"),
                option_type: DioxusOptionType::Flag,
                description: "Build in release mode",
                possible_values: None,
            },
        );

        // Force sequential flag
        dioxus_serve_options.insert(
            "--force-sequential",
            DioxusOptionInfo {
                long: "--force-sequential",
                short: None,
                option_type: DioxusOptionType::Flag,
                description: "Force fullstack builds to run server first, then client",
                possible_values: None,
            },
        );

        // Profile option
        dioxus_serve_options.insert(
            "--profile",
            DioxusOptionInfo {
                long: "--profile",
                short: None,
                option_type: DioxusOptionType::Profile { default: None },
                description: "Build the app with custom a profile",
                possible_values: None,
            },
        );

        // Verbose flag
        dioxus_serve_options.insert(
            "--verbose",
            DioxusOptionInfo {
                long: "--verbose",
                short: None,
                option_type: DioxusOptionType::Flag,
                description: "Use verbose output",
                possible_values: None,
            },
        );

        // Server profile option
        dioxus_serve_options.insert(
            "--server-profile",
            DioxusOptionInfo {
                long: "--server-profile",
                short: None,
                option_type: DioxusOptionType::Profile {
                    default: Some("server-dev".to_string()),
                },
                description: "Build with custom profile for the fullstack server",
                possible_values: None,
            },
        );

        // Trace flag
        dioxus_serve_options.insert(
            "--trace",
            DioxusOptionInfo {
                long: "--trace",
                short: None,
                option_type: DioxusOptionType::Flag,
                description: "Use trace output",
                possible_values: None,
            },
        );

        // JSON output flag
        dioxus_serve_options.insert(
            "--json-output",
            DioxusOptionInfo {
                long: "--json-output",
                short: None,
                option_type: DioxusOptionType::Flag,
                description: "Output logs in JSON format",
                possible_values: None,
            },
        );

        // Platform option - enum with specific values
        dioxus_serve_options.insert(
            "--platform",
            DioxusOptionInfo {
                long: "--platform",
                short: None,
                option_type: DioxusOptionType::Platform {
                    default: Some("default_platform".to_string()),
                },
                description: "Build platform: support Web & Desktop",
                possible_values: Some(vec![
                    "web", "macos", "windows", "linux", "ios", "android", "server", "liveview",
                ]),
            },
        );

        // Fullstack flag
        dioxus_serve_options.insert(
            "--fullstack",
            DioxusOptionInfo {
                long: "--fullstack",
                short: None,
                option_type: DioxusOptionType::Flag,
                description: "Build the fullstack variant of this app",
                possible_values: None,
            },
        );

        // SSG flag
        dioxus_serve_options.insert(
            "--ssg",
            DioxusOptionInfo {
                long: "--ssg",
                short: None,
                option_type: DioxusOptionType::Flag,
                description: "Run the ssg config of the app and generate the files",
                possible_values: None,
            },
        );

        // Skip assets flag
        dioxus_serve_options.insert(
            "--skip-assets",
            DioxusOptionInfo {
                long: "--skip-assets",
                short: None,
                option_type: DioxusOptionType::Flag,
                description: "Skip collecting assets from dependencies",
                possible_values: None,
            },
        );

        // Inject loading scripts flag
        dioxus_serve_options.insert(
            "--inject-loading-scripts",
            DioxusOptionInfo {
                long: "--inject-loading-scripts",
                short: None,
                option_type: DioxusOptionType::Flag,
                description: "Inject scripts to load the wasm and js files",
                possible_values: None,
            },
        );

        // Debug symbols flag
        dioxus_serve_options.insert(
            "--debug-symbols",
            DioxusOptionInfo {
                long: "--debug-symbols",
                short: None,
                option_type: DioxusOptionType::Flag,
                description: "Generate debug symbols for the wasm binary",
                possible_values: None,
            },
        );

        // Nightly flag
        dioxus_serve_options.insert(
            "--nightly",
            DioxusOptionInfo {
                long: "--nightly",
                short: None,
                option_type: DioxusOptionType::Flag,
                description: "Build for nightly",
                possible_values: None,
            },
        );

        // Example option
        dioxus_serve_options.insert(
            "--example",
            DioxusOptionInfo {
                long: "--example",
                short: None,
                option_type: DioxusOptionType::String {
                    default: Some("".to_string()),
                },
                description: "Build a example",
                possible_values: None,
            },
        );

        // Bin option
        dioxus_serve_options.insert(
            "--bin",
            DioxusOptionInfo {
                long: "--bin",
                short: None,
                option_type: DioxusOptionType::String {
                    default: Some("".to_string()),
                },
                description: "Build a binary",
                possible_values: None,
            },
        );

        // Package option
        dioxus_serve_options.insert(
            "--package",
            DioxusOptionInfo {
                long: "--package",
                short: Some("-p"),
                option_type: DioxusOptionType::String { default: None },
                description: "The package to build",
                possible_values: None,
            },
        );

        // Features option
        dioxus_serve_options.insert(
            "--features",
            DioxusOptionInfo {
                long: "--features",
                short: None,
                option_type: DioxusOptionType::Features,
                description: "Space separated list of features to activate",
                possible_values: None,
            },
        );

        // Client features option
        dioxus_serve_options.insert(
            "--client-features",
            DioxusOptionInfo {
                long: "--client-features",
                short: None,
                option_type: DioxusOptionType::String {
                    default: Some("web".to_string()),
                },
                description: "The feature to use for the client in a fullstack app",
                possible_values: None,
            },
        );

        // Server features option
        dioxus_serve_options.insert(
            "--server-features",
            DioxusOptionInfo {
                long: "--server-features",
                short: None,
                option_type: DioxusOptionType::String {
                    default: Some("server".to_string()),
                },
                description: "The feature to use for the server in a fullstack app",
                possible_values: None,
            },
        );

        // No default features flag
        dioxus_serve_options.insert(
            "--no-default-features",
            DioxusOptionInfo {
                long: "--no-default-features",
                short: None,
                option_type: DioxusOptionType::Flag,
                description: "Don't include the default features in the build",
                possible_values: None,
            },
        );

        // Architecture option
        dioxus_serve_options.insert(
            "--arch",
            DioxusOptionInfo {
                long: "--arch",
                short: None,
                option_type: DioxusOptionType::Architecture,
                description: "The architecture to build for",
                possible_values: Some(vec!["arm", "arm64", "x86", "x64"]),
            },
        );

        // Device option - boolean
        dioxus_serve_options.insert(
            "--device",
            DioxusOptionInfo {
                long: "--device",
                short: None,
                option_type: DioxusOptionType::Bool { default: None },
                description: "Are we building for a device or just the simulator",
                possible_values: Some(vec!["true", "false"]),
            },
        );

        // Target option
        dioxus_serve_options.insert(
            "--target",
            DioxusOptionInfo {
                long: "--target",
                short: None,
                option_type: DioxusOptionType::String { default: None },
                description: "Rustc platform triple",
                possible_values: None,
            },
        );

        // ===== LEPTOS CARGO-LEPTOS WATCH OPTIONS =====

        // Release flag
        leptos_watch_options.insert(
            "--release",
            DioxusOptionInfo {
                long: "--release",
                short: Some("-r"),
                option_type: DioxusOptionType::Flag,
                description: "Build artifacts in release mode, with optimizations",
                possible_values: None,
            },
        );

        // Precompress flag
        leptos_watch_options.insert("--precompress", DioxusOptionInfo {
            long: "--precompress",
            short: Some("-P"),
            option_type: DioxusOptionType::Flag,
            description: "Precompress static assets with gzip and brotli. Applies to release builds only",
            possible_values: None,
        });

        // Hot reload flag
        leptos_watch_options.insert(
            "--hot-reload",
            DioxusOptionInfo {
                long: "--hot-reload",
                short: None,
                option_type: DioxusOptionType::Flag,
                description: "Turn on partial hot-reloading. Requires rust nightly [beta]",
                possible_values: None,
            },
        );

        // Project option
        leptos_watch_options.insert(
            "--project",
            DioxusOptionInfo {
                long: "--project",
                short: Some("-p"),
                option_type: DioxusOptionType::String { default: None },
                description: "Which project to use, from a list of projects defined in a workspace",
                possible_values: None,
            },
        );

        // Features option
        leptos_watch_options.insert(
            "--features",
            DioxusOptionInfo {
                long: "--features",
                short: None,
                option_type: DioxusOptionType::Features,
                description: "The features to use when compiling all targets",
                possible_values: None,
            },
        );

        // Lib features option
        leptos_watch_options.insert(
            "--lib-features",
            DioxusOptionInfo {
                long: "--lib-features",
                short: None,
                option_type: DioxusOptionType::Features,
                description: "The features to use when compiling the lib target",
                possible_values: None,
            },
        );

        // Lib cargo args option
        leptos_watch_options.insert(
            "--lib-cargo-args",
            DioxusOptionInfo {
                long: "--lib-cargo-args",
                short: None,
                option_type: DioxusOptionType::String { default: None },
                description: "The cargo flags to pass to cargo when compiling the lib target",
                possible_values: None,
            },
        );

        // Bin features option
        leptos_watch_options.insert(
            "--bin-features",
            DioxusOptionInfo {
                long: "--bin-features",
                short: None,
                option_type: DioxusOptionType::Features,
                description: "The features to use when compiling the bin target",
                possible_values: None,
            },
        );

        // Bin cargo args option
        leptos_watch_options.insert(
            "--bin-cargo-args",
            DioxusOptionInfo {
                long: "--bin-cargo-args",
                short: None,
                option_type: DioxusOptionType::String { default: None },
                description: "The cargo flags to pass to cargo when compiling the bin target",
                possible_values: None,
            },
        );

        // WASM debug flag
        leptos_watch_options.insert("--wasm-debug", DioxusOptionInfo {
            long: "--wasm-debug",
            short: None,
            option_type: DioxusOptionType::Flag,
            description: "Include debug information in Wasm output. Includes source maps and DWARF debug info",
            possible_values: None,
        });

        // Verbosity option (multiple v's allowed)
        leptos_watch_options.insert(
            "-v",
            DioxusOptionInfo {
                long: "-v",
                short: None,
                option_type: DioxusOptionType::Multiple,
                description:
                    "Verbosity (none: info, errors & warnings, -v: verbose, -vv: very verbose)",
                possible_values: None,
            },
        );

        // JS minify option
        leptos_watch_options.insert(
            "--js-minify",
            DioxusOptionInfo {
                long: "--js-minify",
                short: None,
                option_type: DioxusOptionType::Bool {
                    default: Some(true),
                },
                description: "Minify javascript assets with swc. Applies to release builds only",
                possible_values: Some(vec!["true", "false"]),
            },
        );

        Self {
            dioxus_serve_options,
            leptos_watch_options,
            platform_values: vec![
                "web", "macos", "windows", "linux", "ios", "android", "server", "liveview",
            ],
            arch_values: vec!["arm", "arm64", "x86", "x64"],
            bool_values: vec!["true", "false"],
        }
    }

    /// Validate a framework option and its value
    pub fn validate_option(
        &self,
        framework: &str,
        option: &str,
        value: Option<&str>,
    ) -> ValidationResult {
        // Check if option exists in the specified framework
        let option_info = match framework {
            "dioxus" => self.dioxus_serve_options.get(option),
            "leptos" => self.leptos_watch_options.get(option),
            _ => None,
        };

        let option_info = match option_info {
            Some(info) => info,
            None => {
                // Try to find a close match for typo suggestion
                let suggestion = self.find_closest_option(framework, option);
                return ValidationResult::InvalidOption { suggestion };
            }
        };

        // Validate based on option type
        match &option_info.option_type {
            DioxusOptionType::Flag => {
                if let Some(val) = value {
                    ValidationResult::InvalidValue {
                        expected: "no value (this is a flag)".to_string(),
                        got: val.to_string(),
                        suggestions: vec![
                            "Remove the value - this option doesn't take a value".to_string(),
                        ],
                    }
                } else {
                    ValidationResult::Valid
                }
            }

            DioxusOptionType::Bool { default: _ } => {
                if let Some(val) = value {
                    if self.bool_values.contains(&val) {
                        ValidationResult::Valid
                    } else {
                        ValidationResult::InvalidValue {
                            expected: "true or false".to_string(),
                            got: val.to_string(),
                            suggestions: self.suggest_bool_values(val),
                        }
                    }
                } else {
                    // For boolean options, missing value might be valid (uses default)
                    ValidationResult::Valid
                }
            }

            DioxusOptionType::Port { default: _ } => match value {
                Some(val) => match val.parse::<u16>() {
                    Ok(port) => {
                        if port > 0 {
                            ValidationResult::Valid
                        } else {
                            ValidationResult::InvalidValue {
                                expected: "port number (1-65535)".to_string(),
                                got: val.to_string(),
                                suggestions: vec![
                                    "Try a port number like 3000, 8080, or 8000".to_string(),
                                ],
                            }
                        }
                    }
                    Err(_) => ValidationResult::InvalidValue {
                        expected: "port number (1-65535)".to_string(),
                        got: val.to_string(),
                        suggestions: vec![
                            "Port must be a number, e.g., 3000, 8080, 8000".to_string(),
                        ],
                    },
                },
                None => ValidationResult::MissingValue {
                    expected: "port number (e.g., 3000, 8080)".to_string(),
                },
            },

            DioxusOptionType::Address { default: _ } => match value {
                Some(val) => {
                    if self.is_valid_address(val) {
                        ValidationResult::Valid
                    } else {
                        ValidationResult::InvalidValue {
                            expected: "valid IP address or hostname".to_string(),
                            got: val.to_string(),
                            suggestions: vec![
                                "localhost".to_string(),
                                "127.0.0.1".to_string(),
                                "0.0.0.0".to_string(),
                            ],
                        }
                    }
                }
                None => ValidationResult::MissingValue {
                    expected: "IP address or hostname (e.g., localhost, 127.0.0.1)".to_string(),
                },
            },

            DioxusOptionType::Platform { default: _ } => match value {
                Some(val) => {
                    if self.platform_values.contains(&val) {
                        ValidationResult::Valid
                    } else {
                        ValidationResult::InvalidValue {
                            expected: format!("one of: {}", self.platform_values.join(", ")),
                            got: val.to_string(),
                            suggestions: self.suggest_platform_values(val),
                        }
                    }
                }
                None => ValidationResult::MissingValue {
                    expected: format!("platform ({})", self.platform_values.join(", ")),
                },
            },

            DioxusOptionType::Architecture => match value {
                Some(val) => {
                    if self.arch_values.contains(&val) {
                        ValidationResult::Valid
                    } else {
                        ValidationResult::InvalidValue {
                            expected: format!("one of: {}", self.arch_values.join(", ")),
                            got: val.to_string(),
                            suggestions: self.suggest_arch_values(val),
                        }
                    }
                }
                None => ValidationResult::MissingValue {
                    expected: format!("architecture ({})", self.arch_values.join(", ")),
                },
            },

            DioxusOptionType::Interval { default: _ } => match value {
                Some(val) => match val.parse::<u64>() {
                    Ok(_) => ValidationResult::Valid,
                    Err(_) => ValidationResult::InvalidValue {
                        expected: "number of seconds".to_string(),
                        got: val.to_string(),
                        suggestions: vec!["Try a number like 1, 5, or 30".to_string()],
                    },
                },
                None => ValidationResult::MissingValue {
                    expected: "interval in seconds (e.g., 1, 5, 30)".to_string(),
                },
            },

            DioxusOptionType::String { default: _ }
            | DioxusOptionType::Profile { default: _ }
            | DioxusOptionType::Features
            | DioxusOptionType::Multiple => match value {
                Some(_) => ValidationResult::Valid,
                None => ValidationResult::MissingValue {
                    expected: "string value".to_string(),
                },
            },
        }
    }

    /// Find the closest matching option for typo suggestions
    fn find_closest_option(&self, framework: &str, input: &str) -> Option<String> {
        let mut best_match = None;
        let mut best_distance = usize::MAX;

        let options = match framework {
            "dioxus" => self.dioxus_serve_options.keys(),
            "leptos" => self.leptos_watch_options.keys(),
            _ => return None,
        };

        for option in options {
            let distance = levenshtein_distance(input, option);
            if distance < best_distance && distance <= 3 {
                best_distance = distance;
                best_match = Some(option.to_string());
            }
        }

        best_match
    }

    /// Suggest boolean values based on input
    fn suggest_bool_values(&self, input: &str) -> Vec<String> {
        let lower = input.to_lowercase();
        if lower.starts_with('t') || lower.starts_with('y') || lower == "1" {
            vec!["true".to_string()]
        } else if lower.starts_with('f') || lower.starts_with('n') || lower == "0" {
            vec!["false".to_string()]
        } else {
            vec!["true".to_string(), "false".to_string()]
        }
    }

    /// Suggest platform values based on input
    fn suggest_platform_values(&self, input: &str) -> Vec<String> {
        let mut suggestions = Vec::new();
        let lower = input.to_lowercase();

        for platform in &self.platform_values {
            if platform.starts_with(&lower) || levenshtein_distance(&lower, platform) <= 2 {
                suggestions.push(platform.to_string());
            }
        }

        if suggestions.is_empty() {
            // Provide common suggestions
            suggestions.extend_from_slice(&["web".to_string(), "desktop".to_string()]);
        }

        suggestions
    }

    /// Suggest architecture values based on input
    fn suggest_arch_values(&self, input: &str) -> Vec<String> {
        let mut suggestions = Vec::new();
        let lower = input.to_lowercase();

        for arch in &self.arch_values {
            if arch.starts_with(&lower) || levenshtein_distance(&lower, arch) <= 1 {
                suggestions.push(arch.to_string());
            }
        }

        if suggestions.is_empty() {
            suggestions.extend_from_slice(&["x64".to_string(), "arm64".to_string()]);
        }

        suggestions
    }

    /// Basic address validation
    fn is_valid_address(&self, addr: &str) -> bool {
        // Allow localhost, IP addresses, and hostnames
        if addr == "localhost" || addr == "0.0.0.0" {
            return true;
        }

        // Simple IP address validation
        let ip_regex = Regex::new(r"^(\d{1,3}\.){3}\d{1,3}$").unwrap();
        if ip_regex.is_match(addr) {
            // Check each octet is valid (0-255)
            return addr.split('.').all(|octet| octet.parse::<u8>().is_ok());
        }

        // Allow valid hostnames (basic check)
        let hostname_regex = Regex::new(r"^[a-zA-Z0-9.-]+$").unwrap();
        hostname_regex.is_match(addr)
    }

    /// Check if an option is valid for the specified framework
    pub fn is_valid_option(&self, framework: &str, option: &str) -> bool {
        match framework {
            "dioxus" => self.dioxus_serve_options.contains_key(option),
            "leptos" => self.leptos_watch_options.contains_key(option),
            _ => false,
        }
    }

    /// Check if an option is valid for dx serve (legacy method)
    pub fn is_valid_serve_option(&self, option: &str) -> bool {
        self.is_valid_option("dioxus", option)
    }

    /// Get option information for specified framework
    pub fn get_option_info(&self, framework: &str, option: &str) -> Option<&DioxusOptionInfo> {
        match framework {
            "dioxus" => self.dioxus_serve_options.get(option),
            "leptos" => self.leptos_watch_options.get(option),
            _ => None,
        }
    }

    /// Get all valid options for specified framework
    pub fn get_all_options(&self, framework: &str) -> Vec<&str> {
        match framework {
            "dioxus" => self.dioxus_serve_options.keys().copied().collect(),
            "leptos" => self.leptos_watch_options.keys().copied().collect(),
            _ => vec![],
        }
    }

    /// Convenience method for Dioxus validation
    pub fn validate_dioxus_option(&self, option: &str, value: Option<&str>) -> ValidationResult {
        self.validate_option("dioxus", option, value)
    }

    /// Convenience method for Leptos validation
    pub fn validate_leptos_option(&self, option: &str, value: Option<&str>) -> ValidationResult {
        self.validate_option("leptos", option, value)
    }
}

impl Default for DioxusValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate Levenshtein distance for typo suggestions
fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let len1 = s1.chars().count();
    let len2 = s2.chars().count();
    let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

    for (i, row) in matrix.iter_mut().enumerate().take(len1 + 1) {
        row[0] = i;
    }
    for j in 0..=len2 {
        matrix[0][j] = j;
    }

    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();

    for i in 1..=len1 {
        for j in 1..=len2 {
            let cost = if s1_chars[i - 1] == s2_chars[j - 1] {
                0
            } else {
                1
            };
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }

    matrix[len1][len2]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_dioxus_options() {
        let validator = FrameworkValidator::new();

        // Test flag options
        assert!(matches!(
            validator.validate_dioxus_option("--release", None),
            ValidationResult::Valid
        ));
        assert!(matches!(
            validator.validate_dioxus_option("--fullstack", None),
            ValidationResult::Valid
        ));

        // Test boolean options
        assert!(matches!(
            validator.validate_dioxus_option("--device", Some("true")),
            ValidationResult::Valid
        ));
        assert!(matches!(
            validator.validate_dioxus_option("--device", Some("false")),
            ValidationResult::Valid
        ));
        assert!(matches!(
            validator.validate_dioxus_option("--hot-reload", Some("true")),
            ValidationResult::Valid
        ));

        // Test port option
        assert!(matches!(
            validator.validate_dioxus_option("--port", Some("3000")),
            ValidationResult::Valid
        ));
        assert!(matches!(
            validator.validate_dioxus_option("--port", Some("8080")),
            ValidationResult::Valid
        ));

        // Test platform option
        assert!(matches!(
            validator.validate_dioxus_option("--platform", Some("web")),
            ValidationResult::Valid
        ));

        // Test architecture option
        assert!(matches!(
            validator.validate_dioxus_option("--arch", Some("x64")),
            ValidationResult::Valid
        ));
        assert!(matches!(
            validator.validate_dioxus_option("--arch", Some("arm64")),
            ValidationResult::Valid
        ));
    }

    #[test]
    fn test_valid_leptos_options() {
        let validator = FrameworkValidator::new();

        // Test flag options
        assert!(matches!(
            validator.validate_leptos_option("--release", None),
            ValidationResult::Valid
        ));
        assert!(matches!(
            validator.validate_leptos_option("--precompress", None),
            ValidationResult::Valid
        ));
        assert!(matches!(
            validator.validate_leptos_option("--hot-reload", None),
            ValidationResult::Valid
        ));

        // Test boolean options
        assert!(matches!(
            validator.validate_leptos_option("--js-minify", Some("true")),
            ValidationResult::Valid
        ));
        assert!(matches!(
            validator.validate_leptos_option("--js-minify", Some("false")),
            ValidationResult::Valid
        ));

        // Test string options
        assert!(matches!(
            validator.validate_leptos_option("--project", Some("my-project")),
            ValidationResult::Valid
        ));
        assert!(matches!(
            validator.validate_leptos_option("--features", Some("ssr,hydrate")),
            ValidationResult::Valid
        ));
    }

    #[test]
    fn test_invalid_device_value() {
        let validator = FrameworkValidator::new();

        match validator.validate_dioxus_option("--device", Some("example")) {
            ValidationResult::InvalidValue {
                expected,
                got,
                suggestions,
            } => {
                assert_eq!(expected, "true or false");
                assert_eq!(got, "example");
                assert!(!suggestions.is_empty());
            }
            _ => panic!("Expected InvalidValue result"),
        }
    }

    #[test]
    fn test_invalid_platform_value() {
        let validator = FrameworkValidator::new();

        match validator.validate_dioxus_option("--platform", Some("desktop")) {
            ValidationResult::InvalidValue {
                expected,
                got,
                suggestions,
            } => {
                assert!(expected.contains("web"));
                assert_eq!(got, "desktop");
                assert!(
                    suggestions
                        .iter()
                        .any(|s| s.contains("web") || s.contains("macos"))
                );
            }
            _ => panic!("Expected InvalidValue result"),
        }
    }

    #[test]
    fn test_typo_suggestions() {
        let validator = FrameworkValidator::new();

        match validator.validate_dioxus_option("--hot-reloads", None) {
            ValidationResult::InvalidOption { suggestion } => {
                assert_eq!(suggestion, Some("--hot-reload".to_string()));
            }
            _ => panic!("Expected InvalidOption result"),
        }
    }

    #[test]
    fn test_leptos_typo_suggestions() {
        let validator = FrameworkValidator::new();

        match validator.validate_leptos_option("--precompres", None) {
            ValidationResult::InvalidOption { suggestion } => {
                assert_eq!(suggestion, Some("--precompress".to_string()));
            }
            _ => panic!("Expected InvalidOption result"),
        }
    }

    #[test]
    fn test_port_validation() {
        let validator = FrameworkValidator::new();

        // Valid ports
        assert!(matches!(
            validator.validate_dioxus_option("--port", Some("3000")),
            ValidationResult::Valid
        ));
        assert!(matches!(
            validator.validate_dioxus_option("--port", Some("8080")),
            ValidationResult::Valid
        ));

        // Invalid ports
        match validator.validate_dioxus_option("--port", Some("0")) {
            ValidationResult::InvalidValue { .. } => {}
            _ => panic!("Expected InvalidValue for port 0"),
        }

        match validator.validate_dioxus_option("--port", Some("abc")) {
            ValidationResult::InvalidValue { .. } => {}
            _ => panic!("Expected InvalidValue for non-numeric port"),
        }
    }

    #[test]
    fn test_address_validation() {
        let validator = FrameworkValidator::new();

        // Valid addresses
        assert!(matches!(
            validator.validate_dioxus_option("--addr", Some("localhost")),
            ValidationResult::Valid
        ));
        assert!(matches!(
            validator.validate_dioxus_option("--addr", Some("127.0.0.1")),
            ValidationResult::Valid
        ));
        assert!(matches!(
            validator.validate_dioxus_option("--addr", Some("0.0.0.0")),
            ValidationResult::Valid
        ));

        // Invalid addresses would be caught by the validation logic
    }

    #[test]
    fn test_leptos_js_minify() {
        let validator = FrameworkValidator::new();

        // Valid boolean values
        assert!(matches!(
            validator.validate_leptos_option("--js-minify", Some("true")),
            ValidationResult::Valid
        ));
        assert!(matches!(
            validator.validate_leptos_option("--js-minify", Some("false")),
            ValidationResult::Valid
        ));

        // Invalid boolean value
        match validator.validate_leptos_option("--js-minify", Some("maybe")) {
            ValidationResult::InvalidValue {
                expected,
                got,
                suggestions,
            } => {
                assert_eq!(expected, "true or false");
                assert_eq!(got, "maybe");
                assert!(!suggestions.is_empty());
            }
            _ => panic!("Expected InvalidValue result"),
        }
    }
}
