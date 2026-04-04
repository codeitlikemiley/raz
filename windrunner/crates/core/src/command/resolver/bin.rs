//! Bin resolver — matches `src/main.rs` and `src/bin/*.rs` binary entry points.

use super::CargoTargetResolver;
use std::path::Path;

/// Matches binary entry points in a Cargo project.
///
/// - `src/main.rs` → `["--bin", "<package>"]` (uses package name from Cargo.toml).
///   If the package name is unavailable, falls back to the parent directory name.
/// - `src/bin/<name>.rs` → `["--bin", "<name>"]`
pub struct BinResolver {
    package: Option<String>,
}

impl BinResolver {
    pub fn new(package: Option<String>) -> Self {
        Self { package }
    }
}

impl CargoTargetResolver for BinResolver {
    fn resolve(&self, file_path: &Path, package: Option<&str>) -> Option<Vec<String>> {
        let path_str = file_path.to_string_lossy();

        // src/main.rs or just main.rs / src/main.rs at root level
        let is_main_rs = path_str.ends_with("/src/main.rs")
            || path_str == "src/main.rs"
            || path_str == "main.rs";

        if is_main_rs {
            // Prefer caller-supplied, then struct-stored package name,
            // then parent dir name as fallback
            let bin_name = package
                .map(str::to_string)
                .or_else(|| self.package.clone())
                .or_else(|| {
                    // Walk up: find the directory containing src/main.rs
                    // and use that directory's name as the binary name
                    file_path
                        .parent() // src/
                        .and_then(|p| p.parent()) // project root
                        .and_then(|p| p.file_name())
                        .map(|n| n.to_string_lossy().to_string())
                })?;
            return Some(vec!["--bin".to_string(), bin_name]);
        }

        // src/bin/*.rs
        if path_str.contains("/src/bin/") {
            let stem = file_path.file_stem()?.to_string_lossy().to_string();
            return Some(vec!["--bin".to_string(), stem]);
        }

        None
    }

    fn priority(&self) -> i32 {
        200
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_main_rs_with_package() {
        let r = BinResolver::new(Some("myapp".to_string()));
        let flags = r
            .resolve(&PathBuf::from("myapp/src/main.rs"), None)
            .unwrap();
        assert_eq!(flags, vec!["--bin", "myapp"]);
    }

    #[test]
    fn test_main_rs_caller_package_wins() {
        let r = BinResolver::new(Some("struct_pkg".to_string()));
        let flags = r
            .resolve(&PathBuf::from("myapp/src/main.rs"), Some("caller_pkg"))
            .unwrap();
        assert_eq!(flags, vec!["--bin", "caller_pkg"]);
    }

    #[test]
    fn test_main_rs_dir_fallback() {
        let r = BinResolver::new(None);
        // No package name → derive from parent dir of src/
        let flags = r
            .resolve(&PathBuf::from("my-project/src/main.rs"), None)
            .unwrap();
        assert_eq!(flags, vec!["--bin", "my-project"]);
    }

    #[test]
    fn test_bin_custom_name() {
        let r = BinResolver::new(None);
        let flags = r
            .resolve(&PathBuf::from("myapp/src/bin/server.rs"), None)
            .unwrap();
        assert_eq!(flags, vec!["--bin", "server"]);
    }

    #[test]
    fn test_lib_not_matched() {
        let r = BinResolver::new(None);
        assert!(
            r.resolve(&PathBuf::from("myapp/src/lib.rs"), None)
                .is_none()
        );
    }
}
