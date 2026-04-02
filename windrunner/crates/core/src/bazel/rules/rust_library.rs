//! Handler for rust_library rules

use crate::bazel::{RuleCall, BazelTarget, BazelTargetKind, TargetAnalyzer};
use super::RuleHandler;

/// Handler for rust_library rules
pub struct RustLibraryHandler;

impl RuleHandler for RustLibraryHandler {
    fn can_handle(&self, rule_type: &str) -> bool {
        matches!(rule_type, "rust_library" | "rust_proc_macro" | "rust_shared_library" | "rust_static_library")
    }
    
    fn analyze(&self, rule: &RuleCall) -> Option<BazelTarget> {
        Some(BazelTarget {
            label: format!(":{}", rule.name),
            kind: BazelTargetKind::Library,
            name: rule.name.clone(),
            sources: TargetAnalyzer::extract_sources(&rule.attributes),
            dependencies: TargetAnalyzer::extract_dependencies(&rule.attributes),
            test_only: false,
            attributes: TargetAnalyzer::extract_attributes(&rule.attributes),
        })
    }
    
    fn is_runnable(&self) -> bool {
        false  // Libraries are not directly runnable
    }
}