//! Bazel support using proper Starlark parsing

pub mod starlark_parser;
pub mod rule_extractor;
pub mod target_analyzer;
pub mod target_finder;
pub mod rules;

#[cfg(test)]
mod integration_test;
#[cfg(test)]
mod integration_server_test;
#[cfg(test)]
mod debug_integration_test;

pub use starlark_parser::StarlarkParser;
pub use rule_extractor::{RuleCall, RuleExtractor, AttributeValue};
pub use target_analyzer::{BazelTarget, BazelTargetKind, TargetAnalyzer};
pub use target_finder::BazelTargetFinder;