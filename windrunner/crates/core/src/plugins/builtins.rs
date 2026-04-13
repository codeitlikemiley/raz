use crate::{
    command::CargoCommand,
    error::{Error, Result},
    plugins::{
        CommandSpec, CommandStrategy, ProjectContext, RustSourceAnalyzer, SourceAnalyzer, TargetRef,
    },
    runners::{
        bazel_runner::BazelRunner, cargo_runner::CargoRunner, rustc_runner::RustcRunner,
        traits::CommandRunner,
    },
    types::{FileType, Runnable, RunnableKind},
};

pub struct BazelPrimaryPlugin {
    analyzer: RustSourceAnalyzer,
    runner: BazelRunner,
}

pub struct CargoPrimaryPlugin {
    analyzer: RustSourceAnalyzer,
    runner: CargoRunner,
}

pub struct RustcPrimaryPlugin {
    analyzer: RustSourceAnalyzer,
    runner: RustcRunner,
}

pub struct DioxusOverlayPlugin;
pub struct LeptosOverlayPlugin;

impl Default for BazelPrimaryPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl BazelPrimaryPlugin {
    pub fn new() -> Self {
        Self {
            analyzer: RustSourceAnalyzer,
            runner: BazelRunner::new().expect("BazelRunner::new should not fail"),
        }
    }
}

impl Default for CargoPrimaryPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl CargoPrimaryPlugin {
    pub fn new() -> Self {
        Self {
            analyzer: RustSourceAnalyzer,
            runner: CargoRunner::new().expect("CargoRunner::new should not fail"),
        }
    }
}

impl Default for RustcPrimaryPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl RustcPrimaryPlugin {
    pub fn new() -> Self {
        Self {
            analyzer: RustSourceAnalyzer,
            runner: RustcRunner::new().expect("RustcRunner::new should not fail"),
        }
    }
}

impl Default for DioxusOverlayPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl DioxusOverlayPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LeptosOverlayPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl LeptosOverlayPlugin {
    pub fn new() -> Self {
        Self
    }
}

fn file_type_for_runnable(runnable: &Runnable) -> FileType {
    match runnable.kind {
        RunnableKind::SingleFileScript { .. } => FileType::SingleFileScript,
        RunnableKind::Standalone { .. } => FileType::Standalone,
        _ => FileType::CargoProject,
    }
}

fn runnable_from_target(target: &TargetRef) -> Result<&Runnable> {
    target.runnable.as_ref().ok_or_else(|| {
        Error::Other(format!(
            "Target '{}' does not carry a runnable",
            target.label
        ))
    })
}

fn to_spec(command: CargoCommand) -> CommandSpec {
    CommandSpec::from(command)
}

impl crate::plugins::registry::PrimaryPlugin for BazelPrimaryPlugin {
    fn id(&self) -> &'static str {
        "bazel"
    }

    fn default_priority(&self) -> i32 {
        300
    }

    fn matches(&self, ctx: &ProjectContext) -> bool {
        ctx.has_manifest("BUILD.bazel") || ctx.has_manifest("BUILD")
    }

    fn discover_targets(&self, ctx: &ProjectContext, line: Option<u32>) -> Result<Vec<TargetRef>> {
        self.analyzer.analyze(ctx, line)
    }

    fn build_command(&self, target: &TargetRef, ctx: &ProjectContext) -> Result<CommandSpec> {
        let runnable = runnable_from_target(target)?;
        let command =
            self.runner
                .build_command(runnable, &ctx.config, file_type_for_runnable(runnable))?;
        Ok(to_spec(command))
    }
}

impl crate::plugins::registry::PrimaryPlugin for CargoPrimaryPlugin {
    fn id(&self) -> &'static str {
        "cargo"
    }

    fn default_priority(&self) -> i32 {
        200
    }

    fn matches(&self, ctx: &ProjectContext) -> bool {
        ctx.has_manifest("Cargo.toml")
    }

    fn discover_targets(&self, ctx: &ProjectContext, line: Option<u32>) -> Result<Vec<TargetRef>> {
        self.analyzer.analyze(ctx, line)
    }

    fn build_command(&self, target: &TargetRef, ctx: &ProjectContext) -> Result<CommandSpec> {
        let runnable = runnable_from_target(target)?;
        let command =
            self.runner
                .build_command(runnable, &ctx.config, file_type_for_runnable(runnable))?;
        Ok(to_spec(command))
    }
}

impl crate::plugins::registry::PrimaryPlugin for RustcPrimaryPlugin {
    fn id(&self) -> &'static str {
        "rustc"
    }

    fn default_priority(&self) -> i32 {
        100
    }

    fn matches(&self, ctx: &ProjectContext) -> bool {
        ctx.file_path.extension().and_then(|s| s.to_str()) == Some("rs")
    }

    fn discover_targets(&self, ctx: &ProjectContext, line: Option<u32>) -> Result<Vec<TargetRef>> {
        self.analyzer.analyze(ctx, line)
    }

    fn build_command(&self, target: &TargetRef, ctx: &ProjectContext) -> Result<CommandSpec> {
        let runnable = runnable_from_target(target)?;
        let command =
            self.runner
                .build_command(runnable, &ctx.config, file_type_for_runnable(runnable))?;
        Ok(to_spec(command))
    }
}

