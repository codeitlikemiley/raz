use raz_core::Position;
use raz_core::file_detection::FileDetector;
use raz_core::universal_command_generator::UniversalCommandGenerator;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_doc_test_cursor_detection() {
    let temp_dir = TempDir::new().unwrap();

    // Create cargo project
    let cargo_toml = r#"
[package]
name = "doc-test-project"
version = "0.1.0"
edition = "2021"
"#;
    fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml).unwrap();

    // Create lib.rs with doc test
    fs::create_dir(temp_dir.path().join("src")).unwrap();
    let lib_content = r#"/// This function adds two numbers
/// 
/// ```rust
/// use doc_test_project::add;
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

    #[test]
    fn it_fails() {
        assert_eq!(add(2, 2), 5);
    }
}
"#;
    let lib_path = temp_dir.path().join("src/lib.rs");
    fs::write(&lib_path, lib_content).unwrap();

    // Test with cursor on line 5 (inside doc test)
    let cursor_pos = Position {
        line: 5,
        column: 24,
    };
    let context = FileDetector::detect_context(&lib_path, Some(cursor_pos)).unwrap();

    println!("\n=== Doc Test Detection Results ===");
    println!("Project Type: {:?}", context.project_type);
    println!("File Role: {:?}", context.file_role);
    println!("Entry Points: {:?}", context.entry_points);
    println!("Capabilities: {:?}", context.capabilities);

    // Check for doc test entry points
    let doc_test_entries: Vec<_> = context
        .entry_points
        .iter()
        .filter(|ep| ep.entry_type == raz_core::file_detection::EntryPointType::DocTest)
        .collect();

    println!("Doc test entries found: {}", doc_test_entries.len());
    for entry in &doc_test_entries {
        println!(
            "  - {}: {:?} at line {}",
            entry.name, entry.entry_type, entry.line
        );
    }

    let commands =
        UniversalCommandGenerator::generate_commands(&context, Some(cursor_pos)).unwrap();

    println!("\n=== Generated Commands ===");
    println!("Total commands: {}", commands.len());
    for cmd in &commands {
        println!(
            "  - {} ({}): {:?} [Priority: {}]",
            cmd.label, cmd.id, cmd.category, cmd.priority
        );
        println!("    Command: {} {}", cmd.command, cmd.args.join(" "));
    }

    // Should detect doc test and generate appropriate command
    assert!(
        !doc_test_entries.is_empty(),
        "Should detect doc test entry points"
    );

    let doc_test_cmd = commands
        .iter()
        .find(|c| c.command == "cargo" && c.args.contains(&"--doc".to_string()))
        .expect("Should generate doc test command");

    // Should have --doc flag
    assert!(
        doc_test_cmd.args.contains(&"--doc".to_string()),
        "Should have --doc flag"
    );

    // Should filter by function name
    assert!(
        doc_test_cmd.args.contains(&"add".to_string()),
        "Should filter by function name"
    );

    // Should have --show-output
    assert!(
        doc_test_cmd.args.contains(&"--show-output".to_string()),
        "Should have --show-output flag"
    );
}
