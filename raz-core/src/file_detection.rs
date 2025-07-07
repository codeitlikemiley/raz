//! File detection and project type analysis
//!
//! This module provides stateless file detection that can determine the execution
//! context for any Rust file based solely on file path and content analysis.

use crate::{Position, RazResult};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(feature = "tree-sitter-support")]
use crate::tree_sitter_test_detector::TreeSitterTestDetector;

/// Different types of Rust projects and files
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RustProjectType {
    /// Cargo workspace with multiple packages
    CargoWorkspace {
        root: PathBuf,
        members: Vec<WorkspaceMember>,
    },
    /// Single cargo package
    CargoPackage { root: PathBuf, package_name: String },
    /// Cargo script with embedded manifest
    CargoScript {
        file_path: PathBuf,
        manifest: Option<String>,
    },
    /// Single standalone Rust file
    SingleFile {
        file_path: PathBuf,
        file_type: SingleFileType,
    },
}

/// Types of single Rust files
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SingleFileType {
    /// Executable with main function
    Executable,
    /// Library with public API
    Library,
    /// Test file with test functions
    Test,
    /// Module file
    Module,
}

/// Workspace member information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceMember {
    pub name: String,
    pub path: PathBuf,
    pub is_current: bool,
}

/// File execution context
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileExecutionContext {
    pub project_type: RustProjectType,
    pub file_role: FileRole,
    pub entry_points: Vec<EntryPoint>,
    pub capabilities: ExecutionCapabilities,
    pub file_path: PathBuf,
}

impl FileExecutionContext {
    /// Get the workspace root if this is a cargo project
    pub fn get_workspace_root(&self) -> Option<&Path> {
        match &self.project_type {
            RustProjectType::CargoWorkspace { root, .. } => Some(root),
            RustProjectType::CargoPackage { root, .. } => Some(root),
            RustProjectType::CargoScript { .. } => None,
            RustProjectType::SingleFile { .. } => None,
        }
    }
}

/// Role of the specific file in the project
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FileRole {
    /// Main binary entry point
    MainBinary { binary_name: String },
    /// Additional binary in bin/
    AdditionalBinary { binary_name: String },
    /// Library root (lib.rs)
    LibraryRoot,
    /// Frontend library in web framework
    FrontendLibrary { framework: String },
    /// Integration test
    IntegrationTest { test_name: String },
    /// Benchmark file
    Benchmark { bench_name: String },
    /// Example file
    Example { example_name: String },
    /// Build script
    BuildScript,
    /// Regular module
    Module,
    /// Standalone file
    Standalone,
}

/// Entry points found in the file
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntryPoint {
    pub name: String,
    pub entry_type: EntryPointType,
    pub line: u32,
    pub column: u32,
    /// Line range of the entry point (start, end)
    pub line_range: (u32, u32),
    /// Full path for tests (e.g., "tests::it_works" for tests in a module)
    pub full_path: Option<String>,
}

/// Types of entry points
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EntryPointType {
    /// main() function
    Main,
    /// Test function
    Test,
    /// Test module (when cursor is in test module but not on specific test)
    TestModule,
    /// Benchmark function
    Benchmark,
    /// Example function (in examples)
    Example,
    /// Doc test
    DocTest,
}

/// Execution capabilities for this file/project
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionCapabilities {
    pub can_run: bool,
    pub can_test: bool,
    pub can_bench: bool,
    pub can_doc_test: bool,
    pub requires_framework: Option<String>,
}

/// Stateless file detector
pub struct FileDetector;

impl FileDetector {
    /// Detect execution context for any file path
    pub fn detect_context(
        file_path: &Path,
        cursor: Option<Position>,
    ) -> RazResult<FileExecutionContext> {
        Self::detect_context_with_options(file_path, cursor, false)
    }

    /// Detect execution context with force standalone option
    pub fn detect_context_with_options(
        file_path: &Path,
        cursor: Option<Position>,
        force_standalone: bool,
    ) -> RazResult<FileExecutionContext> {
        let mut project_type = if force_standalone {
            // Force standalone analysis, ignore any Cargo.toml
            Self::analyze_single_file(file_path)?
        } else {
            Self::detect_project_type(file_path)?
        };
        let file_role = Self::detect_file_role(file_path, &project_type)?;

        // If the file is standalone in a cargo project, treat it as a single file
        if matches!(file_role, FileRole::Standalone)
            && matches!(
                project_type,
                RustProjectType::CargoPackage { .. } | RustProjectType::CargoWorkspace { .. }
            )
        {
            project_type = Self::analyze_single_file(file_path)?;
        }

        let entry_points = Self::detect_entry_points(file_path, cursor, &project_type)?;
        let capabilities = Self::determine_capabilities(&project_type, &file_role, &entry_points);

        Ok(FileExecutionContext {
            project_type,
            file_role,
            entry_points,
            capabilities,
            file_path: file_path.to_path_buf(),
        })
    }

