#![doc = include_str!("../README.md")]

pub mod backup;
pub mod cli;
pub mod detector;
pub mod error;
pub mod inspector;
pub mod key;
pub mod migration;
pub mod parser;
pub mod preview;
pub mod resolver;
pub mod storage;
pub mod system;

pub use backup::{BackupInfo, BackupManager};
pub use cli::OverrideCli;
pub use detector::{FunctionDetector, FunctionInfo};
pub use error::{OverrideError, Result};
pub use inspector::OverrideInspector;
pub use key::{FunctionContext, OverrideKey};
pub use migration::{MigrationReport, OverrideMigrator};
pub use parser::{
    OptionValue, OverrideOperator, OverrideParser, ParsedOverride, ParsedOverrides,
    SmartOverrideParser, parse_override, parse_override_to_command,
};
pub use preview::{OverridePreview, PreviewChange};
pub use resolver::{OverrideResolver, ResolutionStrategy};
pub use storage::OverrideStorage;
pub use system::{OverrideSystem, PreviewParams};

// Alias for compatibility
pub type OverrideManager = OverrideSystem;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub use raz_config::CommandOverride;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverrideContext {
    pub file_path: PathBuf,
    pub cursor_line: Option<usize>,
    pub cursor_column: Option<usize>,
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverrideEntry {
    pub key: OverrideKey,
    pub override_config: CommandOverride,
    pub metadata: OverrideMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationStatus {
    /// Just saved, not yet executed
    Pending,
    /// Executed successfully at least once
    Validated,
    /// Failed N times
    Failed(u32),
}

impl Default for ValidationStatus {
    fn default() -> Self {
        Self::Pending
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverrideMetadata {
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub modified_at: chrono::DateTime<chrono::Utc>,
    pub file_path: PathBuf,
    pub function_name: Option<String>,
    pub original_line: Option<usize>,
    pub notes: Option<String>,
    #[serde(default)]
    pub validation_status: ValidationStatus,
    pub last_execution_time: Option<chrono::DateTime<chrono::Utc>>,
    pub last_execution_success: Option<bool>,
    #[serde(default)]
    pub failure_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedOverride {
    pub key: OverrideKey,
    pub override_data: raz_config::CommandOverride,
    pub match_quality: MatchQuality,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MatchQuality {
    Exact,
    FuzzyFunction,
    LineRange,
    FileLevel,
    WorkspaceDefault,
}
