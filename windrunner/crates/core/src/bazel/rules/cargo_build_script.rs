//! Handler for cargo_build_script rules

use crate::bazel::{RuleCall, BazelTarget, BazelTargetKind, TargetAnalyzer};
use super::RuleHandler;

/// Handler for cargo_build_script rules
pub struct CargoBuildScriptHandler;

impl RuleHandler for CargoBuildScriptHandler {
    fn can_handle(&self, rule_type: &str) -> bool {
        rule_type == "cargo_build_script"
    }
    
    fn analyze(&self, rule: &RuleCall) -> Option<BazelTarget> {
        Some(BazelTarget {
            label: format!(":{}", rule.name),
            kind: BazelTargetKind::BuildScript,
            name: rule.name.clone(),
            sources: TargetAnalyzer::extract_sources(&rule.attributes),
            dependencies: TargetAnalyzer::extract_dependencies(&rule.attributes),
            test_only: false,
            attributes: TargetAnalyzer::extract_attributes(&rule.attributes),
        })
    }
    
    fn is_runnable(&self) -> bool {
        false  // Build scripts are built but not directly runnable
    }
}