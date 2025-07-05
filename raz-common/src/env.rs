//! Environment variable parsing and manipulation utilities

use crate::error::{CommonError, Result};
use std::collections::HashMap;

/// Environment variable parser
pub struct EnvParser;

impl EnvParser {
    /// Parse a single environment variable assignment (KEY=value)
    pub fn parse_assignment(input: &str) -> Result<(String, String)> {
        let (key, value) = input
            .split_once('=')
            .ok_or_else(|| CommonError::ShellParse(format!("Invalid env var format: {input}")))?;

        if !is_valid_env_var_name(key) {
            return Err(CommonError::ShellParse(format!(
                "Invalid env var name: {key}"
            )));
        }

        Ok((key.to_string(), value.to_string()))
    }

    /// Parse multiple environment variable assignments from a string
    /// Handles quoted values and escaping
    pub fn parse_env_string(input: &str) -> Result<HashMap<String, String>> {
        let mut env_vars = HashMap::new();
        let parts = shell_words::split(input)
            .map_err(|e| CommonError::ShellParse(format!("Failed to parse env string: {e}")))?;

        for part in parts {
            if let Some((key, value)) = part.split_once('=') {
                if is_valid_env_var_name(key) {
                    env_vars.insert(key.to_string(), value.to_string());
                }
            }
        }

        Ok(env_vars)
    }

    /// Parse environment variables from command-line style input
    /// Separates env vars from other arguments
    pub fn extract_env_vars(args: &[String]) -> (HashMap<String, String>, Vec<String>) {
        let mut env_vars = HashMap::new();
        let mut remaining_args = Vec::new();

        for arg in args {
            if let Some((key, value)) = arg.split_once('=') {
                if is_valid_env_var_name(key) {
                    env_vars.insert(key.to_string(), value.to_string());
                } else {
                    remaining_args.push(arg.clone());
                }
            } else {
                remaining_args.push(arg.clone());
            }
        }

        (env_vars, remaining_args)
    }

    /// Merge environment variables, with the second map taking precedence
    pub fn merge_env_vars(
        base: HashMap<String, String>,
        overrides: HashMap<String, String>,
    ) -> HashMap<String, String> {
        let mut result = base;
        result.extend(overrides);
        result
    }

    /// Format environment variables for display
    pub fn format_env_vars(env_vars: &HashMap<String, String>) -> Vec<String> {
        let mut formatted: Vec<_> = env_vars
            .iter()
            .map(|(k, v)| format!("{}={}", k, shell_words::quote(v)))
            .collect();
        formatted.sort();
        formatted
    }
}

/// Check if a string is a valid environment variable name
pub fn is_valid_env_var_name(name: &str) -> bool {
    !name.is_empty()
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
}

/// Builder for environment variables
#[derive(Debug, Default, Clone)]
pub struct EnvBuilder {
    vars: HashMap<String, String>,
}

impl EnvBuilder {
    /// Create a new environment builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an environment variable
    pub fn set(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.vars.insert(key.into(), value.into());
        self
    }

    /// Add multiple environment variables
    pub fn extend(mut self, vars: HashMap<String, String>) -> Self {
        self.vars.extend(vars);
        self
    }

    /// Remove an environment variable
    pub fn unset(mut self, key: &str) -> Self {
        self.vars.remove(key);
        self
    }

    /// Build the environment variables
    pub fn build(self) -> HashMap<String, String> {
        self.vars
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_assignment() {
        let (key, value) = EnvParser::parse_assignment("FOO=bar").unwrap();
        assert_eq!(key, "FOO");
        assert_eq!(value, "bar");

        let (key, value) = EnvParser::parse_assignment("PATH=/usr/bin:/bin").unwrap();
        assert_eq!(key, "PATH");
        assert_eq!(value, "/usr/bin:/bin");

        assert!(EnvParser::parse_assignment("invalid").is_err());
        assert!(EnvParser::parse_assignment("123=value").is_err());
    }

    #[test]
    fn test_extract_env_vars() {
        let args = vec![
            "FOO=bar".to_string(),
            "--flag".to_string(),
            "VALUE=123".to_string(),
            "arg".to_string(),
        ];

        let (env_vars, remaining) = EnvParser::extract_env_vars(&args);

        assert_eq!(env_vars.len(), 2);
        assert_eq!(env_vars.get("FOO"), Some(&"bar".to_string()));
        assert_eq!(env_vars.get("VALUE"), Some(&"123".to_string()));

        assert_eq!(remaining, vec!["--flag", "arg"]);
    }

    #[test]
    fn test_env_builder() {
        let env = EnvBuilder::new()
            .set("FOO", "bar")
            .set("BAZ", "qux")
            .unset("BAZ")
            .build();

        assert_eq!(env.len(), 1);
        assert_eq!(env.get("FOO"), Some(&"bar".to_string()));
        assert_eq!(env.get("BAZ"), None);
    }
}
