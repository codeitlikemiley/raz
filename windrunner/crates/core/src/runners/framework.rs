//! Framework system for different runnable types

use crate::error::Result;
use crate::types::RunnableKind;

/// Different framework kinds supported by runners
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameworkKind {
    Test,
    Binary,
    Benchmark,
    DocTest,
    Build,
}

impl FrameworkKind {
    /// Convert from RunnableKind
    pub fn from_runnable_kind(kind: &RunnableKind) -> Self {
        match kind {
            RunnableKind::Test { .. } => FrameworkKind::Test,
            RunnableKind::ModuleTests { .. } => FrameworkKind::Test,
            RunnableKind::Binary { .. } => FrameworkKind::Binary,
            RunnableKind::Benchmark { .. } => FrameworkKind::Benchmark,
            RunnableKind::DocTest { .. } => FrameworkKind::DocTest,
            RunnableKind::Standalone { .. } => FrameworkKind::Binary,
            RunnableKind::SingleFileScript { .. } => FrameworkKind::Binary,
        }
    }
}

/// Base trait for all frameworks
pub trait Framework: Send + Sync {
    /// Get the framework name
    fn name(&self) -> &'static str;

    /// Get the framework kind
    fn kind(&self) -> FrameworkKind;

    /// Validate options specific to this framework
    fn validate_options(&self, options: &FrameworkOptions) -> Result<()>;

    /// Build command arguments for this framework
    fn build_args(&self, options: &FrameworkOptions) -> Vec<String>;

    /// Get the base command for this framework (e.g., "test", "run", "bench")
    fn base_command(&self) -> &'static str;
}

/// Options that can be configured per framework
#[derive(Debug, Clone, Default)]
pub struct FrameworkOptions {
    /// Test-specific options
    pub test_options: Option<TestOptions>,

    /// Binary-specific options  
    pub binary_options: Option<BinaryOptions>,

    /// Benchmark-specific options
    pub benchmark_options: Option<BenchmarkOptions>,

    /// DocTest-specific options
    pub doctest_options: Option<DocTestOptions>,
}

#[derive(Debug, Clone, Default)]
pub struct TestOptions {
    pub no_run: bool,
    pub no_fail_fast: bool,
    pub test_threads: Option<u32>,
    pub nocapture: bool,
    pub exact: bool,
    pub quiet: bool,
    pub show_output: bool,
    pub ignored: bool,
    pub include_ignored: bool,
    pub test_name_filter: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct BinaryOptions {
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct BenchmarkOptions {
    pub no_run: bool,
    pub no_fail_fast: bool,
    pub bench_name_filter: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct DocTestOptions {
    pub no_run: bool,
    pub test_args: Vec<String>,
}

/// Test framework implementation
pub struct TestFramework;

impl Framework for TestFramework {
    fn name(&self) -> &'static str {
        "test"
    }

    fn kind(&self) -> FrameworkKind {
        FrameworkKind::Test
    }

    fn validate_options(&self, options: &FrameworkOptions) -> Result<()> {
        if let Some(test_opts) = &options.test_options {
            if test_opts.ignored && test_opts.include_ignored {
                return Err(crate::error::Error::Other(
                    "--ignored and --include-ignored cannot be used together".to_string(),
                ));
            }
        }
        Ok(())
    }

    fn build_args(&self, options: &FrameworkOptions) -> Vec<String> {
        let mut args = vec![];

        if let Some(test_opts) = &options.test_options {
            if test_opts.no_run {
                args.push("--no-run".to_string());
            }
            if test_opts.no_fail_fast {
                args.push("--no-fail-fast".to_string());
            }
            if test_opts.nocapture {
                args.push("--".to_string());
                args.push("--nocapture".to_string());
            }
            if test_opts.exact {
                args.push("--".to_string());
                args.push("--exact".to_string());
            }
            if let Some(threads) = test_opts.test_threads {
                args.push("--".to_string());
                args.push("--test-threads".to_string());
                args.push(threads.to_string());
            }
        }

        args
    }

    fn base_command(&self) -> &'static str {
        "test"
    }
}

/// Binary framework implementation
pub struct BinaryFramework;

impl Framework for BinaryFramework {
    fn name(&self) -> &'static str {
        "run"
    }

    fn kind(&self) -> FrameworkKind {
        FrameworkKind::Binary
    }

    fn validate_options(&self, _options: &FrameworkOptions) -> Result<()> {
        Ok(())
    }

    fn build_args(&self, options: &FrameworkOptions) -> Vec<String> {
        let mut args = vec![];

        if let Some(binary_opts) = &options.binary_options {
            if !binary_opts.args.is_empty() {
                args.push("--".to_string());
                args.extend(binary_opts.args.clone());
            }
        }

        args
    }

    fn base_command(&self) -> &'static str {
        "run"
    }
}

/// Benchmark framework implementation
pub struct BenchmarkFramework;

impl Framework for BenchmarkFramework {
    fn name(&self) -> &'static str {
        "bench"
    }

    fn kind(&self) -> FrameworkKind {
        FrameworkKind::Benchmark
    }

    fn validate_options(&self, _options: &FrameworkOptions) -> Result<()> {
        Ok(())
    }

    fn build_args(&self, options: &FrameworkOptions) -> Vec<String> {
        let mut args = vec![];

        if let Some(bench_opts) = &options.benchmark_options {
            if bench_opts.no_run {
                args.push("--no-run".to_string());
            }
            if bench_opts.no_fail_fast {
                args.push("--no-fail-fast".to_string());
            }
        }

        args
    }

    fn base_command(&self) -> &'static str {
        "bench"
    }
}

/// DocTest framework implementation
pub struct DocTestFramework;

impl Framework for DocTestFramework {
    fn name(&self) -> &'static str {
        "test"
    }

    fn kind(&self) -> FrameworkKind {
        FrameworkKind::DocTest
    }

    fn validate_options(&self, _options: &FrameworkOptions) -> Result<()> {
        Ok(())
    }

    fn build_args(&self, options: &FrameworkOptions) -> Vec<String> {
        let mut args = vec!["--doc".to_string()];

        if let Some(doctest_opts) = &options.doctest_options {
            if doctest_opts.no_run {
                args.push("--no-run".to_string());
            }
            if !doctest_opts.test_args.is_empty() {
                args.push("--".to_string());
                args.extend(doctest_opts.test_args.clone());
            }
        }

        args
    }

    fn base_command(&self) -> &'static str {
        "test"
    }
}