    /// Detect the project type by crawling up the file system
    fn detect_project_type(file_path: &Path) -> RazResult<RustProjectType> {
        // Start from the file's directory
        let mut current_dir = if file_path.is_file() {
            file_path.parent().unwrap_or(file_path)
        } else {
            file_path
        };

        // Crawl up looking for Cargo.toml
        loop {
            let cargo_toml = current_dir.join("Cargo.toml");
            if cargo_toml.exists() {
                return Self::analyze_cargo_project(&cargo_toml, file_path);
            }

            match current_dir.parent() {
                Some(parent) => current_dir = parent,
                None => break,
            }
        }

        // No Cargo.toml found, analyze as single file
        Self::analyze_single_file(file_path)
    }

    /// Analyze a Cargo project
    fn analyze_cargo_project(
        cargo_toml_path: &Path,
        file_path: &Path,
    ) -> RazResult<RustProjectType> {
        let content = fs::read_to_string(cargo_toml_path)?;
        let root = cargo_toml_path.parent().unwrap().to_path_buf();

        // Check if it's a workspace
        if content.contains("[workspace]") {
            let members = Self::parse_workspace_members(&content, &root)?;
            Ok(RustProjectType::CargoWorkspace { root, members })
        } else if content.contains("[package]") {
            let package_name = Self::extract_package_name(&content)?;
            Ok(RustProjectType::CargoPackage { root, package_name })
        } else {
            // Fallback to single file if Cargo.toml is malformed
            Self::analyze_single_file(file_path)
        }
    }

    /// Analyze a single file
    fn analyze_single_file(file_path: &Path) -> RazResult<RustProjectType> {
        let content = fs::read_to_string(file_path)?;

        // Check for cargo script shebang
        if Self::is_cargo_script(&content) {
            let manifest = Self::extract_cargo_script_manifest(&content);
            return Ok(RustProjectType::CargoScript {
                file_path: file_path.to_path_buf(),
                manifest,
            });
        }

        // Determine single file type
        let file_type = if content.contains("fn main(") {
            SingleFileType::Executable
        } else if content.contains("pub ") {
            // If it has public API, it's a library (even if it also has tests)
            SingleFileType::Library
        } else if content.contains("#[test]") || content.contains("#[cfg(test)]") {
            SingleFileType::Test
        } else {
            SingleFileType::Module
        };

        Ok(RustProjectType::SingleFile {
            file_path: file_path.to_path_buf(),
            file_type,
        })
    }

    /// Detect the role of a specific file
    fn detect_file_role(file_path: &Path, project_type: &RustProjectType) -> RazResult<FileRole> {
        let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let path_str = file_path.to_string_lossy();

        // Handle different project types
        match project_type {
            RustProjectType::CargoWorkspace { .. } | RustProjectType::CargoPackage { .. } => {
                if file_name == "build.rs" {
                    Ok(FileRole::BuildScript)
                } else if path_str.contains("/src/main.rs") {
                    let binary_name = Self::extract_binary_name_from_path(file_path);
                    Ok(FileRole::MainBinary { binary_name })
                } else if path_str.contains("/src/bin/") {
                    let binary_name = file_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    Ok(FileRole::AdditionalBinary { binary_name })
                } else if file_name == "lib.rs" {
                    // Check if this is a frontend library
                    if Self::is_frontend_library(file_path) {
                        let framework = Self::detect_web_framework(file_path)?;
                        Ok(FileRole::FrontendLibrary { framework })
                    } else {
                        Ok(FileRole::LibraryRoot)
                    }
                } else if path_str.contains("/tests/") {
                    let test_name = file_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    Ok(FileRole::IntegrationTest { test_name })
                } else if path_str.contains("/benches/") {
                    let bench_name = file_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    Ok(FileRole::Benchmark { bench_name })
                } else if path_str.contains("/examples/") {
                    let example_name = file_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    Ok(FileRole::Example { example_name })
                } else if path_str.contains("/src/") && file_name.ends_with(".rs") {
                    // Files in src/ subdirectories are modules
                    Ok(FileRole::Module)
                } else {
                    // Files outside standard cargo directories should be treated as standalone
                    // unless they're explicitly defined in Cargo.toml (which we don't check here)
                    Ok(FileRole::Standalone)
                }
            }
            RustProjectType::CargoScript { .. } => {
                let binary_name = file_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("script")
                    .to_string();
                Ok(FileRole::MainBinary { binary_name })
            }
            RustProjectType::SingleFile { file_type, .. } => {
                // Check for special file names first
                if file_name == "build.rs" {
                    Ok(FileRole::BuildScript)
                } else {
                    match file_type {
                        SingleFileType::Executable => {
                            let binary_name = file_path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("main")
                                .to_string();
                            Ok(FileRole::MainBinary { binary_name })
                        }
                        _ => Ok(FileRole::Standalone),
                    }
                }
            }
        }
    }

