//! Regression test suite for RAZ command generation
//!
//! This file contains integration tests for all the edge cases and bugs we've fixed.
//! These tests ensure that we don't regress on previously fixed issues when making changes.

use raz_core::Position;
use raz_core::file_detection::FileDetector;
use raz_core::universal_command_generator::UniversalCommandGenerator;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Test for build.rs special file handling
/// Issue: RAZ was generating "cargo run --bin build" for build.rs files
/// Fix: build.rs files should be treated as special BuildScript files
#[test]
fn test_build_rs_special_handling() {
    let temp_dir = TempDir::new().unwrap();

    // Test 1: build.rs in a cargo project
    let cargo_toml = r#"
[package]
name = "test-project"
version = "0.1.0"
edition = "2021"
"#;
    fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml).unwrap();

    let build_rs = r#"
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
}
"#;
    let build_path = temp_dir.path().join("build.rs");
    fs::write(&build_path, build_rs).unwrap();

    fs::create_dir(temp_dir.path().join("src")).unwrap();
    fs::write(temp_dir.path().join("src/main.rs"), "fn main() {}").unwrap();

    let context = FileDetector::detect_context(&build_path, None).unwrap();
    let commands = UniversalCommandGenerator::generate_commands(&context, None).unwrap();

    // Should NOT generate "cargo run --bin build"
    assert!(
        !commands.iter().any(|c| {
            c.command == "cargo"
                && c.args.contains(&"run".to_string())
                && c.args.contains(&"--bin".to_string())
                && c.args.contains(&"build".to_string())
        }),
        "Should not generate 'cargo run --bin build' for build.rs files"
    );

    // Should generate proper build commands
    assert!(
        commands
            .iter()
            .any(|c| c.id == "build-script-trigger" || c.id == "build-script-direct"),
        "Should generate build script commands"
    );

    // Test 2: Standalone build.rs file
    let temp_dir2 = TempDir::new().unwrap();
    let standalone_build = temp_dir2.path().join("build.rs");
    fs::write(&standalone_build, build_rs).unwrap();

    let context2 = FileDetector::detect_context(&standalone_build, None).unwrap();
    let commands2 = UniversalCommandGenerator::generate_commands(&context2, None).unwrap();

    assert!(
        commands2
            .iter()
            .any(|c| c.command.contains("rustc") || c.args.iter().any(|a| a.contains("rustc"))),
        "Standalone build.rs should use rustc"
    );
}

/// Test for cursor-based test selection
/// Issue: Cursor inside test function body was running all tests in module
/// Fix: Check if cursor is within test function's line range
#[test]
fn test_cursor_inside_test_function() {
    let temp_dir = TempDir::new().unwrap();
    let project_name = "cursor-test-project";

    // Create cargo project
    let cargo_toml = format!(
        r#"
[package]
name = "{project_name}"
version = "0.1.0"
edition = "2021"
"#
    );
    fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml).unwrap();

    // Create lib.rs with test module
    fs::create_dir(temp_dir.path().join("src")).unwrap();
    let lib_content = r#"
#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
    
    #[test]
    fn it_never_works() {
        assert!(false);  // Line 11 - cursor will be here
    }
    
    #[test]
    fn another_test() {
        assert!(true);
    }
}
"#;
    let lib_path = temp_dir.path().join("src/lib.rs");
    fs::write(&lib_path, lib_content).unwrap();

    // Test cursor inside it_never_works function body (line 11)
    let cursor_pos = Position {
        line: 11,
        column: 20,
    };
    let context = FileDetector::detect_context(&lib_path, Some(cursor_pos)).unwrap();
    let commands =
        UniversalCommandGenerator::generate_commands(&context, Some(cursor_pos)).unwrap();

    // Debug: print all commands
    println!("Generated {} commands:", commands.len());
    for cmd in &commands {
        println!(
            "  - {} ({}): category={:?}",
            cmd.label, cmd.id, cmd.category
        );
    }

    // Find the test command
    let test_cmd = commands
        .iter()
        .find(|c| c.category == raz_core::CommandCategory::Test)
        .expect("Should generate test command");

    // Should run only the specific test, not all tests in module
    assert!(
        test_cmd.args.contains(&"tests::it_never_works".to_string()),
        "Should target specific test function"
    );
    assert!(
        test_cmd.args.contains(&"--exact".to_string()),
        "Should use --exact flag for specific test"
    );

    // Should NOT run all tests in module
    assert!(
        !test_cmd
            .args
            .iter()
            .any(|arg| arg == "tests::" || arg == "tests::*"),
        "Should not run all tests in module when cursor is inside a specific test"
    );
}

