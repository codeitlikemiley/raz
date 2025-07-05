use raz_validation::{ValidationConfig, ValidationEngine, ValidationLevel};

fn main() {
    println!("=== Raz Validation System Demo ===\n");

    // Create validation engine with default config
    let engine = ValidationEngine::new();

    // Test cargo options
    println!("🚀 Testing Cargo Options:");
    test_option(&engine, "build", "--release", None);
    test_option(
        &engine,
        "build",
        "--target",
        Some("x86_64-unknown-linux-gnu"),
    );
    test_option(&engine, "build", "--features", Some("serde,tokio"));
    test_option(&engine, "build", "--unknown-option", None);
    println!();

    // Test leptos options
    println!("🎯 Testing Leptos Options:");
    test_option(&engine, "leptos build", "--bin-features", Some("ssr"));
    test_option(&engine, "leptos build", "--lib-features", Some("hydrate"));
    test_option(&engine, "leptos serve", "--port", Some("3000"));
    test_option(&engine, "leptos serve", "--invalid-port", Some("abc"));
    println!();

    // Test suggestions
    println!("💡 Testing Suggestions:");
    test_suggestions(&engine, "build", "--relase");
    test_suggestions(&engine, "build", "--verbos");
    test_suggestions(&engine, "test", "--no-capture");
    println!();

    // Test strict validation
    println!("🔒 Testing Strict Validation:");
    let strict_config = ValidationConfig::with_level(ValidationLevel::Strict);
    let strict_engine = ValidationEngine::with_config(strict_config);

    test_option(&strict_engine, "build", "--release", None);
    test_option(&strict_engine, "build", "--unknown-option", None);
    println!();

    // Test validation levels
    println!("⚙️  Testing Validation Levels:");
    for level in [
        ValidationLevel::Off,
        ValidationLevel::Normal,
        ValidationLevel::Strict,
    ] {
        let config = ValidationConfig::with_level(level);
        let engine = ValidationEngine::with_config(config);

        let result = engine.validate_option("build", "--unknown-option", None);
        println!(
            "  {:?}: {}",
            level,
            if result.is_ok() {
                "✅ PASS"
            } else {
                "❌ FAIL"
            }
        );
    }
}

fn test_option(engine: &ValidationEngine, command: &str, option: &str, value: Option<&str>) {
    let result = engine.validate_option(command, option, value);
    let value_str = value.map_or(String::new(), |v| format!(" {v}"));

    match result {
        Ok(_) => println!("  ✅ {command} {option}{value_str}"),
        Err(e) => println!("  ❌ {command} {option}{value_str}: {e}"),
    }
}

fn test_suggestions(engine: &ValidationEngine, command: &str, misspelled: &str) {
    let suggestions = engine.suggest_option(command, misspelled);
    if suggestions.is_empty() {
        println!("  💭 {command} {misspelled}: No suggestions");
    } else {
        println!(
            "  💭 {} {}: Did you mean {}?",
            command,
            misspelled,
            suggestions.join(", ")
        );
    }
}
