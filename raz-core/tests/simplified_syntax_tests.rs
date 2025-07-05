//! Tests for the simplified CLI syntax

use raz_override::{OverrideOperator, SmartOverrideParser};

#[test]
fn test_empty_input() {
    let parser = SmartOverrideParser::new("run");
    let result = parser.parse("");

    assert!(result.env_vars.is_empty());
    assert!(result.options.is_empty());
    assert!(result.args.is_empty());
}

#[test]
fn test_env_var_only() {
    let parser = SmartOverrideParser::new("run");
    let result = parser.parse("RUST_BACKTRACE=full");

    assert_eq!(
        result.env_vars.get("RUST_BACKTRACE"),
        Some(&"full".to_string())
    );
    assert!(result.options.is_empty());
    assert!(result.args.is_empty());
}

#[test]
fn test_multiple_env_vars() {
    let parser = SmartOverrideParser::new("run");
    let result = parser.parse("RUST_BACKTRACE=full RUST_LOG=debug CARGO_TARGET_DIR=/tmp");

    assert_eq!(result.env_vars.len(), 3);
    assert_eq!(
        result.env_vars.get("RUST_BACKTRACE"),
        Some(&"full".to_string())
    );
    assert_eq!(result.env_vars.get("RUST_LOG"), Some(&"debug".to_string()));
    assert_eq!(
        result.env_vars.get("CARGO_TARGET_DIR"),
        Some(&"/tmp".to_string())
    );
}

#[test]
fn test_env_var_with_quotes() {
    let parser = SmartOverrideParser::new("run");
    let result = parser.parse(r#"RUST_FLAGS="-A warnings -D unused""#);

    assert_eq!(
        result.env_vars.get("RUST_FLAGS"),
        Some(&"-A warnings -D unused".to_string())
    );
}

#[test]
fn test_options_only() {
    let parser = SmartOverrideParser::new("run");
    let result = parser.parse("--release --target x86_64-unknown-linux-gnu");

    assert_eq!(
        result.options,
        vec!["--release", "--target", "x86_64-unknown-linux-gnu"]
    );
    assert!(result.env_vars.is_empty());
    assert!(result.args.is_empty());
}

#[test]
fn test_framework_options() {
    let parser = SmartOverrideParser::new("run");
    let result = parser.parse("--platform web --device true --open");

    // These are not cargo options but should still be captured
    assert_eq!(
        result.options,
        vec!["--platform", "web", "--device", "true", "--open"]
    );
}

#[test]
fn test_args_after_double_dash() {
    let parser = SmartOverrideParser::new("test");
    let result = parser.parse("-- --exact --nocapture --test-threads 1");

    assert_eq!(
        result.args,
        vec!["--exact", "--nocapture", "--test-threads", "1"]
    );
    assert!(result.options.is_empty());
}

#[test]
fn test_mixed_syntax() {
    let parser = SmartOverrideParser::new("run");
    let result = parser.parse("RUST_LOG=debug --release --features ssr,hydrate -- --port 3000");

    assert_eq!(result.env_vars.get("RUST_LOG"), Some(&"debug".to_string()));
    assert_eq!(
        result.options,
        vec!["--release", "--features", "ssr,hydrate"]
    );
    assert_eq!(result.args, vec!["--port", "3000"]);
}

#[test]
fn test_complex_real_world_example() {
    let parser = SmartOverrideParser::new("test");
    let result = parser.parse(r#"RUST_BACKTRACE=1 RUST_FLAGS="-A warnings" --release --test integration -- --exact --nocapture"#);

    assert_eq!(result.env_vars.len(), 2);
    assert_eq!(
        result.env_vars.get("RUST_BACKTRACE"),
        Some(&"1".to_string())
    );
    assert_eq!(
        result.env_vars.get("RUST_FLAGS"),
        Some(&"-A warnings".to_string())
    );
    assert_eq!(result.options, vec!["--release", "--test", "integration"]);
    assert_eq!(result.args, vec!["--exact", "--nocapture"]);
}

#[test]
fn test_vscode_operators() {
    let parser = SmartOverrideParser::new("run");
    let (result, operators) =
        parser.parse_vscode_input("+--features new_feature ---default-features");

    assert_eq!(
        result.options,
        vec!["--features", "new_feature", "--no-default-features"]
    );
    assert_eq!(operators.get("--features"), Some(&OverrideOperator::Add));
    assert_eq!(
        operators.get("--default-features"),
        Some(&OverrideOperator::Remove)
    );
}

#[test]
fn test_force_operator() {
    let parser = SmartOverrideParser::new("run");
    let (result, operators) = parser.parse_vscode_input("!--target wasm32-unknown-unknown");

    assert_eq!(result.options, vec!["--target", "wasm32-unknown-unknown"]);
    assert_eq!(operators.get("--target"), Some(&OverrideOperator::Force));
}

#[test]
fn test_mixed_operators_and_regular() {
    let parser = SmartOverrideParser::new("run");
    let (result, operators) =
        parser.parse_vscode_input("RUST_LOG=info +--features debug --release ---quiet");

    assert_eq!(result.env_vars.get("RUST_LOG"), Some(&"info".to_string()));
    assert!(result.options.contains(&"--features".to_string()));
    assert!(result.options.contains(&"debug".to_string()));
    assert!(result.options.contains(&"--release".to_string()));
    assert!(result.options.contains(&"--no-quiet".to_string()));

    assert_eq!(operators.get("--features"), Some(&OverrideOperator::Add));
    assert_eq!(operators.get("--quiet"), Some(&OverrideOperator::Remove));
}

#[test]
fn test_cargo_run_options() {
    let parser = SmartOverrideParser::new("run");
    let result = parser.parse("--bin myapp --features cli --release");

    assert_eq!(
        result.options,
        vec!["--bin", "myapp", "--features", "cli", "--release"]
    );
}

#[test]
fn test_cargo_test_options() {
    let parser = SmartOverrideParser::new("test");
    let result = parser.parse("--lib --no-fail-fast --jobs 4");

    assert_eq!(
        result.options,
        vec!["--lib", "--no-fail-fast", "--jobs", "4"]
    );
}

#[test]
fn test_edge_cases() {
    let parser = SmartOverrideParser::new("run");

    // Single dash should not be treated as option
    let result = parser.parse("- not-an-option");
    assert_eq!(result.options, vec!["-", "not-an-option"]);

    // Empty value after option
    let result = parser.parse("--features");
    assert_eq!(result.options, vec!["--features"]);

    // Option-like value after --
    let result = parser.parse("-- --not-an-option");
    assert_eq!(result.args, vec!["--not-an-option"]);
    assert!(result.options.is_empty());
}

#[test]
fn test_special_characters_in_values() {
    let parser = SmartOverrideParser::new("run");

    // Paths with spaces (quoted)
    let result = parser.parse(r#"--target-dir "/path with spaces/target""#);
    assert_eq!(
        result.options,
        vec!["--target-dir", "/path with spaces/target"]
    );

    // Complex env var values
    let result = parser.parse(r#"RUSTFLAGS="-C target-cpu=native -C opt-level=3""#);
    assert_eq!(
        result.env_vars.get("RUSTFLAGS"),
        Some(&"-C target-cpu=native -C opt-level=3".to_string())
    );
}