    /// Detect entry points in the file
    fn detect_entry_points(
        file_path: &Path,
        cursor: Option<Position>,
        project_type: &RustProjectType,
    ) -> RazResult<Vec<EntryPoint>> {
        let content = fs::read_to_string(file_path)?;

        // Use tree-sitter if available for accurate AST-based detection
        #[cfg(feature = "tree-sitter-support")]
        {
            if let Ok(mut detector) = TreeSitterTestDetector::new() {
                if let Ok(mut tree_sitter_entries) = detector.detect_entry_points(&content, cursor)
                {
                    // Build module path from file location for cargo projects
                    let file_module_path =
                        Self::build_module_path_from_file(file_path, project_type);

                    // Update full paths for tests
                    for entry in &mut tree_sitter_entries {
                        if matches!(
                            entry.entry_type,
                            EntryPointType::Test | EntryPointType::TestModule
                        ) {
                            if let Some(existing_path) = &entry.full_path {
                                // Combine file module path with test path
                                let mut full_path_parts = file_module_path.clone();
                                full_path_parts.push(existing_path.clone());
                                entry.full_path = Some(full_path_parts.join("::"));
                            } else if !file_module_path.is_empty() {
                                // No existing path, use file module path + test name
                                let mut full_path_parts = file_module_path.clone();
                                full_path_parts.push(entry.name.clone());
                                entry.full_path = Some(full_path_parts.join("::"));
                            }
                        }
                    }

                    // Also detect non-test entries using regex (main, benchmarks)
                    tree_sitter_entries.extend(Self::detect_non_test_entries(&content)?);

                    // Doc tests are now detected by tree-sitter in detect_test_entry_points above
                    // No need for separate detection

                    return Ok(tree_sitter_entries);
                }
            }
            // Fall back to regex-based detection if tree-sitter fails
        }

        // Regex-based detection (fallback or when tree-sitter not available)
        let mut entry_points = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        // Compile regex once outside the loop
        let test_macro_regex = regex::Regex::new(r"#\[(\w+::)?test\]").unwrap();

        // Build module path from file location for cargo projects
        let file_module_path = Self::build_module_path_from_file(file_path, project_type);

        // Track nested module hierarchy
        let mut module_stack: Vec<String> = Vec::new();
        let mut depth_stack: Vec<u32> = Vec::new();
        let mut current_depth: u32 = 0;

        // Track test module context
        let mut in_test_module = false;
        let mut test_module_depth = 0;

        // Track doc test context
        let mut in_doc_comment = false;
        let mut in_doc_code_block = false;
        let mut doc_test_start = None;
        let mut doc_comment_start_line = None;

        for (line_num, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Track brace depth and update module stack
            let open_braces = trimmed.matches('{').count() as u32;
            let close_braces = trimmed.matches('}').count() as u32;

            // Handle closing braces first - pop modules that are ending
            for _ in 0..close_braces {
                current_depth = current_depth.saturating_sub(1);

                // Pop module from stack if we're closing its scope
                while let Some(&stack_depth) = depth_stack.last() {
                    if current_depth < stack_depth {
                        depth_stack.pop();
                        module_stack.pop();
                    } else {
                        break;
                    }
                }

                // Update test module state
                if in_test_module && current_depth < test_module_depth {
                    in_test_module = false;
                }
            }

            // Handle opening braces
            current_depth = current_depth.saturating_add(open_braces);

            // Check for #[cfg(test)] attribute
            if trimmed == "#[cfg(test)]" {
                in_test_module = true;
                test_module_depth = current_depth;
            }

            // Check for module declarations
            if let Some(mod_start) = trimmed.find("mod ") {
                let after_mod = &trimmed[mod_start + 4..];
                if let Some(name_end) = after_mod.find([' ', '{', ';']) {
                    let module_name = after_mod[..name_end].trim();

                    // Only track modules that have a body (contain '{' or next line has '{')
                    let has_body = trimmed.contains('{')
                        || (line_num + 1 < lines.len()
                            && lines[line_num + 1].trim().starts_with('{'));

                    if has_body && !module_name.is_empty() {
                        // Push this module onto the stack
                        module_stack.push(module_name.to_string());
                        depth_stack.push(current_depth);

                        // Check if this is a test module
                        if module_name.contains("test") && !in_test_module {
                            in_test_module = true;
                            test_module_depth = current_depth;
                        }
                    }
                }
            }

            // Doc test detection
            if trimmed.starts_with("///") || trimmed.starts_with("//!") {
                if !in_doc_comment {
                    doc_comment_start_line = Some(line_num);
                }
                in_doc_comment = true;
                // Track code blocks in doc comments
                if (trimmed == "/// ```"
                    || trimmed == "/// ```rust"
                    || trimmed == "//! ```"
                    || trimmed == "//! ```rust")
                    && !in_doc_code_block
                {
                    // Starting a code block
                    in_doc_code_block = true;
                    doc_test_start = Some(line_num as u32 + 1);
                } else if trimmed == "/// ```" && in_doc_code_block {
                    // Ending a code block
                    in_doc_code_block = false;
                }
            } else if in_doc_comment && !trimmed.starts_with("//") {
                // Doc comment ended, check if cursor is in doc test area
                if let Some(cursor_pos) = cursor {
                    if let Some(start) = doc_test_start {
                        if cursor_pos.line + 1 >= start && cursor_pos.line < line_num as u32 + 1 {
                            // Check if current line is a function/struct/impl declaration
                            let mut item_name = None;
                            if trimmed.starts_with("pub fn") || trimmed.starts_with("fn") {
                                item_name = Self::extract_function_name(trimmed);
                            } else if trimmed.starts_with("pub struct")
                                || trimmed.starts_with("struct")
                            {
                                item_name = Self::extract_struct_name(trimmed);
                            } else if trimmed.starts_with("impl") {
                                item_name = Self::extract_impl_name(trimmed);
                            } else {
                                // Look ahead for the function/struct/impl this doc comment belongs to
                                for i in 1..5 {
                                    if line_num + i < lines.len() {
                                        let ahead_line = lines[line_num + i].trim();
                                        if ahead_line.starts_with("pub fn")
                                            || ahead_line.starts_with("fn")
                                        {
                                            item_name = Self::extract_function_name(ahead_line);
                                            break;
                                        } else if ahead_line.starts_with("pub struct")
                                            || ahead_line.starts_with("struct")
                                        {
                                            item_name = Self::extract_struct_name(ahead_line);
                                            break;
                                        } else if ahead_line.starts_with("impl") {
                                            item_name = Self::extract_impl_name(ahead_line);
                                            break;
                                        }
                                    }
                                }
                            }

                            if let Some(name) = item_name {
                                entry_points.push(EntryPoint {
                                    name: name.clone(),
                                    entry_type: EntryPointType::DocTest,
                                    line: start,
                                    column: 0,
                                    line_range: (
                                        doc_comment_start_line.unwrap_or(0) as u32 + 1,
                                        (line_num as u32).saturating_sub(1), // Exclude the function declaration line
                                    ),
                                    full_path: Some(name),
                                });
                            }
                        }
                    }
                }

                in_doc_comment = false;
                in_doc_code_block = false;
                doc_test_start = None;
                doc_comment_start_line = None;
            }

            // Main function
            if trimmed.contains("fn main(") {
                entry_points.push(EntryPoint {
                    name: "main".to_string(),
                    entry_type: EntryPointType::Main,
                    line: line_num as u32 + 1,
                    column: line.find("fn main(").unwrap_or(0) as u32,
                    line_range: (line_num as u32 + 1, line_num as u32 + 1),
                    full_path: None,
                });
            }

            // Helper function to build current module path
            let build_module_path = |fn_name: &str| -> Option<String> {
                let mut path_parts = file_module_path.clone();
                path_parts.extend(module_stack.clone());
                path_parts.push(fn_name.to_string());
                if path_parts.is_empty() {
                    None
                } else {
                    Some(path_parts.join("::"))
                }
            };

            // Test functions with regex pattern matching
            if test_macro_regex.is_match(trimmed) {
                // Look for the function on the next line(s)
                for i in 1..=3 {
                    if line_num + i < lines.len() {
                        let next_line = lines[line_num + i];
                        if let Some(fn_name) = Self::extract_function_name(next_line) {
                            let full_path = build_module_path(&fn_name);
                            entry_points.push(EntryPoint {
                                name: fn_name.clone(),
                                entry_type: EntryPointType::Test,
                                line: line_num as u32 + i as u32 + 1,
                                column: 0,
                                line_range: (line_num as u32 + 1, line_num as u32 + 1),
                                full_path,
                            });
                            break;
                        }
                    }
                }
            } else if (trimmed.starts_with("fn test_") || trimmed.contains("fn test_"))
                && (in_test_module || trimmed.contains("#["))
            {
                if let Some(fn_name) = Self::extract_function_name(trimmed) {
                    let column = line.find(&format!("fn {fn_name}")).unwrap_or(0) as u32;
                    let full_path = build_module_path(&fn_name);
                    entry_points.push(EntryPoint {
                        name: fn_name.clone(),
                        entry_type: EntryPointType::Test,
                        line: line_num as u32 + 1,
                        column,
                        line_range: (line_num as u32 + 1, line_num as u32 + 1),
                        full_path,
                    });
                }
            }

            // Benchmark functions
            if trimmed.starts_with("#[bench]") {
                if let Some(next_line) = lines.get(line_num + 1) {
                    if let Some(fn_name) = Self::extract_function_name(next_line) {
                        entry_points.push(EntryPoint {
                            name: fn_name,
                            entry_type: EntryPointType::Benchmark,
                            line: line_num as u32 + 2,
                            column: 0,
                            line_range: (line_num as u32 + 1, line_num as u32 + 1),
                            full_path: None,
                        });
                    }
                }
            }

            // criterion benchmarks
            if trimmed.contains("criterion_group!") || trimmed.contains("criterion_main!") {
                entry_points.push(EntryPoint {
                    name: "criterion_bench".to_string(),
                    entry_type: EntryPointType::Benchmark,
                    line: line_num as u32 + 1,
                    column: 0,
                    line_range: (line_num as u32 + 1, line_num as u32 + 1),
                    full_path: None,
                });
            }
        }

        // Final check: if we're still in a doc comment at the end of the loop and cursor is in it
        // This handles the case where the file ends with a doc comment
        if in_doc_comment && cursor.is_some() {
            let cursor_pos = cursor.unwrap();
            if let Some(start) = doc_test_start {
                if cursor_pos.line + 1 >= start {
                    // The doc comment goes to the end of the file, so we can't find a function after it
                    // Use a generic name for now
                    entry_points.push(EntryPoint {
                        name: "doc_test".to_string(),
                        entry_type: EntryPointType::DocTest,
                        line: start,
                        column: 0,
                        line_range: (
                            doc_comment_start_line.unwrap_or(0) as u32 + 1,
                            lines.len() as u32,
                        ),
                        full_path: None,
                    });
                }
            }
        }

        // Add test module entry if cursor is in a test module but not on a specific test
        if let Some(cursor_pos) = cursor {
            if in_test_module && !module_stack.is_empty() {
                // Check if cursor is in test module but not directly on a test function
                let cursor_line = cursor_pos.line + 1;
                let on_specific_test = entry_points
                    .iter()
                    .any(|ep| ep.entry_type == EntryPointType::Test && ep.line == cursor_line);

                if !on_specific_test {
                    let mut path_parts = file_module_path.clone();
                    path_parts.extend(module_stack.clone());
                    let current_module_path = path_parts.join("::");
                    entry_points.push(EntryPoint {
                        name: format!(
                            "{}_module",
                            module_stack.last().unwrap_or(&"tests".to_string())
                        ),
                        entry_type: EntryPointType::TestModule,
                        line: cursor_line,
                        column: 0,
                        line_range: (cursor_line, cursor_line),
                        full_path: Some(current_module_path),
                    });
                }
            }
        }

        Ok(entry_points)
    }

