use std::path::Path;
use std::sync::Arc;

use crate::{
    command::builder::rustc::single_file_script_builder::is_single_file_script_file,
    command::fallback::generate_fallback_command,
    error::Result,
    plugins::{ProjectContext, TargetRef},
    types::{FileType, Runnable},
};

use super::UnifiedRunner;

impl UnifiedRunner {
    /// Detect all runnables in a file
    pub fn detect_runnables(&self, file_path: &Path) -> Result<Vec<Runnable>> {
        let ctx = ProjectContext::from_path(file_path, Arc::clone(&self.config));
        let targets = self.plugins.discover_targets(&ctx, None)?;
        Ok(targets
            .into_iter()
            .filter_map(TargetRef::into_runnable)
            .collect())
    }

    /// Get the best runnable at a specific line
    pub fn get_runnable_at_line(&self, file_path: &Path, line: u32) -> Result<Option<Runnable>> {
        let ctx = ProjectContext::from_path(file_path, Arc::clone(&self.config));
        let targets = self.plugins.discover_targets(&ctx, Some(line))?;
        Ok(targets.into_iter().find_map(TargetRef::into_runnable))
    }

    /// Build a command for a runnable
    pub fn build_command(&self, runnable: &Runnable) -> Result<crate::command::Command> {
        tracing::debug!(
            "UnifiedRunner::build_command called: kind={:?}, file_path={:?}",
            runnable.kind,
            runnable.file_path
        );

        let ctx = ProjectContext::from_path(&runnable.file_path, Arc::clone(&self.config));
        let target = TargetRef::from_runnable("rust", runnable.clone());
        let command = self.plugins.build_command_for_target(&ctx, &target)?;

        tracing::debug!(
            "UnifiedRunner::build_command: final command={}",
            command.to_shell_command()
        );

        // Warn if Bazel doc test limitation is present
        if command
            .env
            .iter()
            .any(|(k, _)| k == "_BAZEL_DOC_TEST_LIMITATION")
        {
            tracing::warn!(
                "Bazel limitation: individual doc tests cannot be targeted. \
                 Running all doc tests in the target. \
                 Use `cargo test --doc` for per-function filtering."
            );
        }

        Ok(command)
    }

    /// Build a command for a position in a file
    pub fn build_command_at_position(
        &self,
        file_path: &Path,
        line: Option<u32>,
    ) -> Result<crate::command::Command> {
        tracing::debug!(
            "UnifiedRunner::build_command_at_position: file_path={:?}, line={:?}",
            file_path,
            line
        );

        if is_single_file_script_file(file_path) {
            tracing::debug!(
                "build_command_at_position: single-file script detected, using file-level command"
            );
            let cargo_root = file_path
                .ancestors()
                .find(|p| p.join("Cargo.toml").exists())
                .map(|p| p.to_path_buf());
            let package_name = self.get_package_name_str(file_path).ok();

            return generate_fallback_command(
                file_path,
                package_name.as_deref(),
                cargo_root.as_deref(),
                Some((*self.config).clone()),
            )?
            .ok_or(crate::error::Error::NoRunnableFound);
        }

        let runnable = if let Some(line_num) = line {
            // Try to get runnable at specific line
            if let Some(runnable) = self.get_runnable_at_line(file_path, line_num)? {
                runnable
            } else {
                // No runnable found at the specific line - fail fast with helpful error
                let all_runnables = self.detect_runnables(file_path)?;
                if all_runnables.is_empty() {
                    return Err(crate::error::Error::NoRunnableFound);
                }

                // Provide helpful error message showing available lines
                let available_lines: Vec<String> = all_runnables
                    .iter()
                    .map(|r| {
                        if matches!(r.kind, crate::types::RunnableKind::DocTest { .. }) {
                            // For doc tests, show the extended range if available
                            if let Some(ext) = &r.extended_scope {
                                let doc_start =
                                    ext.scope.start.line.saturating_sub(ext.doc_comment_lines);
                                format!(
                                    "{}-{} ({})",
                                    doc_start + 1,
                                    ext.scope.end.line + 1,
                                    r.label
                                )
                            } else {
                                format!(
                                    "{}-{} ({})",
                                    r.scope.start.line + 1,
                                    r.scope.end.line + 1,
                                    r.label
                                )
                            }
                        } else {
                            format!(
                                "{}-{} ({})",
                                r.scope.start.line + 1,
                                r.scope.end.line + 1,
                                r.label
                            )
                        }
                    })
                    .collect();

                return Err(crate::error::Error::NoRunnableAtLine {
                    line: line_num + 1,
                    available: available_lines
                        .iter()
                        .filter_map(|s| s.parse::<u32>().ok())
                        .collect(),
                });
            }
        } else {
            // Get any runnable in the file
            self.detect_runnables(file_path)?
                .into_iter()
                .next()
                .ok_or(crate::error::Error::NoRunnableFound)?
        };

        self.build_command(&runnable)
    }

