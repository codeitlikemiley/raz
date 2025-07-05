//! Integration tests for nested module path resolution
//!
//! This tests that module paths are correctly resolved and that
//! the right tests are run when cursor is at different positions

use raz_core::{FileDetector, Position, UniversalCommandGenerator};
use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_nested_module_test_execution() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;

    // Create a valid Cargo project
    let cargo_toml = r#"
[package]
name = "test_project"
version = "0.1.0"
edition = "2021"

[dependencies]
"#;
    fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml)?;

    // Create src directory
    fs::create_dir(temp_dir.path().join("src"))?;

    let test_file = temp_dir.path().join("src/lib.rs");

    // Create a complex nested test structure
    let content = r#"
// This file tests nested module imports and test execution

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_top_level() {
        // Line 8
        assert_eq!(1 + 1, 2);
    }
    
    mod unit {
        use super::*;
        
        #[test]
        fn test_unit_basic() {
            // Line 17
            assert_eq!(2 + 2, 4);
        }
        
        #[test]
        fn test_unit_advanced() {
            // Line 23
            assert_eq!(3 + 3, 6);
        }
    }
    
    mod integration {
        use super::*;
        
        #[test]
        fn test_integration_basic() {
            // Line 33
            assert_eq!(4 + 4, 8);
        }
        
        mod database {
            use super::*;
            
            #[test]
            fn test_db_connection() {
                // Line 42
                assert_eq!(5 + 5, 10);
            }
            
            #[test]
            fn test_db_query() {
                // Line 48
                assert_eq!(6 + 6, 12);
            }
            
            mod transactions {
                use super::*;
                
                #[test]
                fn test_transaction_commit() {
                    // Line 57
                    assert_eq!(7 + 7, 14);
                }
                
                #[test]
                fn test_transaction_rollback() {
                    // Line 63
                    assert_eq!(8 + 8, 16);
                    panic!("This test should only run when specifically targeted!");
                }
            }
        }
        
        mod api {
            use super::*;
            
            #[test]
            fn test_api_endpoint() {
                // Line 75
                assert_eq!(9 + 9, 18);
            }
        }
    }
}

