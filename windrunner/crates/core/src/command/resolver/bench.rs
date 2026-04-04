//! Bench resolver — matches `benches/*.rs` benchmark files.

use super::CargoTargetResolver;
use std::path::Path;

/// Matches Rust benchmark files.
///
/// Returns `["--bench", "<stem>"]` for any `.rs` file inside a `benches/` directory.
pub struct BenchResolver;

impl CargoTargetResolver for BenchResolver {
    fn resolve(&self, file_path: &Path, _package: Option<&str>) -> Option<Vec<String>> {
        let path_str = file_path.to_string_lossy();

        if path_str.contains("/benches/") || path_str.starts_with("benches/") {
            let stem = file_path.file_stem()?.to_string_lossy().to_string();
            Some(vec!["--bench".to_string(), stem])
        } else {
            None
        }
    }

    fn priority(&self) -> i32 {
        100
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_bench_file() {
        let r = BenchResolver;
        let flags = r
            .resolve(&PathBuf::from("myapp/benches/throughput.rs"), None)
            .unwrap();
        assert_eq!(flags, vec!["--bench", "throughput"]);
    }

    #[test]
    fn test_src_not_matched() {
        let r = BenchResolver;
        assert!(
            r.resolve(&PathBuf::from("myapp/src/lib.rs"), None)
                .is_none()
        );
    }
}
