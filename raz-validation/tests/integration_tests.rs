use raz_validation::{ValidationConfig, ValidationEngine, ValidationLevel};

#[test]
fn test_cargo_validation() {
    let engine = ValidationEngine::new();

    // Valid cargo option
    assert!(engine.validate_option("build", "--release", None).is_ok());

    // Valid cargo option with value
    assert!(engine
        .validate_option("build", "--target", Some("x86_64-unknown-linux-gnu"))
        .is_ok());

    // Valid features option
    assert!(engine
        .validate_option("build", "--features", Some("serde,tokio"))
        .is_ok());
}

#[test]
fn test_leptos_validation() {
    let engine = ValidationEngine::new();

    // Valid leptos option
    assert!(engine
        .validate_option("leptos build", "--bin-features", Some("ssr"))
        .is_ok());

    // Valid leptos option
    assert!(engine
        .validate_option("leptos build", "--lib-features", Some("hydrate"))
        .is_ok());

    // Valid leptos serve option
    assert!(engine
        .validate_option("leptos serve", "--port", Some("3000"))
        .is_ok());
}

#[test]
fn test_unknown_option_strict_mode() {
    let config = ValidationConfig::with_level(ValidationLevel::Strict);
    let engine = ValidationEngine::with_config(config);

    // Unknown option should fail in strict mode
    let result = engine.validate_option("build", "--unknown-option", None);
    assert!(result.is_err());

    let error = result.unwrap_err();
    assert!(error.to_string().contains("Unknown option"));
}

#[test]
fn test_unknown_option_normal_mode() {
    let config = ValidationConfig::with_level(ValidationLevel::Normal);
    let engine = ValidationEngine::with_config(config);

    // Unknown option should pass in normal mode
    assert!(engine
        .validate_option("build", "--unknown-option", None)
        .is_ok());
}

#[test]
fn test_suggestions() {
    let engine = ValidationEngine::new();

    let suggestions = engine.suggest_option("build", "--relase");
    assert!(suggestions.contains(&"--release".to_string()));

    let suggestions = engine.suggest_option("build", "--verbos");
    assert!(suggestions.contains(&"--verbose".to_string()));
}

#[test]
fn test_validation_off() {
    let config = ValidationConfig::with_level(ValidationLevel::Off);
    let engine = ValidationEngine::with_config(config);

    // Everything should pass when validation is off
    assert!(engine
        .validate_option("build", "--completely-invalid", None)
        .is_ok());
    assert!(engine
        .validate_option("unknown-command", "--anything", Some("value"))
        .is_ok());
}

#[test]
fn test_option_value_validation() {
    let engine = ValidationEngine::new();

    // Valid number for jobs
    assert!(engine.validate_option("build", "--jobs", Some("4")).is_ok());

    // Invalid number for jobs should fail
    let result = engine.validate_option("build", "--jobs", Some("abc"));
    assert!(result.is_err());
}

#[test]
fn test_leptos_command_detection() {
    let engine = ValidationEngine::new();

    // Should recognize leptos commands
    assert!(engine
        .validate_option("leptos build", "--bin-features", Some("ssr"))
        .is_ok());
    assert!(engine
        .validate_option("leptos serve", "--hot-reload", None)
        .is_ok());
    assert!(engine
        .validate_option("leptos watch", "--reload-port", Some("8080"))
        .is_ok());
}
