//! Integration tests that actually execute the generated commands
//! to verify module paths work correctly
//!
//! This addresses the user's concern: "did you test that modules really correctly run the tests?"

use raz_core::{FileDetector, Position, UniversalCommandGenerator};
use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_nested_module_execution_with_real_cargo() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;

    // Create a real Cargo project
    let cargo_toml = r#"
[package]
name = "test_execution"
version = "0.1.0"
edition = "2021"
"#;
    fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml)?;
    fs::create_dir(temp_dir.path().join("src"))?;

    let test_file = temp_dir.path().join("src/lib.rs");

    // Create complex nested test structure with tests that will pass/fail
    let content = r#"
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn top_level_test() {
        assert_eq!(add(2, 2), 4);
        println!("TOP_LEVEL_TEST_RAN");
    }
    
    mod unit {
        use super::*;
        
        #[test]
        fn unit_test_1() {
            assert_eq!(add(3, 3), 6);
            println!("UNIT_TEST_1_RAN");
        }
        
        #[test]
        fn unit_test_2() {
            assert_eq!(add(4, 4), 8);
            println!("UNIT_TEST_2_RAN");
        }
        
        mod deeper {
            use super::*;
            
            #[test]
            fn deep_test() {
                assert_eq!(add(5, 5), 10);
                println!("DEEP_TEST_RAN");
            }
        }
    }
    
    mod integration {
        use super::*;
        
        #[test]
        fn integration_test() {
            assert_eq!(add(10, 10), 20);
            println!("INTEGRATION_TEST_RAN");
        }
    }
}
"#;

    fs::write(&test_file, content)?;

    // Test 1: Cursor on specific test in nested module
    println!("\n=== Test 1: Running specific nested test ===");
    let cursor = Position {
        line: 19,
        column: 12,
    }; // On unit_test_1

    let context = FileDetector::detect_context(&test_file, Some(cursor))?;
    let commands = UniversalCommandGenerator::generate_commands(&context, Some(cursor))?;

    assert!(!commands.is_empty());
    let cmd = &commands[0];
    println!("Generated command: {} {}", cmd.command, cmd.args.join(" "));

    // Execute the command
    let output = Command::new(&cmd.command)
        .args(&cmd.args)
        .current_dir(temp_dir.path())
        .env("RUST_TEST_NOCAPTURE", "1")
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("Output:\n{stdout}\n{stderr}");

    // Verify only unit_test_1 ran
    assert!(
        stdout.contains("UNIT_TEST_1_RAN"),
        "unit_test_1 should have run"
    );
    assert!(
        !stdout.contains("UNIT_TEST_2_RAN"),
        "unit_test_2 should NOT have run"
    );
    assert!(
        !stdout.contains("TOP_LEVEL_TEST_RAN"),
        "top_level_test should NOT have run"
    );
    assert!(
        !stdout.contains("DEEP_TEST_RAN"),
        "deep_test should NOT have run"
    );
    assert!(
        !stdout.contains("INTEGRATION_TEST_RAN"),
        "integration_test should NOT have run"
    );

    // Verify the command used the correct module path
    let args_str = cmd.args.join(" ");
    assert!(
        args_str.contains("tests::unit::unit_test_1"),
        "Command should use full module path: tests::unit::unit_test_1"
    );
    assert!(
        args_str.contains("--exact"),
        "Should use --exact to run only one test"
    );

    // Test 2: Cursor in unit module but not on specific test
    println!("\n=== Test 2: Running all tests in unit module ===");
    let cursor = Position {
        line: 15,
        column: 4,
    }; // In unit module but not on a test

    let context = FileDetector::detect_context(&test_file, Some(cursor))?;
    let commands = UniversalCommandGenerator::generate_commands(&context, Some(cursor))?;

    assert!(!commands.is_empty());
    let cmd = &commands[0];
    println!("Generated command: {} {}", cmd.command, cmd.args.join(" "));

    // Let's check what module it detected
    println!("Entry points found:");
    for ep in &context.entry_points {
        println!(
            "  - {} ({:?}) line {} path: {:?}",
            ep.name, ep.entry_type, ep.line, ep.full_path
        );
    }

    let output = Command::new(&cmd.command)
        .args(&cmd.args)
        .current_dir(temp_dir.path())
        .env("RUST_TEST_NOCAPTURE", "1")
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("Output:\n{stdout}");

    // Verify all unit tests ran but not tests from other modules
    assert!(
        stdout.contains("UNIT_TEST_1_RAN"),
        "unit_test_1 should have run"
    );
    assert!(
        stdout.contains("UNIT_TEST_2_RAN"),
        "unit_test_2 should have run"
    );
    assert!(
        stdout.contains("DEEP_TEST_RAN"),
        "deep_test should have run (it's in unit::deeper)"
    );
    assert!(
        !stdout.contains("TOP_LEVEL_TEST_RAN"),
        "top_level_test should NOT have run"
    );
    assert!(
        !stdout.contains("INTEGRATION_TEST_RAN"),
        "integration_test should NOT have run"
    );

    // Test 3: Cursor on deeply nested test
    println!("\n=== Test 3: Running deeply nested test ===");
    let cursor = Position {
        line: 34,
        column: 16,
    }; // On deep_test (line 35 in 1-based)

    let context = FileDetector::detect_context(&test_file, Some(cursor))?;
    let commands = UniversalCommandGenerator::generate_commands(&context, Some(cursor))?;

    assert!(!commands.is_empty());
    let cmd = &commands[0];
    println!("Generated command: {} {}", cmd.command, cmd.args.join(" "));

    let output = Command::new(&cmd.command)
        .args(&cmd.args)
        .current_dir(temp_dir.path())
        .env("RUST_TEST_NOCAPTURE", "1")
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify only deep_test ran
    assert!(
        stdout.contains("DEEP_TEST_RAN"),
        "deep_test should have run"
    );
    assert!(
        !stdout.contains("UNIT_TEST_1_RAN"),
        "unit_test_1 should NOT have run"
    );
    assert!(
        !stdout.contains("UNIT_TEST_2_RAN"),
        "unit_test_2 should NOT have run"
    );

    // Verify correct module path
    let args_str = cmd.args.join(" ");
    assert!(
        args_str.contains("tests::unit::deeper::deep_test"),
        "Should use full path: tests::unit::deeper::deep_test"
    );

    println!("\n✅ All module execution tests passed!");
    Ok(())
}

