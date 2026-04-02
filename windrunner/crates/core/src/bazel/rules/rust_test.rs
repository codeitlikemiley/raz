//! Handler for rust_test rules

use crate::bazel::{RuleCall, BazelTarget, BazelTargetKind, TargetAnalyzer};
use super::RuleHandler;

/// Handler for rust_test rules
pub struct RustTestHandler;

impl RuleHandler for RustTestHandler {
    fn can_handle(&self, rule_type: &str) -> bool {
        rule_type == "rust_test"
    }
    
    fn analyze(&self, rule: &RuleCall) -> Option<BazelTarget> {
        Some(BazelTarget {
            label: format!(":{}", rule.name),
            kind: BazelTargetKind::Test,
            name: rule.name.clone(),
            sources: TargetAnalyzer::extract_sources(&rule.attributes),
            dependencies: TargetAnalyzer::extract_dependencies(&rule.attributes),
            test_only: true,
            attributes: TargetAnalyzer::extract_attributes(&rule.attributes),
        })
    }
    
    fn is_runnable(&self) -> bool {
        true
    }
}