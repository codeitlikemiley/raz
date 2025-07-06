//! Investigation test for command generation logic differences between unit tests and integration tests

use raz_core::{FileDetector, FileRole, Position, UniversalCommandGenerator};
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_unit_test_command_generation() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create a typical cargo project structure
    fs::write(
        project_root.join("Cargo.toml"),
        r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();

    fs::create_dir_all(project_root.join("src")).unwrap();

    // Create a src/lib.rs file with unit tests
    let lib_content = r#"
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

    #[test]
    fn test_add_negative() {
        assert_eq!(add(-1, 1), 0);
    }
}
"#;

    let lib_path = project_root.join("src").join("lib.rs");
    fs::write(&lib_path, lib_content).unwrap();

    // Test file role detection
    let context = FileDetector::detect_context(&lib_path, None).unwrap();
    println!("Unit test file role: {:?}", context.file_role);

    // Should be LibraryRoot for src/lib.rs
    assert!(matches!(context.file_role, FileRole::LibraryRoot));

    // Test command generation without cursor (should get general commands)
    let commands = UniversalCommandGenerator::generate_commands(&context, None).unwrap();
    println!(
        "Unit test commands without cursor: {} commands",
        commands.len()
    );
    for cmd in &commands {
        println!("  - {}: {}", cmd.id, cmd.label);
        println!("    Command: {} {}", cmd.command, cmd.args.join(" "));
    }

    // Test command generation with cursor on specific test (line where test_add is declared)
    let context_with_cursor = FileDetector::detect_context(
        &lib_path,
        Some(Position {
            line: 10,
            column: 4,
        }), // On test_add function declaration
    )
    .unwrap();

    let commands_with_cursor = UniversalCommandGenerator::generate_commands(
        &context_with_cursor,
        Some(Position {
            line: 10,
            column: 4,
        }),
    )
    .unwrap();

    println!(
        "\nUnit test commands with cursor on test_add: {} commands",
        commands_with_cursor.len()
    );
    for cmd in &commands_with_cursor {
        println!("  - {}: {}", cmd.id, cmd.label);
        println!("    Command: {} {}", cmd.command, cmd.args.join(" "));
    }

    // Check entry points
    println!("\nEntry points found:");
    for ep in &context_with_cursor.entry_points {
        println!(
            "  - {}: {:?} at line {}, range: {:?}, full_path: {:?}",
            ep.name, ep.entry_type, ep.line, ep.line_range, ep.full_path
        );
    }

    // Debug: Test find_test_at_cursor directly
    let found_test = raz_core::UniversalCommandGenerator::find_test_at_cursor(
        &context_with_cursor.entry_points,
        Position {
            line: 10,
            column: 4,
        },
    );

    println!("\nDirect find_test_at_cursor result:");
    if let Some(test) = found_test {
        println!(
            "  Found: {} ({:?}) at line {}, range: {:?}",
            test.name, test.entry_type, test.line, test.line_range
        );
    } else {
        println!("  No test found at cursor");
    }
}

#[tokio::test]
async fn test_integration_test_command_generation() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create a typical cargo project structure
    fs::write(
        project_root.join("Cargo.toml"),
        r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();

    fs::create_dir_all(project_root.join("src")).unwrap();
    fs::create_dir_all(project_root.join("tests")).unwrap();

    // Create a src/lib.rs file
    let lib_content = r#"
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;

    let lib_path = project_root.join("src").join("lib.rs");
    fs::write(&lib_path, lib_content).unwrap();

    // Create an integration test file
    let integration_content = r#"
use test_project::add;

#[test]
fn test_integration_add() {
    assert_eq!(add(2, 3), 5);
}

#[test]
fn test_integration_add_negative() {
    assert_eq!(add(-1, 1), 0);
}
"#;

    let integration_path = project_root.join("tests").join("simple_aggregate_test.rs");
    fs::write(&integration_path, integration_content).unwrap();

    // Test file role detection
    let context = FileDetector::detect_context(&integration_path, None).unwrap();
    println!("Integration test file role: {:?}", context.file_role);

    // Should be IntegrationTest for tests/*.rs
    assert!(matches!(
        context.file_role,
        FileRole::IntegrationTest { .. }
    ));

    // Test command generation without cursor (should get general commands)
    let commands = UniversalCommandGenerator::generate_commands(&context, None).unwrap();
    println!(
        "Integration test commands without cursor: {} commands",
        commands.len()
    );
    for cmd in &commands {
        println!("  - {}: {}", cmd.id, cmd.label);
        println!("    Command: {} {}", cmd.command, cmd.args.join(" "));
    }

    // Test command generation with cursor on specific test
    let context_with_cursor = FileDetector::detect_context(
        &integration_path,
        Some(Position { line: 4, column: 4 }), // On test_integration_add function
    )
    .unwrap();

    let commands_with_cursor = UniversalCommandGenerator::generate_commands(
        &context_with_cursor,
        Some(Position { line: 4, column: 4 }),
    )
    .unwrap();

    println!(
        "\nIntegration test commands with cursor on test_integration_add: {} commands",
        commands_with_cursor.len()
    );
    for cmd in &commands_with_cursor {
        println!("  - {}: {}", cmd.id, cmd.label);
        println!("    Command: {} {}", cmd.command, cmd.args.join(" "));
    }

    // Check entry points
    println!("\nEntry points found:");
    for ep in &context_with_cursor.entry_points {
        println!(
            "  - {}: {:?} at line {}, full_path: {:?}",
            ep.name, ep.entry_type, ep.line, ep.full_path
        );
    }
}

