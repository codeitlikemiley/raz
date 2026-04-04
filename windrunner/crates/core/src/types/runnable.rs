use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::path::PathBuf;

use super::scope::{ExtendedScope, Scope};

/// Represents a runnable item in Rust code (test, benchmark, binary, etc.)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Runnable {
    pub label: String,
    pub scope: Scope,
    pub kind: RunnableKind,
    pub module_path: String,
    pub file_path: PathBuf,
    /// Extended scope information if available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extended_scope: Option<ExtendedScope>,
}

/// Different kinds of runnable items
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunnableKind {
    Test {
        test_name: String,
        is_async: bool,
    },
    DocTest {
        struct_or_module_name: String,
        method_name: Option<String>,
    },
    Benchmark {
        bench_name: String,
    },
    Binary {
        bin_name: Option<String>,
    },
    ModuleTests {
        module_name: String,
    },
    Standalone {
        has_tests: bool,
    },
    SingleFileScript {
        shebang: String,
    },
}

impl Runnable {
    /// Get function name from the runnable kind
    pub fn get_function_name(&self) -> Option<String> {
        match &self.kind {
            RunnableKind::Test { test_name, .. } => Some(test_name.clone()),
            RunnableKind::DocTest {
                struct_or_module_name,
                method_name,
            } => {
                if let Some(method) = method_name {
                    Some(format!("{}::{}", struct_or_module_name, method))
                } else {
                    Some(struct_or_module_name.clone())
                }
            }
            RunnableKind::Benchmark { bench_name } => Some(bench_name.clone()),
            RunnableKind::Binary { bin_name } => bin_name.clone(),
            RunnableKind::ModuleTests { module_name } => Some(module_name.clone()),
            _ => None,
        }
    }
}

/// Runnable with scoring information for prioritization
#[derive(Debug, Clone)]
pub struct RunnableWithScore {
    pub runnable: Runnable,
    pub range_size: u32,
    pub is_module_test: bool,
}

impl RunnableWithScore {
    pub fn new(runnable: Runnable) -> Self {
        // For doc tests, use the parent scope size if available for better comparison
        let range_size = if matches!(runnable.kind, RunnableKind::DocTest { .. }) {
            if let Some(ref extended) = runnable.extended_scope {
                extended.scope.end.line - extended.scope.start.line
            } else {
                runnable.scope.end.line - runnable.scope.start.line
            }
        } else {
            runnable.scope.end.line - runnable.scope.start.line
        };

        let is_module_test = matches!(runnable.kind, RunnableKind::ModuleTests { .. });
        Self {
            runnable,
            range_size,
            is_module_test,
        }
    }

    /// Get priority score for the runnable (lower is better)
    fn get_priority(&self) -> u32 {
        match &self.runnable.kind {
            // Specific tests have highest priority
            RunnableKind::Test { .. } => 0,
            RunnableKind::Benchmark { .. } => 0,
            // Method doc tests have higher priority than type doc tests
            RunnableKind::DocTest {
                method_name: Some(_),
                ..
            } => 1,
            // Type/impl doc tests
            RunnableKind::DocTest {
                method_name: None, ..
            } => 2,
            // Binary
            RunnableKind::Binary { .. } => 3,
            // Module tests have lowest priority
            RunnableKind::ModuleTests { .. } => 4,
            // Others
            _ => 5,
        }
    }
}

impl Ord for RunnableWithScore {
    fn cmp(&self, other: &Self) -> Ordering {
        // First compare by scope size - smaller scope is more specific and wins
        match self.range_size.cmp(&other.range_size) {
            Ordering::Equal => {
                // If same size, use priority (method doc test > impl doc test > struct doc test)
                self.get_priority().cmp(&other.get_priority())
            }
            other => other,
        }
    }
}

impl PartialOrd for RunnableWithScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for RunnableWithScore {
    fn eq(&self, other: &Self) -> bool {
        self.range_size == other.range_size && self.is_module_test == other.is_module_test
    }
}

impl Eq for RunnableWithScore {}
