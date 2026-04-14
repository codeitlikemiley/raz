use crate::command::Command;

// Backwards compatibility alias for the CLI and API boundaries
pub type CommandSpec = Command;
pub type CommandStrategy = crate::command::CommandStrategy;
