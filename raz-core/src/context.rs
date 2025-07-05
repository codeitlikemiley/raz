//! Context analysis and project structure detection

use crate::error::{RazError, RazResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Project context containing all relevant information for command generation
#[derive(Debug, Clone)]
pub struct ProjectContext {
    /// Root directory of the workspace
    pub workspace_root: PathBuf,

    /// Current file being edited (if any)
    pub current_file: Option<FileContext>,

    /// Current cursor position (if available)
    pub cursor_position: Option<Position>,

    /// Detected project type
    pub project_type: ProjectType,

    /// Project dependencies
    pub dependencies: Vec<Dependency>,

    /// Workspace members (for multi-crate workspaces)
    pub workspace_members: Vec<WorkspaceMember>,

    /// Build targets found in the project
    pub build_targets: Vec<BuildTarget>,

    /// Active cargo features
    pub active_features: Vec<String>,

    /// Environment variables relevant to the project
    pub env_vars: HashMap<String, String>,
}

/// File-specific context information
#[derive(Debug, Clone)]
pub struct FileContext {
    /// Path to the file
    pub path: PathBuf,

    /// Programming language of the file
    pub language: Language,

    /// Symbols found in the file (functions, structs, etc.)
    pub symbols: Vec<Symbol>,

    /// Import statements in the file
    pub imports: Vec<Import>,

    /// Symbol at the current cursor position
    pub cursor_symbol: Option<Symbol>,

    /// Module this file belongs to
    pub module_path: Option<String>,
}

/// Cursor position in a file
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub column: u32,
}

/// Range in a file
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

impl Range {
    pub fn contains(&self, position: Position) -> bool {
        position >= self.start && position <= self.end
    }

    pub fn contains_position(&self, position: Position) -> bool {
        self.contains(position)
    }
}

impl PartialOrd for Position {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Position {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.line
            .cmp(&other.line)
            .then(self.column.cmp(&other.column))
    }
}

/// Types of projects that can be detected
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectType {
    Binary,
    Library,
    Workspace,
    Leptos,
    Dioxus,
    Axum,
    Bevy,
    Tauri,
    Yew,
    Mixed(Vec<ProjectType>),
}

/// Programming language detection
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Language {
    Rust,
    Toml,
    Json,
    Yaml,
    Markdown,
    Unknown,
}

impl Language {
    pub fn from_path(path: &Path) -> Self {
        match path.extension().and_then(|s| s.to_str()) {
            Some("rs") => Language::Rust,
            Some("toml") => Language::Toml,
            Some("json") => Language::Json,
            Some("yaml" | "yml") => Language::Yaml,
            Some("md") => Language::Markdown,
            _ => Language::Unknown,
        }
    }
}

/// Symbols found in source code
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub range: Range,
    pub modifiers: Vec<String>,
    pub children: Vec<Symbol>,
    pub metadata: HashMap<String, String>,
}

/// Types of symbols that can be detected
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Struct,
    Enum,
    Trait,
    Module,
    Test,
    Impl,
    Constant,
    Static,
    TypeAlias,
    Macro,
    Variable,
}

/// Symbol visibility (legacy - now using modifiers)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Private,
    Crate,
    Super,
}

/// Import statements
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    pub path: String,
    pub alias: Option<String>,
    pub items: Vec<String>,
}

/// Project dependency information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependency {
    pub name: String,
    pub version: String,
    pub features: Vec<String>,
    pub optional: bool,
    pub dev_dependency: bool,
}

/// Workspace member information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceMember {
    pub name: String,
    pub path: PathBuf,
    pub package_type: ProjectType,
}

/// Build target information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildTarget {
    pub name: String,
    pub target_type: TargetType,
    pub path: PathBuf,
}

/// Types of build targets
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetType {
    Binary,
    Library,
    Example,
    Test,
    Bench,
}

/// Enhanced test context for precise command generation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestContext {
    /// Package name for the test
    pub package_name: Option<String>,
    /// Target type (lib, bin, test, etc.)
    pub target_type: TestTargetType,
    /// Full module path to the test (e.g., ["middleware", "csrf", "tests"])
    pub module_path: Vec<String>,
    /// Specific test function name
    pub test_name: Option<String>,
    /// Required feature flags
    pub features: Vec<String>,
    /// Environment variables needed for the test
    pub env_vars: Vec<(String, String)>,
    /// Working directory for the test command
    pub working_dir: Option<PathBuf>,
}

/// Test target types for enhanced test command generation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestTargetType {
    /// Library unit tests (--lib)
    Lib,
    /// Binary tests (--bin <name>)
    Bin(String),
    /// Integration tests (--test <name>)
    Test(String),
    /// Benchmark tests (--bench <name>)
    Bench(String),
    /// Example tests (--example <name>)
    Example(String),
}

impl TestContext {
    /// Create a new empty test context
    pub fn new() -> Self {
        Self {
            package_name: None,
            target_type: TestTargetType::Lib,
            module_path: Vec::new(),
            test_name: None,
            features: Vec::new(),
            env_vars: Vec::new(),
            working_dir: None,
        }
    }

    /// Get the full module path as a string (e.g., "middleware::csrf::tests")
    pub fn module_path_string(&self) -> Option<String> {
        if self.module_path.is_empty() {
            None
        } else {
            Some(self.module_path.join("::"))
        }
    }

    /// Get the full test path including module and test name
    pub fn full_test_path(&self) -> Option<String> {
        match (self.module_path_string(), &self.test_name) {
            (Some(module), Some(test)) => Some(format!("{module}::{test}")),
            (None, Some(test)) => Some(test.clone()),
            (Some(module), None) => Some(module),
            (None, None) => None,
        }
    }

