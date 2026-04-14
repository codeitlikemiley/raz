//! cargo-runner - A tool for detecting and running Rust code at specific locations
//!
//! This crate provides functionality to:
//! - Parse Rust source files and detect runnable items (tests, benchmarks, binaries)
//! - Generate appropriate cargo commands for running specific items
//! - Support various project structures and configurations
pub mod bazel;
pub mod build_system;
pub mod command;
pub mod config;
pub mod error;
pub mod parser;
pub mod patterns;
pub mod plugins;
pub mod types;

// Runner architecture
pub mod runners;

// Re-export commonly used types and traits
pub use error::{Error, Result};
pub use types::*;

// Re-export main API components
pub use command::{Command, CommandStrategy};
pub use config::Config;
pub use plugins::{CommandSpec, PluginRegistry, ProjectContext, TargetRef};

// Export the unified runner
pub use runners::UnifiedRunner;

// Legacy alias for backwards compatibility
pub use runners::UnifiedRunner as CargoRunner;
