//! Configuration resolution for commands

use crate::{
    config::{Config, ConfigMerger},
    error::Result,
    types::FunctionIdentity,
};
use std::path::Path;

/// Resolves configuration for a given runnable
pub struct ConfigResolver<'a> {
    project_root: Option<&'a Path>,
    identity: &'a FunctionIdentity,
}

impl<'a> ConfigResolver<'a> {
    pub fn new(project_root: Option<&'a Path>, identity: &'a FunctionIdentity) -> Self {
        Self {
            project_root,
            identity,
        }
    }

    pub fn resolve(&self) -> Result<Config> {
        let mut merger = ConfigMerger::new();

        // Load configs based on the runnable's location
        // Only call load_configs_for_path once - it already handles the hierarchy
        if let Some(ref file_path) = self.identity.file_path {
            merger.load_configs_for_path(file_path)?;
        } else if let Some(root) = self.project_root {
            // No file path but we have project root
            merger.load_configs_for_path(root)?;
        }

        Ok(merger.get_merged_config())
    }
}
