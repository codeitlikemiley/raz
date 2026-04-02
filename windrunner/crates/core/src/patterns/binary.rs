use crate::{
    error::Result,
    patterns::Pattern,
    types::{Runnable, RunnableKind, Scope, ScopeKind},
};
use std::path::Path;

pub struct BinaryPattern;

impl Pattern for BinaryPattern {
    fn detect(&self, scope: &Scope, _source: &str, file_path: &Path) -> Result<Option<Runnable>> {
        if let ScopeKind::Function = scope.kind {
            if scope.name.as_deref() == Some("main") {
                // For src/main.rs or main.rs (when in src/), the binary name is the package name (handled later)
                // For src/bin/foo.rs, the binary name is "foo"
                let file_name = file_path
                    .file_name()
                    .and_then(|f| f.to_str())
                    .unwrap_or("");
                    
                let bin_name = if file_name == "main.rs" {
                    // main.rs always uses the package name as binary name
                    None // Will use package name
                } else {
                    // For src/bin/foo.rs, use the file stem as binary name
                    file_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_string())
                };

                let runnable = Runnable {
                    label: if let Some(ref name) = bin_name {
                        format!("Run binary '{name}'")
                    } else {
                        "Run main()".to_string()
                    },
                    scope: scope.clone(),
                    kind: RunnableKind::Binary { bin_name },
                    module_path: String::new(),
                    file_path: file_path.to_path_buf(),
                    extended_scope: None, // Will be filled by detector
                };
                return Ok(Some(runnable));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Position, ScopeKind};

    #[test]
    fn test_main_rs_detection() {
        let pattern = BinaryPattern;
        
        // Test main.rs without path
        let scope = Scope {
            start: Position::new(0, 0),
            end: Position::new(10, 0),
            kind: ScopeKind::Function,
            name: Some("main".to_string()),
        };
        
        let result = pattern.detect(&scope, "", Path::new("main.rs")).unwrap();
        assert!(result.is_some());
        let runnable = result.unwrap();
        match runnable.kind {
            RunnableKind::Binary { bin_name } => {
                assert_eq!(bin_name, None, "main.rs should have None as bin_name");
            }
            _ => panic!("Expected Binary runnable"),
        }
        
        // Test src/main.rs
        let result = pattern.detect(&scope, "", Path::new("src/main.rs")).unwrap();
        assert!(result.is_some());
        let runnable = result.unwrap();
        match runnable.kind {
            RunnableKind::Binary { bin_name } => {
                assert_eq!(bin_name, None, "src/main.rs should have None as bin_name");
            }
            _ => panic!("Expected Binary runnable"),
        }
        
        // Test src/bin/foo.rs
        let result = pattern.detect(&scope, "", Path::new("src/bin/foo.rs")).unwrap();
        assert!(result.is_some());
        let runnable = result.unwrap();
        match runnable.kind {
            RunnableKind::Binary { bin_name } => {
                assert_eq!(bin_name, Some("foo".to_string()), "src/bin/foo.rs should have 'foo' as bin_name");
            }
            _ => panic!("Expected Binary runnable"),
        }
    }
}
