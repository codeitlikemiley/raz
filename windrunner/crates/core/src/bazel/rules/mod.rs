//! Rule handlers for different Bazel rule types

mod rust_binary;
mod rust_test;
mod rust_test_suite;
mod rust_doc_test;
mod rust_benchmark;
mod rust_library;
mod cargo_build_script;

pub use rust_binary::RustBinaryHandler;
pub use rust_test::RustTestHandler;
pub use rust_test_suite::RustTestSuiteHandler;
pub use rust_doc_test::RustDocTestHandler;
pub use rust_benchmark::RustBenchmarkHandler;
pub use rust_library::RustLibraryHandler;
pub use cargo_build_script::CargoBuildScriptHandler;

use super::{RuleCall, BazelTarget};

/// Trait for handling specific rule types
pub trait RuleHandler: Send + Sync {
    /// Check if this handler can process the given rule type
    fn can_handle(&self, rule_type: &str) -> bool;
    
    /// Analyze a rule call and convert it to a BazelTarget
    fn analyze(&self, rule: &RuleCall) -> Option<BazelTarget>;
    
    /// Check if this rule type produces runnable targets
    fn is_runnable(&self) -> bool;
}