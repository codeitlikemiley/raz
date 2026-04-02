pub mod function_identity;
pub mod position;
pub mod runnable;
pub mod scope;
pub mod scope_kind;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileType {
    CargoProject,
    Standalone,
    SingleFileScript,
}

// Re-export commonly used types
pub use function_identity::FunctionIdentity;
pub use position::Position;
pub use runnable::{Runnable, RunnableKind, RunnableWithScore};
pub use scope::{ExtendedScope, Scope};
pub use scope_kind::{FileScope, ScopeKind};