    /// Get the best runnable at a position (backward compatibility)
    pub fn get_best_runnable_at_line(&self, path: &Path, line: u32) -> Result<Option<Runnable>> {
        self.get_runnable_at_line(path, line)
    }

    /// Get command at position with working directory (backward compatibility)
    pub fn get_command_at_position_with_dir(
        &mut self,
        filepath: &Path,
        line: Option<u32>,
    ) -> Result<crate::command::Command> {
        // Try to get command at the specific position
        match self.build_command_at_position(filepath, line) {
            Ok(cmd) => Ok(cmd),
            Err(e) => {
                // If we have a line number and no runnable was found, try file-level command
                if line.is_some() {
                    // Check if we have file-level commands available
                    if let Ok(Some(file_cmd)) = self.get_file_command(filepath) {
                        Ok(file_cmd)
                    } else {
                        Err(e)
                    }
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Build command for a specific runnable (backward compatibility)
    pub fn build_command_for_runnable(
        &self,
        runnable: &Runnable,
    ) -> Result<Option<crate::command::Command>> {
        Ok(Some(self.build_command(runnable)?))
    }

    /// Detect all runnables (backward compatibility)
    pub fn detect_all_runnables(&mut self, file_path: &Path) -> Result<Vec<Runnable>> {
        self.detect_runnables(file_path)
    }

    /// Detect runnables at a specific line (backward compatibility)
    pub fn detect_runnables_at_line(
        &mut self,
        file_path: &Path,
        line: u32,
    ) -> Result<Vec<Runnable>> {
        let all_runnables = self.detect_runnables(file_path)?;

        // Filter to runnables that contain the line
        let runnables: Vec<_> = all_runnables
            .into_iter()
            .filter(|r| r.scope.contains_line(line))
            .collect();

        Ok(runnables)
    }

    /// Get file command (backward compatibility)
    pub fn get_file_command(
        &mut self,
        file_path: &Path,
    ) -> Result<Option<crate::command::Command>> {
        // Check if this is a lib.rs file
        let is_lib_rs = file_path
            .file_name()
            .and_then(|f| f.to_str())
            .map(|name| name == "lib.rs")
            .unwrap_or(false);

        // Check if this is src/lib.rs specifically
        let is_src_lib_rs = file_path
            .to_str()
            .map(|p| p.ends_with("src/lib.rs") || p.ends_with("/src/lib.rs"))
            .unwrap_or(false);

        if is_lib_rs || is_src_lib_rs {
            // For lib.rs files, create a generic test command without module filters
            let build_system = self.detect_build_system_with_fallback(file_path);
            let runner = self.get_runner(&build_system)?;

            // Get package name if possible (unused for now but may be needed later)
            let _package_name = self.get_package_name_str(file_path).ok();

            // Create a simple file-level runnable for lib.rs
            let file_runnable = Runnable {
                scope: crate::types::Scope {
                    start: crate::types::Position::new(0, 0),
                    end: crate::types::Position::new(u32::MAX, 0),
                    kind: crate::types::ScopeKind::File(crate::types::FileScope::Lib),
                    name: Some("lib.rs".to_string()),
                },
                kind: crate::types::RunnableKind::ModuleTests {
                    module_name: String::new(), // Empty module name for file-level
                },
                module_path: String::new(),
                file_path: file_path.to_path_buf(),
                extended_scope: None,
                label: "Run all tests in library".to_string(),
            };

            let command =
                runner.build_command(&file_runnable, &self.config, FileType::CargoProject)?;
            return Ok(Some(command));
        }

        // Check if this is an integration test file (direct child of tests/)
        let is_integration_test = file_path
            .parent()
            .and_then(|p| p.file_name())
            .map(|name| name == "tests")
            .unwrap_or(false);

        if is_integration_test {
            let build_system = self.detect_build_system_with_fallback(file_path);
            let runner = self.get_runner(&build_system)?;

            let file_stem = file_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("integration_test")
                .to_string();

            // Create a file-level runnable for the integration test file
            let file_runnable = Runnable {
                scope: crate::types::Scope {
                    start: crate::types::Position::new(0, 0),
                    end: crate::types::Position::new(u32::MAX, 0),
                    kind: crate::types::ScopeKind::File(crate::types::FileScope::Unknown),
                    name: Some(file_stem.clone()),
                },
                kind: crate::types::RunnableKind::ModuleTests {
                    module_name: String::new(), // Empty module name for file-level
                },
                module_path: String::new(),
                file_path: file_path.to_path_buf(),
                extended_scope: None,
                label: format!("Run all tests in {file_stem}"),
            };

            let command =
                runner.build_command(&file_runnable, &self.config, FileType::CargoProject)?;
            return Ok(Some(command));
        }

        if is_single_file_script_file(file_path) {
            tracing::debug!("get_file_command: detected single-file script, using fallback");

            let cargo_root = file_path
                .ancestors()
                .find(|p| p.join("Cargo.toml").exists())
                .map(|p| p.to_path_buf());
            let package_name = self.get_package_name_str(file_path).ok();

            if let Some(command) = generate_fallback_command(
                file_path,
                package_name.as_deref(),
                cargo_root.as_deref(),
                Some((*self.config).clone()),
            )? {
                return Ok(Some(command));
            }
        }

        // For non-lib.rs files, use the original logic
        let runnables = self.detect_runnables(file_path)?;

        tracing::debug!(
            "get_file_command: found {} runnables for {:?}",
            runnables.len(),
            file_path
        );
        for (i, runnable) in runnables.iter().enumerate() {
            tracing::debug!("  [{}] {:?} - {:?}", i, runnable.kind, runnable.label);
        }

        // Check if this is a benchmark file
        let is_benchmark_file = file_path.components().any(|c| c.as_os_str() == "benches");

        // Sort runnables to prioritize based on file type
        let mut sorted_runnables = runnables;
        sorted_runnables.sort_by(|a, b| {
            use crate::types::RunnableKind;
            match (&a.kind, &b.kind) {
                // For benchmark files, prioritize Binary/Benchmark over tests
                (RunnableKind::Binary { .. }, RunnableKind::Test { .. }) if is_benchmark_file => {
                    std::cmp::Ordering::Less
                }
                (RunnableKind::Binary { .. }, RunnableKind::ModuleTests { .. })
                    if is_benchmark_file =>
                {
                    std::cmp::Ordering::Less
                }
                (RunnableKind::Benchmark { .. }, RunnableKind::Test { .. })
                    if is_benchmark_file =>
                {
                    std::cmp::Ordering::Less
                }
                (RunnableKind::Benchmark { .. }, RunnableKind::ModuleTests { .. })
                    if is_benchmark_file =>
                {
                    std::cmp::Ordering::Less
                }
                (RunnableKind::Test { .. }, RunnableKind::Binary { .. }) if is_benchmark_file => {
                    std::cmp::Ordering::Greater
                }
                (RunnableKind::ModuleTests { .. }, RunnableKind::Binary { .. })
                    if is_benchmark_file =>
                {
                    std::cmp::Ordering::Greater
                }
                (RunnableKind::Test { .. }, RunnableKind::Benchmark { .. })
                    if is_benchmark_file =>
                {
                    std::cmp::Ordering::Greater
                }
                (RunnableKind::ModuleTests { .. }, RunnableKind::Benchmark { .. })
                    if is_benchmark_file =>
                {
                    std::cmp::Ordering::Greater
                }
                // Deprioritize doc tests for file-level commands
                (RunnableKind::DocTest { .. }, _) => std::cmp::Ordering::Greater,
                (_, RunnableKind::DocTest { .. }) => std::cmp::Ordering::Less,
                // For non-benchmark files, prefer module tests over individual tests
                (RunnableKind::ModuleTests { .. }, RunnableKind::Test { .. })
                    if !is_benchmark_file =>
                {
                    std::cmp::Ordering::Less
                }
                (RunnableKind::Test { .. }, RunnableKind::ModuleTests { .. })
                    if !is_benchmark_file =>
                {
                    std::cmp::Ordering::Greater
                }
                _ => std::cmp::Ordering::Equal,
            }
        });

        tracing::debug!("get_file_command: after sorting:");
        for (i, runnable) in sorted_runnables.iter().enumerate() {
            tracing::debug!("  [{}] {:?} - {:?}", i, runnable.kind, runnable.label);
        }

        if let Some(runnable) = sorted_runnables.into_iter().next() {
            tracing::debug!(
                "get_file_command: selected runnable: {:?} - {:?}",
                runnable.kind,
                runnable.label
            );
            Ok(Some(self.build_command(&runnable)?))
        } else {
            // No AST-based runnable found. Try the fallback detector so we still
            // recognize cargo-script files and other non-standard single-file cases.
            tracing::debug!("get_file_command: no runnables found, trying fallback command");

            let cargo_root = file_path
                .ancestors()
                .find(|p| p.join("Cargo.toml").exists())
                .map(|p| p.to_path_buf());
            let package_name = self.get_package_name_str(file_path).ok();

            if let Some(command) = generate_fallback_command(
                file_path,
                package_name.as_deref(),
                cargo_root.as_deref(),
                Some((*self.config).clone()),
            )? {
                return Ok(Some(command));
            }

            // Preserve the old behavior as a final attempt for any other file types.
            self.build_command_at_position(file_path, None)
                .map(Some)
                .or(Ok(None))
        }
    }

    /// Analyze a file and return all runnables as JSON
    pub fn analyze(&mut self, file_path: &str) -> Result<String> {
        let path = Path::new(file_path);
        let runnables = self.detect_runnables(path)?;
        Ok(serde_json::to_string_pretty(&runnables)?)
    }

    /// Analyze a file at a specific line and return runnables as JSON
    pub fn analyze_at_line(&mut self, file_path: &str, line: usize) -> Result<String> {
        let path = Path::new(file_path);
        let runnables = self.detect_runnables_at_line(path, line as u32)?;
        Ok(serde_json::to_string_pretty(&runnables)?)
    }
}