    /// Determine execution capabilities
    fn determine_capabilities(
        project_type: &RustProjectType,
        file_role: &FileRole,
        entry_points: &[EntryPoint],
    ) -> ExecutionCapabilities {
        let has_main = entry_points
            .iter()
            .any(|ep| ep.entry_type == EntryPointType::Main);
        let has_tests = entry_points
            .iter()
            .any(|ep| ep.entry_type == EntryPointType::Test);
        let has_benches = entry_points
            .iter()
            .any(|ep| ep.entry_type == EntryPointType::Benchmark);

        match file_role {
            FileRole::MainBinary { .. } | FileRole::AdditionalBinary { .. } => {
                ExecutionCapabilities {
                    can_run: has_main,
                    can_test: has_tests,
                    can_bench: has_benches,
                    can_doc_test: false,
                    requires_framework: None,
                }
            }
            FileRole::FrontendLibrary { framework } => ExecutionCapabilities {
                can_run: true, // Framework-specific run
                can_test: has_tests,
                can_bench: has_benches,
                can_doc_test: true,
                requires_framework: Some(framework.clone()),
            },
            FileRole::LibraryRoot => ExecutionCapabilities {
                can_run: false,
                can_test: has_tests,
                can_bench: has_benches,
                can_doc_test: true,
                requires_framework: None,
            },
            FileRole::IntegrationTest { .. } => ExecutionCapabilities {
                can_run: false,
                can_test: true,
                can_bench: false,
                can_doc_test: false,
                requires_framework: None,
            },
            FileRole::Benchmark { .. } => ExecutionCapabilities {
                can_run: false,
                can_test: false,
                can_bench: true,
                can_doc_test: false,
                requires_framework: None,
            },
            FileRole::Example { .. } => ExecutionCapabilities {
                can_run: has_main,
                can_test: has_tests,
                can_bench: has_benches,
                can_doc_test: false,
                requires_framework: None,
            },
            FileRole::BuildScript => ExecutionCapabilities {
                can_run: has_main,
                can_test: false,
                can_bench: false,
                can_doc_test: false,
                requires_framework: None,
            },
            FileRole::Standalone => match project_type {
                RustProjectType::SingleFile { file_type, .. } => match file_type {
                    SingleFileType::Executable => ExecutionCapabilities {
                        can_run: has_main,
                        can_test: has_tests,
                        can_bench: has_benches,
                        can_doc_test: false,
                        requires_framework: None,
                    },
                    SingleFileType::Library => ExecutionCapabilities {
                        can_run: false,
                        can_test: has_tests,
                        can_bench: has_benches,
                        can_doc_test: true,
                        requires_framework: None,
                    },
                    SingleFileType::Test => ExecutionCapabilities {
                        can_run: false,
                        can_test: true,
                        can_bench: false,
                        can_doc_test: false,
                        requires_framework: None,
                    },
                    SingleFileType::Module => ExecutionCapabilities {
                        can_run: false,
                        can_test: has_tests,
                        can_bench: false,
                        can_doc_test: false,
                        requires_framework: None,
                    },
                },
                RustProjectType::CargoScript { .. } => ExecutionCapabilities {
                    can_run: has_main,
                    can_test: has_tests,
                    can_bench: has_benches,
                    can_doc_test: false,
                    requires_framework: None,
                },
                _ => {
                    // For standalone files in cargo projects, determine capabilities based on content
                    ExecutionCapabilities {
                        can_run: has_main,
                        can_test: has_tests,
                        can_bench: has_benches,
                        can_doc_test: false,
                        requires_framework: None,
                    }
                }
            },
            FileRole::Module => ExecutionCapabilities {
                can_run: false,
                can_test: has_tests,
                can_bench: has_benches,
                can_doc_test: false,
                requires_framework: None,
            },
        }
    }

