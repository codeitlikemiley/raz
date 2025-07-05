use crate::error::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Context for generating an override key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionContext {
    /// The file path relative to workspace root
    pub file_path: PathBuf,
    /// The function name if known
    pub function_name: Option<String>,
    /// The line number in the file
    pub line_number: usize,
    /// Additional context for disambiguation
    pub context: Option<String>,
}

/// A stable override key that can survive code refactoring
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OverrideKey {
    /// Primary key based on function signature
    pub primary: String,
    /// Fallback keys for resolution
    pub fallbacks: Vec<String>,
    /// Human-readable representation
    pub display: String,
}

impl OverrideKey {
    /// Create a new override key from function context
    pub fn new(context: &FunctionContext) -> Result<Self> {
        let mut fallbacks = Vec::new();

        // Generate primary key based on function name if available
        let primary = if let Some(ref func_name) = context.function_name {
            // Primary: file_path:function_name
            let key = format!("{}:{}", Self::normalize_path(&context.file_path), func_name);

            // Add fallback with line context
            fallbacks.push(format!(
                "{}:{}:L{}",
                Self::normalize_path(&context.file_path),
                func_name,
                context.line_number
            ));

            key
        } else {
            // No function name, use file + line as primary
            format!(
                "{}:L{}",
                Self::normalize_path(&context.file_path),
                context.line_number
            )
        };

        // Add hash-based fallback for extreme cases
        let hash_key = Self::generate_hash_key(context);
        fallbacks.push(hash_key);

        // Generate display string
        let display = if let Some(ref func_name) = context.function_name {
            format!("{} in {}", func_name, context.file_path.display())
        } else {
            format!("{}:{}", context.file_path.display(), context.line_number)
        };

        Ok(Self {
            primary,
            fallbacks,
            display,
        })
    }

    /// Normalize a path for consistent key generation
    fn normalize_path(path: &Path) -> String {
        path.to_string_lossy()
            .replace('\\', "/")
            .trim_start_matches("./")
            .to_string()
    }

    /// Generate a hash-based key as ultimate fallback
    fn generate_hash_key(context: &FunctionContext) -> String {
        let mut hasher = Sha256::new();
        hasher.update(Self::normalize_path(&context.file_path).as_bytes());

        if let Some(ref func_name) = context.function_name {
            hasher.update(b":");
            hasher.update(func_name.as_bytes());
        }

        if let Some(ref ctx) = context.context {
            hasher.update(b":");
            hasher.update(ctx.as_bytes());
        }

        let hash = hasher.finalize();
        format!("hash:{}", hex::encode(&hash[..8]))
    }

    /// Check if this key matches a given pattern
    pub fn matches(&self, pattern: &str) -> bool {
        self.primary == pattern
            || self.fallbacks.iter().any(|k| k == pattern)
            || self.display.contains(pattern)
    }

    /// Get all possible keys for resolution
    pub fn all_keys(&self) -> Vec<&str> {
        let mut keys = vec![self.primary.as_str()];
        keys.extend(self.fallbacks.iter().map(|s| s.as_str()));
        keys
    }
}

impl std::fmt::Display for OverrideKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_override_key_with_function() {
        let context = FunctionContext {
            file_path: PathBuf::from("src/main.rs"),
            function_name: Some("handle_request".to_string()),
            line_number: 42,
            context: None,
        };

        let key = OverrideKey::new(&context).unwrap();
        assert_eq!(key.primary, "src/main.rs:handle_request");
        assert!(
            key.fallbacks
                .contains(&"src/main.rs:handle_request:L42".to_string())
        );
        assert_eq!(key.display, "handle_request in src/main.rs");
    }

    #[test]
    fn test_override_key_without_function() {
        let context = FunctionContext {
            file_path: PathBuf::from("src/lib.rs"),
            function_name: None,
            line_number: 100,
            context: None,
        };

        let key = OverrideKey::new(&context).unwrap();
        assert_eq!(key.primary, "src/lib.rs:L100");
        assert_eq!(key.display, "src/lib.rs:100");
    }

    #[test]
    fn test_path_normalization() {
        let context = FunctionContext {
            file_path: PathBuf::from("./src\\module\\file.rs"),
            function_name: Some("test".to_string()),
            line_number: 1,
            context: None,
        };

        let key = OverrideKey::new(&context).unwrap();
        assert!(key.primary.starts_with("src/module/file.rs"));
    }
}
