//! Handler for rust_doc_test rules

use crate::bazel::{RuleCall, BazelTarget, BazelTargetKind, TargetAnalyzer};
use super::RuleHandler;

/// Handler for rust_doc_test rules
pub struct RustDocTestHandler;

impl RuleHandler for RustDocTestHandler {
    fn can_handle(&self, rule_type: &str) -> bool {
        rule_type == "rust_doc_test"
    }
    
    fn analyze(&self, rule: &RuleCall) -> Option<BazelTarget> {
        Some(BazelTarget {
            label: format!(":{}", rule.name),
            kind: BazelTargetKind::DocTest,
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