import re
import os

# Task 2: validate_command never called. 
# We'll call it in builtins.rs inside `build_command`.
path_builtins = "crates/core/src/plugins/builtins.rs"
with open(path_builtins, "r") as f:
    t = f.read()

# Fix expect
t = t.replace("runner: BazelRunner::new().expect(\"BazelRunner::new should not fail\")", "runner: BazelRunner")
t = t.replace("runner: CargoRunner::new().expect(\"CargoRunner::new should not fail\")", "runner: CargoRunner")
t = t.replace("runner: RustcRunner::new().expect(\"RustcRunner::new should not fail\")", "runner: RustcRunner")

# Fix validate
t = t.replace("""    fn build_command(&self, target: &TargetRef, ctx: &ProjectContext) -> Result<CommandSpec> {
        let runnable = runnable_from_target(target)?;
        let command =
            self.runner
                .build_command(runnable, &ctx.config, file_type_for_runnable(runnable))?;
        Ok(to_spec(command))
    }""", """    fn build_command(&self, target: &TargetRef, ctx: &ProjectContext) -> Result<CommandSpec> {
        let runnable = runnable_from_target(target)?;
        let command =
            self.runner
                .build_command(runnable, &ctx.config, file_type_for_runnable(runnable))?;
        self.runner.validate_command(&command)?;
        Ok(to_spec(command))
    }""")

with open(path_builtins, "w") as f:
    f.write(t)


# Task 3: Extract UnifiedRunner initialization
path_ur = "crates/core/src/runners/unified_runner.rs"
with open(path_ur, "r") as f:
    t = f.read()

t = t.replace("use std::collections::HashMap;", "use std::collections::HashMap;\nuse crate::runners::cargo_runner::CargoRunner;\nuse crate::runners::bazel_runner::BazelRunner;")

helper = """
fn init_runners() -> Result<HashMap<BuildSystem, Box<dyn CommandRunner<Config = Config, Command = crate::command::CargoCommand>>>> {
    let mut runners = HashMap::new();
    runners.insert(
        BuildSystem::Cargo,
        Box::new(CargoRunner) as Box<dyn CommandRunner<Config = Config, Command = crate::command::CargoCommand>>,
    );
    runners.insert(
        BuildSystem::Bazel,
        Box::new(BazelRunner) as Box<dyn CommandRunner<Config = Config, Command = crate::command::CargoCommand>>,
    );
    Ok(runners)
}
"""

if "init_runners" not in t:
    t = t.replace("impl UnifiedRunner {", helper + "\nimpl UnifiedRunner {")

new_body1 = """    pub fn new() -> Result<Self> {
        let runners = init_runners()?;
        let config = Config::load()?;
        let plugins = PluginRegistry::with_defaults();
        Ok(Self {
            runners,
            plugins,
            config,
        })
    }"""
t = re.sub(r"    pub fn new\(\) -> Result<Self> \{.*?    \}", new_body1, t, flags=re.DOTALL)

new_body2 = """    pub fn with_config(config: Config) -> Result<Self> {
        let runners = init_runners()?;
        let plugins = PluginRegistry::with_defaults();
        Ok(Self {
            runners,
            plugins,
            config,
        })
    }"""
t = re.sub(r"    pub fn with_config\(config: Config\) -> Result<Self> \{.*?    \}", new_body2, t, flags=re.DOTALL)

with open(path_ur, "w") as f:
    f.write(t)

# Task 4: Simplify CommandBuilder match arms
path_bd = "crates/core/src/command/builder/mod.rs"
with open(path_bd, "r") as f:
    t = f.read()

replacement = """        // Standalone routing optimization
        if file_type == FileType::Standalone {
            if matches!(self.runnable.kind, RunnableKind::DocTest { .. }) {
                return Err(crate::error::Error::ParseError(
                    "Doc tests are not supported in standalone files".to_string(),
                ));
            }
            if matches!(self.runnable.kind, RunnableKind::SingleFileScript { .. }) {
                return SingleFileScriptBuilder::build(
                    self.runnable,
                    self.package_name.as_deref(),
                    &config,
                    file_type,
                );
            }
            return RustcCommandBuilder::build(
                self.runnable,
                self.package_name.as_deref(),
                &config,
                file_type,
            );
        }

        // Delegate to specific builders based on file type first, then kind
        match (file_type, &self.runnable.kind) {
            // Single file scripts
"""

t = re.sub(r"        // Delegate to specific builders based on file type first, then kind\n        match \(file_type, &self.runnable.kind\) \{.*?// Single file scripts\n", replacement, t, flags=re.DOTALL)

# Task 8: Remove debug log in command/builder/mod.rs
t = re.sub(r'        tracing::debug!\(\n            "Routing to BinaryCommandBuilder.*?\n        \);', '', t, flags=re.DOTALL)
t = re.sub(r'                if let Ok\(ref cmd\) = result \{\n                    tracing::debug!\("BinaryCommandBuilder returned command: \{\:\?\}", cmd.args\);\n                \}', '', t, flags=re.DOTALL)
t = re.sub(r'                tracing::debug!\("Routing to TestCommandBuilder"\);\n', '', t, flags=re.DOTALL)

with open(path_bd, "w") as f:
    f.write(t)

# Task 6: FunctionIdentity matches
path_fi = "crates/core/src/types/function_identity.rs"
with open(path_fi, "r") as f:
    t = f.read()

helper_macro = """
macro_rules! match_opt_field {
    ($self:ident, $other:ident, $field:ident) => {
        if let Some(ref my_val) = $self.$field {
            if let Some(ref other_val) = $other.$field {
                if my_val != other_val {
                    return false;
                }
            } else {
                return false;
            }
        }
    };
}
"""

replacement_fn = """    pub fn matches(&self, other: &FunctionIdentity) -> bool {
        match_opt_field!(self, other, package);
        match_opt_field!(self, other, module_path);
        match_opt_field!(self, other, function_name);
        match_opt_field!(self, other, file_type);

        if let Some(ref my_file) = self.file_path {
            if let Some(ref other_file) = other.file_path {
                if !paths_match(my_file, other_file) {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }"""

if "macro_rules! match_opt_field" not in t:
    t = t.replace("impl FunctionIdentity {", helper_macro + "\nimpl FunctionIdentity {")

t = re.sub(r"    pub fn matches\(&self, other: &FunctionIdentity\) -> bool \{.*?true\n    \}", replacement_fn, t, flags=re.DOTALL)
with open(path_fi, "w") as f:
    f.write(t)