    /// Check if this context has enough information for a precise test command
    pub fn is_precise(&self) -> bool {
        self.test_name.is_some() || !self.module_path.is_empty()
    }
}

impl Default for TestContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Main project analyzer
pub struct ProjectAnalyzer {
    workspace_detector: WorkspaceDetector,
    dependency_analyzer: DependencyAnalyzer,
    target_detector: TargetDetector,
    framework_detector: FrameworkDetector,
    file_analyzer: FileAnalyzer,
}

impl ProjectAnalyzer {
    pub fn new() -> Self {
        Self {
            workspace_detector: WorkspaceDetector::new(),
            dependency_analyzer: DependencyAnalyzer::new(),
            target_detector: TargetDetector::new(),
            framework_detector: FrameworkDetector::new(),
            file_analyzer: FileAnalyzer::new(),
        }
    }
}

impl Default for ProjectAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl ProjectAnalyzer {
    /// Analyze a project and return comprehensive context
    pub async fn analyze_project(&self, root: &Path) -> RazResult<ProjectContext> {
        // Find Cargo.toml
        let cargo_toml = self.find_cargo_toml(root)?;

        // Analyze workspace structure
        let mut workspace_info = self.workspace_detector.detect(&cargo_toml).await?;

        // Analyze dependencies from root
        let mut all_dependencies = self.dependency_analyzer.analyze(&cargo_toml).await?;

        // If this is a workspace, analyze dependencies from all members
        if workspace_info.is_workspace {
            for member in &mut workspace_info.members {
                let member_cargo_toml = root.join(&member.path).join("Cargo.toml");
                if member_cargo_toml.exists() {
                    // Extract the actual package name from the member's Cargo.toml
                    if let Ok(package_name) =
                        self.extract_package_name_from_toml(&member_cargo_toml)
                    {
                        member.name = package_name;
                    }

                    let member_deps = self.dependency_analyzer.analyze(&member_cargo_toml).await?;

                    // Detect member's project type based on its dependencies
                    let member_root = root.join(&member.path);
                    member.package_type = self
                        .framework_detector
                        .detect(&member_deps, &[], &member_root)
                        .await?;

                    // Add member dependencies to the overall list (avoiding duplicates)
                    for dep in member_deps {
                        if !all_dependencies.iter().any(|d| d.name == dep.name) {
                            all_dependencies.push(dep);
                        }
                    }
                }
            }
        } else {
            // For single package, ensure we have the correct package name
            if !workspace_info.members.is_empty() {
                if let Ok(package_name) = self.extract_package_name_from_toml(&cargo_toml) {
                    workspace_info.members[0].name = package_name;
                }
            }
        }

        // Detect build targets
        let targets = self.target_detector.detect(root).await?;

        // Detect overall framework type based on all dependencies
        let project_type = self
            .framework_detector
            .detect(&all_dependencies, &targets, root)
            .await?;

        Ok(ProjectContext {
            workspace_root: root.to_path_buf(),
            current_file: None,
            cursor_position: None,
            project_type,
            dependencies: all_dependencies,
            workspace_members: workspace_info.members,
            build_targets: targets,
            active_features: Vec::new(),
            env_vars: HashMap::new(),
        })
    }

    /// Analyze a specific file and add its context
    pub async fn analyze_file(
        &self,
        context: &mut ProjectContext,
        file_path: &Path,
        cursor: Option<Position>,
    ) -> RazResult<()> {
        let file_context = self.file_analyzer.analyze_file(file_path, cursor).await?;
        context.current_file = Some(file_context);
        context.cursor_position = cursor;
        Ok(())
    }

    /// Resolve test context from current project context and cursor position
    pub async fn resolve_test_context(
        &self,
        context: &ProjectContext,
    ) -> RazResult<Option<TestContext>> {
        let Some(ref file_context) = context.current_file else {
            return Ok(None);
        };

        // Only proceed if we're in a Rust file
        if file_context.language != Language::Rust {
            return Ok(None);
        }

        let mut test_context = TestContext::new();

        // 1. Resolve package name
        test_context.package_name = self.resolve_package_name(context, &file_context.path)?;

        // 2. Resolve target type (lib/bin/test)
        test_context.target_type = self.resolve_target_type(context, &file_context.path)?;

        // 3. Resolve module path
        test_context.module_path = self.resolve_module_path(&file_context.path)?;

        // 4. Resolve test name if cursor is on a test function
        if let Some(ref cursor_symbol) = file_context.cursor_symbol {
            if cursor_symbol.kind == SymbolKind::Test
                || (cursor_symbol.kind == SymbolKind::Function
                    && cursor_symbol.name.starts_with("test_"))
            {
                test_context.test_name = Some(cursor_symbol.name.clone());
            }
        }

        // 5. Resolve features and environment variables
        test_context.features = self.resolve_test_features(context, &file_context.path)?;
        test_context.env_vars = self.resolve_test_env_vars(context)?;

        // 6. Set working directory
        test_context.working_dir = Some(context.workspace_root.clone());

        // Only return context if we have meaningful test information
        if test_context.is_precise() || test_context.package_name.is_some() {
            Ok(Some(test_context))
        } else {
            Ok(None)
        }
    }

