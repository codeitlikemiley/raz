//! Type-state pattern command builder for safe command construction

use std::marker::PhantomData;
use std::path::PathBuf;

use super::{
    framework::{Framework, FrameworkKind, FrameworkOptions},
    options::*,
    validation::{bazel_validation_rules, cargo_validation_rules},
};
use crate::build_system::BuildSystem;
use crate::error::Result;

/// Type state for unvalidated commands
pub struct Unvalidated;

/// Type state for validated commands
pub struct Validated;

/// Command builder with type-state pattern
pub struct CommandBuilder<State = Unvalidated> {
    build_system: BuildSystem,
    framework_kind: Option<FrameworkKind>,
    package_selection: PackageSelection,
    target_selection: TargetSelection,
    feature_selection: FeatureSelection,
    compilation_options: CompilationOptions,
    manifest_options: ManifestOptions,
    output_options: OutputOptions,
    framework_options: FrameworkOptions,
    _phantom: PhantomData<State>,
}

impl CommandBuilder<Unvalidated> {
    /// Create a new command builder
    pub fn new(build_system: BuildSystem) -> Self {
        Self {
            build_system,
            framework_kind: None,
            package_selection: PackageSelection::default(),
            target_selection: TargetSelection::default(),
            feature_selection: FeatureSelection::default(),
            compilation_options: CompilationOptions::default(),
            manifest_options: ManifestOptions::default(),
            output_options: OutputOptions::default(),
            framework_options: FrameworkOptions::default(),
            _phantom: PhantomData,
        }
    }

    /// Set the framework kind
    pub fn with_framework(mut self, framework: FrameworkKind) -> Self {
        self.framework_kind = Some(framework);
        self
    }

    // Package selection methods
    pub fn with_package(mut self, package: impl Into<String>) -> Self {
        self.package_selection.package.push(package.into());
        self
    }

    pub fn with_workspace(mut self) -> Self {
        self.package_selection.workspace = true;
        self
    }

    pub fn with_exclude(mut self, package: impl Into<String>) -> Self {
        self.package_selection.exclude.push(package.into());
        self
    }

    // Target selection methods
    pub fn with_lib(mut self) -> Self {
        self.target_selection.lib = true;
        self
    }

    pub fn with_bin(mut self, name: Option<impl Into<String>>) -> Self {
        if let Some(n) = name {
            self.target_selection.bin.push(n.into());
        } else {
            self.target_selection.bins = true;
        }
        self
    }

    pub fn with_example(mut self, name: Option<impl Into<String>>) -> Self {
        if let Some(n) = name {
            self.target_selection.example.push(n.into());
        } else {
            self.target_selection.examples = true;
        }
        self
    }

    pub fn with_test(mut self, name: Option<impl Into<String>>) -> Self {
        if let Some(n) = name {
            self.target_selection.test.push(n.into());
        } else {
            self.target_selection.tests = true;
        }
        self
    }

    pub fn with_doc(mut self) -> Self {
        self.target_selection.doc = true;
        self
    }

    pub fn with_all_targets(mut self) -> Self {
        self.target_selection.all_targets = true;
        self
    }

    // Feature selection methods
    pub fn with_features(mut self, features: Vec<String>) -> Self {
        self.feature_selection.features.extend(features);
        self
    }

    pub fn with_all_features(mut self) -> Self {
        self.feature_selection.all_features = true;
        self
    }

    pub fn with_no_default_features(mut self) -> Self {
        self.feature_selection.no_default_features = true;
        self
    }

    // Compilation options methods
    pub fn with_release(mut self) -> Self {
        self.compilation_options.release = true;
        self
    }

    pub fn with_profile(mut self, profile: impl Into<String>) -> Self {
        self.compilation_options.profile = Some(profile.into());
        self
    }

    pub fn with_target(mut self, target: impl Into<String>) -> Self {
        self.compilation_options.target = Some(target.into());
        self
    }

    pub fn with_target_dir(mut self, dir: PathBuf) -> Self {
        self.compilation_options.target_dir = Some(dir);
        self
    }

    pub fn with_jobs(mut self, jobs: u32) -> Self {
        self.compilation_options.jobs = Some(jobs);
        self
    }

    // Manifest options methods
    pub fn with_manifest_path(mut self, path: PathBuf) -> Self {
        self.manifest_options.manifest_path = Some(path);
        self
    }

    pub fn with_locked(mut self) -> Self {
        self.manifest_options.locked = true;
        self
    }

    pub fn with_offline(mut self) -> Self {
        self.manifest_options.offline = true;
        self
    }

    pub fn with_frozen(mut self) -> Self {
        self.manifest_options.frozen = true;
        self.manifest_options.locked = true;
        self.manifest_options.offline = true;
        self
    }

    // Output options methods
    pub fn with_verbose(mut self, level: u8) -> Self {
        self.output_options.verbose = level;
        self
    }

    pub fn with_quiet(mut self) -> Self {
        self.output_options.quiet = true;
        self
    }

    pub fn with_color(mut self, mode: impl Into<String>) -> Self {
        self.output_options.color = Some(mode.into());
        self
    }

    /// Convert to CommandOptions for validation
    fn to_options(&self) -> CommandOptions {
        CommandOptions {
            package_selection: self.package_selection.clone(),
            target_selection: self.target_selection.clone(),
            feature_selection: self.feature_selection.clone(),
            compilation_options: self.compilation_options.clone(),
            manifest_options: self.manifest_options.clone(),
            output_options: self.output_options.clone(),
        }
    }