impl crate::plugins::registry::OverlayPlugin for DioxusOverlayPlugin {
    fn id(&self) -> &'static str {
        "dioxus"
    }

    fn default_priority(&self) -> i32 {
        200
    }

    fn matches(&self, primary_id: &str, ctx: &ProjectContext, target: &TargetRef) -> bool {
        primary_id == "cargo"
            && ctx.has_manifest("Dioxus.toml")
            && target
                .runnable
                .as_ref()
                .is_some_and(|r| matches!(r.kind, RunnableKind::Binary { .. }))
    }

    fn augment_command(
        &self,
        command: CommandSpec,
        target: &TargetRef,
        ctx: &ProjectContext,
    ) -> Result<CommandSpec> {
        let runnable = runnable_from_target(target)?;
        if !matches!(runnable.kind, RunnableKind::Binary { .. }) {
            return Ok(command);
        }

        let mut command_name = "dx".to_string();
        let mut subcommand = "serve".to_string();
        let mut extra_args = Vec::new();
        let mut env = command.env.clone();

        if let Some(override_config) = ctx
            .config
            .get_override_for(&crate::types::FunctionIdentity {
                package: ctx.config.cargo.as_ref().and_then(|c| c.package.clone()),
                module_path: if runnable.module_path.is_empty() {
                    None
                } else {
                    Some(runnable.module_path.clone())
                },
                file_path: Some(runnable.file_path.clone()),
                function_name: runnable.get_function_name(),
                file_type: Some(file_type_for_runnable(runnable)),
            })
            .and_then(|o| o.cargo.as_ref())
        {
            if let Some(cmd) = &override_config.command {
                command_name = cmd.clone();
            }
            if let Some(sub) = &override_config.subcommand {
                subcommand = sub.clone();
            }
            if let Some(args) = &override_config.extra_args {
                extra_args.extend(args.clone());
            }
            if let Some(extra_env) = &override_config.extra_env {
                env.extend(extra_env.iter().map(|(k, v)| (k.clone(), v.clone())));
            }
        }

        Ok(CommandSpec {
            strategy: CommandStrategy::Shell,
            program: command_name,
            args: {
                let mut args = vec![subcommand];
                args.extend(extra_args);
                args
            },
            working_dir: command.working_dir,
            env,
            test_filter: command.test_filter,
        })
    }
}

impl crate::plugins::registry::OverlayPlugin for LeptosOverlayPlugin {
    fn id(&self) -> &'static str {
        "leptos"
    }

    fn default_priority(&self) -> i32 {
        150
    }

    fn matches(&self, primary_id: &str, ctx: &ProjectContext, target: &TargetRef) -> bool {
        primary_id == "cargo"
            && ctx
                .manifest("Cargo.toml")
                .and_then(|cargo_toml| std::fs::read_to_string(cargo_toml).ok())
                .map(|content| content.contains("leptos"))
                .unwrap_or(false)
            && target
                .runnable
                .as_ref()
                .is_some_and(|r| matches!(r.kind, RunnableKind::Binary { .. }))
    }

    fn augment_command(
        &self,
        command: CommandSpec,
        target: &TargetRef,
        ctx: &ProjectContext,
    ) -> Result<CommandSpec> {
        let runnable = runnable_from_target(target)?;
        if !matches!(runnable.kind, RunnableKind::Binary { .. }) {
            return Ok(command);
        }

        let override_config = ctx
            .config
            .get_override_for(&crate::types::FunctionIdentity {
                package: ctx.config.cargo.as_ref().and_then(|c| c.package.clone()),
                module_path: if runnable.module_path.is_empty() {
                    None
                } else {
                    Some(runnable.module_path.clone())
                },
                file_path: Some(runnable.file_path.clone()),
                function_name: runnable.get_function_name(),
                file_type: Some(file_type_for_runnable(runnable)),
            });

        let mut subcommand = "serve".to_string();
        let mut extra_args = Vec::new();
        let mut env = command.env.clone();

        if let Some(ov) = override_config.and_then(|o| o.cargo.as_ref()) {
            if let Some(sub) = &ov.subcommand {
                subcommand = if let Some(stripped) = sub.strip_prefix("leptos ") {
                    stripped.to_string()
                } else {
                    sub.clone()
                };
            }
            if let Some(args) = &ov.extra_args {
                extra_args.extend(args.clone());
            }
            if let Some(extra_env) = &ov.extra_env {
                env.extend(extra_env.iter().map(|(k, v)| (k.clone(), v.clone())));
            }
        }

        Ok(CommandSpec {
            strategy: CommandStrategy::Cargo,
            program: "cargo".to_string(),
            args: {
                let mut args = vec!["leptos".to_string(), subcommand];
                args.extend(extra_args);
                args
            },
            working_dir: command.working_dir,
            env,
            test_filter: command.test_filter,
        })
    }
}
