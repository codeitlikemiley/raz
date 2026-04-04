//! Handler for rust_binary rules

use super::RuleHandler;
use crate::bazel::{BazelTarget, BazelTargetKind, RuleCall, TargetAnalyzer};

/// Handler for rust_binary rules
pub struct RustBinaryHandler;

impl RuleHandler for RustBinaryHandler {
    fn can_handle(&self, rule_type: &str) -> bool {
        rule_type == "rust_binary"
    }

    fn analyze(&self, rule: &RuleCall) -> Option<BazelTarget> {
        Some(BazelTarget {
            label: format!(":{}", rule.name),
            kind: BazelTargetKind::Binary,
            name: rule.name.clone(),
            sources: TargetAnalyzer::extract_sources(&rule.attributes),
            dependencies: TargetAnalyzer::extract_dependencies(&rule.attributes),
            test_only: false,
            attributes: TargetAnalyzer::extract_attributes(&rule.attributes),
        })
    }

    fn is_runnable(&self) -> bool {
        true
    }
}
