//! Rust source code parsing and analysis using tree-sitter

pub mod module_resolver;
pub mod rust_parser;
pub mod scope_detector;
pub mod utils;

// Re-export commonly used items
pub use rust_parser::RustParser;
pub use utils::node_to_position;
