import re

path_ur = "crates/core/src/runners/unified_runner.rs"
with open(path_ur, "r") as f:
    t = f.read()

t = t.replace("use std::collections::HashMap;", "use std::collections::HashMap;\nuse crate::runners::cargo_runner::CargoRunner;\nuse crate::runners::bazel_runner::BazelRunner;")

helper = """fn init_runners() -> Result<HashMap<BuildSystem, Box<dyn CommandRunner<Config = Config, Command = crate::command::CargoCommand>>>> {
    let mut runners = HashMap::new();
    runners.insert(
        BuildSystem::Cargo,
        Box::new(CargoRunner::new()?) as Box<dyn CommandRunner<Config = Config, Command = crate::command::CargoCommand>>,
    );
    runners.insert(
        BuildSystem::Bazel,
        Box::new(BazelRunner::new()?) as Box<dyn CommandRunner<Config = Config, Command = crate::command::CargoCommand>>,
    );
    Ok(runners)
}
"""

if "init_runners" not in t:
    t = t.replace("impl UnifiedRunner {", helper + "\nimpl UnifiedRunner {")

# To accurately replace, I will use precise strings for `new` and `with_config` based on their known original source
orig_new = """    pub fn new() -> Result<Self> {
        let mut runners = HashMap::new();

        // Initialize all runners
        runners.insert(
            BuildSystem::Cargo,
            Box::new(CargoRunner::new()?)
                as Box<dyn CommandRunner<Config = Config, Command = crate::command::CargoCommand>>,
        );
        runners.insert(
            BuildSystem::Bazel,
            Box::new(BazelRunner::new()?)
                as Box<dyn CommandRunner<Config = Config, Command = crate::command::CargoCommand>>,
        );

        // Load config
        let config = Config::load()?;
        let plugins = PluginRegistry::with_defaults();

        Ok(Self {
            runners,
            plugins,
            config,
        })
    }"""

new_new = """    pub fn new() -> Result<Self> {
        let runners = init_runners()?;
        let config = Config::load()?;
        let plugins = PluginRegistry::with_defaults();

        Ok(Self {
            runners,
            plugins,
            config,
        })
    }"""

t = t.replace(orig_new, new_new)

orig_with_config = """    pub fn with_config(config: Config) -> Result<Self> {
        let mut runners = HashMap::new();

        runners.insert(
            BuildSystem::Cargo,
            Box::new(CargoRunner::new()?)
                as Box<dyn CommandRunner<Config = Config, Command = crate::command::CargoCommand>>,
        );
        runners.insert(
            BuildSystem::Bazel,
            Box::new(BazelRunner::new()?)
                as Box<dyn CommandRunner<Config = Config, Command = crate::command::CargoCommand>>,
        );

        let plugins = PluginRegistry::with_defaults();

        Ok(Self {
            runners,
            plugins,
            config,
        })
    }"""

new_with_config = """    pub fn with_config(config: Config) -> Result<Self> {
        let runners = init_runners()?;
        let plugins = PluginRegistry::with_defaults();

        Ok(Self {
            runners,
            plugins,
            config,
        })
    }"""

t = t.replace(orig_with_config, new_with_config)

with open(path_ur, "w") as f:
    f.write(t)
