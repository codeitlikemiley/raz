//! Command structures and execution logic

use crate::browser::{extract_server_url, open_browser};
use crate::error::{RazError, RazResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;

/// Represents a command that can be executed
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Command {
    /// Unique identifier for this command
    pub id: String,

    /// Human-readable label for display
    pub label: String,

    /// Optional description providing more context
    pub description: Option<String>,

    /// The actual command to execute
    pub command: String,

    /// Command arguments
    pub args: Vec<String>,

    /// Environment variables to set
    pub env: HashMap<String, String>,

    /// Working directory for execution
    pub cwd: Option<PathBuf>,

    /// Command category for grouping
    pub category: CommandCategory,

    /// Priority for sorting (higher = more important)
    pub priority: u8,

    /// Conditions that must be met for this command to be available
    pub conditions: Vec<Condition>,

    /// Tags for filtering and search
    pub tags: Vec<String>,

    /// Whether this command requires user input
    pub requires_input: bool,

    /// Estimated execution time in seconds
    pub estimated_duration: Option<u32>,
}

impl Command {
    /// Create a new command builder
    pub fn builder(id: impl Into<String>, command: impl Into<String>) -> CommandBuilder {
        CommandBuilder::new(id, command)
    }

    /// Execute this command asynchronously
    pub async fn execute(&self) -> RazResult<ExecutionResult> {
        self.execute_with_options(false, None).await
    }

    /// Execute this command with browser launching options
    pub async fn execute_with_browser(
        &self,
        open_browser: bool,
        browser: Option<String>,
    ) -> RazResult<ExecutionResult> {
        self.execute_with_options(open_browser, browser).await
    }

    /// Internal execution with options
    async fn execute_with_options(
        &self,
        should_open_browser: bool,
        browser: Option<String>,
    ) -> RazResult<ExecutionResult> {
        // For long-running/interactive commands, we spawn and inherit stdio
        let is_interactive = self.is_interactive();

        if is_interactive {
            self.execute_interactive_with_browser(should_open_browser, browser)
                .await
        } else {
            self.execute_captured().await
        }
    }

    /// Check if this is an interactive/long-running command
    fn is_interactive(&self) -> bool {
        // Commands that typically run indefinitely or need user interaction
        let is_cargo_leptos = matches!(self.command.as_str(), "cargo-leptos");
        let has_serve_or_watch =
            self.args.contains(&"serve".to_string()) || self.args.contains(&"watch".to_string());
        let has_serve_tag = self.tags.contains(&"serve".to_string());
        let has_watch_tag = self.tags.contains(&"watch".to_string());
        let has_interactive_tag = self.tags.contains(&"interactive".to_string());

        (is_cargo_leptos && has_serve_or_watch)
            || has_serve_tag
            || has_watch_tag
            || has_interactive_tag
    }

    /// Execute command with inherited stdio for interactive processes
    async fn execute_interactive_with_browser(
        &self,
        should_open_browser: bool,
        browser: Option<String>,
    ) -> RazResult<ExecutionResult> {
        let mut cmd = TokioCommand::new(&self.command);
        cmd.args(&self.args);

        // Set environment variables
        for (key, value) in &self.env {
            cmd.env(key, value);
        }

        // Set working directory
        if let Some(cwd) = &self.cwd {
            cmd.current_dir(cwd);
        }

        let start_time = std::time::Instant::now();

        let is_server_cmd = self.is_server_command();

        if should_open_browser && is_server_cmd {
            // For server commands with browser launching, we need to capture initial output
            cmd.stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .stdin(Stdio::inherit());

            let mut child = cmd.spawn().map_err(|e| {
                RazError::execution(format!("Failed to spawn command '{}': {}", self.command, e))
            })?;

            // Spawn tasks to read and forward output while looking for server URL
            let stdout = child.stdout.take().unwrap();
            let stderr = child.stderr.take().unwrap();

            let browser_clone = browser.clone();
            let url_opened = Arc::new(tokio::sync::Mutex::new(false));
            let url_opened_clone = url_opened.clone();

            // Handle stdout
            let stdout_task = tokio::spawn(async move {
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();

                while let Ok(Some(line)) = lines.next_line().await {
                    println!("{line}");

                    // Check if we should open browser
                    let mut opened = url_opened_clone.lock().await;
                    if !*opened {
                        if let Some(url) = extract_server_url(&line) {
                            if let Err(e) = open_browser(&url, browser_clone.as_deref()) {
                                eprintln!("Warning: Failed to open browser: {e}");
                            } else {
                                println!("\n🌐 Opening {url} in browser...");
                            }
                            *opened = true;
                        }
                    }
                }
            });

            // Handle stderr
            let stderr_task = tokio::spawn(async move {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();

                while let Ok(Some(line)) = lines.next_line().await {
                    eprintln!("{line}");
                }
            });

            // Wait for process to complete
            let status = child.wait().await.map_err(|e| {
                RazError::execution(format!(
                    "Failed to wait for command '{}': {}",
                    self.command, e
                ))
            })?;

            // Clean up tasks
            let _ = stdout_task.await;
            let _ = stderr_task.await;

            let duration = start_time.elapsed();

            Ok(ExecutionResult {
                exit_code: status.code().unwrap_or(0),
                stdout: String::new(),
                stderr: String::new(),
                duration,
                command: self.clone(),
            })
        } else {
            // Standard interactive execution
            cmd.stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit());

            let mut child = cmd.spawn().map_err(|e| {
                RazError::execution(format!("Failed to spawn command '{}': {}", self.command, e))
            })?;

            let status = child.wait().await.map_err(|e| {
                RazError::execution(format!(
                    "Failed to wait for command '{}': {}",
                    self.command, e
                ))
            })?;

            let duration = start_time.elapsed();

            Ok(ExecutionResult {
                exit_code: status.code().unwrap_or(0),
                stdout: String::new(), // Output was inherited
                stderr: String::new(), // Output was inherited
                duration,
                command: self.clone(),
            })
        }
    }

    /// Check if this is a server command that might output a URL
    fn is_server_command(&self) -> bool {
        let has_serve_tag = self.tags.contains(&"serve".to_string());
        let has_server_tag = self.tags.contains(&"server".to_string());
        let has_dev_tag = self.tags.contains(&"dev".to_string());
        let has_watch_tag = self.tags.contains(&"watch".to_string());
        let is_cargo_leptos =
            self.command == "cargo" && self.args.first() == Some(&"leptos".to_string());
        let is_cargo_leptos_serve =
            self.command == "cargo-leptos" && self.args.contains(&"serve".to_string());
        let is_cargo_leptos_watch =
            self.command == "cargo-leptos" && self.args.contains(&"watch".to_string());
        let is_trunk_serve = self.command == "trunk" && self.args.contains(&"serve".to_string());
        let is_dx_serve = self.command == "dx" && self.args.contains(&"serve".to_string());

        has_serve_tag
            || has_server_tag
            || has_dev_tag
            || has_watch_tag
            || is_cargo_leptos
            || is_cargo_leptos_serve
            || is_cargo_leptos_watch
            || is_trunk_serve
            || is_dx_serve
    }

    /// Execute command with captured output
    async fn execute_captured(&self) -> RazResult<ExecutionResult> {
        let mut cmd = TokioCommand::new(&self.command);
        cmd.args(&self.args);

        // Set environment variables
        for (key, value) in &self.env {
            cmd.env(key, value);
        }

        // Set working directory
        if let Some(cwd) = &self.cwd {
            cmd.current_dir(cwd);
        }

        // Configure stdio
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let start_time = std::time::Instant::now();

        let output = cmd.output().await.map_err(|e| {
            RazError::execution(format!(
                "Failed to execute command '{}': {}",
                self.command, e
            ))
        })?;

        let duration = start_time.elapsed();

        Ok(ExecutionResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            duration,
            command: self.clone(),
        })
    }

    /// Check if all conditions for this command are met
    pub fn is_available(&self, context: &crate::ProjectContext) -> bool {
        self.conditions
            .iter()
            .all(|condition| condition.is_met(context))
    }

    /// Get the full command line as a string
    pub fn command_line(&self) -> String {
        if self.args.is_empty() {
            self.command.clone()
        } else {
            format!("{} {}", self.command, self.args.join(" "))
        }
    }
}

