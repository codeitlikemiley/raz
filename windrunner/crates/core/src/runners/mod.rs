//! New runner architecture with clear separation of concerns

pub mod bazel_runner;
pub mod cargo_runner;
pub mod common;
pub mod rustc_runner;
pub mod traits;
pub mod unified_runner;

// Re-export main types
pub use traits::{CommandRunner, RunnerCommand};
pub use unified_runner::UnifiedRunner;
