pub mod analyzer;
pub mod builtins;
pub mod command_spec;
pub mod context;
pub mod registry;
pub mod target_ref;

pub use analyzer::{RustSourceAnalyzer, SourceAnalyzer};
pub use builtins::{
    BazelPrimaryPlugin, CargoPrimaryPlugin, DioxusOverlayPlugin, LeptosOverlayPlugin,
    RustcPrimaryPlugin,
};
pub use command_spec::{CommandSpec, CommandStrategy};
pub use context::ProjectContext;
pub use registry::{OverlayPlugin, PluginRegistry, PluginResolution, PrimaryPlugin};
pub use target_ref::{SourceRange, TargetRef};
