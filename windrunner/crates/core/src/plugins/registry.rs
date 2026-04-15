use crate::{
    config::{Config, PluginPolicy},
    error::{Error, Result},
    plugins::{CommandSpec, ProjectContext, TargetRef},
    types::Runnable,
};
use std::{cmp::Reverse, path::Path, sync::Arc};

pub trait PrimaryPlugin: Send + Sync {
    fn id(&self) -> &str;
    fn default_priority(&self) -> i32;
    fn matches(&self, ctx: &ProjectContext) -> bool;
    fn discover_targets(&self, ctx: &ProjectContext, line: Option<u32>) -> Result<Vec<TargetRef>>;
    fn build_command(&self, target: &TargetRef, ctx: &ProjectContext) -> Result<CommandSpec>;
}

pub trait OverlayPlugin: Send + Sync {
    fn id(&self) -> &str;
    fn default_priority(&self) -> i32;
    fn matches(&self, primary_id: &str, ctx: &ProjectContext, target: &TargetRef) -> bool;
    fn augment_command(
        &self,
        command: CommandSpec,
        target: &TargetRef,
        ctx: &ProjectContext,
    ) -> Result<CommandSpec>;
}

#[derive(Debug, Clone)]
pub struct PluginResolution {
    pub primary_id: String,
    pub overlay_ids: Vec<String>,
}

pub struct PluginRegistry {
    primaries: Vec<Box<dyn PrimaryPlugin>>,
    overlays: Vec<Box<dyn OverlayPlugin>>,
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            primaries: Vec::new(),
            overlays: Vec::new(),
        }
    }

    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register_primary(crate::plugins::BazelPrimaryPlugin::new());
        registry.register_primary(crate::plugins::CargoPrimaryPlugin::new());
        registry.register_primary(crate::plugins::RustcPrimaryPlugin::new());
        registry.register_overlay(crate::plugins::DioxusOverlayPlugin::new());
        registry.register_overlay(crate::plugins::LeptosOverlayPlugin::new());
        registry
    }

    pub fn register_primary<P: PrimaryPlugin + 'static>(&mut self, plugin: P) {
        self.primaries.push(Box::new(plugin));
        self.primaries
            .sort_by_key(|plugin| Reverse(plugin.default_priority()));
    }

    pub fn register_overlay<P: OverlayPlugin + 'static>(&mut self, plugin: P) {
        self.overlays.push(Box::new(plugin));
        self.overlays
            .sort_by_key(|plugin| Reverse(plugin.default_priority()));
    }

    pub fn select_primary<'a>(&'a self, ctx: &ProjectContext) -> Result<&'a dyn PrimaryPlugin> {
        self.primaries
            .iter()
            .filter(|plugin| self.enabled(plugin.id(), ctx) && plugin.matches(ctx))
            .max_by_key(|plugin| {
                self.effective_priority(plugin.id(), plugin.default_priority(), ctx)
            })
            .map(|plugin| plugin.as_ref())
            .ok_or_else(|| Error::NoPrimaryPlugin {
                path: ctx.file_path.clone(),
            })
    }

    pub fn resolution_for(
        &self,
        ctx: &ProjectContext,
        target: &TargetRef,
    ) -> Result<PluginResolution> {
        let primary = self.select_primary(ctx)?;
        let mut overlay_ids = Vec::new();

        for overlay in &self.overlays {
            if self.enabled(overlay.id(), ctx) && overlay.matches(primary.id(), ctx, target) {
                overlay_ids.push(overlay.id().to_string());
            }
        }

        Ok(PluginResolution {
            primary_id: primary.id().to_string(),
            overlay_ids,
        })
    }

    pub fn discover_targets(
        &self,
        ctx: &ProjectContext,
        line: Option<u32>,
    ) -> Result<Vec<TargetRef>> {
        let primary = self.select_primary(ctx)?;
        primary.discover_targets(ctx, line)
    }

    pub fn build_command_for_target(
        &self,
        ctx: &ProjectContext,
        target: &TargetRef,
    ) -> Result<CommandSpec> {
        let primary = self.select_primary(ctx)?;
        let mut command = primary.build_command(target, ctx)?;

        for overlay in &self.overlays {
            if self.enabled(overlay.id(), ctx) && overlay.matches(primary.id(), ctx, target) {
                command = overlay.augment_command(command, target, ctx)?;
            }
        }

        Ok(command)
    }

    pub fn build_command_for_runnable(
        &self,
        config: Arc<Config>,
        runnable: &Runnable,
    ) -> Result<CommandSpec> {
        let ctx = ProjectContext::from_path(&runnable.file_path, config);
        let target = TargetRef::from_runnable("rust", runnable.clone());
        self.build_command_for_target(&ctx, &target)
    }

    pub fn detect_primary_build_system(
        &self,
        ctx: &ProjectContext,
    ) -> Result<crate::build_system::BuildSystem> {
        let primary = self.select_primary(ctx)?;
        Ok(match primary.id() {
            "bazel" => crate::build_system::BuildSystem::Bazel,
            _ => crate::build_system::BuildSystem::Cargo,
        })
    }

    pub fn detect_primary_build_system_for_path(
        &self,
        path: &Path,
        config: Arc<Config>,
    ) -> Result<crate::build_system::BuildSystem> {
        let ctx = ProjectContext::from_path(path, config);
        self.detect_primary_build_system(&ctx)
    }

    fn enabled(&self, plugin_id: &str, ctx: &ProjectContext) -> bool {
        ctx.config
            .plugins
            .get(plugin_id)
            .and_then(|policy| policy.enabled)
            .unwrap_or(true)
    }

    fn effective_priority(&self, plugin_id: &str, default: i32, ctx: &ProjectContext) -> i32 {
        ctx.config
            .plugins
            .get(plugin_id)
            .and_then(|policy| policy.priority)
            .unwrap_or(default)
    }

    pub fn policy_for<'a>(&self, plugin_id: &str, config: &'a Config) -> Option<&'a PluginPolicy> {
        config.plugins.get(plugin_id)
    }
}