#[test]
fn test_module_with_same_test_names() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;

    // Create a Cargo project
    let cargo_toml = r#"
[package]
name = "same_names_test"
version = "0.1.0"
edition = "2021"
"#;
    fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml)?;
    fs::create_dir(temp_dir.path().join("src"))?;

    let test_file = temp_dir.path().join("src/lib.rs");

    // Create modules with same test names
    let content = r#"
#[cfg(test)]
mod tests {
    #[test]
    fn test_foo() {
        println!("TESTS_FOO_RAN");
        assert!(true);
    }
    
    mod unit {
        #[test]
        fn test_foo() {
            println!("TESTS_UNIT_FOO_RAN");
            assert!(true);
        }
    }
    
    mod integration {
        #[test]
        fn test_foo() {
            println!("TESTS_INTEGRATION_FOO_RAN");
            assert!(true);
        }
    }
}
"#;

    fs::write(&test_file, content)?;

    // Test running each test_foo specifically
    let test_cases = vec![
        (
            Position { line: 4, column: 8 },
            "TESTS_FOO_RAN",
            "tests::test_foo",
        ),
        (
            Position {
                line: 11,
                column: 12,
            },
            "TESTS_UNIT_FOO_RAN",
            "tests::unit::test_foo",
        ),
        (
            Position {
                line: 19,
                column: 12,
            },
            "TESTS_INTEGRATION_FOO_RAN",
            "tests::integration::test_foo",
        ),
    ];

    for (cursor, expected_output, expected_path) in test_cases {
        println!("\n=== Testing cursor at line {} ===", cursor.line + 1);

        let context = FileDetector::detect_context(&test_file, Some(cursor))?;
        let commands = UniversalCommandGenerator::generate_commands(&context, Some(cursor))?;

        assert!(!commands.is_empty());
        let cmd = &commands[0];

        let args_str = cmd.args.join(" ");
        assert!(
            args_str.contains(expected_path),
            "Should target {expected_path}, but got: {args_str}"
        );

        let output = Command::new(&cmd.command)
            .args(&cmd.args)
            .current_dir(temp_dir.path())
            .env("RUST_TEST_NOCAPTURE", "1")
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains(expected_output),
            "Should run correct test_foo variant. Expected: {expected_output}, Output: {stdout}"
        );

        // Count how many test_foo variants ran
        let foo_count = stdout.matches("_FOO_RAN").count();
        assert_eq!(foo_count, 1, "Should run exactly one test_foo variant");
    }

    println!("\n✅ Same name test execution passed!");
    Ok(())
}