/// Test for nested module path construction
/// Issue: Nested modules like src/power/over/level_900.rs were treated as standalone files
/// Fix: Properly detect module files and construct correct module paths
#[test]
fn test_nested_module_path_construction() {
    let temp_dir = TempDir::new().unwrap();
    let project_name = "nested-module-project";

    // Create cargo project
    let cargo_toml = format!(
        r#"
[package]
name = "{project_name}"
version = "0.1.0"
edition = "2021"
"#
    );
    fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml).unwrap();

    // Create nested module structure
    fs::create_dir_all(temp_dir.path().join("src/power/over")).unwrap();

    // Create lib.rs that declares the module
    let lib_content = r#"
mod power;

pub fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}
"#;
    fs::write(temp_dir.path().join("src/lib.rs"), lib_content).unwrap();

    // Create power/mod.rs
    fs::write(temp_dir.path().join("src/power/mod.rs"), "pub mod over;").unwrap();

    // Create power/over/mod.rs
    fs::write(
        temp_dir.path().join("src/power/over/mod.rs"),
        "pub mod level_900;",
    )
    .unwrap();

    // Create the nested module file
    let level_900_content = r#"
#[cfg(test)]
mod tests {
    use crate::fibonacci;
    
    #[test]
    fn it_works() {
        assert_eq!(fibonacci(10), 89);
    }
    
    #[test]
    fn it_never_works() {
        assert!(false);  // Line 12
    }
}
"#;
    let nested_path = temp_dir.path().join("src/power/over/level_900.rs");
    fs::write(&nested_path, level_900_content).unwrap();

    // Test with cursor on line 12 (inside it_never_works)
    let cursor_pos = Position {
        line: 12,
        column: 20,
    };
    let context = FileDetector::detect_context(&nested_path, Some(cursor_pos)).unwrap();
    let commands =
        UniversalCommandGenerator::generate_commands(&context, Some(cursor_pos)).unwrap();

    // Debug: print all commands
    println!("Nested module - Generated {} commands:", commands.len());
    for cmd in &commands {
        println!(
            "  - {} ({}): category={:?}",
            cmd.label, cmd.id, cmd.category
        );
    }

    // Find test command
    let test_cmd = commands
        .iter()
        .find(|c| c.category == raz_core::CommandCategory::Test)
        .expect("Should generate test command");

    // Should use cargo test, NOT rustc
    assert_eq!(
        test_cmd.command, "cargo",
        "Should use cargo for module files"
    );

    // Should have correct module path
    assert!(
        test_cmd
            .args
            .contains(&"power::over::level_900::tests::it_never_works".to_string()),
        "Should construct full module path for nested modules"
    );

    // Debug: print the actual command
    println!(
        "Test command: {} {}",
        test_cmd.command,
        test_cmd.args.join(" ")
    );

    // The important thing is that it uses cargo test with the correct module path
    // The --lib flag is not always necessary for module files
}

/// Test for binary file test commands
/// Issue: Binary files in src/bin/ had wrong test paths (bin::name::tests instead of just tests)
/// Fix: Use --bin flag and correct test path for binaries
#[test]
fn test_binary_file_test_commands() {
    let temp_dir = TempDir::new().unwrap();
    let project_name = "binary-test-project";

    // Create cargo project
    let cargo_toml = format!(
        r#"
[package]
name = "{project_name}"
version = "0.1.0"
edition = "2021"
"#
    );
    fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml).unwrap();

    // Create directories
    fs::create_dir_all(temp_dir.path().join("src/bin")).unwrap();

    // Create a simple lib.rs
    fs::write(temp_dir.path().join("src/lib.rs"), "// lib").unwrap();

    // Create binary file with tests
    let bogdan_content = r#"
fn main() {
    println!("this is bogdan");
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_main() {
        assert!(true);
    }
    
    #[test]
    fn test_power() {
        assert!(false);  // Line 16
    }
}
"#;
    let binary_path = temp_dir.path().join("src/bin/bogdan.rs");
    fs::write(&binary_path, bogdan_content).unwrap();

    // Test with cursor on line 16 (inside test_power)
    let cursor_pos = Position {
        line: 16,
        column: 20,
    };
    let context = FileDetector::detect_context(&binary_path, Some(cursor_pos)).unwrap();
    let commands =
        UniversalCommandGenerator::generate_commands(&context, Some(cursor_pos)).unwrap();

    // Debug: print all commands
    println!("Binary test - Generated {} commands:", commands.len());
    for cmd in &commands {
        println!(
            "  - {} ({}): category={:?}",
            cmd.label, cmd.id, cmd.category
        );
    }

    // Find test command
    let test_cmd = commands
        .iter()
        .find(|c| c.category == raz_core::CommandCategory::Test)
        .expect("Should generate test command for binary");

    // Should use cargo test
    assert_eq!(
        test_cmd.command, "cargo",
        "Should use cargo for binary tests"
    );

    // Should have --bin flag
    assert!(
        test_cmd.args.contains(&"--bin".to_string()),
        "Should have --bin flag"
    );
    assert!(
        test_cmd.args.contains(&"bogdan".to_string()),
        "Should specify binary name"
    );

    // Test path should be just "tests::test_power", NOT "bin::bogdan::tests::test_power"
    assert!(
        test_cmd.args.contains(&"tests::test_power".to_string()),
        "Binary test path should not include bin:: prefix"
    );

    // Should have --show-output
    assert!(
        test_cmd.args.contains(&"--show-output".to_string()),
        "Should include --show-output for better test visibility"
    );
}

