use crate::{
    error::Result,
    types::{Runnable, Scope},
};
use std::path::Path;

/// Trait for detecting different kinds of runnable items in Rust code
pub trait Pattern: Send + Sync {
    fn detect(&self, scope: &Scope, source: &str, file_path: &Path) -> Result<Option<Runnable>>;
}
