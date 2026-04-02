use serde::{Deserialize, Serialize};

/// Represents different types of file scopes in a Rust project
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileScope {
    /// Library crate (src/lib.rs)
    Lib,
    /// Binary target (src/main.rs, src/bin/*.rs, or custom path from Cargo.toml)
    Bin { name: Option<String> },
    /// Benchmark (benches/*.rs or custom path from Cargo.toml)
    Bench { name: Option<String> },
    /// Build script (build.rs or custom path)
    Build,
    /// Integration test (tests/*.rs or custom path from Cargo.toml)
    Test { name: Option<String> },
    /// Example (examples/*.rs or custom path from Cargo.toml)
    Example { name: Option<String> },
    /// Standalone Rust file (outside of a Cargo project)
    Standalone { name: Option<String> },
    /// Unknown/generic file
    Unknown,
}

/// Represents different kinds of scopes in Rust source code
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScopeKind {
    File(FileScope),
    Module,
    Struct,
    Enum,
    Union,
    Impl,
    Function,
    Test,
    Benchmark,
    DocTest,
}
