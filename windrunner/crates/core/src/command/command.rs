use std::{collections::BTreeMap, io, path::PathBuf, process::{self, ExitStatus}};

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
    pub env: BTreeMap<String, String>,
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
            env: BTreeMap::new(),
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

    pub fn execute(&self) -> io::Result<ExitStatus> {
        match self.strategy {
            CommandStrategy::Rustc => {
                // Extract the output filename from args (after -o flag)
                let mut output_name = None;
                for i in 0..self.args.len() {
                    if self.args[i] == "-o" && i + 1 < self.args.len() {
                        output_name = Some(&self.args[i + 1]);
                        break;
                    }
                }

                // First compile with rustc
                let mut rustc_cmd = process::Command::new("rustc");
                rustc_cmd.args(&self.args);

                // Set working directory if specified
                if let Some(ref dir) = self.working_dir {
                    rustc_cmd.current_dir(dir);
                }

                // Set environment variables
                for (key, value) in &self.env {
                    tracing::debug!("Setting env: {}={}", key, value);
                    rustc_cmd.env(key, value);
                }

                // Compile
                let compile_status = rustc_cmd.status()?;
                if !compile_status.success() {
                    return Ok(compile_status);
                }

                // If compilation succeeded and we have an output name, run it
                if let Some(output) = output_name {
                    // Check if output is an absolute path
                    let exec_path = if output.starts_with('/') || output.starts_with("./") {
                        output.to_string()
                    } else {
                        format!("./{output}")
                    };

                    let mut run_cmd = if self.pipe_command.is_some() {
                        // If we have a pipe command, we need to use shell
                        let mut cmd = process::Command::new("sh");
                        cmd.arg("-c");
                        cmd
                    } else {
                        process::Command::new(exec_path.clone())
                    };

                    // Build args based on whether we're using shell or not
                    if let Some(pipe_to) = &self.pipe_command {
                        // Build the full shell command
                        let mut shell_cmd = exec_path;

                        // Add test args if this is a test command
                        if self.args.contains(&"--test".to_string()) {
                            // Check if we have exec phase args (like --bench)
                            if let Some(exec_args) = &self.exec_args {
                                // Add exec args BEFORE the test filter
                                for arg in exec_args {
                                    if arg != "{bench_name}" && arg != "{test_name}" {
                                        shell_cmd.push_str(&format!(" {arg}"));
                                    }
                                }
                            }

                            if let Some(ref test_filter) = self.test_filter {
                                shell_cmd.push_str(&format!(" {test_filter}"));
                            }

                            // Add extra test binary args if present
                            if let Some(extra_args) = &self.test_binary_args {
                                // No separator needed for test binaries - args are mixed with test names
                                for arg in extra_args {
                                    shell_cmd.push_str(&format!(" {arg}"));
                                }
                            }
                        }

                        // Add the pipe command
                        shell_cmd.push_str(&format!(" | {pipe_to}"));

                        // Set the shell command as argument
                        run_cmd.arg(shell_cmd);
                    } else {
                        // Normal execution without shell
                        if self.args.contains(&"--test".to_string()) {
                            // Check if we have exec phase args (like --bench)
                            if let Some(exec_args) = &self.exec_args {
                                // Add exec args BEFORE the test filter
                                for arg in exec_args {
                                    if arg != "{bench_name}" && arg != "{test_name}" {
                                        run_cmd.arg(arg);
                                    }
                                }
                            }

                            if let Some(ref test_filter) = self.test_filter {
                                run_cmd.arg(test_filter);
                            }

                            // Add extra test binary args if present
                            if let Some(extra_args) = &self.test_binary_args {
                                // No separator needed for test binaries - args are mixed with test names
                                for arg in extra_args {
                                    run_cmd.arg(arg);
                                }
                            }
                        }
                    }

                    // Set working directory if specified
                    if let Some(ref dir) = self.working_dir {
                        run_cmd.current_dir(dir);
                    }

                    // Set environment variables
                    for (key, value) in &self.env {
                        run_cmd.env(key, value);
                    }

                    run_cmd.status()
                } else {
                    Ok(compile_status)
                }
            }
            CommandStrategy::Shell => {
                let mut cmd = process::Command::new(&self.program);
                cmd.args(&self.args);

                // Set working directory if specified
                if let Some(ref dir) = self.working_dir {
                    cmd.current_dir(dir);
                }

                // Set environment variables
                for (key, value) in &self.env {
                    tracing::debug!("Setting env: {}={}", key, value);
                    cmd.env(key, value);
                }

                cmd.status()
            }
            CommandStrategy::CargoScript | CommandStrategy::Cargo => {
                let mut cmd = process::Command::new("cargo");
                cmd.args(&self.args);

                // Set working directory if specified
                if let Some(ref dir) = self.working_dir {
                    cmd.current_dir(dir);
                }

                // Set environment variables
                for (key, value) in &self.env {
                    tracing::debug!("Setting env: {}={}", key, value);
                    cmd.env(key, value);
                }

                cmd.status()
            }
            CommandStrategy::Bazel => {
                let mut cmd = process::Command::new("bazel");
                cmd.args(&self.args);

                // Set working directory if specified
                if let Some(ref dir) = self.working_dir {
                    cmd.current_dir(dir);
                }

                // Set environment variables
                for (key, value) in &self.env {
                    tracing::debug!("Setting env: {}={}", key, value);
                    cmd.env(key, value);
                }

                cmd.status()
            }
        }
    }
}