    // Helper methods

    fn is_cargo_script(content: &str) -> bool {
        content.starts_with("#!/usr/bin/env -S cargo")
            || content.starts_with("#!/usr/bin/env cargo")
            || content.starts_with("#!/usr/bin/env cargo-eval")
            || content.starts_with("#!/usr/bin/env run-cargo-script")
    }

    fn extract_cargo_script_manifest(content: &str) -> Option<String> {
        // Look for embedded manifest
        if let Some(start) = content.find("//! ```cargo") {
            let search_start = start + 13; // Length of "//! ```cargo"
            if let Some(relative_end) = content[search_start..].find("//! ```") {
                let end = search_start + relative_end;
                return Some(content[search_start..end].trim().to_string());
            }
        }
        None
    }

    fn parse_workspace_members(content: &str, root: &Path) -> RazResult<Vec<WorkspaceMember>> {
        let mut members = Vec::new();

        // Simple parsing - in a real implementation, use a TOML parser
        if let Some(members_section) = content.find("members = [") {
            let section = &content[members_section..];
            if let Some(end) = section.find(']') {
                let members_str = &section[11..end];
                for member in members_str.split(',') {
                    let member_name = member.trim().trim_matches('"').trim_matches('\'');
                    if !member_name.is_empty() {
                        members.push(WorkspaceMember {
                            name: member_name.to_string(),
                            path: root.join(member_name),
                            is_current: false, // Will be determined later
                        });
                    }
                }
            }
        }

        Ok(members)
    }

