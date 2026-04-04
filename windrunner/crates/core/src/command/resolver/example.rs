//! Example resolver — matches `examples/*.rs` files.

use super::CargoTargetResolver;
use std::path::Path;

/// Matches Rust example files.
///
/// Returns `["--example", "<stem>"]` for any `.rs` file directly inside an
/// `examples/` directory.
pub struct ExampleResolver;

impl CargoTargetResolver for ExampleResolver {
    fn resolve(&self, file_path: &Path, _package: Option<&str>) -> Option<Vec<String>> {
        let path_str = file_path.to_string_lossy();

        if path_str.contains("/examples/") || path_str.starts_with("examples/") {
            let stem = file_path.file_stem()?.to_string_lossy().to_string();
            Some(vec!["--example".to_string(), stem])
        } else {
            None
        }
    }

    fn priority(&self) -> i32 {
        150
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_example_file() {
        let r = ExampleResolver;
        let flags = r
            .resolve(&PathBuf::from("myapp/examples/demo.rs"), None)
            .unwrap();
        assert_eq!(flags, vec!["--example", "demo"]);
    }

    #[test]
    fn test_src_not_matched() {
        let r = ExampleResolver;
        assert!(
            r.resolve(&PathBuf::from("myapp/src/lib.rs"), None)
                .is_none()
        );
    }
}
