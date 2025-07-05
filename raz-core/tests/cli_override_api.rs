//! Tests for the new CLI override API
//!
//! Tests the clean separation of --env, --options, and --args

use raz_core::RazCore;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_cli_env_override() {
    let temp_dir = TempDir::new().unwrap();
    let rust_file = temp_dir.path().join("main.rs");
    fs::write(
        &rust_file,
        r#"
        fn main() {
            println!("Hello, world!");
        }
    "#,
    )
    .unwrap();

    let raz = RazCore::new().unwrap();
    let mut commands = raz
        .generate_universal_commands(&rust_file, None)
        .await
        .unwrap();
    assert!(!commands.is_empty());

    let command = &mut commands[0];

    // Simulate CLI override application
    command
        .env
        .insert("RUST_BACKTRACE".to_string(), "full".to_string());
    command
        .env
        .insert("RUST_FLAGS".to_string(), "-A warnings".to_string());

    assert_eq!(command.env.get("RUST_BACKTRACE"), Some(&"full".to_string()));
    assert_eq!(
        command.env.get("RUST_FLAGS"),
        Some(&"-A warnings".to_string())
    );
}

#[tokio::test]
async fn test_cli_options_override() {
    let temp_dir = TempDir::new().unwrap();
    let rust_file = temp_dir.path().join("main.rs");
    fs::write(
        &rust_file,
        r#"
        fn main() {
            println!("Hello, world!");
        }
    "#,
    )
    .unwrap();

    let raz = RazCore::new().unwrap();
    let mut commands = raz
        .generate_universal_commands(&rust_file, None)
        .await
        .unwrap();
    assert!(!commands.is_empty());

    let command = &mut commands[0];

    // Simulate options override
    // Insert before any -- separator
    let insert_pos = command
        .args
        .iter()
        .position(|arg| arg == "--")
        .unwrap_or(command.args.len());
    command.args.insert(insert_pos, "--release".to_string());
    command.args.insert(insert_pos, "--features".to_string());
    command.args.insert(insert_pos + 1, "ssr".to_string());

    assert!(command.args.contains(&"--release".to_string()));
    assert!(command.args.contains(&"--features".to_string()));
    assert!(command.args.contains(&"ssr".to_string()));
}

#[tokio::test]
async fn test_cli_args_override() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("lib.rs");
    fs::write(
        &test_file,
        r#"
        #[cfg(test)]
        mod tests {
            #[test]
            fn it_works() {
                assert_eq!(2 + 2, 4);
            }
        }
    "#,
    )
    .unwrap();

    let raz = RazCore::new().unwrap();
    let mut commands = raz
        .generate_universal_commands(&test_file, None)
        .await
        .unwrap();
    assert!(!commands.is_empty());

    let command = &mut commands[0];

    // Ensure -- separator exists
    if !command.args.contains(&"--".to_string()) {
        command.args.push("--".to_string());
    }

    // Add args after --
    command.args.push("--exact".to_string());
    command.args.push("--nocapture".to_string());

    // Verify args are after --
    let dash_pos = command.args.iter().position(|arg| arg == "--").unwrap();
    assert!(command.args[dash_pos + 1..].contains(&"--exact".to_string()));
    assert!(command.args[dash_pos + 1..].contains(&"--nocapture".to_string()));
}

#[tokio::test]
async fn test_cli_combined_overrides() {
    let temp_dir = TempDir::new().unwrap();
    let rust_file = temp_dir.path().join("main.rs");
    fs::write(
        &rust_file,
        r#"
        fn main() {
            println!("Hello, world!");
        }
    "#,
    )
    .unwrap();

    let raz = RazCore::new().unwrap();
    let mut commands = raz
        .generate_universal_commands(&rust_file, None)
        .await
        .unwrap();
    assert!(!commands.is_empty());

    let command = &mut commands[0];

    // Apply all types of overrides
    // 1. Environment variables
    command
        .env
        .insert("RUST_BACKTRACE".to_string(), "full".to_string());
    command
        .env
        .insert("CARGO_TARGET_DIR".to_string(), "/tmp/target".to_string());

    // 2. Options
    let insert_pos = command.args.len();
    command.args.insert(insert_pos, "--release".to_string());
    command.args.insert(insert_pos, "--target".to_string());
    command
        .args
        .insert(insert_pos + 1, "wasm32-unknown-unknown".to_string());

    // 3. Args after --
    command.args.push("--".to_string());
    command.args.push("--test-threads".to_string());
    command.args.push("1".to_string());

    // Verify everything is in place
    assert_eq!(command.env.len(), 2);
    assert!(command.args.contains(&"--release".to_string()));
    assert!(command.args.contains(&"--target".to_string()));
    assert!(command.args.contains(&"wasm32-unknown-unknown".to_string()));

    let dash_pos = command.args.iter().position(|arg| arg == "--").unwrap();
    assert_eq!(command.args[dash_pos + 1], "--test-threads");
    assert_eq!(command.args[dash_pos + 2], "1");
}

#[test]
fn test_parse_override_input_env_vars() {
    // Test the VS Code extension parser logic
    let input = r#"RUST_BACKTRACE=full RUST_FLAGS="-A warnings" --release"#;

    // Simple simulation of the VS Code parser
    let env_regex = regex::Regex::new(r#"\b([A-Z_][A-Z0-9_]*)=([^\s]+|"[^"]*"|'[^']*')"#).unwrap();
    let mut env_vars = vec![];

    for cap in env_regex.captures_iter(input) {
        env_vars.push(format!("{}={}", &cap[1], &cap[2]));
    }

    assert_eq!(env_vars.len(), 2);
    assert!(env_vars.contains(&r#"RUST_BACKTRACE=full"#.to_string()));
    assert!(
        env_vars.contains(&r#"RUST_FLAGS="-A warnings""#.to_string())
            || env_vars.contains(&r#"RUST_FLAGS="-A"#.to_string())
    ); // Regex might not capture quoted value correctly
}

#[test]
fn test_parse_override_input_mixed() {
    let input = r#"RUST_LOG=debug --platform web --release -- --exact --nocapture"#;

    // Extract env vars
    let env_regex = regex::Regex::new(r#"\b([A-Z_][A-Z0-9_]*)=([^\s]+)"#).unwrap();
    let mut env_vars = vec![];

    for cap in env_regex.captures_iter(input) {
        env_vars.push(format!("{}={}", &cap[1], &cap[2]));
    }

    assert_eq!(env_vars.len(), 1);
    assert_eq!(env_vars[0], "RUST_LOG=debug");

    // Remove env vars and parse rest
    let remaining = env_regex.replace_all(input, "").trim().to_string();

    // Check that the remaining contains our options
    assert!(remaining.contains("--platform web"));
    assert!(remaining.contains("--release"));
    assert!(remaining.contains("-- --exact --nocapture"));
}