/// Builder for creating commands
pub struct CommandBuilder {
    command: Command,
}

impl CommandBuilder {
    pub fn new(id: impl Into<String>, command: impl Into<String>) -> Self {
        let command_str = command.into();
        Self {
            command: Command {
                id: id.into(),
                label: command_str.clone(),
                description: None,
                command: command_str,
                args: Vec::new(),
                env: HashMap::new(),
                cwd: None,
                category: CommandCategory::Custom("default".to_string()),
                priority: 50,
                conditions: Vec::new(),
                tags: Vec::new(),
                requires_input: false,
                estimated_duration: None,
            },
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.command.label = label.into();
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.command.description = Some(description.into());
        self
    }

    pub fn args(mut self, args: Vec<String>) -> Self {
        self.command.args = args;
        self
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.command.args.push(arg.into());
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.command.env.insert(key.into(), value.into());
        self
    }

    pub fn cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.command.cwd = Some(cwd.into());
        self
    }

    pub fn category(mut self, category: CommandCategory) -> Self {
        self.command.category = category;
        self
    }

    pub fn priority(mut self, priority: u8) -> Self {
        self.command.priority = priority;
        self
    }

    pub fn condition(mut self, condition: Condition) -> Self {
        self.command.conditions.push(condition);
        self
    }

    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.command.tags.push(tag.into());
        self
    }

    pub fn requires_input(mut self, requires_input: bool) -> Self {
        self.command.requires_input = requires_input;
        self
    }

    pub fn estimated_duration(mut self, seconds: u32) -> Self {
        self.command.estimated_duration = Some(seconds);
        self
    }

    pub fn build(self) -> Command {
        self.command
    }
}