fn main() {
    println!("This file is for testing nested modules");
}
"#;

    fs::write(&test_file, content)?;

    // Test 1: Cursor on specific deeply nested test (line 65 - test_transaction_rollback)
    println!("Test 1: Testing specific deeply nested test execution");

    let cursor_pos = Position {
        line: 63,
        column: 1,
    }; // Position on test_transaction_rollback function (0-based)
    let context = FileDetector::detect_context(&test_file, Some(cursor_pos))?;
    let commands = UniversalCommandGenerator::generate_commands(&context, Some(cursor_pos))?;

    assert!(
        !commands.is_empty(),
        "Should generate commands for cursor position"
    );

    let best_command = &commands[0];
    println!(
        "Generated command: {} {}",
        best_command.command,
        best_command.args.join(" ")
    );

    // Verify the command targets the specific test
    let args_str = best_command.args.join(" ");
    assert!(
        args_str.contains("tests::integration::database::transactions::test_transaction_rollback"),
        "Command should target the specific test, not all tests. Got: {args_str}"
    );

    // Actually run the command and verify it fails (because of the panic)
    let output = Command::new(&best_command.command)
        .args(&best_command.args)
        .current_dir(temp_dir.path())
        .output()?;

    // Print the output for debugging if verbose
    if std::env::var("VERBOSE_TEST").is_ok() {
        println!("Command output:");
        println!("Status: {:?}", output.status);
        println!("Stdout: {}", String::from_utf8_lossy(&output.stdout));
        println!("Stderr: {}", String::from_utf8_lossy(&output.stderr));
    }

    // This specific test should fail because it has a panic
    assert!(
        !output.status.success(),
        "The test_transaction_rollback should fail due to panic"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined_output = format!("{stdout}\n{stderr}");

    assert!(
        combined_output.contains("test_transaction_rollback") && combined_output.contains("panic"),
        "Should run only the specific test that panics. Output: {combined_output}"
    );

    // Test 2: Cursor on a different test in same module (line 58 - test_transaction_commit)
    println!("\nTest 2: Testing adjacent test doesn't run the panicking test");
    let context2 = FileDetector::detect_context(
        &test_file,
        Some(Position {
            line: 57,
            column: 1,
        }),
    )?;
    let commands2 = UniversalCommandGenerator::generate_commands(
        &context2,
        Some(Position {
            line: 57,
            column: 1,
        }),
    )?;

    let best_command2 = &commands2[0];
    let args_str2 = best_command2.args.join(" ");
    assert!(
        args_str2.contains("tests::integration::database::transactions::test_transaction_commit"),
        "Should target test_transaction_commit, not test_transaction_rollback"
    );

    // Run this test - it should succeed
    let output2 = Command::new(&best_command2.command)
        .args(&best_command2.args)
        .current_dir(temp_dir.path())
        .output()?;

    assert!(
        output2.status.success(),
        "test_transaction_commit should succeed (no panic)"
    );

    // Test 3: Cursor in module but not on specific test
    println!("\nTest 3: Testing module-level execution");
    let context3 = FileDetector::detect_context(
        &test_file,
        Some(Position {
            line: 52,
            column: 1,
        }),
    )?; // Empty line in transactions module
    let commands3 = UniversalCommandGenerator::generate_commands(
        &context3,
        Some(Position {
            line: 52,
            column: 1,
        }),
    )?;

    if !commands3.is_empty() {
        let best_command3 = &commands3[0];
        let args_str3 = best_command3.args.join(" ");

        // Should run all tests in the transactions module
        if args_str3.contains("tests::integration::database::transactions") {
            println!("Module test command generated: {args_str3}");

            // This should fail because it includes the panicking test
            let output3 = Command::new(&best_command3.command)
                .args(&best_command3.args)
                .current_dir(temp_dir.path())
                .output()?;

            assert!(
                !output3.status.success(),
                "Module tests should fail due to panic in one test"
            );
        }
    }

    // Test 4: Verify different nesting levels
    println!("\nTest 4: Testing various nesting levels");

    // Top level test
    let context4 = FileDetector::detect_context(&test_file, Some(Position { line: 8, column: 1 }))?;
    let commands4 = UniversalCommandGenerator::generate_commands(
        &context4,
        Some(Position { line: 8, column: 1 }),
    )?;

    let args4 = commands4[0].args.join(" ");
    assert!(
        args4.contains("tests::test_top_level"),
        "Should generate correct path for top-level test"
    );

    // Mid-level test (test_db_connection)
    let context5 = FileDetector::detect_context(
        &test_file,
        Some(Position {
            line: 42,
            column: 1,
        }),
    )?;
    let commands5 = UniversalCommandGenerator::generate_commands(
        &context5,
        Some(Position {
            line: 42,
            column: 1,
        }),
    )?;

    let args5 = commands5[0].args.join(" ");
    assert!(
        args5.contains("tests::integration::database::test_db_connection"),
        "Should generate correct path for mid-level test"
    );

    println!("\n✅ All nested module tests passed!");
    Ok(())
}

#[test]
fn test_module_path_accuracy() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;

    // Create a valid Cargo project
    let cargo_toml = r#"
[package]
name = "path_accuracy_test"
version = "0.1.0"
edition = "2021"
"#;
    fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml)?;

    // Create src directory
    fs::create_dir(temp_dir.path().join("src"))?;

    let test_file = temp_dir.path().join("src/lib.rs");

    // Create a file with similarly named tests in different modules
    let content = r#"
#[cfg(test)]
mod tests {
    #[test]
    fn test_foo() {
        assert_eq!(1, 1); // Line 5
    }
    
    mod unit {
        #[test]
        fn test_foo() {
            assert_eq!(2, 2); // Line 11
        }
    }
    
    mod integration {
        #[test]
        fn test_foo() {
            assert_eq!(3, 3); // Line 18
        }
        
        mod deep {
            #[test]
            fn test_foo() {
                assert_eq!(4, 4); // Line 24
            }
        }
    }
}
"#;

    fs::write(&test_file, content)?;

    // Test that each cursor position generates the correct module path
    let test_cases = vec![
        (4, "tests::test_foo"),
        (10, "tests::unit::test_foo"),
        (17, "tests::integration::test_foo"),
        (23, "tests::integration::deep::test_foo"),
    ];

    for (line, expected_path) in test_cases {
        let context = FileDetector::detect_context(&test_file, Some(Position { line, column: 1 }))?;
        let commands = UniversalCommandGenerator::generate_commands(
            &context,
            Some(Position { line, column: 1 }),
        )?;

        assert!(
            !commands.is_empty(),
            "Should generate commands for line {}",
            line + 1
        );

        let args = commands[0].args.join(" ");
        assert!(
            args.contains(expected_path),
            "Line {} should generate path '{}', but got: {}",
            line + 1,
            expected_path,
            args
        );
    }

    println!("✅ Module path accuracy test passed!");
    Ok(())
}

#[test]
fn test_cursor_proximity_with_modules() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;

    // Create a valid Cargo project
    let cargo_toml = r#"
