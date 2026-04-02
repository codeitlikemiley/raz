//! Handler for rust_benchmark rules

use crate::bazel::{RuleCall, BazelTarget, BazelTargetKind, TargetAnalyzer};
use super::RuleHandler;

/// Handler for rust_benchmark rules
pub struct RustBenchmarkHandler;

impl RuleHandler for RustBenchmarkHandler {
    fn can_handle(&self, rule_type: &str) -> bool {
        rule_type == "rust_benchmark" || rule_type == "rust_bench"
    }
    
    fn analyze(&self, rule: &RuleCall) -> Option<BazelTarget> {
        Some(BazelTarget {
            label: format!(":{}", rule.name),
            kind: BazelTargetKind::Benchmark,
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