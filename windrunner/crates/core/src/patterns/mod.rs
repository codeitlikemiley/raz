//! Pattern detection for finding runnable items in Rust code

pub mod benchmark;
pub mod binary;
pub mod detector;
pub mod doc_test;
pub mod mod_test;
pub mod pattern;
pub mod test_fn;

// Re-export pattern trait and implementations
pub use benchmark::BenchmarkPattern;
pub use binary::BinaryPattern;
pub use detector::RunnableDetector;
pub use doc_test::DocTestPattern;
pub use mod_test::ModTestPattern;
pub use pattern::Pattern;
pub use test_fn::TestFnPattern;
