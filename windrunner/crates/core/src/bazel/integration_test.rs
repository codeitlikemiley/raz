//! Integration tests for the new Bazel system

#[cfg(test)]
mod tests {
    use crate::bazel::{BazelTargetFinder, BazelTargetKind};
    use tempfile::TempDir;
    use std::fs;
    
    #[test]
    fn test_integration_with_command_builder() {
        // Create a test workspace
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();
        
        // Create MODULE.bazel to mark workspace root
        fs::write(workspace_root.join("MODULE.bazel"), "").unwrap();
        
        // Create mylib directory structure
        let mylib_dir = workspace_root.join("mylib");
        fs::create_dir(&mylib_dir).unwrap();
        let src_dir = mylib_dir.join("src");
        fs::create_dir(&src_dir).unwrap();
        let tests_dir = mylib_dir.join("tests");
        fs::create_dir(&tests_dir).unwrap();
        
        // Create BUILD.bazel with comprehensive rules
        let build_content = r#"
load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test", "rust_binary", "rust_doc_test", "rust_test_suite")
load("@rules_rust//cargo:defs.bzl", "cargo_build_script")

rust_library(
    name = "mylib",
    srcs = ["src/lib.rs"],
    visibility = ["//visibility:public"],
)

rust_test(
    name = "mylib_test",
    crate = ":mylib",
)

rust_binary(
    name = "mybin",
    srcs = ["src/main.rs"],
    deps = [":mylib"],
)

rust_doc_test(
    name = "mylib_doc_test",
    crate = ":mylib",
)

rust_test_suite(
    name = "integration_tests",
    srcs = glob(["tests/*.rs"]),
    deps = [":mylib"],
)

cargo_build_script(
    name = "build_script",
    srcs = ["build.rs"],
)
"#;
        fs::write(mylib_dir.join("BUILD.bazel"), build_content).unwrap();
        
        // Create source files
        let lib_rs = src_dir.join("lib.rs");
        fs::write(&lib_rs, r#"
//! My library
/// Adds two numbers
/// 
/// # Examples
/// ```
/// assert_eq!(mylib::add(2, 3), 5);
/// ```
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_add() {
        assert_eq!(add(2, 3), 5);
    }
}
"#).unwrap();
        
        let main_rs = src_dir.join("main.rs");
        fs::write(&main_rs, r#"
fn main() {
    println!("Hello from Bazel!");
}
"#).unwrap();
        
        let integration_test = tests_dir.join("integration_test.rs");
        fs::write(&integration_test, r#"
#[test]
fn test_integration() {
    assert_eq!(mylib::add(1, 1), 2);
}
"#).unwrap();
        
        let build_rs = mylib_dir.join("build.rs");
        fs::write(&build_rs, r#"
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
}
"#).unwrap();
        
        // Test the target finder
        let mut finder = BazelTargetFinder::new().unwrap();
        
        // Test finding all targets for the lib file first
        let all_lib_targets = finder.find_targets_for_file(
            &lib_rs,
            workspace_root,
        ).unwrap();
        println!("Found {} targets for lib.rs", all_lib_targets.len());
        for target in &all_lib_targets {
            println!("  - {} ({}): {:?}", target.name, target.label, target.kind);
        }
        
        // Test finding unit test target
        let test_target = finder.find_runnable_target(
            &lib_rs,
            workspace_root,
            Some(BazelTargetKind::Test),
        ).unwrap();
        assert!(test_target.is_some(), "Should find test target for lib.rs");
        let test_target = test_target.unwrap();
        assert_eq!(test_target.name, "mylib_test");
        assert_eq!(test_target.kind, BazelTargetKind::Test);
        assert_eq!(test_target.label, "//mylib:mylib_test");
        
        // Test finding binary target
        let binary_target = finder.find_runnable_target(
            &main_rs,
            workspace_root,
            Some(BazelTargetKind::Binary),
        ).unwrap();
        assert!(binary_target.is_some());
        let binary_target = binary_target.unwrap();
        assert_eq!(binary_target.name, "mybin");
        assert_eq!(binary_target.kind, BazelTargetKind::Binary);
        assert_eq!(binary_target.label, "//mylib:mybin");
        
        // Test finding doc test target
        let doc_test_target = finder.find_doc_test_target(
            &lib_rs,
            workspace_root,
        ).unwrap();
        assert!(doc_test_target.is_some());
        let doc_test_target = doc_test_target.unwrap();
        assert_eq!(doc_test_target.name, "mylib_doc_test");
        assert_eq!(doc_test_target.kind, BazelTargetKind::DocTest);
        
        // Test finding integration test target
        let integration_target = finder.find_integration_test_target(
            &integration_test,
            workspace_root,
        ).unwrap();
        assert!(integration_target.is_some());
        let integration_target = integration_target.unwrap();
        assert_eq!(integration_target.name, "integration_tests");
        assert_eq!(integration_target.kind, BazelTargetKind::TestSuite);
        
        // Test finding all targets for a file
        let all_targets = finder.find_targets_for_file(
            &lib_rs,
            workspace_root,
        ).unwrap();
        assert!(all_targets.len() >= 2); // Should include library and test
    }
    
    #[test]
    fn test_command_builder_integration() {
        use crate::types::{Runnable, RunnableKind, Scope, Position, ScopeKind, FileType};
        use crate::command::builder::{CommandBuilderImpl, bazel::BazelCommandBuilder};
        use crate::config::Config;
        use std::path::PathBuf;
        
        // Create a test runnable
        let runnable = Runnable {
            label: "Test function".to_string(),
            scope: Scope {
                kind: ScopeKind::Function,
                name: Some("test_add".to_string()),
                start: Position { line: 10, character: 0 },
                end: Position { line: 15, character: 0 },
            },
            kind: RunnableKind::Test {
                test_name: "test_add".to_string(),
                is_async: false,
            },
            module_path: "tests".to_string(),
            file_path: PathBuf::from("mylib/src/lib.rs"),
            extended_scope: None,
        };
        
        // Create a config
        let config = Config::default();
        
        // Build command
        let command = BazelCommandBuilder::build(
            &runnable,
            None,
            &config,
            FileType::CargoProject, // Bazel projects use CargoProject FileType
        );
        
        // The command may fail if no workspace is found, but that's expected in unit tests
        // Just verify we can call the builder without panicking
        let _ = command;
    }
}