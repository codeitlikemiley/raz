//! Integration test to verify the fix for nested module path detection

use raz_core::{FileDetector, Position, UniversalCommandGenerator};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_nested_module_file_detection() -> Result<(), Box<dyn std::error::Error>> {
    // Create a cargo project structure like rust-bench-guide
    let temp_dir = TempDir::new()?;
    let project_root = temp_dir.path();

    // Create Cargo.toml
    fs::write(
        project_root.join("Cargo.toml"),
        r#"[package]
name = "rust-bench-guide"
version = "0.1.0"
edition = "2021"
"#,
    )?;

    // Create nested directory structure
    fs::create_dir_all(project_root.join("src/power/over"))?;

    // Create lib.rs
    fs::write(project_root.join("src/lib.rs"), r#"pub mod power;"#)?;

    // Create power/mod.rs
    fs::write(project_root.join("src/power/mod.rs"), r#"pub mod over;"#)?;

    // Create power/over/mod.rs
    fs::write(
        project_root.join("src/power/over/mod.rs"),
        r#"pub mod level_900;"#,
    )?;

    // Create the test file
    let test_file = project_root.join("src/power/over/level_900.rs");
    fs::write(
        &test_file,
        r#"pub fn power_level() -> u32 {
    9000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(power_level(), 9000);
    }

    #[test]
    fn it_never_works() {
        assert_eq!(power_level(), 9001);
    }
}
"#,
    )?;

    // Test with cursor on line 15 (1-based) where it_never_works is defined
    // Line 15 is 0-based line 14
    let cursor = Position {
        line: 14, // 0-based line number for "fn it_never_works()"
        column: 4,
    };
    let context = FileDetector::detect_context(&test_file, Some(cursor))?;

    // Verify the file is detected as a Module in a cargo package
    match &context.project_type {
        raz_core::file_detection::RustProjectType::CargoPackage { package_name, .. } => {
            assert_eq!(package_name, "rust-bench-guide");
        }
        _ => panic!("Expected CargoPackage project type"),
    }

    assert_eq!(
        context.file_role,
        raz_core::file_detection::FileRole::Module
    );

    // Find the specific test entry point
    let test_entry = context
        .entry_points
        .iter()
        .find(|ep| ep.name == "it_never_works")
        .expect("Should find it_never_works test");

    // Verify the full path includes the module hierarchy
    assert_eq!(
        test_entry.full_path.as_ref().unwrap(),
        "power::over::level_900::tests::it_never_works"
    );

    // Generate commands and verify
    let commands = UniversalCommandGenerator::generate_commands(&context, Some(cursor))?;

    // Find the specific test command - it might be in the "all tests in module" command
    let test_command = commands
        .iter()
        .find(|c| {
            c.args
                .iter()
                .any(|arg| arg.contains("power::over::level_900::tests::it_never_works"))
                || (c.label.contains("module")
                    && c.args
                        .iter()
                        .any(|arg| arg.contains("power::over::level_900::tests::")))
        })
        .expect("Should generate test command for the module or specific test");

    // Verify it's a cargo test command
    assert_eq!(test_command.command, "cargo");
    assert!(test_command.args.contains(&"test".to_string()));
    assert!(test_command.args.contains(&"--package".to_string()));
    assert!(test_command.args.contains(&"rust-bench-guide".to_string()));

    // Verify the full module path is in the arguments
    let args_str = test_command.args.join(" ");
    assert!(
        args_str.contains("power::over::level_900::tests::it_never_works"),
        "Command should include full module path. Got: {args_str}"
    );
    assert!(
        args_str.contains("--exact"),
        "Command should include --exact flag"
    );

    Ok(())
}

#[test]
fn test_module_file_not_treated_as_standalone() -> Result<(), Box<dyn std::error::Error>> {
    // Create a cargo project
    let temp_dir = TempDir::new()?;
    let project_root = temp_dir.path();

    fs::write(
        project_root.join("Cargo.toml"),
        r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"
"#,
    )?;

    // Create a module file in src/
    fs::create_dir_all(project_root.join("src/utils"))?;
    let module_file = project_root.join("src/utils/helpers.rs");
    fs::write(
        &module_file,
        r#"
pub fn helper() -> i32 {
    42
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_helper() {
        assert_eq!(helper(), 42);
    }
}
"#,
    )?;

    let context = FileDetector::detect_context(&module_file, None)?;

    // Should be detected as Module, not Standalone
    assert_eq!(
        context.file_role,
        raz_core::file_detection::FileRole::Module
    );

    // Should remain a CargoPackage, not be converted to SingleFile
    match context.project_type {
        raz_core::file_detection::RustProjectType::CargoPackage { .. } => {}
        _ => panic!(
            "Module file should not be converted to SingleFile: {:?}",
            context.project_type
        ),
    }

    // Generate commands to ensure it uses cargo test, not rustc
    let commands = UniversalCommandGenerator::generate_commands(&context, None)?;
    let test_command = commands
        .iter()
        .find(|c| c.id == "cargo-test")
        .expect("Should have cargo test command");

    assert_eq!(test_command.command, "cargo");
    assert!(!commands.iter().any(|c| c.command == "rustc"));

    Ok(())
}
