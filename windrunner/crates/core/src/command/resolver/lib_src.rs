//! Lib resolver — matches Rust source files inside `src/` that are not binaries.

use super::CargoTargetResolver;
use std::path::Path;

/// Matches test functions inside library source files.
///
/// Returns `["--lib"]` for any `.rs` file under `src/` that is not
/// `src/main.rs` or inside `src/bin/`.
pub struct LibResolver;

impl CargoTargetResolver for LibResolver {
    fn resolve(&self, file_path: &Path, _package: Option<&str>) -> Option<Vec<String>> {
        let path_str = file_path.to_string_lossy();

        let in_src = path_str.contains("/src/") || path_str.starts_with("src/");
        let is_main =
            path_str.ends_with("/main.rs") || path_str == "main.rs" || path_str == "src/main.rs";
        let in_bin = path_str.contains("/src/bin/");

        if in_src && !is_main && !in_bin {
            Some(vec!["--lib".to_string()])
        } else {
            None
        }
    }

    fn priority(&self) -> i32 {
        50 // Lowest — fallback for anything in src/ after more specific resolvers
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_lib_rs() {
        let r = LibResolver;
        assert_eq!(
            r.resolve(&PathBuf::from("myproject/src/lib.rs"), None),
            Some(vec!["--lib".to_string()])
        );
    }

    #[test]
    fn test_nested_module() {
        let r = LibResolver;
        assert_eq!(
            r.resolve(&PathBuf::from("myproject/src/domain/user.rs"), None),
            Some(vec!["--lib".to_string()])
        );
    }

    #[test]
    fn test_main_rs_excluded() {
        let r = LibResolver;
        assert!(
            r.resolve(&PathBuf::from("myproject/src/main.rs"), None)
                .is_none()
        );
    }

    #[test]
    fn test_bin_excluded() {
        let r = LibResolver;
        assert!(
            r.resolve(&PathBuf::from("myproject/src/bin/tool.rs"), None)
                .is_none()
        );
    }

    #[test]
    fn test_integration_test_excluded() {
        let r = LibResolver;
        // tests/ files are not under src/, so LibResolver correctly returns None
        assert!(
            r.resolve(&PathBuf::from("myproject/tests/foo.rs"), None)
                .is_none()
        );
    }
}
