use crate::{
    error::Result,
    patterns::Pattern,
    types::{Runnable, Scope, ScopeKind},
};
use std::path::Path;

pub struct DocTestPattern;

impl Pattern for DocTestPattern {
    fn detect(&self, scope: &Scope, _source: &str, _file_path: &Path) -> Result<Option<Runnable>> {
        if let ScopeKind::DocTest = scope.kind {
            // Doc test detection is handled by the detector module
            // This pattern exists for completeness but returns None
        }
        Ok(None)
    }
}
