//! Common utilities and shared types for the raz project
//!
//! This crate contains functionality that is shared across multiple raz crates
//! to reduce code duplication and ensure consistency.

pub mod cargo_flags;
pub mod env;
pub mod error;
pub mod output;
pub mod parse;
pub mod shell;
pub mod time;

// Re-export commonly used items
pub use env::{EnvBuilder, EnvParser};
pub use error::{CommonError, ErrorContext, OptionContext, Result};
pub use output::OutputFormatter;
pub use shell::ShellCommand;
pub use time::{Elapsed, TimeUtils};