    /// Resolve package name from file path and workspace structure
    fn resolve_package_name(
        &self,
        context: &ProjectContext,
        file_path: &Path,
    ) -> RazResult<Option<String>> {
        // For single-package projects
        if context.workspace_members.len() == 1 {
            return Ok(Some(context.workspace_members[0].name.clone()));
        }

        // For workspace projects, find which member contains this file
        for member in &context.workspace_members {
            let member_path = if member.path.is_absolute() {
                member.path.clone()
            } else {
                context.workspace_root.join(&member.path)
            };

            if file_path.starts_with(&member_path) {
                return Ok(Some(member.name.clone()));
            }
        }

        // Fallback: extract from the nearest Cargo.toml
        let mut current_dir = file_path.parent();
        while let Some(dir) = current_dir {
            let cargo_toml = dir.join("Cargo.toml");
            if cargo_toml.exists() {
                if let Ok(name) = self.extract_package_name_from_toml(&cargo_toml) {
                    return Ok(Some(name));
                }
            }
            current_dir = dir.parent();
        }

        Ok(None)
    }

    /// Extract package name from Cargo.toml
    fn extract_package_name_from_toml(&self, cargo_toml: &Path) -> RazResult<String> {
        let content = fs::read_to_string(cargo_toml)?;
        let parsed: toml::Value = toml::from_str(&content)
            .map_err(|e| RazError::parse(format!("Invalid Cargo.toml: {e}")))?;

        parsed
            .get("package")
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| RazError::parse("No package name found in Cargo.toml".to_string()))
    }

    /// Resolve target type from file path
    fn resolve_target_type(
        &self,
        context: &ProjectContext,
        file_path: &Path,
    ) -> RazResult<TestTargetType> {
        let file_str = file_path.to_string_lossy();

        // Check for integration tests (tests/ directory)
        if file_str.contains("/tests/") {
            if let Some(test_name) = file_path.file_stem().and_then(|s| s.to_str()) {
                return Ok(TestTargetType::Test(test_name.to_string()));
            }
        }

        // Check for examples (examples/ directory)
        if file_str.contains("/examples/") {
            if let Some(example_name) = file_path.file_stem().and_then(|s| s.to_str()) {
                return Ok(TestTargetType::Example(example_name.to_string()));
            }
        }

        // Check for benchmarks (benches/ directory)
        if file_str.contains("/benches/") {
            if let Some(bench_name) = file_path.file_stem().and_then(|s| s.to_str()) {
                return Ok(TestTargetType::Bench(bench_name.to_string()));
            }
        }

        // Check if it's a library file
        if file_str.contains("/src/lib.rs") || file_str.contains("/src/mod.rs") {
            return Ok(TestTargetType::Lib);
        }

        // Check for binary files
        if file_str.contains("/src/main.rs") {
            // Main binary
            return Ok(TestTargetType::Bin("main".to_string()));
        }

        if file_str.contains("/src/bin/") {
            if let Some(bin_name) = file_path.file_stem().and_then(|s| s.to_str()) {
                return Ok(TestTargetType::Bin(bin_name.to_string()));
            }
        }

        // Check against build targets for more precise detection
        for target in &context.build_targets {
            if file_path.starts_with(target.path.parent().unwrap_or(&context.workspace_root)) {
                return match target.target_type {
                    TargetType::Binary => Ok(TestTargetType::Bin(target.name.clone())),
                    TargetType::Library => Ok(TestTargetType::Lib),
                    TargetType::Test => Ok(TestTargetType::Test(target.name.clone())),
                    TargetType::Bench => Ok(TestTargetType::Bench(target.name.clone())),
                    TargetType::Example => Ok(TestTargetType::Example(target.name.clone())),
                };
            }
        }

        // Default to library
        Ok(TestTargetType::Lib)
    }

    /// Resolve module path from file path using multiple strategies
    fn resolve_module_path(&self, file_path: &Path) -> RazResult<Vec<String>> {
        // Strategy 1: Tree-sitter AST analysis (most precise)
        if let Ok(module_path) = self.resolve_module_path_treesitter(file_path) {
            if !module_path.is_empty() {
                return Ok(module_path);
            }
        }

        // Strategy 2: Rust Analyzer LSP integration (if available)
        if let Ok(module_path) = self.resolve_module_path_rust_analyzer(file_path) {
            if !module_path.is_empty() {
                return Ok(module_path);
            }
        }

        // Strategy 3: Basic file path analysis (fallback)
        if let Some(module_str) = FileAnalyzer::extract_module_path(file_path) {
            Ok(module_str
                .split("::")
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect())
        } else {
            Ok(Vec::new())
        }
    }

    /// Resolve module path using tree-sitter AST analysis
    fn resolve_module_path_treesitter(&self, file_path: &Path) -> RazResult<Vec<String>> {
        // Only proceed if tree-sitter is available
        if self.file_analyzer.rust_analyzer.is_none() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(file_path)?;
        let mut rust_analyzer = crate::ast::RustAnalyzer::new()?;
        let tree = rust_analyzer.parse(&content)?;

        // 1. Start with file-based module path
        let mut module_path = Vec::new();
        if let Some(base_path) = FileAnalyzer::extract_module_path(file_path) {
            module_path = base_path
                .split("::")
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect();
        }

        // 2. Look for nested module declarations and test modules
        let symbols = rust_analyzer.extract_symbols(&tree, &content)?;

        // Find test modules (common pattern: mod tests { ... })
        let test_modules: Vec<_> = symbols
            .iter()
            .filter(|s| {
                s.kind == SymbolKind::Module && (s.name == "tests" || s.name.contains("test"))
            })
            .collect();

        // If we're in a test module, add it to the path
        if !test_modules.is_empty() {
            // For now, assume we're in the first test module found
            // TODO: Use cursor position to determine exact module when available
            if let Some(test_mod) = test_modules.first() {
                module_path.push(test_mod.name.clone());
            }
        }

        // 3. Enhance path detection based on common Rust patterns
        module_path = self.enhance_module_path_with_patterns(module_path, file_path, &content)?;

        Ok(module_path)
    }

    /// Enhance module path detection with common Rust patterns
    fn enhance_module_path_with_patterns(
        &self,
        mut module_path: Vec<String>,
        file_path: &Path,
        content: &str,
    ) -> RazResult<Vec<String>> {
        let file_str = file_path.to_string_lossy();

        // Pattern 1: Test files often have tests module
        if (content.contains("#[cfg(test)]") || content.contains("mod tests"))
            && !module_path.iter().any(|m| m == "tests")
        {
            module_path.push("tests".to_string());
        }

        // Pattern 2: Integration tests in tests/ directory
        if file_str.contains("/tests/") {
            // Integration tests don't typically have nested module paths
            // The test file name becomes the module
            if let Some(test_name) = file_path.file_stem().and_then(|s| s.to_str()) {
                module_path = vec![test_name.to_string()];
            }
        }

        // Pattern 3: Common module patterns (middleware, handlers, etc.)
        let common_modules = [
            "middleware",
            "handlers",
            "controllers",
            "services",
            "utils",
            "helpers",
        ];
        for common in &common_modules {
            if file_str.contains(&format!("/{common}/"))
                && !module_path.contains(&common.to_string())
            {
                // Find the position to insert this module
                if let Some(pos) = module_path.iter().position(|m| m == "tests") {
                    module_path.insert(pos, common.to_string());
                } else {
                    module_path.insert(
                        module_path.len().saturating_sub(1).max(0),
                        common.to_string(),
                    );
                }
            }
        }

        Ok(module_path)
    }

    /// Resolve module path using rust-analyzer LSP integration
    fn resolve_module_path_rust_analyzer(&self, _file_path: &Path) -> RazResult<Vec<String>> {
        // TODO: Implement rust-analyzer LSP integration
        // This would involve:
        // 1. Starting/connecting to rust-analyzer LSP server
        // 2. Sending textDocument/documentSymbol request
        // 3. Parsing the response to extract module hierarchy
        // 4. Mapping cursor position to exact module path

        // For now, return empty to fall back to other strategies
        Ok(Vec::new())
    }

    /// Resolve features required for testing this file
    fn resolve_test_features(
        &self,
        context: &ProjectContext,
        _file_path: &Path,
    ) -> RazResult<Vec<String>> {
        // For now, return active features from the project context
        // TODO: In the future, we could parse #[cfg(feature = "...")] attributes from the file
        Ok(context.active_features.clone())
    }

    /// Resolve environment variables needed for testing
    fn resolve_test_env_vars(&self, context: &ProjectContext) -> RazResult<Vec<(String, String)>> {
        Ok(context
            .env_vars
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect())
    }

    fn find_cargo_toml(&self, root: &Path) -> RazResult<PathBuf> {
        let cargo_toml = root.join("Cargo.toml");
        if cargo_toml.exists() {
            Ok(cargo_toml)
        } else {
            Err(RazError::invalid_workspace(root))
        }
    }
}