/// Command execution result
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration: std::time::Duration,
    pub command: Command,
}

impl ExecutionResult {
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }

    pub fn output(&self) -> &str {
        if self.stdout.is_empty() {
            &self.stderr
        } else {
            &self.stdout
        }
    }
}

/// Categories for grouping commands
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CommandCategory {
    Build,
    Test,
    Run,
    Debug,
    Deploy,
    Lint,
    Format,
    Generate,
    Install,
    Update,
    Clean,
    Custom(String),
}

impl CommandCategory {
    pub fn as_str(&self) -> &str {
        match self {
            CommandCategory::Build => "build",
            CommandCategory::Test => "test",
            CommandCategory::Run => "run",
            CommandCategory::Debug => "debug",
            CommandCategory::Deploy => "deploy",
            CommandCategory::Lint => "lint",
            CommandCategory::Format => "format",
            CommandCategory::Generate => "generate",
            CommandCategory::Install => "install",
            CommandCategory::Update => "update",
            CommandCategory::Clean => "clean",
            CommandCategory::Custom(name) => name,
        }
    }
}

/// Conditions that determine when a command is available
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Condition {
    /// File must exist
    FileExists(PathBuf),

    /// File pattern must match
    FilePattern(String),

    /// Cursor must be in a function with this name
    CursorInFunction(String),

    /// Cursor must be in a struct with this name
    CursorInStruct(String),

    /// Cursor must be in a test function
    CursorInTest,

    /// Project must have this dependency
    HasDependency(String),

    /// Must be in a workspace
    InWorkspace,

    /// Must be on a specific platform
    Platform(String),

    /// Custom condition with expression
    Expression(String),
}

