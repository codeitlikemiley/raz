//! Smart override parser for simplified CLI syntax
//!
//! Parses user input without requiring --env, --options, --args flags

use regex::Regex;
use std::collections::HashMap;

/// Parsed override components
#[derive(Debug, Clone, Default)]
pub struct ParsedOverrides {
    /// Environment variables
    pub env_vars: HashMap<String, String>,
    /// Command options (before --)
    pub options: Vec<String>,
    /// Arguments (after --)
    pub args: Vec<String>,
    /// Raw input for debugging
    pub raw_input: String,
}

impl ParsedOverrides {
    /// Check if the overrides are empty
    pub fn is_empty(&self) -> bool {
        self.env_vars.is_empty() && self.options.is_empty() && self.args.is_empty()
    }
}

/// Override operator for VS Code syntax
#[derive(Debug, Clone, PartialEq)]
pub enum OverrideOperator {
    /// Default: replace existing
    Replace,
    /// +: Add/merge with existing
    Add,
    /// -: Remove from existing
    Remove,
    /// !: Force replace (override conflicts)
    Force,
}

/// Parse operator prefix from option
fn parse_operator(option: &str) -> (OverrideOperator, &str) {
    if option.starts_with("+--") {
        (OverrideOperator::Add, &option[1..])
    } else if option.starts_with("---") {
        (OverrideOperator::Remove, &option[1..])
    } else if option.starts_with("!--") {
        (OverrideOperator::Force, &option[1..])
    } else {
        (OverrideOperator::Replace, option)
    }
}

/// Smart override parser
pub struct SmartOverrideParser {
    /// Regex for environment variables
    env_regex: Regex,
    /// Current cargo subcommand context
    #[allow(dead_code)]
    subcommand: String,
}

