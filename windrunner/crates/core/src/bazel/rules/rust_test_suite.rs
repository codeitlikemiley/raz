//! Handler for rust_test_suite rules

use crate::bazel::{RuleCall, BazelTarget, BazelTargetKind, TargetAnalyzer};
use super::RuleHandler;

/// Handler for rust_test_suite rules
pub struct RustTestSuiteHandler;

impl RuleHandler for RustTestSuiteHandler {
    fn can_handle(&self, rule_type: &str) -> bool {
        rule_type == "rust_test_suite"
    }
    
    fn analyze(&self, rule: &RuleCall) -> Option<BazelTarget> {
        Some(BazelTarget {
            label: format!(":{}", rule.name),
            kind: BazelTargetKind::TestSuite,
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