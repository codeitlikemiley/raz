use crate::{
    config::PluginPolicy,
    error::Result,
    plugins::{CommandSpec, OverlayPlugin, ProjectContext, TargetRef},
};

pub struct CustomOverlayPlugin {
    id: String,
}

impl CustomOverlayPlugin {
    pub fn new(id: String) -> Self {
        Self { id }
    }
}

impl OverlayPlugin for CustomOverlayPlugin {
    fn id(&self) -> &str {
        &self.id
    }

    fn default_priority(&self) -> i32 {
        50
    }

    fn matches(&self, _primary_id: &str, ctx: &ProjectContext, _target: &TargetRef) -> bool {
        ctx.config.plugins.contains_key(&self.id)
    }

    fn augment_command(
        &self,
        mut command: CommandSpec,
        _target: &TargetRef,
        ctx: &ProjectContext,
    ) -> Result<CommandSpec> {
        if let Some(policy) = ctx.config.plugins.get(&self.id) {
            if let Some(settings) = &policy.settings {
                if let Some(cmd) = settings.get("command").and_then(|c| c.as_str()) {
                    command.program = cmd.to_string();
                }
                
                if let Some(args) = settings.get("args").and_then(|a| a.as_array()) {
                    let mut new_args = Vec::new();
                    for arg in args {
                        if let Some(s) = arg.as_str() {
                            new_args.push(s.to_string());
                        }
                    }
                    command.args = new_args.into_iter().chain(command.args.into_iter()).collect();
                }
            }
        }
        Ok(command)
    }
}
