//! Output formatting utilities for consistent CLI output

use colored::{ColoredString, Colorize};
use std::fmt::Display;

/// Output formatter for consistent styling across the project
pub struct OutputFormatter;

impl OutputFormatter {
    /// Format a success message
    pub fn success(message: impl Display) -> ColoredString {
        format!("✓ {message}").green()
    }

    /// Format an error message
    pub fn error(message: impl Display) -> ColoredString {
        format!("✗ {message}").red()
    }

    /// Format a warning message
    pub fn warning(message: impl Display) -> ColoredString {
        format!("⚠ {message}").yellow()
    }

    /// Format an info message
    pub fn info(message: impl Display) -> ColoredString {
        format!("ℹ {message}").blue()
    }

    /// Format a debug message
    pub fn debug(message: impl Display) -> ColoredString {
        format!("🔍 {message}").dimmed()
    }

    /// Format a label (like "Info:", "Error:", etc.)
    pub fn label(label: impl Display) -> ColoredString {
        format!("{label}:").bold()
    }

    /// Format a command for display
    pub fn command(cmd: impl Display) -> ColoredString {
        format!("`{cmd}`").cyan()
    }

    /// Format a file path
    pub fn path(path: impl Display) -> ColoredString {
        path.to_string().yellow()
    }

    /// Format a key or identifier
    pub fn key(key: impl Display) -> ColoredString {
        key.to_string().magenta()
    }

    /// Format a dimmed/secondary message
    pub fn dim(message: impl Display) -> ColoredString {
        message.to_string().dimmed()
    }

    /// Create a progress indicator
    pub fn progress(current: usize, total: usize, message: impl Display) -> String {
        format!("[{current}/{total}] {message}")
    }

    /// Create a header with consistent formatting
    pub fn header(title: impl Display) -> String {
        let title_str = title.to_string();
        let line = "─".repeat(title_str.len() + 4);
        format!("{}\n  {}  \n{}", line, title_str.bold(), line)
    }
}

/// Trait for types that can be formatted with context
pub trait Contextual {
    /// Format with additional context
    fn with_context(&self, context: impl Display) -> String;
}

impl<T: Display> Contextual for T {
    fn with_context(&self, context: impl Display) -> String {
        format!("{context}: {self}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_formatter() {
        // Just test that methods don't panic
        let _ = OutputFormatter::success("Test");
        let _ = OutputFormatter::error("Test");
        let _ = OutputFormatter::warning("Test");
        let _ = OutputFormatter::info("Test");
        let _ = OutputFormatter::command("cargo test");
        let _ = OutputFormatter::path("/some/path");
        let _ = OutputFormatter::progress(1, 10, "Processing");
    }

    #[test]
    fn test_contextual() {
        let msg = "error occurred";
        let with_ctx = msg.with_context("File processing");
        assert_eq!(with_ctx, "File processing: error occurred");
    }
}
