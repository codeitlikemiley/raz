//! Bazel command builders

pub mod bazel_builder;
mod benchmark_builder;
mod binary_builder;
mod doctest_builder;
mod test_builder;

pub use bazel_builder::BazelCommandBuilder;
