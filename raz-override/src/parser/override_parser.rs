//! Override parser for categorizing user input
//!
//! This module parses user input from command overrides and categorizes
//! them into environment variables, cargo options, and additional arguments.

use crate::error::{OverrideError, Result};
use raz_common::env::{EnvParser, is_valid_env_var_name};
use raz_config::{CommandOverride, OverrideMode};
use raz_validation::{ValidationConfig, ValidationEngine};
use std::collections::HashMap;

/// Option value types for command overrides
#[derive(Debug, Clone, PartialEq)]
pub enum OptionValue {
    /// Boolean flag (e.g., --release)
    Flag(bool),
    /// Single value (e.g., --bin myapp)
    Single(String),
    /// Multiple values (e.g., --features "a,b,c")
    Multiple(Vec<String>),
}

/// Parsed override components
#[derive(Debug, Clone)]
pub struct ParsedOverride {
    /// Environment variables to add
    pub env: HashMap<String, String>,
    /// Environment variables to remove
    pub env_removals: Vec<String>,
    /// Cargo options to add/set
    pub options: HashMap<String, OptionValue>,
    /// Cargo options to remove
    pub option_removals: Vec<String>,
    /// Additional arguments (after --)
    pub extra_args: Vec<String>,
    /// Arguments to remove
    pub arg_removals: Vec<String>,
    /// Raw unparsed tokens (for fallback)
    pub raw_tokens: Vec<String>,
    /// Reset flags
    pub reset_all: bool,
    pub reset_env: bool,
    pub reset_options: bool,
    pub reset_args: bool,
}

impl Default for ParsedOverride {
    fn default() -> Self {
        Self::new()
    }
}

impl ParsedOverride {
    /// Create a new empty parsed override
    pub fn new() -> Self {
        Self {
            env: HashMap::new(),
            env_removals: Vec::new(),
            options: HashMap::new(),
            option_removals: Vec::new(),
            extra_args: Vec::new(),
            arg_removals: Vec::new(),
            raw_tokens: Vec::new(),
            reset_all: false,
            reset_env: false,
            reset_options: false,
            reset_args: false,
        }
    }

    /// Convert to CommandOverride
    pub fn to_command_override(&self, append_mode: bool) -> CommandOverride {
        let mut override_cmd = CommandOverride::new(String::new());

        // Set mode based on append_mode
        override_cmd.mode = if append_mode {
            OverrideMode::Append
        } else {
            OverrideMode::Replace
        };

        // Convert env to IndexMap
        for (key, value) in &self.env {
            override_cmd.env.insert(key.clone(), value.clone());
        }

        // Convert options to cargo_options and rustc_options
        for (key, value) in &self.options {
            let option_str = match value {
                OptionValue::Flag(true) => key.clone(),
                OptionValue::Flag(false) => continue, // Skip false flags
                OptionValue::Single(val) => format!("{key} {val}"),
                OptionValue::Multiple(vals) => format!("{} {}", key, vals.join(" ")),
            };

            // Determine if it's a cargo or rustc option
            if key.starts_with("--") || key.starts_with("-") {
                override_cmd.cargo_options.push(option_str);
            }
        }

        // Add extra args
        override_cmd.args = self.extra_args.clone();

        override_cmd
    }
}

/// Override parser
pub struct OverrideParser {
    command: String,
    validation_engine: ValidationEngine,
}

impl OverrideParser {
    /// Create a new parser for a specific command
    pub fn new(command: &str) -> Self {
        Self {
            command: command.to_string(),
            validation_engine: ValidationEngine::new(),
        }
    }

    /// Create a new parser with custom validation config
    pub fn with_validation_config(command: &str, config: ValidationConfig) -> Self {
        Self {
            command: command.to_string(),
            validation_engine: ValidationEngine::with_config(config),
        }
    }

    /// Parse user input into categorized components
    pub fn parse(&self, input: &str) -> Result<ParsedOverride> {
        self.parse_with_mode(input).map(|(parsed, _)| parsed)
    }

    /// Parse user input and return both parsed override and append mode
    pub fn parse_with_mode(&self, input: &str) -> Result<(ParsedOverride, bool)> {
        let mut result = ParsedOverride::new();

        // Check for append mode prefix
        let (append_mode, input) = if let Some(stripped) = input.strip_prefix("+ ") {
            (true, stripped)
        } else {
            (false, input)
        };

        // Tokenize input
        let tokens = self.tokenize(input)?;
        result.raw_tokens = tokens.clone();

        let mut i = 0;
        let mut after_double_dash = false;

        while i < tokens.len() {
            let token = &tokens[i];

            if token == "--" {
                after_double_dash = true;
                i += 1;
                continue;
            }

            // Handle reset commands first
            if self.handle_reset_command(token, &mut result) {
                i += 1;
                continue;
            }

            if after_double_dash {
                // Everything after -- is extra args
                self.handle_arg_token(token, &mut result)?;
            } else {
                // Parse different token types
                if let Some(consumed) = self.handle_env_token(token, &mut result)? {
                    i += consumed - 1;
                } else if let Some(consumed) = self.handle_option_token(&tokens, i, &mut result)? {
                    i += consumed - 1;
                } else {
                    // Unknown token - add to extra args
                    result.extra_args.push(token.clone());
                }
            }

            i += 1;
        }

        Ok((result, append_mode))
    }

