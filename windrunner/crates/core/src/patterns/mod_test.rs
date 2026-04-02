use crate::{
    error::Result,
    patterns::Pattern,
    types::{Runnable, RunnableKind, Scope, ScopeKind},
};
use std::path::Path;

pub struct ModTestPattern;

impl Pattern for ModTestPattern {
    fn detect(&self, scope: &Scope, _source: &str, file_path: &Path) -> Result<Option<Runnable>> {
        if let ScopeKind::Module = scope.kind {
            // Check if this is a test module (named "tests" or has #[cfg(test)])
            if scope.name.as_deref() == Some("tests") {
                let runnable = Runnable {
                    label: format!("Run all tests in module '{}'", scope.name.as_ref().unwrap()),
                    scope: scope.clone(),
                    kind: RunnableKind::ModuleTests {
                        module_name: scope.name.as_ref().unwrap().clone(),
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
    fn test_mod_test_pattern() {
        let pattern = ModTestPattern;
        let scope = Scope {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 10,
                character: 0,
            },
            kind: ScopeKind::Module,
            name: Some("tests".to_string()),
        };

        let result = pattern.detect(&scope, "", Path::new("test.rs")).unwrap();
        assert!(result.is_some());

        let runnable = result.unwrap();
        assert_eq!(runnable.label, "Run all tests in module 'tests'");
        match &runnable.kind {
            RunnableKind::ModuleTests { module_name } => {
                assert_eq!(module_name, "tests");
            }
            _ => panic!("Expected ModuleTests runnable kind"),
        }
    }

    #[test]
    fn test_non_test_module_ignored() {
        let pattern = ModTestPattern;
        let scope = Scope {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 10,
                character: 0,
            },
            kind: ScopeKind::Module,
            name: Some("utils".to_string()),
        };

        let result = pattern.detect(&scope, "", Path::new("test.rs")).unwrap();
        assert!(result.is_none());
    }
}