/// Test tree-sitter attribute detection
/// Issue: Test attributes are siblings, not children of function nodes
/// Fix: Check previous sibling for test attributes
#[test]
fn test_tree_sitter_attribute_detection() {
    #[cfg(feature = "tree-sitter-support")]
    {
        use raz_core::tree_sitter_test_detector::TreeSitterTestDetector;

        let content = r#"
#[test]
fn my_test() {
    assert!(true);
}

#[tokio::test]
async fn async_test() {
    assert!(true);
}

#[cfg(test)]
mod tests {
    #[test]
    fn nested_test() {
        assert!(true);
    }
}
"#;

        let mut detector = TreeSitterTestDetector::new().unwrap();
        let entries = detector.detect_entry_points(content, None).unwrap();

        // Should detect all test functions
        assert!(
            entries.iter().any(|e| e.name == "my_test"),
            "Should detect #[test] attribute"
        );
        assert!(
            entries.iter().any(|e| e.name == "async_test"),
            "Should detect #[tokio::test] attribute"
        );
        assert!(
            entries.iter().any(|e| e.name == "tests"),
            "Should detect test module"
        );
    }
}

/// Integration test that actually executes commands
/// This test creates real projects and runs the generated commands to ensure they work
#[test]
#[ignore] // Ignore by default as it's slower and requires cargo
fn test_command_execution_integration() {
    let temp_dir = TempDir::new().unwrap();

    // Create a test project
    let cargo_toml = r#"
[package]
name = "integration-test"
version = "0.1.0"
edition = "2021"
"#;
    fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml).unwrap();

    // Create src directory and files
    fs::create_dir(temp_dir.path().join("src")).unwrap();

    // Test 1: Library with specific test
    let lib_content = r#"
#[cfg(test)]
mod tests {
    #[test]
    fn specific_test() {
        assert_eq!(1 + 1, 2);
    }
    
    #[test]
    fn another_test() {
        panic!("This should not run");
    }
}
"#;
    let lib_path = temp_dir.path().join("src/lib.rs");
    fs::write(&lib_path, lib_content).unwrap();

    // Generate command for specific test
    let cursor = Position {
        line: 5,
        column: 20,
    }; // Inside specific_test
    let context = FileDetector::detect_context(&lib_path, Some(cursor)).unwrap();
    let commands = UniversalCommandGenerator::generate_commands(&context, Some(cursor)).unwrap();

    // Debug commands
    println!("Integration test - Generated {} commands:", commands.len());
    for cmd in &commands {
        println!(
            "  - {} ({}): category={:?}",
            cmd.label, cmd.id, cmd.category
        );
    }

    let test_cmd = commands
        .iter()
        .find(|c| c.category == raz_core::CommandCategory::Test)
        .expect("Should find test command");

    // Execute the command
    let output = Command::new(&test_cmd.command)
        .args(&test_cmd.args)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verify the command succeeded and ran only the specific test
    assert!(
        output.status.success(),
        "Command should succeed. stdout: {stdout}, stderr: {stderr}"
    );
    assert!(
        stdout.contains("1 passed") || stderr.contains("1 passed"),
        "Should run exactly one test"
    );
    assert!(
        !stdout.contains("another_test"),
        "Should not run the other test"
    );

    // Test 2: Binary with tests
    fs::create_dir(temp_dir.path().join("src/bin")).unwrap();
    let binary_content = r#"
fn main() {
    println!("Binary");
}

