//! Bazel command builders

pub mod bazel_builder;
mod binary_builder;
mod test_builder;
mod benchmark_builder;
mod doctest_builder;

pub use bazel_builder::BazelCommandBuilder;
