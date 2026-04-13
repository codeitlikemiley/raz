//! Resolver chain for Cargo target argument selection.
//!
//! Each resolver handles one case (lib, bin, integration test, example)
//! and returns the appropriate `--lib`, `--bin <name>`, `--test <name>`,
//! or `--example <name>` flags to append to the cargo command.
//!
//! Resolvers are tried in priority order; the first one that matches wins.

mod bench;
mod bin;
mod example;
mod integration;
mod lib_src;

pub use bench::BenchResolver;
pub use bin::BinResolver;
pub use example::ExampleResolver;
pub use integration::IntegrationTestResolver;
pub use lib_src::LibResolver;

use std::path::Path;

/// Returns the cargo target flags (e.g. `["--lib"]`, `["--test", "foo"]`)
/// for a given file path and optional package name, or `None` to skip.
pub trait CargoTargetResolver: Send + Sync {
    fn resolve(&self, file_path: &Path, package: Option<&str>) -> Option<Vec<String>>;
    fn priority(&self) -> i32 {
        0
    }
}

/// Ordered chain of resolvers. Tries each in ascending priority order
/// (highest priority first); returns the first `Some` result.
pub struct ResolverChain {
    resolvers: Vec<(i32, Box<dyn CargoTargetResolver>)>,
}

impl ResolverChain {
    pub fn new() -> Self {
        Self {
            resolvers: Vec::new(),
        }
    }

    /// Add a resolver. The chain is sorted by priority (descending) before use.
    pub fn push_resolver(mut self, resolver: impl CargoTargetResolver + 'static) -> Self {
        let prio = resolver.priority();
        self.resolvers.push((prio, Box::new(resolver)));
        // Re-sort: highest priority first
        self.resolvers.sort_by(|a, b| b.0.cmp(&a.0));
        self
    }

    /// Return the target flags from the first matching resolver, or `None`.
    pub fn resolve(&self, file_path: &Path, package: Option<&str>) -> Option<Vec<String>> {
        for (_, resolver) in &self.resolvers {
            if let Some(flags) = resolver.resolve(file_path, package) {
                return Some(flags);
            }
        }
        None
    }

    /// Build the standard chain used for Cargo test target selection.
    pub fn cargo_test_defaults(package: Option<&str>) -> Self {
        // Priorities (highest wins):
        //   300  IntegrationTestResolver  — tests/*.rs
        //   200  BinResolver              — src/main.rs, src/bin/*.rs
        //   150  ExampleResolver          — examples/*.rs
        //   100  BenchResolver            — benches/*.rs
        //    50  LibResolver              — src/**/*.rs (default)
        Self::new()
            .push_resolver(IntegrationTestResolver)
            .push_resolver(BinResolver::new(package.map(str::to_string)))
            .push_resolver(ExampleResolver)
            .push_resolver(BenchResolver)
            .push_resolver(LibResolver)
    }
}

impl Default for ResolverChain {
    fn default() -> Self {
        Self::new()
    }
}
