//! Bazel support using proper Starlark parsing

pub mod rule_extractor;
pub mod rules;
pub mod starlark_parser;
pub mod target_analyzer;
pub mod target_finder;

#[cfg(test)]
#[cfg(test)]
mod integration_server_test;
#[cfg(test)]
mod integration_test;

pub use rule_extractor::{AttributeValue, RuleCall, RuleExtractor};
pub use starlark_parser::StarlarkParser;
pub use target_analyzer::{BazelTarget, BazelTargetKind, TargetAnalyzer};
pub use target_finder::BazelTargetFinder;