/// Workspace detection and analysis
pub struct WorkspaceDetector;

impl WorkspaceDetector {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WorkspaceDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkspaceDetector {
    pub async fn detect(&self, cargo_toml: &Path) -> RazResult<WorkspaceInfo> {
        let content = fs::read_to_string(cargo_toml)?;
        let parsed: toml::Value = toml::from_str(&content)
            .map_err(|e| RazError::parse(format!("Invalid Cargo.toml: {e}")))?;

        if let Some(workspace) = parsed.get("workspace") {
            let members = self.extract_workspace_members(workspace)?;
            Ok(WorkspaceInfo {
                is_workspace: true,
                members,
            })
        } else {
            // Single package
            let package_name = parsed
                .get("package")
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or("unknown")
                .to_string();

            Ok(WorkspaceInfo {
                is_workspace: false,
                members: vec![WorkspaceMember {
                    name: package_name,
                    path: cargo_toml.parent().unwrap().to_path_buf(),
                    package_type: ProjectType::Binary, // Will be refined later
                }],
            })
        }
    }

    fn extract_workspace_members(
        &self,
        workspace: &toml::Value,
    ) -> RazResult<Vec<WorkspaceMember>> {
        let members = workspace
            .get("members")
            .and_then(|m| m.as_array())
            .ok_or_else(|| RazError::parse("Workspace missing members"))?;

        let mut result = Vec::new();

        for member in members {
            if let Some(path_str) = member.as_str() {
                result.push(WorkspaceMember {
                    name: path_str.to_string(), // Will be updated with actual package name
                    path: PathBuf::from(path_str),
                    package_type: ProjectType::Binary, // Will be refined later
                });
            }
        }

        Ok(result)
    }
}

#[derive(Debug)]
pub struct WorkspaceInfo {
    pub is_workspace: bool,
    pub members: Vec<WorkspaceMember>,
}

/// Dependency analysis
pub struct DependencyAnalyzer;

impl DependencyAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DependencyAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl DependencyAnalyzer {
    pub async fn analyze(&self, cargo_toml: &Path) -> RazResult<Vec<Dependency>> {
        let content = fs::read_to_string(cargo_toml)?;
        let parsed: toml::Value = toml::from_str(&content)
            .map_err(|e| RazError::parse(format!("Invalid Cargo.toml: {e}")))?;

        let mut dependencies = Vec::new();

        // Regular dependencies
        if let Some(deps) = parsed.get("dependencies").and_then(|d| d.as_table()) {
            dependencies.extend(self.parse_dependencies(deps, false)?);
        }

        // Dev dependencies
        if let Some(dev_deps) = parsed.get("dev-dependencies").and_then(|d| d.as_table()) {
            dependencies.extend(self.parse_dependencies(dev_deps, true)?);
        }

        Ok(dependencies)
    }