impl Condition {
    /// Check if this condition is met in the given context
    pub fn is_met(&self, context: &crate::ProjectContext) -> bool {
        match self {
            Condition::FileExists(path) => {
                let full_path = if path.is_absolute() {
                    path.clone()
                } else {
                    context.workspace_root.join(path)
                };
                full_path.exists()
            }
            Condition::FilePattern(pattern) => {
                Self::check_file_pattern(&context.workspace_root, pattern)
            }
            Condition::CursorInFunction(name) => {
                if let Some(file_context) = &context.current_file {
                    if let Some(symbol) = &file_context.cursor_symbol {
                        return symbol.name == *name && symbol.kind == crate::SymbolKind::Function;
                    }
                }
                false
            }
            Condition::CursorInStruct(name) => {
                if let Some(file_context) = &context.current_file {
                    if let Some(symbol) = &file_context.cursor_symbol {
                        return symbol.name == *name && symbol.kind == crate::SymbolKind::Struct;
                    }
                }
                false
            }
            Condition::CursorInTest => {
                if let Some(file_context) = &context.current_file {
                    if let Some(symbol) = &file_context.cursor_symbol {
                        return symbol.kind == crate::SymbolKind::Test;
                    }
                }
                false
            }
            Condition::HasDependency(dep) => context.dependencies.iter().any(|d| d.name == *dep),
            Condition::InWorkspace => context.workspace_members.len() > 1,
            Condition::Platform(platform) => {
                cfg!(target_os = "windows") && platform == "windows"
                    || cfg!(target_os = "macos") && platform == "macos"
                    || cfg!(target_os = "linux") && platform == "linux"
            }
            Condition::Expression(_expr) => {
                // TODO: Implement expression evaluation
                false
            }
        }
    }

    /// Check if any files matching the given pattern exist in the workspace
    fn check_file_pattern(workspace_root: &Path, pattern: &str) -> bool {
        // Create the full pattern by joining with workspace root
        let full_pattern = workspace_root.join(pattern);
        let pattern_str = full_pattern.to_string_lossy();

        // Use glob to find matching files
        match glob::glob(&pattern_str) {
            Ok(paths) => {
                // Check if any paths match
                paths.filter_map(Result::ok).next().is_some()
            }
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_builder() {
        let cmd = Command::builder("test-build", "cargo")
            .label("Build Project")
            .description("Build the project in debug mode")
            .arg("build")
            .category(CommandCategory::Build)
            .priority(10)
            .tag("cargo")
            .build();

        assert_eq!(cmd.id, "test-build");
        assert_eq!(cmd.command, "cargo");
        assert_eq!(cmd.args, vec!["build"]);
        assert_eq!(cmd.category, CommandCategory::Build);
        assert_eq!(cmd.priority, 10);
        assert!(cmd.tags.contains(&"cargo".to_string()));
    }

    #[test]
    fn test_command_line() {
        let cmd = Command::builder("test", "cargo")
            .args(vec!["test".to_string(), "--release".to_string()])
            .build();

        assert_eq!(cmd.command_line(), "cargo test --release");
    }

    #[tokio::test]
    async fn test_command_execution() {
        let cmd = Command::builder("echo-test", "echo").arg("hello").build();

        let result = cmd.execute().await.unwrap();
        assert!(result.is_success());
        assert_eq!(result.stdout.trim(), "hello");
    }

    #[tokio::test]
    async fn test_leptos_watch_command() {
        // Create a command that mimics the leptos-watch command from LeptosProvider
        let command = CommandBuilder::new("leptos-watch", "cargo-leptos")
            .label("Leptos Dev Watch")
            .description("Development server with auto-reload (recommended for development)")
            .arg("watch")
            .category(CommandCategory::Run)
            .priority(95)
            .tag("dev")
            .tag("watch")
            .tag("leptos")
            .estimated_duration(5)
            .build();

        // Test that the command is properly configured
        assert_eq!(command.command, "cargo-leptos");
        assert_eq!(command.args, vec!["watch"]);
        assert!(command.tags.contains(&"dev".to_string()));
        assert!(command.tags.contains(&"watch".to_string()));
        assert!(command.tags.contains(&"leptos".to_string()));

        // Note: Actual execution would fail without cargo-leptos installed
        // This test just verifies the command structure
    }
}
