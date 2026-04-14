use crate::{
    error::Result,
    patterns::Pattern,
    types::{Runnable, RunnableKind, Scope, ScopeKind},
};
use std::path::Path;

pub struct TestFnPattern;

impl Pattern for TestFnPattern {
    fn detect(&self, scope: &Scope, _source: &str, file_path: &Path) -> Result<Option<Runnable>> {
        if let ScopeKind::Test = scope.kind {
            if let Some(name) = &scope.name {
                let runnable = Runnable {
                    label: format!("Run test '{name}'"),
                    scope: scope.clone(),
                    kind: RunnableKind::Test {
                        test_name: name.clone(),
                        is_async: false, // TODO: detect async tests
                    },
                    module_path: String::new(), // Will be filled by module resolver
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
    use crate::types::Position;

    #[test]
    fn test_test_fn_pattern() {
        let pattern = TestFnPattern;
        let scope = Scope {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 5,
                character: 0,
            },
            kind: ScopeKind::Test,
            name: Some("test_example".to_string()),
        };

        let result = pattern.detect(&scope, "", Path::new("test.rs")).unwrap();
        assert!(result.is_some());

        let runnable = result.unwrap();
        assert_eq!(runnable.label, "Run test 'test_example'");
        match &runnable.kind {
            RunnableKind::Test { test_name, .. } => {
                assert_eq!(test_name, "test_example");
            }
            _ => panic!("Expected Test runnable kind"),
        }
    }
}