    /// Tokenize input respecting quotes and escapes
    fn tokenize(&self, input: &str) -> Result<Vec<String>> {
        let mut tokens = Vec::new();
        let mut current = String::new();
        let mut in_quote = false;
        let mut quote_char = ' ';
        let mut chars = input.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                '"' | '\'' if !in_quote => {
                    in_quote = true;
                    quote_char = ch;
                }
                '"' | '\'' if in_quote && ch == quote_char => {
                    in_quote = false;
                    if !current.is_empty() {
                        tokens.push(current.clone());
                        current.clear();
                    }
                }
                '\\' if chars.peek().is_some() => {
                    // Escape next character
                    if let Some(next) = chars.next() {
                        current.push(next);
                    }
                }
                ' ' | '\t' if !in_quote => {
                    if !current.is_empty() {
                        tokens.push(current.clone());
                        current.clear();
                    }
                }
                _ => {
                    current.push(ch);
                }
            }
        }

        if in_quote {
            return Err(OverrideError::ParseError(format!(
                "Unclosed quote in input: {input}"
            )));
        }

        if !current.is_empty() {
            tokens.push(current);
        }

        Ok(tokens)
    }

    /// Check if a token is an environment variable
    /// Environment variables must be SCREAMING_CAPS with optional underscores
    fn is_env_var(&self, token: &str) -> bool {
        if !token.contains('=') || token.starts_with('-') {
            return false;
        }

        let var_name = token.split('=').next().unwrap();
        // For raz, we enforce SCREAMING_CAPS convention
        is_valid_env_var_name(var_name)
            && var_name
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
    }

    /// Parse environment variable
    fn parse_env_var(&self, token: &str) -> Result<(String, String)> {
        EnvParser::parse_assignment(token).map_err(|e| OverrideError::ParseError(e.to_string()))
    }

    /// Parse a command option - simplified version without cargo options catalog
    fn parse_option(
        &self,
        tokens: &[String],
        index: usize,
    ) -> Result<(String, OptionValue, usize)> {
        let option = &tokens[index];

        // Simple heuristic: if next token exists and doesn't start with -, it's a value
        if index + 1 < tokens.len() && !tokens[index + 1].starts_with('-') {
            // Check if it looks like multiple values (contains comma)
            if tokens[index + 1].contains(',') {
                let values: Vec<String> = tokens[index + 1]
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect();
                Ok((option.to_string(), OptionValue::Multiple(values), 2))
            } else {
                Ok((
                    option.to_string(),
                    OptionValue::Single(tokens[index + 1].clone()),
                    2,
                ))
            }
        } else {
            // No value, treat as flag
            Ok((option.to_string(), OptionValue::Flag(true), 1))
        }
    }

    /// Handle reset commands (!!,!e,!env,!o,!options,!a,!args) and removal commands (!--option, !ENV)
    fn handle_reset_command(&self, token: &str, result: &mut ParsedOverride) -> bool {
        match token {
            "!!" => {
                result.reset_all = true;
                true
            }
            "!e" | "!env" => {
                result.reset_env = true;
                true
            }
            "!o" | "!options" => {
                result.reset_options = true;
                true
            }
            "!a" | "!args" => {
                result.reset_args = true;
                true
            }
            _ => {
                // Check for removal syntax: !--option or !ENV
                if token.starts_with('!') && token.len() > 1 {
                    let removal_part = &token[1..];

                    if removal_part.starts_with("--") {
                        // Remove option: !--option
                        result.option_removals.push(removal_part.to_string());
                        return true;
                    } else if removal_part.contains('=') {
                        // Remove environment variable with value: !ENV=value
                        let env_name = removal_part.split('=').next().unwrap();
                        if env_name
                            .chars()
                            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
                        {
                            result.env_removals.push(env_name.to_string());
                            return true;
                        }
                    } else if removal_part
                        .chars()
                        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
                    {
                        // Remove environment variable: !ENV
                        result.env_removals.push(removal_part.to_string());
                        return true;
                    }
                }
                false
            }
        }
    }

    /// Handle environment variable tokens (+ENV=value, -ENV, ENV=value)
    fn handle_env_token(&self, token: &str, result: &mut ParsedOverride) -> Result<Option<usize>> {
        if token.starts_with('+') && self.is_env_var(&token[1..]) {
            // Add environment variable: +ENV=value
            let (key, value) = self.parse_env_var(&token[1..])?;
            result.env.insert(key, value);
            Ok(Some(1))
        } else if token.starts_with('-') && !token.starts_with("--") {
            // Remove environment variable: -ENV or -ENV=value
            let env_part = &token[1..];
            if env_part.contains('=') {
                let (key, _) = self.parse_env_var(env_part)?;
                result.env_removals.push(key);
            } else if env_part
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
            {
                result.env_removals.push(env_part.to_string());
            } else {
                return Ok(None); // Not an env var
            }
            Ok(Some(1))
        } else if self.is_env_var(token) {
            // Regular environment variable: ENV=value
            let (key, value) = self.parse_env_var(token)?;
            result.env.insert(key, value);
            Ok(Some(1))
        } else {
            Ok(None)
        }
    }

    /// Handle option tokens (+--option, ---option, --option)
    fn handle_option_token(
        &self,
        tokens: &[String],
        index: usize,
        result: &mut ParsedOverride,
    ) -> Result<Option<usize>> {
        let token = &tokens[index];

        if token.starts_with('+') && token.len() > 3 && token[1..].starts_with("--") {
            // Add option: +--option or +--option value
            let option_part = &token[1..];
            let (option, value, consumed) =
                self.parse_option_at_index(tokens, index, option_part)?;
            result.options.insert(option, value);
            Ok(Some(consumed))
        } else if token.starts_with("---") && token.len() > 3 {
            // Remove option: ---option
            let option_to_remove = &token[1..]; // Keep the -- prefix
            result.option_removals.push(option_to_remove.to_string());
            Ok(Some(1))
        } else if token.starts_with("--") {
            // Regular option: --option
            let (option, value, consumed) = self.parse_option(tokens, index)?;
            result.options.insert(option, value);
            Ok(Some(consumed))
        } else {
            Ok(None)
        }
    }

    /// Handle argument tokens (+arg, -arg)
    fn handle_arg_token(&self, token: &str, result: &mut ParsedOverride) -> Result<()> {
        if let Some(stripped) = token.strip_prefix('+') {
            // Add argument: +arg
            result.extra_args.push(stripped.to_string());
        } else if let Some(stripped) = token.strip_prefix('-') {
            if !token.starts_with("--") {
                // Remove argument: -arg
                result.arg_removals.push(stripped.to_string());
            } else {
                // Regular argument
                result.extra_args.push(token.to_string());
            }
        } else {
            // Regular argument
            result.extra_args.push(token.to_string());
        }
        Ok(())
    }

    /// Parse option at specific index with custom option string
    fn parse_option_at_index(
        &self,
        tokens: &[String],
        index: usize,
        option: &str,
    ) -> Result<(String, OptionValue, usize)> {
        // Simple heuristic: if next token exists and doesn't start with -, it's a value
        if index + 1 < tokens.len() && !tokens[index + 1].starts_with('-') {
            // Check if it looks like multiple values (contains comma)
            if tokens[index + 1].contains(',') {
                let values: Vec<String> = tokens[index + 1]
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect();
                Ok((option.to_string(), OptionValue::Multiple(values), 2))
            } else {
                Ok((
                    option.to_string(),
                    OptionValue::Single(tokens[index + 1].clone()),
                    2,
                ))
            }
        } else {
            // No value, treat as flag
            Ok((option.to_string(), OptionValue::Flag(true), 1))
        }
    }

    /// Validate parsed override using the validation engine
    pub fn validate(&self, parsed: &ParsedOverride) -> Result<()> {
        // Validate each option using the validation engine
        for (option, value) in &parsed.options {
            let joined_values;
            let option_value = match value {
                OptionValue::Flag(_) => None,
                OptionValue::Single(val) => Some(val.as_str()),
                OptionValue::Multiple(vals) => {
                    joined_values = vals.join(",");
                    Some(joined_values.as_str())
                }
            };

            if let Err(validation_error) =
                self.validation_engine
                    .validate_option(&self.command, option, option_value)
            {
                return Err(OverrideError::ValidationError(validation_error.to_string()));
            }
        }

        // Convert options to HashMap for conflict checking
        let option_map: HashMap<String, Option<String>> = parsed
            .options
            .iter()
            .map(|(key, value)| {
                let val = match value {
                    OptionValue::Flag(_) => None,
                    OptionValue::Single(val) => Some(val.clone()),
                    OptionValue::Multiple(vals) => Some(vals.join(",")),
                };
                (key.clone(), val)
            })
            .collect();

        // Validate conflicts using the validation engine
        if let Err(validation_error) = self
            .validation_engine
            .validate_options(&self.command, &option_map)
        {
            return Err(OverrideError::ValidationError(validation_error.to_string()));
        }

        Ok(())
    }

    /// Get suggestions for a misspelled option
    pub fn suggest_option(&self, option: &str) -> Vec<String> {
        self.validation_engine.suggest_option(&self.command, option)
    }
}