[package]
name = "proximity_test"
version = "0.1.0"
edition = "2021"
"#;
    fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml)?;

    // Create src directory
    fs::create_dir(temp_dir.path().join("src"))?;

    let test_file = temp_dir.path().join("src/lib.rs");

    let content = r#"
#[cfg(test)]
mod tests {
    // Some comment
    // Another comment
    // Line 5 - should not trigger test detection (too far)
    
    
    
    
    #[test]
    fn test_example() {
        // Line 12
        assert!(true);
    }
    
    // Line 16 - within 5 lines of test
    
    
    
    
    // Line 21 - too far from test
    
    mod nested {
        // Line 24 - in module but no test nearby
        
        
        
        #[test]
        fn test_nested() {
            // Line 30
            assert!(true);
        }
    }
}
"#;

    fs::write(&test_file, content)?;

    // Test cursor too far from any test
    let context1 = FileDetector::detect_context(&test_file, Some(Position { line: 4, column: 1 }))?;
    let commands1 = UniversalCommandGenerator::generate_commands(
        &context1,
        Some(Position { line: 4, column: 1 }),
    )?;

    // Should not generate test-specific commands when cursor is far from tests
    if !commands1.is_empty() && commands1[0].args.join(" ").contains("--exact") {
        panic!("Should not generate exact test command when cursor is far from tests");
    }

    // Test cursor close to test
    let context2 = FileDetector::detect_context(
        &test_file,
        Some(Position {
            line: 15,
            column: 1,
        }),
    )?;
    let commands2 = UniversalCommandGenerator::generate_commands(
        &context2,
        Some(Position {
            line: 15,
            column: 1,
        }),
    )?;

    if !commands2.is_empty() {
        let args = commands2[0].args.join(" ");
        if args.contains("test") && args.contains("--exact") {
            assert!(
                args.contains("tests::test_example"),
                "Should target the nearby test"
            );
        }
    }

    println!("✅ Cursor proximity test passed!");
    Ok(())
}

#[test]
fn test_standalone_vs_cargo_module_paths() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;

    // Test 1: Standalone file (no Cargo.toml)
    let standalone_dir = temp_dir.path().join("standalone");
    fs::create_dir(&standalone_dir)?;

    let standalone_file = standalone_dir.join("standalone.rs");
    let content = r#"
#[cfg(test)]
mod tests {
    mod integration {
        #[test]
        fn test_standalone() {
            assert!(true); // Line 6
        }
    }
}
"#;

    fs::write(&standalone_file, content)?;

    let context =
        FileDetector::detect_context(&standalone_file, Some(Position { line: 5, column: 1 }))?;
    let commands = UniversalCommandGenerator::generate_commands(
        &context,
        Some(Position { line: 5, column: 1 }),
    )?;

    assert!(!commands.is_empty());
    let cmd = &commands[0];

    // This test file is standalone (not in a cargo project), so should use rustc
    assert!(
        cmd.command == "sh" && cmd.args.join(" ").contains("rustc --test"),
        "Standalone files should use rustc"
    );

    // Should still have correct module path
    assert!(
        cmd.args
            .join(" ")
            .contains("tests::integration::test_standalone"),
        "Should have correct module path even for standalone files"
    );

    // Test 2: File in a Cargo project should use cargo
    let cargo_dir = temp_dir.path().join("cargo_project");
    fs::create_dir(&cargo_dir)?;

    let cargo_toml = r#"
[package]
name = "test_cargo"
version = "0.1.0"
edition = "2021"
"#;
    fs::write(cargo_dir.join("Cargo.toml"), cargo_toml)?;

    fs::create_dir(cargo_dir.join("src"))?;
    let cargo_file = cargo_dir.join("src/lib.rs");

    fs::write(&cargo_file, content)?;

    let context2 =
        FileDetector::detect_context(&cargo_file, Some(Position { line: 5, column: 1 }))?;
    let commands2 = UniversalCommandGenerator::generate_commands(
        &context2,
        Some(Position { line: 5, column: 1 }),
    )?;

    assert!(!commands2.is_empty());
    let cmd2 = &commands2[0];

    // File in Cargo project should use cargo test
    assert!(
        cmd2.command == "cargo" && cmd2.args[0] == "test",
        "Files in Cargo projects should use cargo test"
    );

    // Should have correct module path
    assert!(
        cmd2.args
            .join(" ")
            .contains("tests::integration::test_standalone"),
        "Should have correct module path for cargo projects"
    );

    println!("✅ Standalone vs Cargo module path test passed!");
    Ok(())
}