    fn extract_package_name(content: &str) -> RazResult<String> {
        if let Some(name_start) = content.find("name = \"") {
            let name_section = &content[name_start + 8..];
            if let Some(name_end) = name_section.find('"') {
                return Ok(name_section[..name_end].to_string());
            }
        }
        Ok("unknown".to_string())
    }

    fn extract_binary_name_from_path(file_path: &Path) -> String {
        // For src/main.rs, try to determine package name from Cargo.toml
        if let Some(parent) = file_path.parent() {
            if let Some(cargo_dir) = parent.parent() {
                let cargo_toml = cargo_dir.join("Cargo.toml");
                if cargo_toml.exists() {
                    if let Ok(content) = fs::read_to_string(&cargo_toml) {
                        if let Ok(name) = Self::extract_package_name(&content) {
                            return name;
                        }
                    }
                }
            }
        }
        "main".to_string()
    }

    fn is_frontend_library(file_path: &Path) -> bool {
        let path_str = file_path.to_string_lossy();
        let file_name = file_path.file_name().unwrap_or_default().to_string_lossy();

        file_name == "lib.rs"
            && (path_str.contains("/frontend/")
                || path_str.contains("/app/")
                || path_str.contains("/client/")
                || path_str.contains("/web/"))
    }

    fn detect_web_framework(file_path: &Path) -> RazResult<String> {
        // Walk up to find Cargo.toml and check dependencies
        let mut current = file_path;
        while let Some(parent) = current.parent() {
            let cargo_toml = parent.join("Cargo.toml");
            if cargo_toml.exists() {
                if let Ok(content) = fs::read_to_string(&cargo_toml) {
                    if content.contains("leptos") {
                        return Ok("leptos".to_string());
                    } else if content.contains("dioxus") {
                        return Ok("dioxus".to_string());
                    } else if content.contains("yew") {
                        return Ok("yew".to_string());
                    }
                }
            }
            current = parent;
        }
        Ok("unknown".to_string())
    }

    fn detect_non_test_entries(content: &str) -> RazResult<Vec<EntryPoint>> {
        let mut entry_points = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut in_doc_comment = false;

        for (line_num, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Track doc comments to avoid false positives
            if trimmed.starts_with("///") || trimmed.starts_with("//!") {
                in_doc_comment = true;
            } else if in_doc_comment && !trimmed.starts_with("//") {
                in_doc_comment = false;
            }

            // Main function - only detect if not in doc comment
            if !in_doc_comment && trimmed.contains("fn main(") {
                entry_points.push(EntryPoint {
                    name: "main".to_string(),
                    entry_type: EntryPointType::Main,
                    line: line_num as u32 + 1,
                    column: line.find("fn main(").unwrap_or(0) as u32,
                    line_range: (line_num as u32 + 1, line_num as u32 + 1),
                    full_path: None,
                });
            }

            // Benchmark functions - only detect if not in doc comment
            if !in_doc_comment && trimmed.starts_with("#[bench]") {
                if let Some(next_line) = lines.get(line_num + 1) {
                    if let Some(fn_name) = Self::extract_function_name(next_line) {
                        entry_points.push(EntryPoint {
                            name: fn_name,
                            entry_type: EntryPointType::Benchmark,
                            line: line_num as u32 + 2,
                            column: 0,
                            line_range: (line_num as u32 + 1, line_num as u32 + 1),
                            full_path: None,
                        });
                    }
                }
            }

            // criterion benchmarks - only detect if not in doc comment
            if !in_doc_comment
                && (trimmed.contains("criterion_group!") || trimmed.contains("criterion_main!"))
            {
                entry_points.push(EntryPoint {
                    name: "criterion_bench".to_string(),
                    entry_type: EntryPointType::Benchmark,
                    line: line_num as u32 + 1,
                    column: 0,
                    line_range: (line_num as u32 + 1, line_num as u32 + 1),
                    full_path: None,
                });
            }
        }

        Ok(entry_points)
    }

