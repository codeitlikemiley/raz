use crate::command::{CargoCommand, CommandType};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io,
    path::{Path, PathBuf},
    process::ExitStatus,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandStrategy {
    Cargo,
    CargoScript,
    Rustc,
    Shell,
    Bazel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandSpec {
    pub strategy: CommandStrategy,
    pub program: String,
    pub args: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_filter: Option<String>,
}

impl CommandSpec {
    pub fn new(strategy: CommandStrategy, program: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            strategy,
            program: program.into(),
            args,
            working_dir: None,
            env: BTreeMap::new(),
            test_filter: None,
        }
    }

    pub fn shell(program: impl Into<String>, args: Vec<String>) -> Self {
        Self::new(CommandStrategy::Shell, program, args)
    }

    pub fn with_working_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    pub fn with_test_filter(mut self, filter: impl Into<String>) -> Self {
        self.test_filter = Some(filter.into());
        self
    }

    pub fn to_shell_command(&self) -> String {
        let command = self.clone().into_cargo_command();
        command.to_shell_command()
    }

    pub fn execute(&self) -> io::Result<ExitStatus> {
        self.clone().into_cargo_command().execute()
    }

    pub fn working_dir(&self) -> Option<&Path> {
        self.working_dir.as_deref()
    }

    pub fn into_cargo_command(self) -> CargoCommand {
        let mut command = match self.strategy {
            CommandStrategy::Cargo => CargoCommand::new(self.args),
            CommandStrategy::CargoScript => CargoCommand::new_rust_sf_script(self.args),
            CommandStrategy::Rustc => CargoCommand::new_rustc(self.args),
            CommandStrategy::Shell => CargoCommand::new_shell(self.program, self.args),
            CommandStrategy::Bazel => CargoCommand::new_bazel(self.args),
        };

        if let Some(dir) = self.working_dir {
            command = command.with_working_dir(dir.to_string_lossy().to_string());
        }

        for (key, value) in self.env {
            command = command.with_env(key, value);
        }

        if let Some(filter) = self.test_filter {
            command = command.with_test_filter(filter);
        }

        command
    }
}

impl From<CargoCommand> for CommandSpec {
    fn from(command: CargoCommand) -> Self {
        let strategy = match command.command_type {
            CommandType::Cargo => CommandStrategy::Cargo,
            CommandType::Rustc => CommandStrategy::Rustc,
            CommandType::Shell => CommandStrategy::Shell,
            CommandType::RustSFScript => CommandStrategy::CargoScript,
            CommandType::Bazel => CommandStrategy::Bazel,
        };

        let (program, args) = match strategy {
            CommandStrategy::Shell => command
                .args
                .split_first()
                .map(|(program, args)| (program.clone(), args.to_vec()))
                .unwrap_or_else(|| (String::new(), Vec::new())),
            _ => match command.command_type {
                CommandType::Bazel => ("bazel".to_string(), command.args),
                CommandType::Rustc => ("rustc".to_string(), command.args),
                _ => ("cargo".to_string(), command.args),
            },
        };

        Self {
            strategy,
            program,
            args,
            working_dir: command.working_dir.map(PathBuf::from),
            env: command.env.into_iter().collect(),
            test_filter: command.test_filter,
        }
    }
}
