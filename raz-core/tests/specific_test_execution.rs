//! Test to verify that cursor-based test selection runs only the specific test
//!
//! This directly addresses the user's concern: "if the cursor is pointed on a
//! function it might run all test instead of 1"

use raz_core::{FileDetector, Position, UniversalCommandGenerator};
use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_cursor_runs_only_specific_test() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;

    // Create a valid Cargo project
    let cargo_toml = r#"
[package]
name = "test_specific"
version = "0.1.0"
edition = "2021"
"#;
    fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml)?;

    // Create src directory
    fs::create_dir(temp_dir.path().join("src"))?;

    let test_file = temp_dir.path().join("src/lib.rs");

    // Create a file with multiple tests where some will panic
    let content = r#"
#[cfg(test)]
mod tests {
    #[test]
    fn test_that_passes() {
        println!("Test 1: This test passes");
        assert_eq!(1 + 1, 2); // Line 6
    }
    
    #[test]
    fn test_that_fails() {
        println!("Test 2: This test should NOT run when cursor is on test_that_passes");
        panic!("If you see this panic, something is wrong!"); // Line 12
    }
    
    #[test]
    fn another_test_that_fails() {
        println!("Test 3: This test should also NOT run");
        panic!("This test should not execute!"); // Line 18
    }
    
    mod nested {
        #[test]
        fn nested_test_that_fails() {
            println!("Test 4: Nested test that should NOT run");
            panic!("Nested test should not execute!"); // Line 25
        }
    }
}
"#;

    fs::write(&test_file, content)?;

    // Test 1: Cursor on the passing test (line 5 in 1-based indexing)
    println!("Testing cursor on specific test (test_that_passes)...");

    let cursor_pos = Position { line: 4, column: 1 }; // 0-based line for test_that_passes (1-based line 5)
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

    // Verify the command targets only the specific test
    let args_str = best_command.args.join(" ");
    assert!(
        args_str.contains("tests::test_that_passes") && args_str.contains("--exact"),
        "Command should target only test_that_passes with --exact flag. Got: {args_str}"
    );

    // Actually run the command
    let output = Command::new(&best_command.command)
        .args(&best_command.args)
        .current_dir(temp_dir.path())
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let _stderr = String::from_utf8_lossy(&output.stderr);

    println!("Test output:");
    println!("{stdout}");

    // Verify:
    // 1. The command succeeded (the passing test passed)
    assert!(
        output.status.success(),
        "The test_that_passes should succeed"
    );

    // 2. Only one test ran
    assert!(
        stdout.contains("running 1 test"),
        "Should run exactly 1 test, not all tests"
    );

    // 3. The correct test ran
    assert!(
        stdout.contains("test tests::test_that_passes ... ok"),
        "Should run test_that_passes"
    );

    // 4. Other tests did NOT run (no panics from other tests)
    assert!(
        !stdout.contains("This test should NOT run"),
        "Should not run test_that_fails"
    );
    assert!(
        !stdout.contains("This test should also NOT run"),
        "Should not run another_test_that_fails"
    );
    assert!(
        !stdout.contains("Nested test that should NOT run"),
        "Should not run nested_test_that_fails"
    );

    // Test 2: Run all tests to verify they would fail if run
    println!("\nVerifying that other tests would fail if run...");
    let all_tests_output = Command::new("cargo")
        .args(["test", "--package", "test_specific"])
        .current_dir(temp_dir.path())
        .output()?;

    assert!(
        !all_tests_output.status.success(),
        "Running all tests should fail due to panics"
    );

    let all_stdout = String::from_utf8_lossy(&all_tests_output.stdout);
    assert!(
        all_stdout.contains("3 failed"),
        "Should have 3 failing tests when running all"
    );

    println!("\n✅ Success! Cursor-based test selection correctly runs only the specific test!");
    println!("   - When cursor is on test_that_passes, only that test runs");
    println!("   - The 3 other tests that would panic are NOT executed");
    println!("   - This prevents false test failures from unrelated tests");

    Ok(())
}

#[test]
fn test_cursor_in_nested_module_runs_only_that_test() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;

    // Create a valid Cargo project
    let cargo_toml = r#"
[package]
name = "test_nested_specific"
version = "0.1.0"
edition = "2021"
"#;
    fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml)?;

    // Create src directory
    fs::create_dir(temp_dir.path().join("src"))?;

    let test_file = temp_dir.path().join("src/lib.rs");

    // Create a file with nested modules
    let content = r#"
#[cfg(test)]
mod tests {
    mod unit {
        #[test]
        fn unit_test_1() {
            panic!("Unit test 1 should not run"); // Line 6
        }
        
        #[test]
        fn unit_test_2() {
            assert_eq!(2 + 2, 4); // Line 11 - cursor here
        }
    }
    
    mod integration {
        #[test]
        fn integration_test_1() {
            panic!("Integration test 1 should not run"); // Line 18
        }
        
        #[test]
        fn integration_test_2() {
            panic!("Integration test 2 should not run"); // Line 23
        }
    }
}
"#;

    fs::write(&test_file, content)?;

    // Cursor on unit_test_2 (line 11)
    let context = FileDetector::detect_context(
        &test_file,
        Some(Position {
            line: 10,
            column: 1,
        }),
    )?;
    let commands = UniversalCommandGenerator::generate_commands(
        &context,
        Some(Position {
            line: 10,
            column: 1,
        }),
    )?;

    let best_command = &commands[0];
    let args_str = best_command.args.join(" ");

    // Verify correct module path
    assert!(
        args_str.contains("tests::unit::unit_test_2") && args_str.contains("--exact"),
        "Should target tests::unit::unit_test_2 specifically"
    );

    // Run the command
    let output = Command::new(&best_command.command)
        .args(&best_command.args)
        .current_dir(temp_dir.path())
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify only one test ran and it passed
    assert!(output.status.success(), "unit_test_2 should pass");
    assert!(
        stdout.contains("running 1 test"),
        "Should run exactly 1 test"
    );
    assert!(
        stdout.contains("test tests::unit::unit_test_2 ... ok"),
        "Should run the correct nested test"
    );

    println!("✅ Nested module test selection works correctly!");

    Ok(())
}
