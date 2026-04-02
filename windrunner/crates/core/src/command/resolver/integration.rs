//! Integration test resolver — matches files that are direct children of a `tests/` directory.

use super::CargoTargetResolver;
use std::path::Path;

/// Matches Rust integration test files: direct children of any `tests/` directory.
///
/// Returns `["--test", "<stem>"]` for `tests/my_test.rs`, but NOT for
/// `tests/helpers/util.rs` (that is a support module, not a test binary entry point).
pub struct IntegrationTestResolver;

impl CargoTargetResolver for IntegrationTestResolver {
    fn resolve(&self, file_path: &Path, _package: Option<&str>) -> Option<Vec<String>> {
        let parent = file_path.parent()?;

        // The immediate parent directory must be named exactly "tests"
        let parent_name = parent.file_name()?;
        if parent_name != "tests" {
            return None;
        }

        let stem = file_path.file_stem()?.to_string_lossy();
        Some(vec!["--test".to_string(), stem.to_string()])
    }

    fn priority(&self) -> i32 {
        300 // Highest — integration tests are very specific
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_direct_tests_child() {
        let r = IntegrationTestResolver;
        let path = PathBuf::from("project/tests/api_test.rs");
        let flags = r.resolve(&path, None).unwrap();
        assert_eq!(flags, vec!["--test", "api_test"]);
    }

    #[test]
    fn test_nested_tests_child_not_matched() {
        let r = IntegrationTestResolver;
        // tests/helpers/util.rs — nested, should NOT get --test flag
        let path = PathBuf::from("project/tests/helpers/util.rs");
        assert!(r.resolve(&path, None).is_none());
    }

    #[test]
    fn test_src_file_not_matched() {
        let r = IntegrationTestResolver;
        let path = PathBuf::from("project/src/lib.rs");
        assert!(r.resolve(&path, None).is_none());
    }
}
