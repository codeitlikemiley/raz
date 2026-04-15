use std::io;
use std::path::PathBuf;

/// Errors that can occur during cargo-runner operations
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("TOML Parsing Error: {0}")]
    CargoTomlParse(#[from] cargo_toml::Error),

    #[error("Tree-sitter language error: {0}")]
    TreeSitterLanguage(#[from] tree_sitter::LanguageError),

    #[error("Failed to parse config: {0}")]
    ConfigParse(String), // We can use display strings if type is complex

    #[error("Failed to parse source code")]
    SourceParse,

    #[error("BUILD file syntax error")]
    BuildFileParse,

    #[error("No runnable found at the specified location")]
    NoRunnableFound,

    #[error("No runnable found at line {line}. Available lines: {available:?}")]
    NoRunnableAtLine { line: u32, available: Vec<u32> },

    #[error("Invalid file path: {0}")]
    InvalidPath(&'static str),

    #[error("Invalid file name")]
    InvalidFileName,

    #[error("Reached filesystem root without finding target")]
    FsRootReached,

    #[error("No Cargo.toml found")]
    NoCargoToml,

    #[error("No [package] section found in Cargo.toml")]
    NoPackageSection,

    #[error("No BUILD file found")]
    NoBuildFile,

    #[error("File not under BUILD directory")]
    NotInBuildDirectory,

    #[error("{entity} without name")]
    MissingEntityName { entity: &'static str },

    #[error("Invalid UTF-8 in {entity} name: {err}")]
    InvalidUtf8Name {
        entity: &'static str,
        err: std::str::Utf8Error,
    },

    #[error("Target '{label}' does not carry a runnable")]
    TargetNotRunnable { label: String },

    #[error("No primary plugin detected for {path}")]
    NoPrimaryPlugin { path: PathBuf },

    #[error("Command validation failed: {0}")]
    Validation(&'static str),

    #[error("No runner available for build system: {0}")]
    NoRunner(String),

    #[error("No build system detected for path: {0}")]
    NoBuildSystem(PathBuf),

    #[error("Template syntax error: {0}")]
    TemplateError(&'static str),

    #[error("Unsupported runnable type for {context}")]
    UnsupportedRunnable { context: &'static str },

    #[error("No Bazel target found for file: {file}. {hint}")]
    MissingBazelTarget { file: PathBuf, hint: &'static str },
}

/// Result type alias for cargo-runner operations
pub type Result<T> = std::result::Result<T, Error>;