#[tokio::test]
async fn test_command_generation_comparison() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create a typical cargo project structure
    fs::write(
        project_root.join("Cargo.toml"),
        r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();

    fs::create_dir_all(project_root.join("src")).unwrap();
    fs::create_dir_all(project_root.join("tests")).unwrap();

    // Create a src/lib.rs file with unit tests
    let lib_content = r#"
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
"#;

    let lib_path = project_root.join("src").join("lib.rs");
    fs::write(&lib_path, lib_content).unwrap();

    // Create an integration test file
    let integration_content = r#"
use test_project::add;

#[test]
fn test_integration_add() {
    assert_eq!(add(2, 3), 5);
}
"#;

    let integration_path = project_root.join("tests").join("simple_aggregate_test.rs");
    fs::write(&integration_path, integration_content).unwrap();

    // Compare unit test and integration test command generation
    let unit_context = FileDetector::detect_context(
        &lib_path,
        Some(Position {
            line: 10,
            column: 4,
        }), // On test_add function declaration
    )
    .unwrap();

    let unit_commands = UniversalCommandGenerator::generate_commands(
        &unit_context,
        Some(Position {
            line: 10,
            column: 4,
        }),
    )
    .unwrap();

    let integration_context = FileDetector::detect_context(
        &integration_path,
        Some(Position { line: 4, column: 4 }), // On test_integration_add function
    )
    .unwrap();

    let integration_commands = UniversalCommandGenerator::generate_commands(
        &integration_context,
        Some(Position { line: 4, column: 4 }),
    )
    .unwrap();

    println!("\n=== COMPARISON SUMMARY ===");
    println!("Unit test file role: {:?}", unit_context.file_role);
    println!(
        "Integration test file role: {:?}",
        integration_context.file_role
    );

    println!("\nUnit test commands ({}):", unit_commands.len());
    for cmd in &unit_commands {
        println!("  - {}: {} {}", cmd.id, cmd.command, cmd.args.join(" "));
    }

    println!(
        "\nIntegration test commands ({}):",
        integration_commands.len()
    );
    for cmd in &integration_commands {
        println!("  - {}: {} {}", cmd.id, cmd.command, cmd.args.join(" "));
    }

    // Check if specific test commands are present
    let unit_specific = unit_commands
        .iter()
        .find(|cmd| cmd.label.contains("specific test") || cmd.label.contains("Run Test:"));
    let integration_specific = integration_commands
        .iter()
        .find(|cmd| cmd.label.contains("specific test") || cmd.label.contains("Run Test:"));

    if let Some(unit_cmd) = unit_specific {
        println!(
            "\nUnit test specific command: {} {}",
            unit_cmd.command,
            unit_cmd.args.join(" ")
        );
    } else {
        println!("\nNo unit test specific command found");
    }

    if let Some(integration_cmd) = integration_specific {
        println!(
            "Integration test specific command: {} {}",
            integration_cmd.command,
            integration_cmd.args.join(" ")
        );
    } else {
        println!("No integration test specific command found");
    }

    // Verify that both have appropriate target flags
    println!("\nAnalyzing target flags:");

    if let Some(unit_cmd) = unit_specific {
        println!("Unit test command args: {:?}", unit_cmd.args);
        // Unit tests might not need --lib flag if using test filter
        // The test filter approach is valid for unit tests
    }

    if let Some(integration_cmd) = integration_specific {
        println!("Integration test command args: {:?}", integration_cmd.args);
        // Integration tests should use --test flag
        assert!(
            integration_cmd.args.contains(&"--test".to_string())
                || integration_cmd
                    .args
                    .iter()
                    .any(|arg| arg.contains("simple_aggregate_test")),
            "Integration test command should include --test flag or test name"
        );
    }

    // Verify that both generate valid specific test commands
    assert!(
        unit_specific.is_some(),
        "Should generate specific command for unit test"
    );
    assert!(
        integration_specific.is_some(),
        "Should generate specific command for integration test"
    );

    // Verify that both use --exact flag for precision
    if let Some(unit_cmd) = unit_specific {
        assert!(
            unit_cmd.args.contains(&"--exact".to_string()),
            "Unit test command should include --exact flag"
        );
    }

    if let Some(integration_cmd) = integration_specific {
        assert!(
            integration_cmd.args.contains(&"--exact".to_string()),
            "Integration test command should include --exact flag"
        );
    }
}
