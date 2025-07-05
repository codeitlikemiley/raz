//! Parsing utilities for command-line arguments and options

use crate::error::{CommonError, Result};

/// Parse a command option string into parts, handling quoted arguments properly
///
/// # Examples
/// ```
/// use raz_common::parse::parse_option;
///
/// let parts = parse_option("--features foo bar").unwrap();
/// assert_eq!(parts, vec!["--features", "foo", "bar"]);
///
/// let parts = parse_option("--message \"hello world\"").unwrap();
/// assert_eq!(parts, vec!["--message", "hello world"]);
/// ```
pub fn parse_option(option: &str) -> Result<Vec<String>> {
    shell_words::split(option)
        .map_err(|e| CommonError::ShellParse(format!("Failed to parse option '{option}': {e}")))
}

/// Parse multiple space-separated options, each potentially containing quoted values
pub fn parse_options(options: &[String]) -> Result<Vec<String>> {
    let mut result = Vec::new();
    for option in options {
        result.extend(parse_option(option)?);
    }
    Ok(result)
}

/// Check if a command argument list contains a specific flag
pub fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

/// Extract the value for a flag from command arguments
/// Returns None if flag not found or if it has no value
pub fn get_flag_value(args: &[String], flag: &str) -> Option<String> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == flag {
            return iter.next().cloned();
        }
    }
    None
}

/// Remove a flag and its value from command arguments
pub fn remove_flag(args: &mut Vec<String>, flag: &str) -> Option<String> {
    if let Some(pos) = args.iter().position(|arg| arg == flag) {
        args.remove(pos);
        if pos < args.len() {
            Some(args.remove(pos))
        } else {
            None
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_option() {
        assert_eq!(
            parse_option("--features foo").unwrap(),
            vec!["--features", "foo"]
        );

        assert_eq!(
            parse_option("--message \"hello world\"").unwrap(),
            vec!["--message", "hello world"]
        );
    }

    #[test]
    fn test_has_flag() {
        let args = vec!["--release".to_string(), "--verbose".to_string()];
        assert!(has_flag(&args, "--release"));
        assert!(!has_flag(&args, "--debug"));
    }

    #[test]
    fn test_get_flag_value() {
        let args = vec![
            "--features".to_string(),
            "foo".to_string(),
            "--target".to_string(),
            "wasm32".to_string(),
        ];
        assert_eq!(get_flag_value(&args, "--features"), Some("foo".to_string()));
        assert_eq!(
            get_flag_value(&args, "--target"),
            Some("wasm32".to_string())
        );
        assert_eq!(get_flag_value(&args, "--missing"), None);
    }
}