    fn extract_function_name(line: &str) -> Option<String> {
        let trimmed = line.trim();
        if let Some(fn_start) = trimmed.find("fn ") {
            let fn_part = &trimmed[fn_start + 3..];
            if let Some(paren_pos) = fn_part.find('(') {
                let fn_name = fn_part[..paren_pos].trim();
                if !fn_name.is_empty() {
                    return Some(fn_name.to_string());
                }
            }
        }
        None
    }

    fn extract_struct_name(line: &str) -> Option<String> {
        let trimmed = line.trim();
        let struct_start = if trimmed.starts_with("pub struct ") {
            11 // Length of "pub struct "
        } else if trimmed.starts_with("struct ") {
            7 // Length of "struct "
        } else {
            return None;
        };

        let after_struct = &trimmed[struct_start..];
        // Find the end of the struct name (space, <, {, or ;)
        let name_end = after_struct
            .find([' ', '<', '{', ';'])
            .unwrap_or(after_struct.len());
        let struct_name = after_struct[..name_end].trim();

        if !struct_name.is_empty() {
            Some(struct_name.to_string())
        } else {
            None
        }
    }

    fn extract_impl_name(line: &str) -> Option<String> {
        let trimmed = line.trim();
        if !trimmed.starts_with("impl") {
            return None;
        }

        // Handle "impl Trait for Type" or "impl Type"
        if let Some(for_pos) = trimmed.find(" for ") {
            // Extract the type after "for"
            let after_for = &trimmed[for_pos + 5..];
            let name_end = after_for.find([' ', '{', '<']).unwrap_or(after_for.len());
            let type_name = after_for[..name_end].trim();
            if !type_name.is_empty() {
                return Some(type_name.to_string());
            }
        } else {
            // Handle "impl Type" or "impl<T> Type"
            let after_impl = if trimmed.starts_with("impl<") {
                // Skip generic parameters
                if let Some(gt_pos) = trimmed.find('>') {
                    &trimmed[gt_pos + 1..].trim()
                } else {
                    return None;
                }
            } else {
                &trimmed[4..].trim() // Skip "impl"
            };

            let name_end = after_impl.find([' ', '{', '<']).unwrap_or(after_impl.len());
            let type_name = after_impl[..name_end].trim();
            if !type_name.is_empty() {
                return Some(type_name.to_string());
            }
        }
        None
    }