/// Convenience function to parse override input
pub fn parse_override(command: &str, input: &str) -> Result<ParsedOverride> {
    let parser = OverrideParser::new(command);
    let parsed = parser.parse(input)?;
    parser.validate(&parsed)?;
    Ok(parsed)
}

/// Parse override input and convert to CommandOverride
pub fn parse_override_to_command(command: &str, input: &str) -> Result<CommandOverride> {
    let parser = OverrideParser::new(command);
    let (parsed, append_mode) = parser.parse_with_mode(input)?;
    parser.validate(&parsed)?;
    Ok(parsed.to_command_override(append_mode))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize() {
        let parser = OverrideParser::new("test");

        // Basic tokenization
        let tokens = parser.tokenize("--features foo bar").unwrap();
        assert_eq!(tokens, vec!["--features", "foo", "bar"]);

        // Quoted strings
        let tokens = parser.tokenize("--features \"foo bar\" baz").unwrap();
        assert_eq!(tokens, vec!["--features", "foo bar", "baz"]);

        // Mixed quotes
        let tokens = parser
            .tokenize("--name 'my app' --desc \"hello world\"")
            .unwrap();
        assert_eq!(tokens, vec!["--name", "my app", "--desc", "hello world"]);

        // Escaped characters
        let tokens = parser
            .tokenize(r#"--path /home/user\ with\ spaces"#)
            .unwrap();
        assert_eq!(tokens, vec!["--path", "/home/user with spaces"]);
    }

    #[test]
    fn test_parse_env_vars() {
        let parser = OverrideParser::new("run");

        // Valid environment variables (SCREAMING_CAPS)
        let parsed = parser
            .parse("RUST_LOG=debug FOO_BAR=bar SOME123=value --release")
            .unwrap();
        assert_eq!(parsed.env.get("RUST_LOG"), Some(&"debug".to_string()));
        assert_eq!(parsed.env.get("FOO_BAR"), Some(&"bar".to_string()));
        assert_eq!(parsed.env.get("SOME123"), Some(&"value".to_string()));
        assert!(parsed.options.contains_key("--release"));

        // Invalid environment variables (should be treated as args)
        let parsed = parser
            .parse("lowercase=value MixedCase=value 123INVALID=value")
            .unwrap();
        assert!(parsed.env.is_empty());
        assert_eq!(parsed.extra_args.len(), 3);
    }

    #[test]
    fn test_parse_args_after_double_dash() {
        let parser = OverrideParser::new("test");

        // Test with -- separator
        let parsed = parser
            .parse("--release --features foo -- --nocapture --test-threads=1")
            .unwrap();

        // Options before -- should be in options
        assert!(parsed.options.contains_key("--release"));
        assert!(parsed.options.contains_key("--features"));
        if let Some(OptionValue::Single(val)) = parsed.options.get("--features") {
            assert_eq!(val, "foo");
        } else {
            panic!("Expected --features to have value 'foo'");
        }

        // Everything after -- should be in extra_args
        assert_eq!(parsed.extra_args, vec!["--nocapture", "--test-threads=1"]);

        // Test without -- separator (args go to options or extra_args based on format)
        let parsed = parser.parse("--release --nocapture").unwrap();
        assert!(parsed.options.contains_key("--release"));
        assert!(parsed.options.contains_key("--nocapture"));
        assert!(parsed.extra_args.is_empty());
    }

    #[test]
    fn test_parse_mixed_content_with_double_dash() {
        let parser = OverrideParser::new("test");

        // Complex case with env vars, options, and args after --
        let parsed = parser
            .parse("RUST_LOG=debug --release --bin myapp -- --nocapture arg1 arg2")
            .unwrap();

        // Check environment variables
        assert_eq!(parsed.env.get("RUST_LOG"), Some(&"debug".to_string()));

        // Check options (before --)
        assert!(parsed.options.contains_key("--release"));
        assert!(parsed.options.contains_key("--bin"));
        if let Some(OptionValue::Single(val)) = parsed.options.get("--bin") {
            assert_eq!(val, "myapp");
        }

        // Check args (after --)
        assert_eq!(parsed.extra_args, vec!["--nocapture", "arg1", "arg2"]);
    }
}
