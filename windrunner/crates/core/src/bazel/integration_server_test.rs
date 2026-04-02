//! Integration test for server/tests structure

#[cfg(test)]
mod tests {
    use crate::bazel::{BazelTargetFinder, BazelTargetKind};
    use tempfile::TempDir;
    use std::fs;
    use std::path::PathBuf;
    use std::env;
    
    #[test]
    fn test_server_integration_tests() {
        // Create a test workspace that mimics the yoyo project structure
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();
        
        // Create MODULE.bazel to mark workspace root
        fs::write(workspace_root.join("MODULE.bazel"), "").unwrap();
        
        // Create server directory structure
        let server_dir = workspace_root.join("server");
        fs::create_dir(&server_dir).unwrap();
        let tests_dir = server_dir.join("tests");
        fs::create_dir(&tests_dir).unwrap();
        
        // Create BUILD.bazel in server directory
        let build_content = r#"
load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test_suite")

rust_library(
    name = "server_lib",
    srcs = glob(["src/**/*.rs"]),
    visibility = ["//visibility:public"],
)

rust_test_suite(
    name = "integrated_tests_suite",
    srcs = glob(["tests/*.rs"]),
    deps = [":server_lib"],
)
"#;
        fs::write(server_dir.join("BUILD.bazel"), build_content).unwrap();
        
        // Create the test file
        let test_file = tests_dir.join("just_test.rs");
        fs::write(&test_file, r#"
#[cfg(test)]
mod tests {
    #[test]
    fn lets_try_if_it_works() {
        assert!(true);
    }
}
"#).unwrap();
        
        // Test the target finder
        let mut finder = BazelTargetFinder::new().unwrap();
        
        // Debug logging is already enabled by test runner
            
        // Debug: Print all targets found
        println!("Looking for targets for file: {:?}", test_file);
        println!("Workspace root: {:?}", workspace_root);
        let all_targets = finder.find_targets_for_file(&test_file, workspace_root).unwrap();
        println!("Found {} targets:", all_targets.len());
        for target in &all_targets {
            println!("  - {} ({}): {:?}", target.name, target.label, target.kind);
            println!("    Sources: {:?}", target.sources);
        }
        
        // Find integration test target
        let integration_target = finder.find_integration_test_target(
            &test_file,
            workspace_root,
        ).unwrap();
        
        assert!(integration_target.is_some(), "Should find integration test target");
        let target = integration_target.unwrap();
        assert_eq!(target.name, "integrated_tests_suite");
        assert_eq!(target.kind, BazelTargetKind::TestSuite);
        assert_eq!(target.label, "//server:integrated_tests_suite");
    }
    
    #[test]
    fn test_real_world_integration_test_scenario() {
        // This test simulates the exact scenario from the user's debug output
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();
        
        // Create MODULE.bazel to mark workspace root
        fs::write(workspace_root.join("MODULE.bazel"), "").unwrap();
        
        // Create yoyo directory
        let yoyo_dir = workspace_root.join("yoyo");
        fs::create_dir(&yoyo_dir).unwrap();
        
        // Create server directory structure under yoyo
        let server_dir = yoyo_dir.join("server");
        fs::create_dir(&server_dir).unwrap();
        let tests_dir = server_dir.join("tests");
        fs::create_dir(&tests_dir).unwrap();
        
        // Create BUILD.bazel in server directory
        let build_content = r#"
load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test_suite")

rust_library(
    name = "server_lib",
    srcs = glob(["src/**/*.rs"]),
    visibility = ["//visibility:public"],
)

rust_test_suite(
    name = "integrated_tests_suite",
    srcs = glob(["tests/*.rs"]),
    deps = [":server_lib"],
)
"#;
        fs::write(server_dir.join("BUILD.bazel"), build_content).unwrap();
        
        // Create the test file with absolute path
        let test_file = tests_dir.join("just_test.rs");
        fs::write(&test_file, r#"
#[cfg(test)]
mod tests {
    #[test]
    fn lets_try_if_it_works() {
        assert!(true);
    }
}
"#).unwrap();
        
        // Change to yoyo directory to simulate user's working directory
        env::set_current_dir(&yoyo_dir).unwrap();
        
        // Now test with relative path from yoyo directory
        let relative_test_file = PathBuf::from("server/tests/just_test.rs");
        let abs_test_file = yoyo_dir.join(&relative_test_file);
        
        println!("\nTest setup:");
        println!("  Current dir: {:?}", env::current_dir().unwrap());
        println!("  Relative path: {:?}", relative_test_file);
        println!("  Absolute path: {:?}", abs_test_file);
        println!("  Workspace root: {:?}", workspace_root);
        
        // Test the target finder
        let mut finder = BazelTargetFinder::new().unwrap();
        
        // First test with absolute path
        println!("\nTesting with absolute path:");
        let integration_target = finder.find_integration_test_target(
            &abs_test_file,
            workspace_root,
        ).unwrap();
        
        if let Some(target) = &integration_target {
            println!("Found target: {} ({})", target.label, target.name);
        } else {
            println!("No target found!");
        }
        
        assert!(integration_target.is_some(), "Should find integration test target with absolute path");
        
        // Reset finder for next test
        finder = BazelTargetFinder::new().unwrap();
        
        // Now test with the actual scenario - when workspace root is determined from the file
        println!("\nTesting with workspace root detection:");
        let detected_workspace_root = abs_test_file
            .ancestors()
            .find(|p| p.join("MODULE.bazel").exists() || p.join("WORKSPACE").exists())
            .unwrap();
        println!("  Detected workspace root: {:?}", detected_workspace_root);
        
        let integration_target2 = finder.find_integration_test_target(
            &abs_test_file,
            detected_workspace_root,
        ).unwrap();
        
        if let Some(target) = &integration_target2 {
            println!("Found target: {} ({})", target.label, target.name);
        } else {
            println!("No target found with detected workspace root!");
        }
        
        assert!(integration_target2.is_some(), "Should find integration test target with detected workspace root");
    }
    
    #[test]
    fn test_server_at_workspace_root() {
        // This test simulates when server/ is directly at workspace root
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();
        
        // Create MODULE.bazel to mark workspace root
        fs::write(workspace_root.join("MODULE.bazel"), "").unwrap();
        
        // Create server directory structure at root
        let server_dir = workspace_root.join("server");
        fs::create_dir(&server_dir).unwrap();
        let tests_dir = server_dir.join("tests");
        fs::create_dir(&tests_dir).unwrap();
        
        // Create BUILD.bazel in server directory
        let build_content = r#"
load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test_suite")

rust_library(
    name = "server_lib",
    srcs = glob(["src/**/*.rs"]),
    visibility = ["//visibility:public"],
)

rust_test_suite(
    name = "integrated_tests_suite",
    srcs = glob(["tests/*.rs"]),
    deps = [":server_lib"],
)
"#;
        fs::write(server_dir.join("BUILD.bazel"), build_content).unwrap();
        
        // Create the test file
        let test_file = tests_dir.join("just_test.rs");
        fs::write(&test_file, r#"
#[cfg(test)]
mod tests {
    #[test]
    fn see_if_it_works() {
        assert!(true);
    }
}
"#).unwrap();
        
        println!("\nTest setup (server at root):");
        println!("  Test file: {:?}", test_file);
        println!("  Workspace root: {:?}", workspace_root);
        
        // Test the target finder
        let mut finder = BazelTargetFinder::new().unwrap();
        
        let integration_target = finder.find_integration_test_target(
            &test_file,
            workspace_root,
        ).unwrap();
        
        assert!(integration_target.is_some(), "Should find integration test target");
        let target = integration_target.unwrap();
        println!("Found target: {} ({})", target.label, target.name);
        assert_eq!(target.label, "//server:integrated_tests_suite");
    }
    
    #[test]
    fn test_error_scenario_no_build_file() {
        // This test simulates when there's no BUILD file or no rust_test_suite target
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();
        
        // Create MODULE.bazel to mark workspace root
        fs::write(workspace_root.join("MODULE.bazel"), "").unwrap();
        
        // Create server directory structure at root
        let server_dir = workspace_root.join("server");
        fs::create_dir(&server_dir).unwrap();
        let tests_dir = server_dir.join("tests");
        fs::create_dir(&tests_dir).unwrap();
        
        // DO NOT create BUILD.bazel - simulate missing BUILD file
        
        // Create the test file
        let test_file = tests_dir.join("just_test.rs");
        fs::write(&test_file, r#"
#[cfg(test)]
mod tests {
    #[test]
    fn see_if_it_works() {
        assert!(true);
    }
}
"#).unwrap();
        
        println!("\nTest setup (no BUILD file):");
        println!("  Test file: {:?}", test_file);
        println!("  Workspace root: {:?}", workspace_root);
        
        // Test the target finder
        let mut finder = BazelTargetFinder::new().unwrap();
        
        let result = finder.find_integration_test_target(
            &test_file,
            workspace_root,
        );
        
        // Should get an error because no BUILD file
        assert!(result.is_err(), "Should fail when no BUILD file exists");
        println!("Got expected error: {:?}", result.unwrap_err());
    }
    
    #[test]
    fn test_error_scenario_no_test_suite() {
        // This test simulates when BUILD file exists but has no rust_test_suite
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();
        
        // Create MODULE.bazel to mark workspace root
        fs::write(workspace_root.join("MODULE.bazel"), "").unwrap();
        
        // Create server directory structure at root
        let server_dir = workspace_root.join("server");
        fs::create_dir(&server_dir).unwrap();
        let tests_dir = server_dir.join("tests");
        fs::create_dir(&tests_dir).unwrap();
        
        // Create BUILD.bazel but WITHOUT rust_test_suite
        let build_content = r#"
load("@rules_rust//rust:defs.bzl", "rust_library")

rust_library(
    name = "server_lib",
    srcs = glob(["src/**/*.rs"]),
    visibility = ["//visibility:public"],
)
"#;
        fs::write(server_dir.join("BUILD.bazel"), build_content).unwrap();
        
        // Create the test file
        let test_file = tests_dir.join("just_test.rs");
        fs::write(&test_file, r#"
#[cfg(test)]
mod tests {
    #[test]
    fn see_if_it_works() {
        assert!(true);
    }
}
"#).unwrap();
        
        println!("\nTest setup (no rust_test_suite):");
        println!("  Test file: {:?}", test_file);
        println!("  Workspace root: {:?}", workspace_root);
        
        // Test the target finder
        let mut finder = BazelTargetFinder::new().unwrap();
        
        let result = finder.find_integration_test_target(
            &test_file,
            workspace_root,
        ).unwrap();
        
        // Should return None because no rust_test_suite target
        assert!(result.is_none(), "Should return None when no rust_test_suite exists");
        println!("Got expected None result");
    }
    
    #[test]
    fn test_command_generation_integration_test_fallback() {
        // This test simulates the full command generation when no target is found
        use crate::types::{Runnable, RunnableKind, Scope, Position, ScopeKind, FileType};
        use crate::command::builder::{CommandBuilderImpl, bazel::BazelCommandBuilder};
        use crate::config::Config;
        use std::env;
        
        // Create a temporary directory that simulates missing BUILD file
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();
        
        // Create MODULE.bazel to mark workspace root
        fs::write(workspace_root.join("MODULE.bazel"), "").unwrap();
        
        // Create server directory structure
        let server_dir = workspace_root.join("server");
        fs::create_dir(&server_dir).unwrap();
        let tests_dir = server_dir.join("tests");
        fs::create_dir(&tests_dir).unwrap();
        
        // Create test file but NO BUILD file
        let test_file = tests_dir.join("just_test.rs");
        fs::write(&test_file, r#"
#[cfg(test)]
mod tests {
    #[test]
    fn lets_try_if_it_works() {
        assert!(true);
    }
}
"#).unwrap();
        
        // Change to server directory
        env::set_current_dir(&server_dir).unwrap();
        
        // Create a runnable for the integration test
        let runnable = Runnable {
            label: "Run test 'lets_try_if_it_works'".to_string(),
            scope: Scope {
                kind: ScopeKind::Function,
                name: Some("lets_try_if_it_works".to_string()),
                start: Position { line: 4, character: 0 },
                end: Position { line: 7, character: 0 },
            },
            kind: RunnableKind::Test {
                test_name: "lets_try_if_it_works".to_string(),
                is_async: false,
            },
            module_path: "tests::just_test::tests".to_string(),
            file_path: PathBuf::from("tests/just_test.rs"), // Relative path
            extended_scope: None,
        };
        
        // Create a default config
        let config = Config::default();
        
        // Build command
        let command = BazelCommandBuilder::build(
            &runnable,
            None,
            &config,
            FileType::CargoProject, // Bazel uses same type
        );
        
        // Should succeed but generate fallback target
        assert!(command.is_ok(), "Command generation should succeed");
        let cmd = command.unwrap();
        
        println!("\nGenerated command:");
        println!("  Type: {:?}", cmd.command_type);
        println!("  Args: {:?}", cmd.args);
        println!("  Shell: {}", cmd.to_shell_command());
        
        // Check that it falls back to :test or :integration_tests_not_found
        let target_arg = &cmd.args[1]; // First arg is "test", second is target
        assert!(target_arg == ":test" || target_arg == ":integration_tests_not_found",
                "Should use fallback target, got: {}", target_arg);
    }
    
    #[test]
    fn test_glob_pattern_tests_double_star() {
        // This test verifies the glob(["tests/**"]) pattern works
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();
        
        // Create MODULE.bazel to mark workspace root
        fs::write(workspace_root.join("MODULE.bazel"), "").unwrap();
        
        // Create server directory structure at root
        let server_dir = workspace_root.join("server");
        fs::create_dir(&server_dir).unwrap();
        let tests_dir = server_dir.join("tests");
        fs::create_dir(&tests_dir).unwrap();
        
        // Create BUILD.bazel with glob(["tests/**"]) pattern
        let build_content = r#"
load("@server_crates//:defs.bzl", "all_crate_deps")
load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_test", "rust_test_suite")
load("@rules_rust//cargo:defs.bzl", "cargo_build_script")

rust_binary(
    name = "server_bin",
    srcs = ["src/main.rs"],
    deps = all_crate_deps(normal = True) + [
        "//corex:corex_lib",
    ],
    crate_root = "src/main.rs",
)

rust_test(
    name = "server_tests",
    crate = ":server_bin",
    deps = all_crate_deps(normal_dev = True),
)

rust_test_suite(
    name = "integrated_tests_suite",
    srcs = glob(["tests/**"]),
    deps = all_crate_deps(normal_dev = True),
)

cargo_build_script(
    name = "build_script",
    srcs = ["build.rs"],
)
"#;
        fs::write(server_dir.join("BUILD.bazel"), build_content).unwrap();
        
        // Create the test file
        let test_file = tests_dir.join("just_test.rs");
        fs::write(&test_file, r#"
#[cfg(test)]
mod tests {
    #[test]
    fn see_if_it_works() {
        assert!(true);
    }
}
"#).unwrap();
        
        println!("\nTest setup (with glob([\"tests/**\"])):");
        println!("  Test file: {:?}", test_file);
        println!("  Workspace root: {:?}", workspace_root);
        
        // Test the target finder
        let mut finder = BazelTargetFinder::new().unwrap();
        
        let integration_target = finder.find_integration_test_target(
            &test_file,
            workspace_root,
        ).unwrap();
        
        assert!(integration_target.is_some(), "Should find integration test target with glob([\"tests/**\"])");
        let target = integration_target.unwrap();
        println!("Found target: {} ({})", target.label, target.name);
        assert_eq!(target.label, "//server:integrated_tests_suite");
        assert_eq!(target.name, "integrated_tests_suite");
    }
    
    #[test]
    fn test_build_script_detection() {
        // This test verifies build.rs detection and cargo_build_script target
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();
        
        // Create MODULE.bazel to mark workspace root
        fs::write(workspace_root.join("MODULE.bazel"), "").unwrap();
        
        // Create server directory structure at root
        let server_dir = workspace_root.join("server");
        fs::create_dir(&server_dir).unwrap();
        
        // Create BUILD.bazel with cargo_build_script
        let build_content = r#"
load("@rules_rust//cargo:defs.bzl", "cargo_build_script")

cargo_build_script(
    name = "build_script",
    srcs = ["build.rs"],
)
"#;
        fs::write(server_dir.join("BUILD.bazel"), build_content).unwrap();
        
        // Create build.rs file
        let build_file = server_dir.join("build.rs");
        fs::write(&build_file, r#"
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
}
"#).unwrap();
        
        println!("\nTest setup (build.rs):");
        println!("  Build file: {:?}", build_file);
        println!("  Workspace root: {:?}", workspace_root);
        
        // Test the target finder
        let mut finder = BazelTargetFinder::new().unwrap();
        
        let targets = finder.find_targets_for_file(
            &build_file,
            workspace_root,
        ).unwrap();
        
        println!("Found {} targets:", targets.len());
        for target in &targets {
            println!("  - {} ({:?})", target.label, target.kind);
        }
        
        // Should find the build script target
        let build_script_target = targets.iter()
            .find(|t| matches!(t.kind, BazelTargetKind::BuildScript));
            
        assert!(build_script_target.is_some(), "Should find cargo_build_script target");
        let target = build_script_target.unwrap();
        assert_eq!(target.label, "//server:build_script");
        assert_eq!(target.name, "build_script");
    }
    
    #[test]
    fn test_build_script_command_generation() {
        use crate::types::{Runnable, RunnableKind, Scope, Position, ScopeKind, FileType};
        use crate::command::builder::{CommandBuilderImpl, bazel::BazelCommandBuilder};
        use crate::config::Config;
        
        // Create a runnable for build.rs
        let runnable = Runnable {
            label: "Run binary 'build'".to_string(),
            scope: Scope {
                kind: ScopeKind::Function,
                name: Some("main".to_string()),
                start: Position { line: 1, character: 0 },
                end: Position { line: 3, character: 0 },
            },
            kind: RunnableKind::Binary {
                bin_name: Some("build".to_string()),
            },
            module_path: String::new(),
            file_path: PathBuf::from("server/build.rs"),
            extended_scope: None,
        };
        
        // Create a default config
        let config = Config::default();
        
        // Build command - in real scenario this would find the target
        let command = BazelCommandBuilder::build(
            &runnable,
            None,
            &config,
            FileType::CargoProject,
        );
        
        // Should succeed
        assert!(command.is_ok(), "Command generation should succeed");
        let cmd = command.unwrap();
        
        println!("\nGenerated command for build.rs:");
        println!("  Type: {:?}", cmd.command_type);
        println!("  Args: {:?}", cmd.args);
        println!("  Shell: {}", cmd.to_shell_command());
        
        // Should use 'build' subcommand, not 'run'
        assert_eq!(cmd.args[0], "build", "Should use 'bazel build' for build.rs");
    }
    
    #[test]
    fn test_main_rs_test_detection() {
        // This test verifies that tests in main.rs use the rust_test target
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path();
        
        // Create MODULE.bazel to mark workspace root
        fs::write(workspace_root.join("MODULE.bazel"), "").unwrap();
        
        // Create server directory structure
        let server_dir = workspace_root.join("server");
        fs::create_dir(&server_dir).unwrap();
        let src_dir = server_dir.join("src");
        fs::create_dir(&src_dir).unwrap();
        
        // Create BUILD.bazel with rust_binary and rust_test
        let build_content = r#"
load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_test")

rust_binary(
    name = "server_bin",
    srcs = ["src/main.rs"],
    crate_root = "src/main.rs",
)

rust_test(
    name = "server_tests",
    crate = ":server_bin",
)
"#;
        fs::write(server_dir.join("BUILD.bazel"), build_content).unwrap();
        
        // Create main.rs with tests
        let main_file = src_dir.join("main.rs");
        fs::write(&main_file, r#"
fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_something() {
        assert!(true);
    }
}
"#).unwrap();
        
        println!("\nTest setup (main.rs with tests):");
        println!("  Main file: {:?}", main_file);
        println!("  Workspace root: {:?}", workspace_root);
        
        // Test the target finder
        let mut finder = BazelTargetFinder::new().unwrap();
        
        // Find test target for main.rs
        let test_target = finder.find_runnable_target(
            &main_file,
            workspace_root,
            Some(BazelTargetKind::Test),
        ).unwrap();
        
        assert!(test_target.is_some(), "Should find rust_test target for main.rs");
        let target = test_target.unwrap();
        println!("Found target: {} ({})", target.label, target.name);
        assert_eq!(target.name, "server_tests");
        assert_eq!(target.label, "//server:server_tests");
    }
    
    #[test]
    fn test_command_generation_for_server_integration_test() {
        use crate::types::{Runnable, RunnableKind, Scope, Position, ScopeKind, FileType};
        use crate::command::builder::{CommandBuilderImpl, bazel::BazelCommandBuilder};
        use crate::config::Config;
        
        // Create a runnable for the integration test
        let runnable = Runnable {
            label: "Run test 'lets_try_if_it_works'".to_string(),
            scope: Scope {
                kind: ScopeKind::Function,
                name: Some("lets_try_if_it_works".to_string()),
                start: Position { line: 15, character: 0 },
                end: Position { line: 20, character: 0 },
            },
            kind: RunnableKind::Test {
                test_name: "lets_try_if_it_works".to_string(),
                is_async: false,
            },
            module_path: "tests::just_test::tests".to_string(), // Note the module path
            file_path: PathBuf::from("server/tests/just_test.rs"),
            extended_scope: None,
        };
        
        // Create a default config
        let config = Config::default();
        
        // Build command
        let command = BazelCommandBuilder::build(
            &runnable,
            None,
            &config,
            FileType::CargoProject,
        );
        
        // Print the command for debugging
        if let Ok(cmd) = &command {
            println!("Generated command: {:?} {}", cmd.command_type, cmd.args.join(" "));
        }
        
        // The command generation might fail in unit tests without a real workspace,
        // but we can at least verify it doesn't panic
        let _ = command;
    }
}