    /// Build module path from file path relative to src/
    fn build_module_path_from_file(
        file_path: &Path,
        project_type: &RustProjectType,
    ) -> Vec<String> {
        let mut module_path = Vec::new();

        // Only build module paths for cargo projects
        let src_dir = match project_type {
            RustProjectType::CargoWorkspace { root, .. } => root.join("src"),
            RustProjectType::CargoPackage { root, .. } => root.join("src"),
            _ => return module_path,
        };

        // Get the relative path from src/
        if let Ok(relative_path) = file_path.strip_prefix(&src_dir) {
            // Convert path components to module names
            for component in relative_path.components() {
                if let std::path::Component::Normal(os_str) = component {
                    if let Some(name) = os_str.to_str() {
                        // Remove .rs extension and handle special names
                        let module_name = if name == "lib.rs" || name == "main.rs" {
                            continue; // lib.rs and main.rs are root modules
                        } else if name == "mod.rs" {
                            // mod.rs represents the parent directory name
                            if let Some(parent) = relative_path.parent() {
                                if let Some(parent_name) = parent.file_name() {
                                    parent_name.to_str().unwrap_or("").to_string()
                                } else {
                                    continue;
                                }
                            } else {
                                continue;
                            }
                        } else if name.ends_with(".rs") {
                            name.trim_end_matches(".rs").to_string()
                        } else {
                            // Directory name
                            name.to_string()
                        };

                        if !module_name.is_empty() {
                            module_path.push(module_name);
                        }
                    }
                }
            }
        }

        module_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_cargo_script_detection() {
        let content = "#!/usr/bin/env -S cargo +nightly -Zscript\n\nfn main() {}";
        assert!(FileDetector::is_cargo_script(content));
    }

    #[test]
    fn test_function_name_extraction() {
        assert_eq!(
            FileDetector::extract_function_name("fn test_something() {"),
            Some("test_something".to_string())
        );
        assert_eq!(
            FileDetector::extract_function_name("    fn main() {"),
            Some("main".to_string())
        );
    }

    #[test]
    fn test_frontend_library_detection() {
        let temp_dir = TempDir::new().unwrap();
        let frontend_lib = temp_dir.path().join("frontend").join("src").join("lib.rs");
        assert!(FileDetector::is_frontend_library(&frontend_lib));

        let regular_lib = temp_dir.path().join("src").join("lib.rs");
        assert!(!FileDetector::is_frontend_library(&regular_lib));
    }

    #[test]
    fn test_nested_module_path_resolution() -> RazResult<()> {
        let temp_dir = TempDir::new()?;
        let test_file = temp_dir.path().join("nested_test.rs");

        let content = r#"
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_top_level() {
        assert_eq!(2 + 2, 4);
    }

    mod integration {
        use super::*;

        #[test] 
        fn test_integration_basic() {
            assert!(true);
        }

        mod database {
            use super::*;

            #[test]
            fn test_db_connection() {
                assert!(true);
            }

            mod transactions {
                use super::*;

                #[test]
                fn test_transaction_rollback() {
                    assert!(true);
                }
            }
        }
    }
}
"#;

        fs::write(&test_file, content)?;

        // Test detecting nested test at deeply nested level (line 28 - test_transaction_rollback)
        let context = FileDetector::detect_context(
            &test_file,
            Some(Position {
                line: 27,
                column: 1,
            }),
        )?;

        // Find the deeply nested test
        let nested_test = context.entry_points.iter().find(|ep| {
            ep.name == "test_transaction_rollback" && ep.entry_type == EntryPointType::Test
        });

        assert!(
            nested_test.is_some(),
            "Should find the deeply nested test function"
        );

        let test_entry = nested_test.unwrap();
        assert_eq!(
            test_entry.full_path.as_ref().unwrap(),
            "tests::integration::database::transactions::test_transaction_rollback",
            "Should have correct full module path for deeply nested test"
        );

        // Test detecting test at intermediate level (line 20 - test_db_connection)
        let context2 = FileDetector::detect_context(
            &test_file,
            Some(Position {
                line: 19,
                column: 1,
            }),
        )?;

        let mid_level_test = context2
            .entry_points
            .iter()
            .find(|ep| ep.name == "test_db_connection" && ep.entry_type == EntryPointType::Test);

        assert!(
            mid_level_test.is_some(),
            "Should find the mid-level nested test"
        );

        let test_entry2 = mid_level_test.unwrap();
        assert_eq!(
            test_entry2.full_path.as_ref().unwrap(),
            "tests::integration::database::test_db_connection",
            "Should have correct full module path for mid-level nested test"
        );

        // Test detecting top-level test
        let top_level_test = context
            .entry_points
            .iter()
            .find(|ep| ep.name == "test_top_level" && ep.entry_type == EntryPointType::Test);

        assert!(top_level_test.is_some(), "Should find the top-level test");

        let test_entry3 = top_level_test.unwrap();
        assert_eq!(
            test_entry3.full_path.as_ref().unwrap(),
            "tests::test_top_level",
            "Should have correct module path for top-level test"
        );

        Ok(())
    }

    #[test]
    fn test_module_stack_handling_with_complex_nesting() -> RazResult<()> {
        let temp_dir = TempDir::new()?;
        let test_file = temp_dir.path().join("complex_nested.rs");

        let content = r#"
mod outer {
    mod middle {
        #[cfg(test)]
        mod tests {
            #[test]
            fn test_deeply_nested() {
                assert!(true);
            }
            
            mod sub_tests {
                #[test]
                fn test_sub_nested() {
                    assert!(true);
                }
            }
        }
        
        mod other {
            fn regular_function() {}
        }
    }
}

#[cfg(test)]
mod main_tests {
    #[test]
    fn test_main_level() {
        assert!(true);
    }
}
"#;

        fs::write(&test_file, content)?;

        let context = FileDetector::detect_context(&test_file, None)?;

        // Check that we correctly detect tests in different nested locations
        let deeply_nested = context
            .entry_points
            .iter()
            .find(|ep| ep.name == "test_deeply_nested");

        assert!(deeply_nested.is_some(), "Should find deeply nested test");
        assert_eq!(
            deeply_nested.unwrap().full_path.as_ref().unwrap(),
            "outer::middle::tests::test_deeply_nested",
            "Should correctly build path through multiple modules"
        );

        let sub_nested = context
            .entry_points
            .iter()
            .find(|ep| ep.name == "test_sub_nested");

        assert!(sub_nested.is_some(), "Should find sub-nested test");
        assert_eq!(
            sub_nested.unwrap().full_path.as_ref().unwrap(),
            "outer::middle::tests::sub_tests::test_sub_nested",
            "Should correctly build path through all nested modules"
        );

        let main_test = context
            .entry_points
            .iter()
            .find(|ep| ep.name == "test_main_level");

        assert!(main_test.is_some(), "Should find main level test");
        assert_eq!(
            main_test.unwrap().full_path.as_ref().unwrap(),
            "main_tests::test_main_level",
            "Should correctly build path for main level test"
        );

        Ok(())
    }
}
