//! Cargo command generation and execution

pub mod builder;
pub mod cargo_command;
pub mod fallback;
pub mod resolver;
pub mod target;
pub mod template;

// Re-export commonly used types
pub use crate::plugins::{CommandSpec, CommandStrategy};
pub use cargo_command::{CargoCommand, CommandType};
pub use resolver::{CargoTargetResolver, ResolverChain};
pub use target::Target;
pub use template::{CommandTemplate, Templates};

// Clean public API for library users
pub use builder::CommandBuilder;
