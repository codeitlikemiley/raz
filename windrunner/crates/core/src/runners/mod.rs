//! New runner architecture with clear separation of concerns

pub mod bazel_runner;
pub mod builder;
pub mod cargo_runner;
pub mod common;

pub mod framework;

pub mod options;
pub mod rustc_runner;

pub mod traits;
pub mod unified_runner;
pub mod validation;

// Re-export main types
pub use builder::{CommandBuilder, Unvalidated, Validated};
pub use framework::{Framework, FrameworkKind};
pub use traits::{CommandRunner, RunnerCommand};
pub use unified_runner::UnifiedRunner;
pub use validation::{ValidationError, ValidationRule};
