use std::path::Path;

/// Represents different cargo target types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Lib,
    Bin(String),
    Example(String),
    Test(String),
    Bench(String),
}

impl Target {
    pub fn from_file_path(file_path: &Path) -> Option<Self> {
        let path_str = file_path.to_str()?;

        if path_str.contains("/src/bin/") {
            let name = file_path.file_stem()?.to_str()?.to_string();
            Some(Target::Bin(name))
        } else if path_str.contains("/examples/") {
            let name = file_path.file_stem()?.to_str()?.to_string();
            Some(Target::Example(name))
        } else if path_str.contains("/tests/") && !path_str.ends_with("/mod.rs") {
            let name = file_path.file_stem()?.to_str()?.to_string();
            Some(Target::Test(name))
        } else if path_str.contains("/benches/") {
            let name = file_path.file_stem()?.to_str()?.to_string();
            Some(Target::Bench(name))
        } else if path_str.ends_with("/src/lib.rs") || path_str == "src/lib.rs" {
            Some(Target::Lib)
        } else if path_str.ends_with("/src/main.rs") || path_str == "src/main.rs" {
            Some(Target::Bin("main".to_string()))
        } else if (path_str.contains("/src/") || path_str.starts_with("src/"))
            && !path_str.contains("/src/bin/")
            && !path_str.starts_with("src/bin/")
        {
            // Any other file under src/ is part of the library
            Some(Target::Lib)
        } else {
            None
        }
    }
}
