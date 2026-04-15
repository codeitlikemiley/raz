use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::{build_system::BuildSystem, error::Result, plugins::ProjectContext};

use super::UnifiedRunner;

impl UnifiedRunner {
    /// Detect the build system for a given path
    pub fn detect_build_system(&self, path: &Path) -> Result<BuildSystem> {
        // Convert to absolute path to ensure consistent detection
        let abs_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .map_err(crate::error::Error::IoError)?
                .join(path)
        };

        tracing::debug!("detect_build_system: starting with path {:?}", abs_path);

        // Start from the file's directory and walk up to find build files
        let start_path = if abs_path.is_file() {
            abs_path.parent().unwrap_or(&abs_path)
        } else {
            &abs_path
        };

        // Only honor a search boundary when the target path actually lives under it.
        // Workspaces can live outside HOME, and stopping immediately in that case
        // makes build-system detection fail and fall back to Cargo.
        let project_boundary = std::env::var("PROJECT_DIR")
            .ok()
            .map(PathBuf::from)
            .filter(|boundary| abs_path.starts_with(boundary));

        let home_boundary = std::env::var("HOME")
            .ok()
            .map(PathBuf::from)
            .filter(|boundary| abs_path.starts_with(boundary));

        let mut check_path = start_path;
        let mut depth = 0;
        const MAX_DEPTH: usize = 10; // Reasonable depth limit

        tracing::debug!(
            "detect_build_system: starting from directory {:?}",
            check_path
        );
        tracing::debug!("detect_build_system: HOME boundary: {:?}", home_boundary);
        if let Some(ref boundary) = project_boundary {
            tracing::debug!("detect_build_system: PROJECT_DIR boundary: {:?}", boundary);
        }

        // Walk up the directory tree looking for build files
        loop {
            // Check depth limit
            if depth >= MAX_DEPTH {
                tracing::debug!(
                    "detect_build_system: reached max depth {}, stopping",
                    MAX_DEPTH
                );
                break;
            }
            // Check boundaries BEFORE checking for build system
            if let Some(ref boundary) = project_boundary {
                if !check_path.starts_with(boundary) {
                    tracing::debug!("detect_build_system: reached PROJECT_DIR boundary, stopping");
                    break;
                }
            }

            if home_boundary
                .as_ref()
                .is_some_and(|boundary| !check_path.starts_with(boundary))
            {
                tracing::debug!("detect_build_system: reached HOME boundary, stopping");
                break;
            }

            tracing::debug!("detect_build_system: checking directory {:?}", check_path);

            let ctx = ProjectContext::from_path(check_path, Arc::clone(&self.config));
            if let Ok(build_system) = self.plugins.detect_primary_build_system(&ctx) {
                tracing::info!(
                    "detect_build_system: found {:?} at {:?}",
                    build_system,
                    check_path
                );
                return Ok(build_system);
            }

            // Go up one directory
            match check_path.parent() {
                Some(parent) => {
                    tracing::debug!("detect_build_system: moving up to parent {:?}", parent);
                    check_path = parent;
                    depth += 1;
                }
                None => {
                    tracing::debug!("detect_build_system: reached root, no build system found");
                    break;
                }
            }
        }

        Err(crate::error::Error::NoBuildSystem(path.to_path_buf()))
    }

    /// Detect build system with fallback to standalone rustc
    pub fn detect_build_system_with_fallback(&self, path: &Path) -> BuildSystem {
        match self.detect_build_system(path) {
            Ok(bs) => bs,
            Err(_) => {
                // For now, default to Cargo when no build system is detected
                // This allows standalone files to be handled by CargoRunner
                BuildSystem::Cargo
            }
        }
    }

    /// Get the name of the currently detected build system
    pub fn current_build_system_name(&self, path: &Path) -> &'static str {
        match self.detect_build_system_with_fallback(path) {
            BuildSystem::Cargo => "cargo",
            BuildSystem::Bazel => "bazel",
        }
    }
}