    /// Validate the command and transition to Validated state
    pub fn validate(self) -> Result<CommandBuilder<Validated>> {
        // First do basic validation on each option group
        let options = self.to_options();
        options
            .validate()
            .map_err(|e| crate::error::Error::Other(e.to_string()))?;

        // Then apply build-system specific validation rules
        let rules = match self.build_system {
            BuildSystem::Cargo => cargo_validation_rules(),
            BuildSystem::Bazel => bazel_validation_rules(),
        };

        rules.validate(&options)?;

        // Validate framework-specific options if framework is set
        if let Some(framework_kind) = self.framework_kind {
            let framework = create_framework(framework_kind);
            framework.validate_options(&self.framework_options)?;
        }

        Ok(CommandBuilder {
            build_system: self.build_system,
            framework_kind: self.framework_kind,
            package_selection: self.package_selection,
            target_selection: self.target_selection,
            feature_selection: self.feature_selection,
            compilation_options: self.compilation_options,
            manifest_options: self.manifest_options,
            output_options: self.output_options,
            framework_options: self.framework_options,
            _phantom: PhantomData,
        })
    }
}

impl CommandBuilder<Validated> {
    /// Build the final command arguments
    pub fn build(self) -> Vec<String> {
        let mut args = vec![];

        // Add framework base command if set
        if let Some(framework_kind) = self.framework_kind {
            let framework = create_framework(framework_kind);
            args.push(framework.base_command().to_string());
        }

        // Package selection
        for package in &self.package_selection.package {
            args.push("-p".to_string());
            args.push(package.clone());
        }
        if self.package_selection.workspace {
            args.push("--workspace".to_string());
        }
        for exclude in &self.package_selection.exclude {
            args.push("--exclude".to_string());
            args.push(exclude.clone());
        }

        // Target selection
        if self.target_selection.lib {
            args.push("--lib".to_string());
        }
        if self.target_selection.bins {
            args.push("--bins".to_string());
        }
        for bin in &self.target_selection.bin {
            args.push("--bin".to_string());
            args.push(bin.clone());
        }
        if self.target_selection.examples {
            args.push("--examples".to_string());
        }
        for example in &self.target_selection.example {
            args.push("--example".to_string());
            args.push(example.clone());
        }
        if self.target_selection.doc {
            args.push("--doc".to_string());
        }
        if self.target_selection.all_targets {
            args.push("--all-targets".to_string());
        }

        // Feature selection
        if !self.feature_selection.features.is_empty() {
            args.push("--features".to_string());
            args.push(self.feature_selection.features.join(","));
        }
        if self.feature_selection.all_features {
            args.push("--all-features".to_string());
        }
        if self.feature_selection.no_default_features {
            args.push("--no-default-features".to_string());
        }

        // Compilation options
        if let Some(jobs) = self.compilation_options.jobs {
            args.push("-j".to_string());
            args.push(jobs.to_string());
        }
        if self.compilation_options.release {
            args.push("--release".to_string());
        }
        if let Some(profile) = &self.compilation_options.profile {
            args.push("--profile".to_string());
            args.push(profile.clone());
        }
        if let Some(target) = &self.compilation_options.target {
            args.push("--target".to_string());
            args.push(target.clone());
        }
        if let Some(target_dir) = &self.compilation_options.target_dir {
            args.push("--target-dir".to_string());
            args.push(target_dir.to_string_lossy().to_string());
        }

        // Manifest options
        if let Some(manifest_path) = &self.manifest_options.manifest_path {
            args.push("--manifest-path".to_string());
            args.push(manifest_path.to_string_lossy().to_string());
        }
        if self.manifest_options.frozen {
            args.push("--frozen".to_string());
        } else {
            if self.manifest_options.locked {
                args.push("--locked".to_string());
            }
            if self.manifest_options.offline {
                args.push("--offline".to_string());
            }
        }

        // Output options
        for _ in 0..self.output_options.verbose {
            args.push("-v".to_string());
        }
        if self.output_options.quiet {
            args.push("-q".to_string());
        }
        if let Some(color) = &self.output_options.color {
            args.push("--color".to_string());
            args.push(color.clone());
        }

        // Add framework-specific args
        if let Some(framework_kind) = self.framework_kind {
            let framework = create_framework(framework_kind);
            args.extend(framework.build_args(&self.framework_options));
        }

        args
    }

    /// Get the command options
    pub fn options(&self) -> CommandOptions {
        CommandOptions {
            package_selection: self.package_selection.clone(),
            target_selection: self.target_selection.clone(),
            feature_selection: self.feature_selection.clone(),
            compilation_options: self.compilation_options.clone(),
            manifest_options: self.manifest_options.clone(),
            output_options: self.output_options.clone(),
        }
    }
}

/// Create a framework instance based on kind
fn create_framework(kind: FrameworkKind) -> Box<dyn Framework> {
    use super::framework::*;

    match kind {
        FrameworkKind::Test => Box::new(TestFramework),
        FrameworkKind::Binary => Box::new(BinaryFramework),
        FrameworkKind::Benchmark => Box::new(BenchmarkFramework),
        FrameworkKind::DocTest => Box::new(DocTestFramework),
        FrameworkKind::Build => Box::new(TestFramework), // TODO: Add BuildFramework
    }
}
