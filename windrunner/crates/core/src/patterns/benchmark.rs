use crate::{
    error::Result,
    patterns::Pattern,
    types::{Runnable, RunnableKind, Scope, ScopeKind},
};
use std::path::Path;

pub struct BenchmarkPattern;

impl Pattern for BenchmarkPattern {
    fn detect(&self, scope: &Scope, _source: &str, file_path: &Path) -> Result<Option<Runnable>> {
        if let ScopeKind::Benchmark = scope.kind {
            if let Some(name) = &scope.name {
                let runnable = Runnable {
                    label: format!("Run benchmark '{name}'"),
                    scope: scope.clone(),
                    kind: RunnableKind::Benchmark {
                        bench_name: name.clone(),
                    },
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
