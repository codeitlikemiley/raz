use anyhow::Result;
use raz_config::{CommandOverride, EffectiveConfig, GlobalConfig, WorkspaceConfig};
use std::path::Path;
use std::sync::Arc;

/// Adapter to maintain compatibility with the old ConfigManager interface
pub struct ConfigManager {
    global_config: Arc<GlobalConfig>,
}

impl ConfigManager {
    pub fn new() -> Result<Self> {
        let global_config = GlobalConfig::load()
            .map_err(|e| anyhow::anyhow!("Failed to load global config: {}", e))?;

        Ok(Self {
            global_config: Arc::new(global_config),
        })
    }

    pub fn get_effective_config(&self, workspace: &Path) -> EffectiveConfig {
        let workspace_config = WorkspaceConfig::load(workspace).ok().flatten();
        EffectiveConfig::new((*self.global_config).clone(), workspace_config)
    }

    pub fn get_command_override(&self, workspace: &Path, key: &str) -> Option<CommandOverride> {
        let effective = self.get_effective_config(workspace);
        effective.overrides.overrides.get(key).cloned()
    }

    pub fn set_command_override(
        &mut self,
        workspace: &Path,
        key: String,
        override_config: CommandOverride,
    ) -> Result<()> {
        let mut workspace_config = WorkspaceConfig::load(workspace)
            .map_err(|e| anyhow::anyhow!("Failed to load workspace config: {}", e))?
            .unwrap_or_else(|| WorkspaceConfig::new(workspace.to_path_buf()));

        if workspace_config.overrides.is_none() {
            workspace_config.overrides =
                Some(raz_config::override_config::OverrideCollection::new());
        }

        workspace_config
            .overrides
            .as_mut()
            .unwrap()
            .overrides
            .insert(key, override_config);

        workspace_config
            .save()
            .map_err(|e| anyhow::anyhow!("Failed to save workspace config: {}", e))
    }

    pub fn set_command_override_from_input(
        &mut self,
        workspace: &Path,
        key: String,
        command: &str,
        input: &str,
    ) -> Result<()> {
        use raz_override::parse_override_to_command;

        let override_config = parse_override_to_command(command, input)
            .map_err(|e| anyhow::anyhow!("Failed to parse override: {}", e))?;

        self.set_command_override(workspace, key, override_config)
    }
}
