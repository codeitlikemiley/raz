//! Integration tests for the universal command generator

use raz_core::{FileDetector, Position, UniversalCommandGenerator};
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_single_file_executable() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("hello.rs");

    fs::write(&file_path, "fn main() {\n    println!(\"Hello, world!\");\n}\n\n#[test]\nfn test_hello() {\n    assert_eq!(1 + 1, 2);\n}").unwrap();

    let context = FileDetector::detect_context(&file_path, None).unwrap();
    let commands = UniversalCommandGenerator::generate_commands(&context, None).unwrap();

    assert!(!commands.is_empty());
    // Should have rustc run commands for single file
    assert!(commands.iter().any(|c| c.id == "rustc-run"));
    assert!(commands.iter().any(|c| c.id == "rustc-test"));
    assert!(commands.iter().any(|c| c.command == "sh"));
}

#[tokio::test]
async fn test_single_file_library() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("lib.rs");

    fs::write(&file_path, "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n\n    #[test]\n    fn test_add() {\n        assert_eq!(add(2, 3), 5);\n    }\n}").unwrap();

    let context = FileDetector::detect_context(&file_path, None).unwrap();
    let commands = UniversalCommandGenerator::generate_commands(&context, None).unwrap();

    assert!(!commands.is_empty());
    // Should have rustc test command for library
    assert!(commands.iter().any(|c| c.id == "rustc-test"));
    // Should NOT have run commands for library
    assert!(!commands.iter().any(|c| c.id == "rustc-run"));
    // Should NOT have rustdoc-test unless there are actual doc tests
    // (our test file doesn't have doc tests, only regular tests)
}

#[tokio::test]
async fn test_cargo_script() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("script.rs");

    fs::write(&file_path, "#!/usr/bin/env -S cargo +nightly -Zscript\n\nfn main() {\n    println!(\"Hello from script!\");\n}\n\n#[test]\nfn test_script() {\n    assert_eq!(2 + 2, 4);\n}").unwrap();

    let context = FileDetector::detect_context(&file_path, None).unwrap();
    let commands = UniversalCommandGenerator::generate_commands(&context, None).unwrap();

    assert!(!commands.is_empty());
    // Should have cargo script commands
    assert!(commands.iter().any(|c| c.id == "cargo-script-run"));
    assert!(commands.iter().any(|c| c.id == "cargo-script-test"));
    assert!(
        commands
            .iter()
            .any(|c| c.args.contains(&"+nightly".to_string()))
    );
    assert!(
        commands
            .iter()
            .any(|c| c.args.contains(&"-Zscript".to_string()))
    );
}

#[tokio::test]
async fn test_cargo_package_binary() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create Cargo.toml
    fs::write(
        project_root.join("Cargo.toml"),
        "[package]\nname = \"hello-world\"\nversion = \"0.1.0\"\nedition = \"2021\"",
    )
    .unwrap();

    // Create src directory and main.rs
    fs::create_dir_all(project_root.join("src")).unwrap();
    let main_path = project_root.join("src").join("main.rs");
    fs::write(&main_path, "fn main() {\n    println!(\"Hello, world!\");\n}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn test_main() {\n        assert!(true);\n    }\n}").unwrap();

    let context = FileDetector::detect_context(&main_path, None).unwrap();
    let commands = UniversalCommandGenerator::generate_commands(&context, None).unwrap();

    assert!(!commands.is_empty());
    // Should have cargo run commands
    assert!(commands.iter().any(|c| c.id == "cargo-run"));
    assert!(commands.iter().any(|c| c.command == "cargo"));
    assert!(commands.iter().any(|c| c.args.contains(&"run".to_string())));
    // Should have cargo test commands
    assert!(commands.iter().any(|c| c.id == "cargo-test"));
    assert!(
        commands
            .iter()
            .any(|c| c.args.contains(&"test".to_string()))
    );
}

#[tokio::test]
async fn test_leptos_frontend_library() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create workspace Cargo.toml
    fs::write(project_root.join("Cargo.toml"), "[workspace]\nmembers = [\"frontend\", \"server\"]\n\n[workspace.dependencies]\nleptos = \"0.6\"\nleptos_axum = \"0.6\"").unwrap();

    // Create frontend package
    fs::create_dir_all(project_root.join("frontend").join("src")).unwrap();
    fs::write(project_root.join("frontend").join("Cargo.toml"), "[package]\nname = \"frontend\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\ncrate-type = [\"cdylib\"]\n\n[dependencies]\nleptos = { workspace = true }").unwrap();

    let frontend_lib = project_root.join("frontend").join("src").join("lib.rs");
    fs::write(&frontend_lib, "use leptos::*;\n\n#[component]\npub fn App() -> impl IntoView {\n    view! {\n        <h1>\"Hello Leptos!\"</h1>\n    }\n}").unwrap();

    let context = FileDetector::detect_context(&frontend_lib, None).unwrap();
    let commands = UniversalCommandGenerator::generate_commands(&context, None).unwrap();

    assert!(!commands.is_empty());
    // Should have Leptos-specific commands for frontend library
    assert!(commands.iter().any(|c| c.id == "leptos-watch"));
    assert!(commands.iter().any(|c| c.command == "cargo"));
    assert!(
        commands
            .iter()
            .any(|c| c.args.contains(&"leptos".to_string()))
    );
    assert!(
        commands
            .iter()
            .any(|c| c.args.contains(&"watch".to_string()))
    );
}

