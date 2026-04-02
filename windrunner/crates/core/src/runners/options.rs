//! Command options organized by category

use std::path::PathBuf;

/// All options for a command, organized by category
#[derive(Debug, Clone, Default)]
pub struct CommandOptions {
    pub package_selection: PackageSelection,
    pub target_selection: TargetSelection,
    pub feature_selection: FeatureSelection,
    pub compilation_options: CompilationOptions,
    pub manifest_options: ManifestOptions,
    pub output_options: OutputOptions,
}

/// Package selection options
#[derive(Debug, Clone, Default)]
pub struct PackageSelection {
    /// Specific packages to operate on
    pub package: Vec<String>,
    /// Operate on all packages in workspace
    pub workspace: bool,
    /// Packages to exclude
    pub exclude: Vec<String>,
    /// Deprecated alias for workspace
    pub all: bool,
}

impl PackageSelection {
    pub fn is_workspace_mode(&self) -> bool {
        self.workspace || self.all
    }

    pub fn validate(&self) -> Result<(), String> {
        if !self.package.is_empty() && self.is_workspace_mode() {
            return Err("Cannot specify both --package and --workspace/--all".to_string());
        }
        Ok(())
    }
}

/// Target selection options
#[derive(Debug, Clone, Default)]
pub struct TargetSelection {
    /// Target the library
    pub lib: bool,
    /// Target all binaries
    pub bins: bool,
    /// Target specific binaries
    pub bin: Vec<String>,
    /// Target all examples
    pub examples: bool,
    /// Target specific examples
    pub example: Vec<String>,
    /// Target all tests
    pub tests: bool,
    /// Target specific tests
    pub test: Vec<String>,
    /// Target all benches
    pub benches: bool,
    /// Target specific benches
    pub bench: Vec<String>,
    /// Target all targets
    pub all_targets: bool,
    /// Target documentation (test only)
    pub doc: bool,
}

impl TargetSelection {
    pub fn validate(&self) -> Result<(), String> {
        // For run command, only one target type is allowed
        let target_count = [
            self.lib,
            !self.bin.is_empty() || self.bins,
            !self.example.is_empty() || self.examples,
        ]
        .iter()
        .filter(|&&x| x)
        .count();

        if target_count > 1 {
            return Err("Cannot specify multiple target types for run command".to_string());
        }

        // Doc tests conflict with other test targets
        if self.doc && (self.lib || self.bins || self.tests || self.examples) {
            return Err("--doc cannot be used with other target selections".to_string());
        }

        Ok(())
    }

    pub fn has_specific_target(&self) -> bool {
        self.lib
            || self.bins
            || !self.bin.is_empty()
            || self.examples
            || !self.example.is_empty()
            || self.tests
            || !self.test.is_empty()
            || self.benches
            || !self.bench.is_empty()
            || self.all_targets
            || self.doc
    }
}

/// Feature selection options
#[derive(Debug, Clone, Default)]
pub struct FeatureSelection {
    /// Specific features to enable
    pub features: Vec<String>,
    /// Enable all features
    pub all_features: bool,
    /// Disable default features
    pub no_default_features: bool,
}

impl FeatureSelection {
    pub fn validate(&self) -> Result<(), String> {
        if self.all_features && self.no_default_features {
            return Err("Cannot specify both --all-features and --no-default-features".to_string());
        }

        if self.all_features && !self.features.is_empty() {
            return Err("Cannot specify both --all-features and --features".to_string());
        }

        Ok(())
    }
}

/// Compilation options
#[derive(Debug, Clone, Default)]
pub struct CompilationOptions {
    /// Number of parallel jobs
    pub jobs: Option<u32>,
    /// Build in release mode
    pub release: bool,
    /// Build with specific profile
    pub profile: Option<String>,
    /// Target triple
    pub target: Option<String>,
    /// Target directory
    pub target_dir: Option<PathBuf>,
    /// Timing output formats
    pub timings: Option<Vec<String>>,
    /// Output build graph
    pub unit_graph: bool,
    /// Keep going on error
    pub keep_going: bool,
}

impl CompilationOptions {
    pub fn validate(&self) -> Result<(), String> {
        if self.release && self.profile.is_some() {
            return Err("Cannot specify both --release and --profile".to_string());
        }
        Ok(())
    }
}

/// Manifest options
#[derive(Debug, Clone, Default)]
pub struct ManifestOptions {
    /// Path to Cargo.toml
    pub manifest_path: Option<PathBuf>,
    /// Path to Cargo.lock
    pub lockfile_path: Option<PathBuf>,
    /// Ignore rust-version
    pub ignore_rust_version: bool,
    /// Ensure Cargo.lock unchanged
    pub locked: bool,
    /// Run offline
    pub offline: bool,
    /// Frozen (locked + offline)
    pub frozen: bool,
}

impl ManifestOptions {
    pub fn validate(&self) -> Result<(), String> {
        if self.frozen && (!self.locked || !self.offline) {
            // Frozen implies both locked and offline
            // This is more of a normalization than an error
        }
        Ok(())
    }

    pub fn is_locked(&self) -> bool {
        self.locked || self.frozen
    }

    pub fn is_offline(&self) -> bool {
        self.offline || self.frozen
    }
}

/// Output options
#[derive(Debug, Clone, Default)]
pub struct OutputOptions {
    /// Message format
    pub message_format: Option<String>,
    /// Verbosity level
    pub verbose: u8,
    /// Quiet mode
    pub quiet: bool,
    /// Color mode
    pub color: Option<String>,
}

impl OutputOptions {
    pub fn validate(&self) -> Result<(), String> {
        if self.quiet && self.verbose > 0 {
            return Err("Cannot specify both --quiet and --verbose".to_string());
        }

        if let Some(color) = &self.color {
            if !["auto", "always", "never"].contains(&color.as_str()) {
                return Err(format!("Invalid color mode: {}", color));
            }
        }

        Ok(())
    }
}

impl CommandOptions {
    /// Validate all options
    pub fn validate(&self) -> Result<(), String> {
        self.package_selection.validate()?;
        self.target_selection.validate()?;
        self.feature_selection.validate()?;
        self.compilation_options.validate()?;
        self.manifest_options.validate()?;
        self.output_options.validate()?;
        Ok(())
    }
}