    fn parse_dependencies(
        &self,
        deps: &toml::value::Table,
        is_dev: bool,
    ) -> RazResult<Vec<Dependency>> {
        let mut result = Vec::new();

        for (name, value) in deps {
            let dependency = match value {
                toml::Value::String(version) => Dependency {
                    name: name.clone(),
                    version: version.clone(),
                    features: Vec::new(),
                    optional: false,
                    dev_dependency: is_dev,
                },
                toml::Value::Table(table) => {
                    let version = table
                        .get("version")
                        .and_then(|v| v.as_str())
                        .unwrap_or("*")
                        .to_string();

                    let features = table
                        .get("features")
                        .and_then(|f| f.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str())
                                .map(|s| s.to_string())
                                .collect()
                        })
                        .unwrap_or_default();

                    let optional = table
                        .get("optional")
                        .and_then(|o| o.as_bool())
                        .unwrap_or(false);

                    Dependency {
                        name: name.clone(),
                        version,
                        features,
                        optional,
                        dev_dependency: is_dev,
                    }
                }
                _ => continue,
            };

            result.push(dependency);
        }

        Ok(result)
    }
}

/// Build target detection
pub struct TargetDetector;

impl TargetDetector {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TargetDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl TargetDetector {
    pub async fn detect(&self, root: &Path) -> RazResult<Vec<BuildTarget>> {
        let mut targets = Vec::new();

        // Check for main.rs (binary)
        let main_rs = root.join("src/main.rs");
        if main_rs.exists() {
            targets.push(BuildTarget {
                name: "main".to_string(),
                target_type: TargetType::Binary,
                path: main_rs,
            });
        }

        // Check for lib.rs (library)
        let lib_rs = root.join("src/lib.rs");
        if lib_rs.exists() {
            targets.push(BuildTarget {
                name: "lib".to_string(),
                target_type: TargetType::Library,
                path: lib_rs,
            });
        }

        // Check examples directory
        let examples_dir = root.join("examples");
        if examples_dir.exists() {
            for entry in fs::read_dir(&examples_dir)? {
                let entry = entry?;
                if let Some(name) = entry.file_name().to_str() {
                    if name.ends_with(".rs") {
                        let name = name.strip_suffix(".rs").unwrap();
                        targets.push(BuildTarget {
                            name: name.to_string(),
                            target_type: TargetType::Example,
                            path: entry.path(),
                        });
                    }
                }
            }
        }

        // Check tests directory
        let tests_dir = root.join("tests");
        if tests_dir.exists() {
            for entry in fs::read_dir(&tests_dir)? {
                let entry = entry?;
                if let Some(name) = entry.file_name().to_str() {
                    if name.ends_with(".rs") {
                        let name = name.strip_suffix(".rs").unwrap();
                        targets.push(BuildTarget {
                            name: name.to_string(),
                            target_type: TargetType::Test,
                            path: entry.path(),
                        });
                    }
                }
            }
        }

        Ok(targets)
    }
}

/// Framework detection based on dependencies and project structure
pub struct FrameworkDetector {
    rules: Vec<DetectionRule>,
}

impl FrameworkDetector {
    pub fn new() -> Self {
        Self {
            rules: Self::default_rules(),
        }
    }
}

impl Default for FrameworkDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameworkDetector {
    pub async fn detect(
        &self,
        dependencies: &[Dependency],
        _targets: &[BuildTarget],
        workspace_root: &Path,
    ) -> RazResult<ProjectType> {
        let mut detected_frameworks = Vec::new();

        for rule in &self.rules {
            if rule.matches(dependencies, workspace_root) {
                // Check if we already detected this framework type
                if !detected_frameworks.contains(&rule.project_type) {
                    detected_frameworks.push(rule.project_type.clone());
                }
            }
        }

        match detected_frameworks.len() {
            0 => Ok(ProjectType::Binary), // Default fallback
            1 => Ok(detected_frameworks.into_iter().next().unwrap()),
            _ => {
                // Multiple frameworks detected - create a mixed project type
                // Sort frameworks by priority (Tauri first as it's often the wrapper)
                detected_frameworks.sort_by_key(|framework| match framework {
                    ProjectType::Tauri => 0,  // Highest priority - desktop wrapper
                    ProjectType::Leptos => 1, // Web framework
                    ProjectType::Dioxus => 2, // Cross-platform UI
                    ProjectType::Yew => 3,    // Web framework
                    ProjectType::Bevy => 4,   // Game engine
                    ProjectType::Axum => 5,   // Web server
                    _ => 6,                   // Other types
                });
                Ok(ProjectType::Mixed(detected_frameworks))
            }
        }
    }

