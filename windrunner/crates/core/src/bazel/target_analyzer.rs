//! Analyze Bazel rules to create target representations

use std::collections::HashMap;
use super::rule_extractor::{RuleCall, AttributeValue};
use super::rules::{RuleHandler, RustBinaryHandler, RustTestHandler, RustTestSuiteHandler, 
                   RustDocTestHandler, RustBenchmarkHandler, RustLibraryHandler, 
                   CargoBuildScriptHandler};

/// A Bazel target extracted from BUILD files
#[derive(Debug, Clone)]
pub struct BazelTarget {
    /// Full label (e.g., "//mylib:test")
    pub label: String,
    /// Target kind
    pub kind: BazelTargetKind,
    /// Target name
    pub name: String,
    /// Source files
    pub sources: Vec<String>,
    /// Dependencies
    pub dependencies: Vec<String>,
    /// Whether this is test-only
    pub test_only: bool,
    /// Additional attributes
    pub attributes: TargetAttributes,
}

/// Different kinds of Bazel targets
#[derive(Debug, Clone, PartialEq)]
pub enum BazelTargetKind {
    Binary,
    Test,
    TestSuite,
    DocTest,
    Benchmark,
    BuildScript,
    Library,
    Unknown(String),
}

impl BazelTargetKind {
    /// Check if this target kind is runnable
    pub fn is_runnable(&self) -> bool {
        matches!(
            self,
            BazelTargetKind::Binary
                | BazelTargetKind::Test
                | BazelTargetKind::TestSuite
                | BazelTargetKind::DocTest
                | BazelTargetKind::Benchmark
        )
    }
}

/// Additional target attributes
#[derive(Debug, Clone, Default)]
pub struct TargetAttributes {
    /// For tests: the crate being tested
    pub crate_ref: Option<String>,
    /// Visibility settings
    pub visibility: Vec<String>,
    /// Test size (small, medium, large)
    pub size: Option<String>,
    /// Test timeout
    pub timeout: Option<String>,
    /// Custom attributes
    pub custom: HashMap<String, AttributeValue>,
}

/// Analyzes rule calls to create BazelTarget instances
pub struct TargetAnalyzer {
    rule_handlers: Vec<Box<dyn RuleHandler>>,
}

impl TargetAnalyzer {
    /// Create a new target analyzer with default handlers
    pub fn new() -> Self {
        let handlers: Vec<Box<dyn RuleHandler>> = vec![
            Box::new(RustBinaryHandler),
            Box::new(RustTestHandler),
            Box::new(RustTestSuiteHandler),
            Box::new(RustDocTestHandler),
            Box::new(RustBenchmarkHandler),
            Box::new(RustLibraryHandler),
            Box::new(CargoBuildScriptHandler),
        ];
        
        Self {
            rule_handlers: handlers,
        }
    }
    
    /// Analyze a rule call
    pub fn analyze_rule(&self, rule: &RuleCall) -> Option<BazelTarget> {
        for handler in &self.rule_handlers {
            if handler.can_handle(&rule.rule_type) {
                return handler.analyze(rule);
            }
        }
        
        // Unknown rule type - create a generic target
        Some(BazelTarget {
            label: format!(":{}", rule.name),
            kind: BazelTargetKind::Unknown(rule.rule_type.clone()),
            name: rule.name.clone(),
            sources: Self::extract_sources(&rule.attributes),
            dependencies: Self::extract_dependencies(&rule.attributes),
            test_only: false,
            attributes: Self::extract_attributes(&rule.attributes),
        })
    }
    
    /// Filter rules to only runnable ones
    pub fn filter_runnable_rules(&self, rules: Vec<RuleCall>) -> Vec<RuleCall> {
        rules.into_iter()
            .filter(|rule| {
                self.rule_handlers
                    .iter()
                    .any(|handler| handler.can_handle(&rule.rule_type) && handler.is_runnable())
            })
            .collect()
    }
    
    /// Extract source files from attributes
    pub fn extract_sources(attributes: &HashMap<String, AttributeValue>) -> Vec<String> {
        match attributes.get("srcs") {
            Some(AttributeValue::List(srcs)) => srcs.clone(),
            Some(AttributeValue::Glob(glob)) => glob.patterns.clone(),
            _ => Vec::new(),
        }
    }
    
    /// Extract dependencies from attributes
    pub fn extract_dependencies(attributes: &HashMap<String, AttributeValue>) -> Vec<String> {
        let mut deps = Vec::new();
        
        // Extract from deps attribute
        if let Some(AttributeValue::List(dep_list)) = attributes.get("deps") {
            deps.extend(dep_list.clone());
        }
        
        // Extract from crate attribute (for tests)
        if let Some(AttributeValue::Label(crate_ref)) = attributes.get("crate") {
            deps.push(crate_ref.clone());
        } else if let Some(AttributeValue::String(crate_ref)) = attributes.get("crate") {
            deps.push(crate_ref.clone());
        }
        
        deps
    }
    
    /// Extract additional attributes
    pub fn extract_attributes(attributes: &HashMap<String, AttributeValue>) -> TargetAttributes {
        let mut target_attrs = TargetAttributes::default();
        
        // Extract crate reference
        if let Some(AttributeValue::Label(crate_ref)) = attributes.get("crate") {
            target_attrs.crate_ref = Some(crate_ref.clone());
        } else if let Some(AttributeValue::String(crate_ref)) = attributes.get("crate") {
            target_attrs.crate_ref = Some(crate_ref.clone());
        }
        
        // Extract visibility
        if let Some(AttributeValue::List(vis)) = attributes.get("visibility") {
            target_attrs.visibility = vis.clone();
        }
        
        // Extract test attributes
        if let Some(AttributeValue::String(size)) = attributes.get("size") {
            target_attrs.size = Some(size.clone());
        }
        
        if let Some(AttributeValue::String(timeout)) = attributes.get("timeout") {
            target_attrs.timeout = Some(timeout.clone());
        }
        
        // Store other attributes
        for (key, value) in attributes {
            if !["name", "srcs", "deps", "crate", "visibility", "size", "timeout"].contains(&key.as_str()) {
                target_attrs.custom.insert(key.clone(), value.clone());
            }
        }
        
        target_attrs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_target_kind_is_runnable() {
        assert!(BazelTargetKind::Binary.is_runnable());
        assert!(BazelTargetKind::Test.is_runnable());
        assert!(BazelTargetKind::TestSuite.is_runnable());
        assert!(BazelTargetKind::DocTest.is_runnable());
        assert!(BazelTargetKind::Benchmark.is_runnable());
        
        assert!(!BazelTargetKind::Library.is_runnable());
        assert!(!BazelTargetKind::BuildScript.is_runnable());
        assert!(!BazelTargetKind::Unknown("custom".to_string()).is_runnable());
    }
}