#[cfg(test)]
mod tests {
    #[test]
    fn bin_test() {
        assert!(true);
    }
}
"#;
    let bin_path = temp_dir.path().join("src/bin/mybin.rs");
    fs::write(&bin_path, binary_content).unwrap();

    let cursor2 = Position {
        line: 9,
        column: 20,
    }; // Inside bin_test
    let context2 = FileDetector::detect_context(&bin_path, Some(cursor2)).unwrap();
    let commands2 = UniversalCommandGenerator::generate_commands(&context2, Some(cursor2)).unwrap();

    let bin_test_cmd = commands2
        .iter()
        .find(|c| c.category == raz_core::CommandCategory::Test)
        .expect("Should find binary test command");

    // Execute binary test command
    let output2 = Command::new(&bin_test_cmd.command)
        .args(&bin_test_cmd.args)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute binary test command");

    assert!(
        output2.status.success(),
        "Binary test command should succeed. stdout: {}, stderr: {}",
        String::from_utf8_lossy(&output2.stdout),
        String::from_utf8_lossy(&output2.stderr)
    );
}

/// Test doc test cursor detection and prioritization
/// Issue: When cursor is in doc test, RAZ was running library tests instead of doc tests
/// Fix: Detect doc test cursor position and prioritize doc test commands
#[test]
fn test_doc_test_cursor_prioritization() {
    let temp_dir = TempDir::new().unwrap();

    let cargo_toml = r#"
[package]
name = "doc-test-priority-test"
version = "0.1.0"
edition = "2021"
"#;
    fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml).unwrap();

    fs::create_dir(temp_dir.path().join("src")).unwrap();
    let lib_content = r#"/// This function adds two numbers
/// 
/// ```rust
/// use doc_test_priority_test::add;
/// let x = 10;  // Line 5 - cursor here
/// let y = 20;
/// let sum = add(x, y);
/// assert_eq!(sum, 30);
/// ```
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(add(2, 2), 4);
    }
}
"#;
    let lib_path = temp_dir.path().join("src/lib.rs");
    fs::write(&lib_path, lib_content).unwrap();

    // Cursor on line 5 (inside doc test)
    let cursor = Position {
        line: 5,
        column: 10,
    };
    let context = FileDetector::detect_context(&lib_path, Some(cursor)).unwrap();
    let commands = UniversalCommandGenerator::generate_commands(&context, Some(cursor)).unwrap();

    // Find highest priority command
    let highest_priority_cmd = commands.iter().max_by_key(|c| c.priority).unwrap();

    // Should prioritize doc test command when cursor is in doc test
    assert!(
        highest_priority_cmd.args.contains(&"--doc".to_string()),
        "Highest priority command should be doc test when cursor is in doc test"
    );
    assert!(
        highest_priority_cmd.args.contains(&"add".to_string()),
        "Should filter by function name (add)"
    );
    assert!(
        highest_priority_cmd
            .args
            .contains(&"--show-output".to_string()),
        "Should include --show-output flag"
    );

    // Verify it's not running library tests as highest priority
    assert!(
        !highest_priority_cmd.args.contains(&"--lib".to_string()),
        "Should not prioritize library tests when cursor is in doc test"
    );
}

/// Test module-level cursor positioning
/// Issue: Cursor in test module but not on specific test was running wrong tests
/// Fix: Properly detect when cursor is in module but not on specific test
#[test]
fn test_module_level_cursor_detection() {
    let temp_dir = TempDir::new().unwrap();

    let cargo_toml = r#"
[package]
name = "module-cursor-test"
version = "0.1.0"
edition = "2021"
"#;
    fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml).unwrap();

    fs::create_dir(temp_dir.path().join("src")).unwrap();
    let lib_content = r#"
#[cfg(test)]
mod tests {
    // Line 3 - inside module but not in any test
    
    #[test]
    fn first_test() {
        assert!(true);
    }
    
    #[test]
    fn second_test() {
        assert!(true);
    }
}

#[cfg(test)]
mod other_tests {
    #[test]
    fn other_test() {
        assert!(true);
    }
}
"#;
    let lib_path = temp_dir.path().join("src/lib.rs");
    fs::write(&lib_path, lib_content).unwrap();

    // Cursor on line 3 - inside 'tests' module but not in any specific test
    let cursor = Position {
        line: 3,
        column: 10,
    };
    let context = FileDetector::detect_context(&lib_path, Some(cursor)).unwrap();
    let commands = UniversalCommandGenerator::generate_commands(&context, Some(cursor)).unwrap();

    let test_cmd = commands
        .iter()
        .find(|c| c.category == raz_core::CommandCategory::Test)
        .expect("Should generate test command");

    // Should run all tests in the 'tests' module, not 'other_tests'
    assert!(
        test_cmd.args.iter().any(|arg| arg.contains("tests::")),
        "Should target the 'tests' module"
    );
    assert!(
        !test_cmd.args.iter().any(|arg| arg.contains("other_tests")),
        "Should not target the 'other_tests' module when cursor is in 'tests' module"
    );
}