#[tokio::test]
async fn test_integration_test_file() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create Cargo.toml
    fs::write(
        project_root.join("Cargo.toml"),
        "[package]\nname = \"test-project\"\nversion = \"0.1.0\"\nedition = \"2021\"",
    )
    .unwrap();

    // Create src/lib.rs
    fs::create_dir_all(project_root.join("src")).unwrap();
    fs::write(
        project_root.join("src").join("lib.rs"),
        "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}",
    )
    .unwrap();

    // Create tests directory and integration test
    fs::create_dir_all(project_root.join("tests")).unwrap();
    let test_file = project_root.join("tests").join("integration.rs");
    fs::write(&test_file, "use test_project::add;\n\n#[test]\nfn test_integration_add() {\n    assert_eq!(add(2, 3), 5);\n}").unwrap();

    let context = FileDetector::detect_context(&test_file, None).unwrap();
    let commands = UniversalCommandGenerator::generate_commands(&context, None).unwrap();

    assert!(!commands.is_empty());
    // Should have integration test specific commands
    assert!(commands.iter().any(|c| c.id == "cargo-test"));
    assert!(
        commands
            .iter()
            .any(|c| c.args.contains(&"--test".to_string()))
    );
    assert!(
        commands
            .iter()
            .any(|c| c.args.contains(&"integration".to_string()))
    );
    // Should NOT have run commands for test file
    assert!(!commands.iter().any(|c| c.id == "cargo-run"));
}

#[tokio::test]
async fn test_stateless_operation() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("standalone.rs");

    fs::write(
        &file_path,
        "fn main() {\n    println!(\"This is a standalone file\");\n}",
    )
    .unwrap();

    // Test from different "working directories" - should always work the same
    let context1 = FileDetector::detect_context(&file_path, None).unwrap();
    let commands1 = UniversalCommandGenerator::generate_commands(&context1, None).unwrap();

    let context2 = FileDetector::detect_context(&file_path, None).unwrap();
    let commands2 = UniversalCommandGenerator::generate_commands(&context2, None).unwrap();

    // Results should be identical regardless of working directory
    assert_eq!(commands1.len(), commands2.len());
    assert_eq!(commands1.first().unwrap().id, commands2.first().unwrap().id);
    assert_eq!(
        commands1.first().unwrap().command,
        commands2.first().unwrap().command
    );

    // Working directory should be set to the file's directory
    assert_eq!(
        commands1.first().unwrap().cwd,
        Some(file_path.parent().unwrap().to_path_buf())
    );
}

#[tokio::test]
async fn test_binary_file_test_commands() {
    // Create a project structure with a binary in src/bin/
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();

    // Create Cargo.toml
    let cargo_toml = r#"
[package]
name = "test-project"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "mybin"
path = "src/bin/mybin.rs"
"#;
    fs::write(project_root.join("Cargo.toml"), cargo_toml).unwrap();

    // Create src/bin directory
    fs::create_dir_all(project_root.join("src/bin")).unwrap();

    // Create binary file with tests
    let binary_content = r#"
fn main() {
    println!("Hello from mybin");
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_something() {
        assert_eq!(1 + 1, 2);
    }
}
"#;

    let binary_path = project_root.join("src/bin/mybin.rs");
    fs::write(&binary_path, binary_content).unwrap();

    // Test command generation with cursor on test
    let context = FileDetector::detect_context(
        &binary_path,
        Some(Position { line: 8, column: 1 }), // On test function
    )
    .unwrap();

    let commands = UniversalCommandGenerator::generate_commands(
        &context,
        Some(Position { line: 8, column: 1 }),
    )
    .unwrap();

    // Find the specific test command
    let test_command = commands
        .iter()
        .find(|cmd| cmd.label.contains("specific test"))
        .expect("Should generate specific test command for binary");

    // Verify the command uses --bin flag
    assert_eq!(test_command.command, "cargo");
    assert!(test_command.args.contains(&"test".to_string()));
    assert!(test_command.args.contains(&"--bin".to_string()));
    assert!(test_command.args.contains(&"mybin".to_string()));

    // The test name should not include bin:: prefix
    let separator_idx = test_command
        .args
        .iter()
        .position(|arg| arg == "--")
        .unwrap();
    let test_name = &test_command.args[separator_idx + 1];
    assert_eq!(test_name, "tests::test_something");
    assert!(test_command.args.contains(&"--exact".to_string()));
    assert!(test_command.args.contains(&"--show-output".to_string()));
}
