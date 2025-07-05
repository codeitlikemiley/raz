//! Common error types used across raz crates

use thiserror::Error;

/// Common error type for raz operations
#[derive(Error, Debug)]
pub enum CommonError {
    /// Shell command parsing error
    #[error("Failed to parse shell command: {0}")]
    ShellParse(String),

    /// Command execution error
    #[error("Command execution failed: {0}")]
    CommandExecution(String),

    /// File system operation error
    #[error("File system error: {0}")]
    FileSystem(#[from] std::io::Error),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Generic error with context
    #[error("{context}: {source}")]
    WithContext {
        context: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Other errors
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Common result type
pub type Result<T> = std::result::Result<T, CommonError>;

impl CommonError {
    /// Create an error with additional context
    pub fn with_context<E>(context: impl Into<String>, error: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::WithContext {
            context: context.into(),
            source: Box::new(error),
        }
    }

    /// Create a shell parse error
    pub fn shell_parse(msg: impl Into<String>) -> Self {
        Self::ShellParse(msg.into())
    }

    /// Create a command execution error
    pub fn command_execution(msg: impl Into<String>) -> Self {
        Self::CommandExecution(msg.into())
    }
}

/// Extension trait for adding context to Results
pub trait ErrorContext<T> {
    /// Add context to an error
    fn context(self, context: impl Into<String>) -> Result<T>;

    /// Add context with a closure (only called on error)
    fn with_context<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> String;
}

impl<T, E> ErrorContext<T> for std::result::Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn context(self, context: impl Into<String>) -> Result<T> {
        self.map_err(|e| CommonError::with_context(context, e))
    }

    fn with_context<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|e| CommonError::with_context(f(), e))
    }
}

/// Extension trait for adding context to Options
pub trait OptionContext<T> {
    /// Convert None to an error with context
    fn context(self, context: impl Into<String>) -> Result<T>;
}

impl<T> OptionContext<T> for Option<T> {
    fn context(self, context: impl Into<String>) -> Result<T> {
        self.ok_or_else(|| CommonError::Other(anyhow::anyhow!(context.into())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_context() {
        let result: std::result::Result<i32, std::io::Error> = Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file not found",
        ));

        let with_context = result.context("Failed to read config file");
        assert!(with_context.is_err());

        let err = with_context.unwrap_err();
        match err {
            CommonError::WithContext { context, .. } => {
                assert_eq!(context, "Failed to read config file");
            }
            _ => panic!("Expected WithContext error"),
        }
    }

    #[test]
    fn test_option_context() {
        let value: Option<i32> = None;
        let result = value.context("Value not found");

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, CommonError::Other(_)));
    }
}