#[test]
fn test_cursor_not_on_test_runs_all_module_tests() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;

    let cargo_toml = r#"
[package]
name = "module_test"
version = "0.1.0"
edition = "2021"
"#;
    fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml)?;
    fs::create_dir(temp_dir.path().join("src"))?;

    let test_file = temp_dir.path().join("src/lib.rs");

    let content = r#"
#[cfg(test)]
mod tests {
    use super::*;
    
    // Some helper function
    fn setup() -> i32 {
        42
    }
    
    #[test]
    fn test_1() {
        println!("TEST_1_RAN");
        assert_eq!(setup(), 42);
    }
    
    #[test]
    fn test_2() {
        println!("TEST_2_RAN");
        assert_eq!(setup(), 42);
    }
    
    mod nested {
        #[test]
        fn test_3() {
            println!("TEST_3_RAN");
            assert!(true);
        }
    }
}
"#;

    fs::write(&test_file, content)?;

    // Cursor on helper function (not a test)
    let cursor = Position { line: 6, column: 8 }; // On setup() function

    let context = FileDetector::detect_context(&test_file, Some(cursor))?;
    let commands = UniversalCommandGenerator::generate_commands(&context, Some(cursor))?;

    assert!(!commands.is_empty());
    let cmd = &commands[0];
    println!(
        "Command when cursor on non-test function: {} {}",
        cmd.command,
        cmd.args.join(" ")
    );

    let output = Command::new(&cmd.command)
        .args(&cmd.args)
        .current_dir(temp_dir.path())
        .env("RUST_TEST_NOCAPTURE", "1")
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should run all tests in the module
    assert!(stdout.contains("TEST_1_RAN"), "test_1 should run");
    assert!(stdout.contains("TEST_2_RAN"), "test_2 should run");
    assert!(
        stdout.contains("TEST_3_RAN"),
        "test_3 in nested module should run"
    );

    // Verify it ran module tests, not a specific test
    let args_str = cmd.args.join(" ");
    assert!(
        !args_str.contains("--exact"),
        "Should NOT use --exact when running module tests"
    );
    assert!(args_str.contains("tests"), "Should target tests module");

    println!("\n✅ Module test execution passed!");
    Ok(())
}

#[test]
fn test_standalone_file_module_paths() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let standalone_file = temp_dir.path().join("standalone.rs");

    // Create a standalone file with nested tests
    let content = r#"
fn main() {
    println!("Main function");
}

#[cfg(test)]
mod tests {
    mod unit {
        #[test]
        fn test_something() {
            println!("STANDALONE_TEST_RAN");
            assert!(true);
        }
    }
}
"#;

    fs::write(&standalone_file, content)?;

    // Cursor on the test
    let cursor = Position {
        line: 9,
        column: 12,
    };

    let context = FileDetector::detect_context(&standalone_file, Some(cursor))?;
    let commands = UniversalCommandGenerator::generate_commands(&context, Some(cursor))?;

    assert!(!commands.is_empty());
    let cmd = &commands[0];

    // For standalone files, should use rustc
    assert_eq!(cmd.command, "sh");
    assert!(cmd.args.join(" ").contains("rustc --test"));

    // Should still have correct module path
    let args_str = cmd.args.join(" ");
    assert!(
        args_str.contains("tests::unit::test_something"),
        "Should have full module path even for standalone files"
    );

    // Execute to verify it works
    let output = Command::new(&cmd.command)
        .args(&cmd.args)
        .current_dir(temp_dir.path())
        .env("RUST_TEST_NOCAPTURE", "1")
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("STANDALONE_TEST_RAN"),
        "Standalone test should run"
    );

    println!("\n✅ Standalone file module paths work!");
    Ok(())
}
