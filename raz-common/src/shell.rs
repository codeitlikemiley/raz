//! Shell command parsing and manipulation utilities

use crate::error::{CommonError, Result};
use shell_words;
use std::fmt;

/// Shell command builder and parser
#[derive(Debug, Clone, PartialEq)]
pub struct ShellCommand {
    /// The base command
    pub command: String,
    /// Command arguments
    pub args: Vec<String>,
}

impl ShellCommand {
    /// Create a new shell command
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            args: Vec::new(),
        }
    }

    /// Add an argument
    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Add multiple arguments
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    /// Parse a shell command string
    pub fn parse(input: &str) -> Result<Self> {
        let parts =
            shell_words::split(input).map_err(|e| CommonError::ShellParse(e.to_string()))?;

        if parts.is_empty() {
            return Err(CommonError::ShellParse("Empty command".to_string()));
        }

        Ok(Self {
            command: parts[0].clone(),
            args: parts[1..].to_vec(),
        })
    }

    /// Convert to a shell-escaped string
    pub fn to_shell_string(&self) -> String {
        let mut parts = vec![self.command.clone()];
        parts.extend(self.args.clone());
        shell_words::join(&parts)
    }

    /// Get all parts (command + args) as a vector
    pub fn parts(&self) -> Vec<&str> {
        let mut parts = vec![self.command.as_str()];
        parts.extend(self.args.iter().map(|s| s.as_str()));
        parts
    }
}

impl fmt::Display for ShellCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_shell_string())
    }
}

/// Parse shell-style key=value pairs
pub fn parse_env_vars(input: &str) -> Result<Vec<(String, String)>> {
    let mut env_vars = Vec::new();
    let parts = shell_words::split(input).map_err(|e| CommonError::ShellParse(e.to_string()))?;

    for part in parts {
        if let Some((key, value)) = part.split_once('=') {
            if is_valid_env_var_name(key) {
                env_vars.push((key.to_string(), value.to_string()));
            }
        }
    }

    Ok(env_vars)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_command_parse() {
        let cmd = ShellCommand::parse("cargo test --features foo").unwrap();
        assert_eq!(cmd.command, "cargo");
        assert_eq!(cmd.args, vec!["test", "--features", "foo"]);
    }

    #[test]
    fn test_shell_command_display() {
        let cmd = ShellCommand::new("echo").arg("hello world").arg("test");
        assert_eq!(cmd.to_string(), "echo 'hello world' test");
        assert_eq!(cmd.to_shell_string(), "echo 'hello world' test");
    }

    #[test]
    fn test_parse_env_vars() {
        let vars = parse_env_vars("FOO=bar BAZ='quoted value'").unwrap();
        assert_eq!(
            vars,
            vec![
                ("FOO".to_string(), "bar".to_string()),
                ("BAZ".to_string(), "quoted value".to_string()),
            ]
        );
    }
}
