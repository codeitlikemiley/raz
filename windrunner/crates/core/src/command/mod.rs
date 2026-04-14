//! Cargo command generation and execution

pub mod builder;
pub mod command;
pub mod fallback;
pub mod resolver;
pub mod target;
pub mod template;

// Re-export commonly used types
pub use crate::plugins::CommandSpec;
pub use command::{Command, CommandStrategy};
pub use resolver::{CargoTargetResolver, ResolverChain};
pub use target::Target;
pub use template::{CommandTemplate, Templates};

// Clean public API for library users
pub use builder::CommandBuilder;
