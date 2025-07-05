use raz_core::Position;
use raz_core::file_detection::FileDetector;
use raz_core::universal_command_generator::UniversalCommandGenerator;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_hidden_doctest_lines() {
    let temp_dir = TempDir::new().unwrap();

    // Create cargo project
    let cargo_toml = r#"
[package]
name = "hidden-doctest-project"
version = "0.1.0"
edition = "2021"
"#;
    fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml).unwrap();

    // Create lib.rs with doc test that has hidden lines
    fs::create_dir(temp_dir.path().join("src")).unwrap();
    let lib_content = r#"/// This function adds two numbers
/// 
/// ```rust
/// # use hidden_doctest_project::add;
/// # let x = 10;  // Line 5 - hidden comment line
/// let y = 20;    // Line 6 - visible line
/// # let z = 30;  // Line 7 - hidden comment line
/// let sum = add(x, y);
/// assert_eq!(sum, 30);
/// ```
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;
    let lib_path = temp_dir.path().join("src/lib.rs");
    fs::write(&lib_path, lib_content).unwrap();

    // Test case 1: Cursor on hidden comment line (line 5)
    println!("\n=== Test Case 1: Cursor on hidden comment line ===");
    let cursor_pos_hidden = Position {
        line: 4, // 0-based, so line 5 in 1-based
        column: 10,
    };
    let context_hidden = FileDetector::detect_context(&lib_path, Some(cursor_pos_hidden)).unwrap();
    let commands_hidden =
        UniversalCommandGenerator::generate_commands(&context_hidden, Some(cursor_pos_hidden))
            .unwrap();

    // Debug: Check what find_test_at_cursor returns
    let found_test =
        raz_core::universal_command_generator::UniversalCommandGenerator::find_test_at_cursor(
            &context_hidden.entry_points,
            cursor_pos_hidden,
        );
    println!("find_test_at_cursor result: {found_test:?}");

    // Check if this is a doctest type
    if let Some(test_entry) = found_test {
        println!("Test entry type: {:?}", test_entry.entry_type);
        println!(
            "Is DocTest: {}",
            matches!(
                test_entry.entry_type,
                raz_core::file_detection::EntryPointType::DocTest
            )
        );
    }

    println!("Entry points found: {}", context_hidden.entry_points.len());
    for entry in &context_hidden.entry_points {
        println!(
            "  - {}: {:?} at line {} (range: {:?})",
            entry.name, entry.entry_type, entry.line, entry.line_range
        );
    }
    println!("Cursor at 1-based line: {}", cursor_pos_hidden.line + 1);
    println!("Commands generated: {}", commands_hidden.len());
    for cmd in &commands_hidden {
        println!(
            "  - {} [Priority: {}]: {} {}",
            cmd.label,
            cmd.priority,
            cmd.command,
            cmd.args.join(" ")
        );
    }

    // Test case 2: Cursor on visible line (line 6)
    println!("\n=== Test Case 2: Cursor on visible line ===");
    let cursor_pos_visible = Position {
        line: 5, // 0-based, so line 6 in 1-based
        column: 10,
    };
    let context_visible =
        FileDetector::detect_context(&lib_path, Some(cursor_pos_visible)).unwrap();
    let commands_visible =
        UniversalCommandGenerator::generate_commands(&context_visible, Some(cursor_pos_visible))
            .unwrap();

    println!("Entry points found: {}", context_visible.entry_points.len());
    for entry in &context_visible.entry_points {
        println!(
            "  - {}: {:?} at line {}",
            entry.name, entry.entry_type, entry.line
        );
    }
    println!("Commands generated: {}", commands_visible.len());
    for cmd in &commands_visible {
        println!(
            "  - {} [Priority: {}]: {} {}",
            cmd.label,
            cmd.priority,
            cmd.command,
            cmd.args.join(" ")
        );
    }

    // Test case 3: Cursor on another hidden comment line (line 7)
    println!("\n=== Test Case 3: Cursor on another hidden comment line ===");
    let cursor_pos_hidden2 = Position {
        line: 6, // 0-based, so line 7 in 1-based
        column: 10,
    };
    let context_hidden2 =
        FileDetector::detect_context(&lib_path, Some(cursor_pos_hidden2)).unwrap();
    let commands_hidden2 =
        UniversalCommandGenerator::generate_commands(&context_hidden2, Some(cursor_pos_hidden2))
            .unwrap();

    println!("Entry points found: {}", context_hidden2.entry_points.len());
    for entry in &context_hidden2.entry_points {
        println!(
            "  - {}: {:?} at line {}",
            entry.name, entry.entry_type, entry.line
        );
    }
    println!("Commands generated: {}", commands_hidden2.len());
    for cmd in &commands_hidden2 {
        println!(
            "  - {} [Priority: {}]: {} {}",
            cmd.label,
            cmd.priority,
            cmd.command,
            cmd.args.join(" ")
        );
    }

    // Check if doc test commands are available for all cursor positions
    let has_doctest_hidden = commands_hidden
        .iter()
        .any(|c| c.args.contains(&"--doc".to_string()));
    let has_doctest_visible = commands_visible
        .iter()
        .any(|c| c.args.contains(&"--doc".to_string()));
    let has_doctest_hidden2 = commands_hidden2
        .iter()
        .any(|c| c.args.contains(&"--doc".to_string()));

    println!("\n=== Results ===");
    println!("Hidden line 1 has doc test: {has_doctest_hidden}");
    println!("Visible line has doc test: {has_doctest_visible}");
    println!("Hidden line 2 has doc test: {has_doctest_hidden2}");

    // The issue is likely that hidden lines are not being detected properly
    // All positions within the doctest code block should have access to doctest commands
    if !has_doctest_hidden || !has_doctest_hidden2 {
        println!(
            "Issue reproduced: Hidden comment lines in doctest don't have doc test commands available"
        );
    }
}
