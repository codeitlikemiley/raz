use std::{
    collections::HashMap,
    io,
    path::PathBuf,
    process::{self, ExitStatus},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandStrategy {
    Cargo,
    CargoScript,
    Rustc,
    Shell,
    Bazel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    pub strategy: CommandStrategy,
    pub program: String,
    pub args: Vec<String>,
    pub working_dir: Option<PathBuf>,
    pub env: HashMap<String, String>,
    pub test_filter: Option<String>,
    /// Rustc-specific: args to pass to the compiled binary during execution
    pub exec_args: Option<Vec<String>>,
    /// Rustc-specific: pipe output through this command
    pub pipe_command: Option<String>,
    /// Rustc-specific: extra args for test binary
    pub test_binary_args: Option<Vec<String>>,
}

impl Command {
    pub fn new(strategy: CommandStrategy, program: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            strategy,
            program: program.into(),
            args,
            working_dir: None,
            env: HashMap::new(),
            test_filter: None,
            exec_args: None,
            pipe_command: None,
            test_binary_args: None,
        }
    }

    pub fn cargo(args: Vec<String>) -> Self {
        Self::new(CommandStrategy::Cargo, "cargo", args)
    }

    pub fn rustc(args: Vec<String>) -> Self {
        Self::new(CommandStrategy::Rustc, "rustc", args)
    }

    pub fn shell(program: impl Into<String>, args: Vec<String>) -> Self {
        Self::new(CommandStrategy::Shell, program, args)
    }

    pub fn cargo_script(args: Vec<String>) -> Self {
        Self::new(CommandStrategy::CargoScript, "cargo", args)
    }

    pub fn bazel(args: Vec<String>) -> Self {
        Self::new(CommandStrategy::Bazel, "bazel", args)
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
        match self.strategy {
            CommandStrategy::Rustc => {
                let mut cmd = String::from("rustc");
                for arg in &self.args {
                    cmd.push(' ');
                    if arg.contains(' ') && !arg.starts_with('\'') {
                        cmd.push_str(&format!("'{arg}'"));
                    } else {
                        cmd.push_str(arg);
                    }
                }

                // Extract output name and append run command
                for i in 0..self.args.len() {
                    if self.args[i] == "-o" && i + 1 < self.args.len() {
                        let output = &self.args[i + 1];
                        // Check if output is an absolute path
                        let exec_path = if output.starts_with('/') || output.starts_with("./") {
                            output.to_string()
                        } else {
                            format!("./{output}")
                        };
                        cmd.push_str(&format!(" && {exec_path}"));

                        // If this is a test command with a filter, add it
                        if self.args.contains(&"--test".to_string()) {
                            // Check if we have exec phase args (like --bench)
                            if let Some(exec_args) = &self.exec_args {
                                // Add exec args BEFORE the test filter
                                for arg in exec_args {
                                    if arg != "{bench_name}" && arg != "{test_name}" {
                                        cmd.push_str(&format!(" {arg}"));
                                    }
                                }
                            }

                            if let Some(ref test_filter) = self.test_filter {
                                cmd.push_str(&format!(" {test_filter}"));
                            }

                            // Add extra test binary args if present
                            if let Some(extra_args) = &self.test_binary_args {
                                // No separator for test binaries - args are mixed with test names
                                for arg in extra_args {
                                    cmd.push_str(&format!(" {arg}"));
                                }
                            }
                        }

                        // Add pipe command if present
                        if let Some(pipe_cmd) = &self.pipe_command {
                            cmd.push_str(&format!(" | {pipe_cmd}"));
                        }

                        break;
                    }
                }
                cmd
            }
            CommandStrategy::Shell => {
                // For shell commands, first arg is the command itself
                let mut cmd = self.program.clone();
                for arg in &self.args {
                    cmd.push(' ');
                    if arg.contains(' ') && !arg.starts_with('\'') {
                        cmd.push_str(&format!("'{arg}'"));
                    } else {
                        cmd.push_str(arg);
                    }
                }
                cmd
            }
            CommandStrategy::CargoScript | CommandStrategy::Cargo => {
                let mut cmd = String::from("cargo");
                for arg in &self.args {
                    cmd.push(' ');
                    if arg.contains(' ') && !arg.starts_with('\'') {
                        cmd.push_str(&format!("'{arg}'"));
                    } else {
                        cmd.push_str(arg);
                    }
                }
                cmd
            }
            CommandStrategy::Bazel => {
                let mut cmd = String::from("bazel");
                for arg in &self.args {
                    cmd.push(' ');
                    if arg.contains(' ') && !arg.starts_with('\'') {
                        cmd.push_str(&format!("'{arg}'"));
                    } else {
                        cmd.push_str(arg);
                    }
                }
                cmd
            }
        }
    }

    fn build_process(&self, program: &str, add_args: bool) -> process::Command {
        let mut cmd = process::Command::new(program);
        if add_args {
            cmd.args(&self.args);
        }
        if let Some(ref dir) = self.working_dir {
            cmd.current_dir(dir);
        }
        for (key, value) in &self.env {
            cmd.env(key, value);
        }
        cmd
    }

    fn apply_test_args_to_shell_cmd(&self, shell_cmd: &mut String) {
        if self.args.contains(&"--test".to_string()) {
            if let Some(exec_args) = &self.exec_args {
                for arg in exec_args {
                    if arg != "{bench_name}" && arg != "{test_name}" {
                        shell_cmd.push_str(&format!(" {arg}"));
                    }
                }
            }
            if let Some(ref test_filter) = self.test_filter {
                shell_cmd.push_str(&format!(" {test_filter}"));
            }
            if let Some(extra_args) = &self.test_binary_args {
                for arg in extra_args {
                    shell_cmd.push_str(&format!(" {arg}"));
                }
            }
        }
    }

    fn apply_test_args_to_run_cmd(&self, run_cmd: &mut process::Command) {
        if self.args.contains(&"--test".to_string()) {
            if let Some(exec_args) = &self.exec_args {
                for arg in exec_args {
                    if arg != "{bench_name}" && arg != "{test_name}" {
                        run_cmd.arg(arg);
                    }
                }
            }
            if let Some(ref test_filter) = self.test_filter {
                run_cmd.arg(test_filter);
            }
            if let Some(extra_args) = &self.test_binary_args {
                for arg in extra_args {
                    run_cmd.arg(arg);
                }
            }
        }
    }

    pub fn execute(&self) -> io::Result<ExitStatus> {
        match self.strategy {
            CommandStrategy::Rustc => {
                let mut output_name = None;
                for i in 0..self.args.len() {
                    if self.args[i] == "-o" && i + 1 < self.args.len() {
                        output_name = Some(&self.args[i + 1]);
                        break;
                    }
                }

                let mut rustc_cmd = self.build_process("rustc", true);
                let compile_status = rustc_cmd.status()?;
                if !compile_status.success() {
                    return Ok(compile_status);
                }

                if let Some(output) = output_name {
                    let exec_path = if output.starts_with('/') || output.starts_with("./") {
                        output.to_string()
                    } else {
                        format!("./{output}")
                    };

                    let mut run_cmd = if self.pipe_command.is_some() {
                        let mut cmd = self.build_process("sh", false);
                        cmd.arg("-c");
                        cmd
                    } else {
                        self.build_process(&exec_path, false)
                    };

                    if let Some(pipe_to) = &self.pipe_command {
                        let mut shell_cmd = exec_path;
                        self.apply_test_args_to_shell_cmd(&mut shell_cmd);
                        shell_cmd.push_str(&format!(" | {pipe_to}"));
                        run_cmd.arg(shell_cmd);
                    } else {
                        self.apply_test_args_to_run_cmd(&mut run_cmd);
                    }

                    run_cmd.status()
                } else {
                    Ok(compile_status)
                }
            }
            CommandStrategy::Shell => self.build_process(&self.program, true).status(),
            CommandStrategy::CargoScript | CommandStrategy::Cargo => {
                self.build_process("cargo", true).status()
            }
            CommandStrategy::Bazel => self.build_process("bazel", true).status(),
        }
    }
}