    fn default_rules() -> Vec<DetectionRule> {
        vec![
            // Tauri detection (highest priority due to specific structure)
            DetectionRule {
                framework: "tauri".to_string(),
                project_type: ProjectType::Tauri,
                conditions: vec![
                    DetectionCondition::HasFilePattern("src-tauri/Cargo.toml".to_string()),
                    DetectionCondition::HasFilePattern("src-tauri/tauri.conf.json".to_string()),
                ],
            },
            // Alternative Tauri detection by dependency
            DetectionRule {
                framework: "tauri".to_string(),
                project_type: ProjectType::Tauri,
                conditions: vec![DetectionCondition::HasDependency("tauri".to_string())],
            },
            // Leptos detection with configuration file
            DetectionRule {
                framework: "leptos".to_string(),
                project_type: ProjectType::Leptos,
                conditions: vec![
                    DetectionCondition::HasAnyDependency(vec![
                        "leptos".to_string(),
                        "leptos_axum".to_string(),
                        "leptos_actix".to_string(),
                        "leptos_router".to_string(),
                        "leptos_reactive".to_string(),
                    ]),
                    DetectionCondition::ConfigFileContains(
                        PathBuf::from("Cargo.toml"),
                        "[package.metadata.leptos]".to_string(),
                    ),
                ],
            },
            // Leptos detection by dependency only (fallback)
            DetectionRule {
                framework: "leptos".to_string(),
                project_type: ProjectType::Leptos,
                conditions: vec![DetectionCondition::HasAnyDependency(vec![
                    "leptos".to_string(),
                    "leptos_axum".to_string(),
                    "leptos_actix".to_string(),
                    "leptos_router".to_string(),
                    "leptos_reactive".to_string(),
                ])],
            },
            // Dioxus detection with configuration file
            DetectionRule {
                framework: "dioxus".to_string(),
                project_type: ProjectType::Dioxus,
                conditions: vec![
                    DetectionCondition::HasDependency("dioxus".to_string()),
                    DetectionCondition::HasFile(PathBuf::from("Dioxus.toml")),
                ],
            },
            // Dioxus detection by dependency only (fallback)
            DetectionRule {
                framework: "dioxus".to_string(),
                project_type: ProjectType::Dioxus,
                conditions: vec![DetectionCondition::HasAnyDependency(vec![
                    "dioxus".to_string(),
                    "dioxus-web".to_string(),
                    "dioxus-desktop".to_string(),
                    "dioxus-mobile".to_string(),
                ])],
            },
            // Bevy detection
            DetectionRule {
                framework: "bevy".to_string(),
                project_type: ProjectType::Bevy,
                conditions: vec![DetectionCondition::HasDependency("bevy".to_string())],
            },
            // Axum detection
            DetectionRule {
                framework: "axum".to_string(),
                project_type: ProjectType::Axum,
                conditions: vec![DetectionCondition::HasDependency("axum".to_string())],
            },
            // Yew detection with Trunk configuration
            DetectionRule {
                framework: "yew".to_string(),
                project_type: ProjectType::Yew,
                conditions: vec![
                    DetectionCondition::HasDependency("yew".to_string()),
                    DetectionCondition::HasFile(PathBuf::from("Trunk.toml")),
                ],
            },
            // Yew detection by dependency only (fallback)
            DetectionRule {
                framework: "yew".to_string(),
                project_type: ProjectType::Yew,
                conditions: vec![DetectionCondition::HasAnyDependency(vec![
                    "yew".to_string(),
                    "yew-router".to_string(),
                    "yew-hooks".to_string(),
                ])],
            },
        ]
    }
}

#[derive(Debug)]
pub struct DetectionRule {
    pub framework: String,
    pub project_type: ProjectType,
    pub conditions: Vec<DetectionCondition>,
}

impl DetectionRule {
    pub fn matches(&self, dependencies: &[Dependency], workspace_root: &Path) -> bool {
        self.conditions
            .iter()
            .all(|condition| condition.is_met(dependencies, workspace_root))
    }
}

#[derive(Debug)]
pub enum DetectionCondition {
    HasDependency(String),
    HasAnyDependency(Vec<String>),
    HasFile(PathBuf),
    HasFilePattern(String),
    ConfigFileContains(PathBuf, String),
}

impl DetectionCondition {
    pub fn is_met(&self, dependencies: &[Dependency], workspace_root: &Path) -> bool {
        match self {
            DetectionCondition::HasDependency(name) => {
                dependencies.iter().any(|dep| dep.name == *name)
            }
            DetectionCondition::HasAnyDependency(names) => {
                dependencies.iter().any(|dep| names.contains(&dep.name))
            }
            DetectionCondition::HasFile(path) => path.exists(),
            DetectionCondition::HasFilePattern(pattern) => {
                Self::check_file_pattern(workspace_root, pattern)
            }
            DetectionCondition::ConfigFileContains(path, content) => {
                Self::check_file_content(workspace_root, path, content)
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

    /// Check if a file contains specific content
    fn check_file_content(workspace_root: &Path, file_path: &Path, content: &str) -> bool {
        let full_path = if file_path.is_absolute() {
            file_path.to_path_buf()
        } else {
            workspace_root.join(file_path)
        };

        match std::fs::read_to_string(&full_path) {
            Ok(file_content) => file_content.contains(content),
            Err(_) => false,
        }
    }
}

/// File analysis using tree-sitter
pub struct FileAnalyzer {
    rust_analyzer: Option<crate::ast::RustAnalyzer>,
}

impl Default for FileAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl FileAnalyzer {
    pub fn new() -> Self {
        let rust_analyzer = crate::ast::RustAnalyzer::new().ok();
        Self { rust_analyzer }
    }

    pub async fn analyze_file(
        &self,
        path: &Path,
        cursor: Option<Position>,
    ) -> RazResult<FileContext> {
        let language = Language::from_path(path);

        if language == Language::Rust && self.rust_analyzer.is_some() {
            self.analyze_rust_file(path, cursor).await
        } else {
            // Fallback for non-Rust files or if tree-sitter fails
            Ok(FileContext {
                path: path.to_path_buf(),
                language,
                symbols: Vec::new(),
                imports: Vec::new(),
                cursor_symbol: None,
                module_path: Self::extract_module_path(path),
            })
        }
    }

    async fn analyze_rust_file(
        &self,
        path: &Path,
        cursor: Option<Position>,
    ) -> RazResult<FileContext> {
        if let Some(ref _analyzer) = self.rust_analyzer {
            let content = fs::read_to_string(path)?;
            let mut rust_analyzer = crate::ast::RustAnalyzer::new()?;
            let tree = rust_analyzer.parse(&content)?;

            // Extract symbols
            let symbols = rust_analyzer.extract_symbols(&tree, &content)?;

            // Find cursor symbol if position is provided
            let cursor_symbol = if let Some(pos) = cursor {
                rust_analyzer.symbol_at_position(&tree, &content, pos)?
            } else {
                None
            };

            // Extract imports (basic implementation)
            let imports = self.extract_imports(&content);

            Ok(FileContext {
                path: path.to_path_buf(),
                language: Language::Rust,
                symbols,
                imports,
                cursor_symbol,
                module_path: Self::extract_module_path(path),
            })
        } else {
            Err(RazError::analysis(
                "Rust analyzer not available".to_string(),
            ))
        }
    }

    fn extract_imports(&self, content: &str) -> Vec<Import> {
        let mut imports = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("use ") {
                if let Some(import) = self.parse_use_statement(trimmed) {
                    imports.push(import);
                }
            }
        }

        imports
    }

    fn parse_use_statement(&self, use_line: &str) -> Option<Import> {
        let use_part = use_line.strip_prefix("use ")?.strip_suffix(";")?;

        // Handle glob imports: use module::*;
        if use_part.ends_with("::*") {
            let path = use_part.strip_suffix("::*")?.to_string();
            return Some(Import {
                path,
                alias: None,
                items: vec!["*".to_string()],
            });
        }

        // Handle braced imports first: use path::{item1, item2 as alias, item3};
        // Check for braces before checking for simple aliases
        if let Some(brace_start) = use_part.find("::{") {
            if let Some(brace_end) = use_part.rfind('}') {
                let path = use_part[..brace_start].trim().to_string();
                let items_str = &use_part[brace_start + 3..brace_end];
                let items = self.parse_import_items(items_str);

                return Some(Import {
                    path,
                    alias: None,
                    items,
                });
            }
        }

        // Handle simple alias: use path as alias;
        if let Some(as_pos) = use_part.find(" as ") {
            let path = use_part[..as_pos].trim();
            let alias = use_part[as_pos + 4..].trim();
            return Some(Import {
                path: path.to_string(),
                alias: Some(alias.to_string()),
                items: vec![],
            });
        }

        // Simple import: use path;
        Some(Import {
            path: use_part.to_string(),
            alias: None,
            items: vec![],
        })
    }

    /// Parse comma-separated import items, handling aliases and nested groups
    fn parse_import_items(&self, items_str: &str) -> Vec<String> {
        let mut items = Vec::new();
        let mut current_item = String::new();
        let mut brace_depth = 0;

        for ch in items_str.chars() {
            match ch {
                '{' => {
                    brace_depth += 1;
                    current_item.push(ch);
                }
                '}' => {
                    brace_depth -= 1;
                    current_item.push(ch);
                }
                ',' if brace_depth == 0 => {
                    let item = current_item.trim();
                    if !item.is_empty() {
                        items.push(item.to_string());
                    }
                    current_item.clear();
                }
                _ => current_item.push(ch),
            }
        }

        // Add the last item
        let item = current_item.trim();
        if !item.is_empty() {
            items.push(item.to_string());
        }

        items
    }

    pub fn extract_module_path(path: &Path) -> Option<String> {
        // Extract module path from file path
        // e.g., src/handlers/auth.rs -> handlers::auth
        if let Some(src_index) = path.to_str()?.find("src/") {
            let relative_path = &path.to_str()?[src_index + 4..];
            let without_extension = relative_path.strip_suffix(".rs")?;
            let module_path = without_extension.replace('/', "::").replace("main", "");
            if module_path.is_empty() {
                None
            } else {
                Some(module_path)
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_project_analysis() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        fs::write(
            &cargo_toml,
            r#"
            [package]
            name = "test-project"
            version = "0.1.0"
            edition = "2021"
            
            [dependencies]
            leptos = "0.5"
        "#,
        )
        .unwrap();

        let analyzer = ProjectAnalyzer::new();
        let context = analyzer.analyze_project(temp_dir.path()).await.unwrap();

        assert_eq!(context.workspace_root, temp_dir.path());
        assert_eq!(context.project_type, ProjectType::Leptos);
        assert!(context.dependencies.iter().any(|d| d.name == "leptos"));
    }

    #[test]
    fn test_language_detection() {
        assert_eq!(Language::from_path(Path::new("main.rs")), Language::Rust);
        assert_eq!(Language::from_path(Path::new("Cargo.toml")), Language::Toml);
        assert_eq!(
            Language::from_path(Path::new("package.json")),
            Language::Json
        );
    }

    #[test]
    fn test_position_ordering() {
        let pos1 = Position { line: 1, column: 5 };
        let pos2 = Position { line: 2, column: 3 };
        let pos3 = Position {
            line: 1,
            column: 10,
        };

        assert!(pos1 < pos2);
        assert!(pos1 < pos3);
        assert!(pos3 < pos2);
    }

    #[test]
    fn test_range_contains() {
        let range = Range {
            start: Position { line: 1, column: 0 },
            end: Position {
                line: 3,
                column: 10,
            },
        };

        assert!(range.contains(Position { line: 2, column: 5 }));
        assert!(range.contains(Position { line: 1, column: 0 }));
        assert!(range.contains(Position {
            line: 3,
            column: 10
        }));
        assert!(!range.contains(Position { line: 0, column: 5 }));
        assert!(!range.contains(Position { line: 4, column: 5 }));
    }

    #[test]
    fn test_file_pattern_detection() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create test files
        std::fs::create_dir_all(temp_path.join("src-tauri")).unwrap();
        std::fs::write(temp_path.join("src-tauri/Cargo.toml"), "").unwrap();
        std::fs::write(temp_path.join("src-tauri/tauri.conf.json"), "{}").unwrap();
        std::fs::write(temp_path.join("Dioxus.toml"), "[application]").unwrap();

        // Test pattern matching
        assert!(DetectionCondition::check_file_pattern(
            temp_path,
            "src-tauri/Cargo.toml"
        ));
        assert!(DetectionCondition::check_file_pattern(
            temp_path,
            "src-tauri/*.json"
        ));
        assert!(DetectionCondition::check_file_pattern(
            temp_path,
            "Dioxus.toml"
        ));
        assert!(!DetectionCondition::check_file_pattern(
            temp_path,
            "nonexistent.toml"
        ));
        assert!(!DetectionCondition::check_file_pattern(
            temp_path,
            "src-nonexistent/*.toml"
        ));
    }

    #[test]
    fn test_file_content_detection() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create test file with content
        let cargo_toml_content = r#"
[package]
name = "test"

[package.metadata.leptos]
output-name = "my-app"
        "#;
        std::fs::write(temp_path.join("Cargo.toml"), cargo_toml_content).unwrap();

        // Test content checking
        assert!(DetectionCondition::check_file_content(
            temp_path,
            &PathBuf::from("Cargo.toml"),
            "[package.metadata.leptos]"
        ));
        assert!(DetectionCondition::check_file_content(
            temp_path,
            &PathBuf::from("Cargo.toml"),
            "output-name"
        ));
        assert!(!DetectionCondition::check_file_content(
            temp_path,
            &PathBuf::from("Cargo.toml"),
            "nonexistent-content"
        ));
        assert!(!DetectionCondition::check_file_content(
            temp_path,
            &PathBuf::from("nonexistent.toml"),
            "any-content"
        ));
    }

    #[tokio::test]
    async fn test_multi_framework_detection() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create a mixed Tauri + Leptos project structure
        std::fs::create_dir_all(temp_path.join("src-tauri")).unwrap();
        std::fs::write(temp_path.join("src-tauri/Cargo.toml"), "").unwrap();
        std::fs::write(temp_path.join("src-tauri/tauri.conf.json"), "{}").unwrap();

        let cargo_toml_content = r#"
[package]
name = "mixed-app"

[package.metadata.leptos]
output-name = "my-app"

[dependencies]
leptos = "0.6"
tauri = "1.0"
        "#;
        std::fs::write(temp_path.join("Cargo.toml"), cargo_toml_content).unwrap();

        // Test framework detection
        let detector = FrameworkDetector::new();
        let deps = vec![
            Dependency {
                name: "leptos".to_string(),
                version: "0.6".to_string(),
                features: vec![],
                optional: false,
                dev_dependency: false,
            },
            Dependency {
                name: "tauri".to_string(),
                version: "1.0".to_string(),
                features: vec![],
                optional: false,
                dev_dependency: false,
            },
        ];

        let result = detector.detect(&deps, &[], temp_path).await.unwrap();

        match result {
            ProjectType::Mixed(frameworks) => {
                assert!(frameworks.contains(&ProjectType::Tauri));
                assert!(frameworks.contains(&ProjectType::Leptos));
                // Tauri should be first (higher priority)
                assert_eq!(frameworks[0], ProjectType::Tauri);
            }
            _ => panic!("Expected Mixed project type, got {result:?}"),
        }
    }

    #[test]
    fn test_complex_use_statement_parsing() {
        let analyzer = FileAnalyzer::new();

        // Test glob import
        let import = analyzer.parse_use_statement("use std::*;").unwrap();
        assert_eq!(import.path, "std");
        assert_eq!(import.items, vec!["*"]);

        // Test simple alias
        let import = analyzer
            .parse_use_statement("use std::collections::HashMap as Map;")
            .unwrap();
        assert_eq!(import.path, "std::collections::HashMap");
        assert_eq!(import.alias, Some("Map".to_string()));

        // Test braced imports
        let import = analyzer
            .parse_use_statement("use std::{fs, io, collections::HashMap};")
            .unwrap();
        assert_eq!(import.path, "std");
        assert_eq!(import.items, vec!["fs", "io", "collections::HashMap"]);

        // Test complex braced imports with aliases
        let import = analyzer
            .parse_use_statement("use crate::module::{Item1, Item2 as Alias, Item3};")
            .unwrap();
        assert_eq!(import.path, "crate::module");
        assert_eq!(import.items, vec!["Item1", "Item2 as Alias", "Item3"]);

        // Test nested braces (complex case)
        let import = analyzer
            .parse_use_statement("use std::{fs, io::{self, Read, Write}};")
            .unwrap();
        assert_eq!(import.path, "std");
        assert_eq!(import.items, vec!["fs", "io::{self, Read, Write}"]);

        // Test simple import
        let import = analyzer
            .parse_use_statement("use std::collections::HashMap;")
            .unwrap();
        assert_eq!(import.path, "std::collections::HashMap");
        assert!(import.items.is_empty());
        assert!(import.alias.is_none());
    }
}
