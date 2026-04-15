//! Common utilities shared by all runners

use crate::{error::Result, parser::module_resolver::ModuleResolver, types::Runnable};
use std::path::Path;
use tracing::debug;

/// Get package name from Cargo.toml if available
pub fn get_cargo_package_name(file_path: &Path) -> Option<String> {
    if let Some(cargo_toml) = ModuleResolver::find_cargo_toml(file_path) {
        ModuleResolver::get_package_name_from_cargo_toml(&cargo_toml).ok()
    } else {
        None
    }
}

/// Resolve module paths for a list of runnables
pub fn resolve_module_paths(
    runnables: &mut [Runnable],
    file_path: &Path,
    package_name: Option<&str>,
    detector: &mut crate::patterns::RunnableDetector,
) -> Result<()> {
    // Create module resolver
    let resolver = if let Some(pkg) = package_name {
        ModuleResolver::with_package_name(pkg.to_string())
    } else {
        ModuleResolver::new()
    };

    // Get cached scopes from detector
    let scopes = detector.get_cached_scopes(file_path)?;

    // Resolve module paths for each runnable
    for runnable in runnables {
        match resolver.resolve_module_path(file_path, &scopes, &runnable.scope) {
            Ok(module_path) => {
                debug!(
                    "Resolved module path for {}: {}",
                    runnable.label, module_path
                );
                runnable.module_path = module_path;
            }
            Err(e) => {
                debug!(
                    "Failed to resolve module path for {}: {}",
                    runnable.label, e
                );
                // Keep the default module path on error
            }
        }
    }

    Ok(())
}

/// Resolve module path for a single runnable
pub fn resolve_module_path_single(
    runnable: &mut Runnable,
    file_path: &Path,
    package_name: Option<&str>,
    detector: &mut crate::patterns::RunnableDetector,
) -> Result<()> {
    resolve_module_paths(
        std::slice::from_mut(runnable),
        file_path,
        package_name,
        detector,
    )
}