impl SmartOverrideParser {
    /// Create a new parser with the given cargo subcommand context
    pub fn new(subcommand: impl Into<String>) -> Self {
        Self {
            // Match SCREAMING_CASE=value or PascalCase=value
            env_regex: Regex::new(r#"^([A-Z][A-Z0-9_]*|[A-Z][a-zA-Z0-9]*)=(.+)$"#).unwrap(),
            subcommand: subcommand.into(),
        }
    }

    /// Parse raw input string into override components
    pub fn parse(&self, input: &str) -> ParsedOverrides {
        let mut result = ParsedOverrides {
            raw_input: input.to_string(),
            ..Default::default()
        };

        // First, split by -- to separate options from args
        // Special case: if input starts with "-- ", everything is args
        if let Some(stripped) = input.strip_prefix("-- ") {
            result.args = shell_words::split(stripped)
                .unwrap_or_else(|_| stripped.split_whitespace().map(String::from).collect());
            return result;
        }

        let parts: Vec<&str> = input.splitn(2, " -- ").collect();
        let before_dash = parts[0];
        let after_dash = parts.get(1);

        // Parse everything after --
        if let Some(args_str) = after_dash {
            result.args = shell_words::split(args_str).unwrap_or_else(|_| {
                // Fallback to simple split if shell_words fails
                args_str.split_whitespace().map(String::from).collect()
            });
        }

        // Parse everything before --
        let tokens = shell_words::split(before_dash).unwrap_or_else(|_| {
            // Fallback to simple split if shell_words fails
            before_dash.split_whitespace().map(String::from).collect()
        });

        let mut i = 0;
        while i < tokens.len() {
            let token = &tokens[i];

            // Check if it's an environment variable
            if let Some(captures) = self.env_regex.captures(token) {
                let key = captures.get(1).unwrap().as_str().to_string();
                let value = captures.get(2).unwrap().as_str().to_string();
                result.env_vars.insert(key, value);
                i += 1;
                continue;
            }

            // Check if it's an option (starts with -)
            let (_operator, clean_option) = parse_operator(token);

            if clean_option.starts_with("-") {
                // It's an option
                result.options.push(clean_option.to_string());

                // Check if next token might be its value
                if i + 1 < tokens.len() && !tokens[i + 1].starts_with("-") {
                    // Simple heuristic: if next token doesn't start with -, assume it's a value
                    // Skip obvious non-value patterns
                    let next_token = &tokens[i + 1];
                    if !self.env_regex.is_match(next_token) {
                        result.options.push(next_token.clone());
                        i += 1;
                    }
                }
                i += 1;
            } else {
                // Neither env var nor option - could be a positional argument
                // This shouldn't happen in override context, but handle gracefully
                result.options.push(token.clone());
                i += 1;
            }
        }

        result
    }

    /// Parse VS Code input with operator support
    pub fn parse_vscode_input(
        &self,
        input: &str,
    ) -> (ParsedOverrides, HashMap<String, OverrideOperator>) {
        let mut operators = HashMap::new();
        let mut result = ParsedOverrides {
            raw_input: input.to_string(),
            ..Default::default()
        };

        // Split by -- first
        let parts: Vec<&str> = input.splitn(2, " -- ").collect();
        let before_dash = parts[0];
        let after_dash = parts.get(1);

        // Parse args after --
        if let Some(args_str) = after_dash {
            result.args = shell_words::split(args_str)
                .unwrap_or_else(|_| args_str.split_whitespace().map(String::from).collect());
        }

        // Parse tokens before --
        let tokens = shell_words::split(before_dash)
            .unwrap_or_else(|_| before_dash.split_whitespace().map(String::from).collect());

        let mut i = 0;
        while i < tokens.len() {
            let token = &tokens[i];

            // Check for environment variable
            if let Some(captures) = self.env_regex.captures(token) {
                let key = captures.get(1).unwrap().as_str().to_string();
                let value = captures.get(2).unwrap().as_str().to_string();
                result.env_vars.insert(key, value);
                i += 1;
                continue;
            }

            // Parse operator and option
            let (operator, clean_option) = parse_operator(token);

            // Store operator for this option
            if operator != OverrideOperator::Replace {
                operators.insert(clean_option.to_string(), operator.clone());
            }

            // Handle remove operator by converting to --no- prefix
            let final_option =
                if operator == OverrideOperator::Remove && clean_option.starts_with("--") {
                    format!("--no-{}", &clean_option[2..])
                } else {
                    clean_option.to_string()
                };

            // Add the option
            result.options.push(final_option);

            // Check for option value
            if i + 1 < tokens.len() && !tokens[i + 1].starts_with("-") {
                result.options.push(tokens[i + 1].clone());
                i += 2;
            } else {
                i += 1;
            }
        }

        (result, operators)
    }
}

/// Convert parsed overrides back to CLI arguments
pub fn overrides_to_cli_args(overrides: &ParsedOverrides) -> Vec<String> {
    let mut args = vec![];

    // Add environment variables
    for (key, value) in &overrides.env_vars {
        args.push(format!("--env={key}={value}"));
    }

    // Add options
    if !overrides.options.is_empty() {
        args.push(format!("--options={}", overrides.options.join(" ")));
    }

    // Add args
    if !overrides.args.is_empty() {
        args.push(format!("--args={}", overrides.args.join(" ")));
    }

    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_env() {
        let parser = SmartOverrideParser::new("run");
        let result = parser.parse("RUST_BACKTRACE=full");

        assert_eq!(
            result.env_vars.get("RUST_BACKTRACE"),
            Some(&"full".to_string())
        );
        assert!(result.options.is_empty());
        assert!(result.args.is_empty());
    }

    #[test]
    fn test_parse_quoted_env() {
        let parser = SmartOverrideParser::new("run");
        let result = parser.parse(r#"RUST_FLAGS="-A warnings""#);

        assert_eq!(
            result.env_vars.get("RUST_FLAGS"),
            Some(&"-A warnings".to_string())
        );
    }

    #[test]
    fn test_parse_options() {
        let parser = SmartOverrideParser::new("run");
        let result = parser.parse("--release --target aarch64-apple-darwin");

        assert_eq!(
            result.options,
            vec!["--release", "--target", "aarch64-apple-darwin"]
        );
        assert!(result.env_vars.is_empty());
        assert!(result.args.is_empty());
    }

    #[test]
    fn test_parse_with_args() {
        let parser = SmartOverrideParser::new("test");
        let result = parser.parse("--release -- --exact --nocapture");

        assert_eq!(result.options, vec!["--release"]);
        assert_eq!(result.args, vec!["--exact", "--nocapture"]);
    }

    #[test]
    fn test_parse_mixed() {
        let parser = SmartOverrideParser::new("run");
        let result = parser.parse("RUST_LOG=debug --release --features ssr -- --port 3000");

        assert_eq!(result.env_vars.get("RUST_LOG"), Some(&"debug".to_string()));
        assert_eq!(result.options, vec!["--release", "--features", "ssr"]);
        assert_eq!(result.args, vec!["--port", "3000"]);
    }

    #[test]
    fn test_parse_vscode_operators() {
        let parser = SmartOverrideParser::new("run");
        let (result, operators) = parser.parse_vscode_input("+--features new ---default-features");

        assert_eq!(
            result.options,
            vec!["--features", "new", "--no-default-features"]
        );
        assert_eq!(operators.get("--features"), Some(&OverrideOperator::Add));
        assert_eq!(
            operators.get("--default-features"),
            Some(&OverrideOperator::Remove)
        );
    }

    #[test]
    fn test_framework_options() {
        let parser = SmartOverrideParser::new("run");
        let result = parser.parse("--platform web --device true");

        // These are not cargo options, but should still be captured
        assert_eq!(
            result.options,
            vec!["--platform", "web", "--device", "true"]
        );
    }
}
