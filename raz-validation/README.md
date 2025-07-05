# Raz Validation

Smart options validation system for the Raz command generator.

## Features

- 🎯 **Context-Aware Validation** - Knows which options are valid for which commands
- 🚀 **Framework Support** - Built-in support for Cargo, Leptos, Dioxus, and more
- 💡 **Smart Suggestions** - "Did you mean --release?" error messages
- 🔧 **Configurable** - Multiple validation levels from strict to permissive
- 🧩 **Extensible** - Easy to add new frameworks and custom options

## Quick Start

```rust
use raz_validation::{ValidationEngine, ValidationConfig, ValidationLevel};

// Create validation engine
let engine = ValidationEngine::new();

// Validate cargo options
assert!(engine.validate_option("build", "--release", None).is_ok());
assert!(engine.validate_option("build", "--features", Some("serde,tokio")).is_ok());

// Validate leptos options
assert!(engine.validate_option("leptos build", "--bin-features", Some("ssr")).is_ok());

// Get suggestions for typos
let suggestions = engine.suggest_option("build", "--relase");
assert!(suggestions.contains(&"--release".to_string()));
```

## Validation Levels

- **Off** - No validation (pass everything through)
- **Normal** - Validate known options, allow unknown ones
- **Strict** - Reject unknown options with helpful error messages

```rust
let config = ValidationConfig::with_level(ValidationLevel::Strict);
let engine = ValidationEngine::with_config(config);

// This will fail in strict mode
let result = engine.validate_option("build", "--unknown-option", None);
assert!(result.is_err());
```

## Supported Frameworks

### Cargo
- Core cargo commands: `build`, `test`, `run`, `check`, `clippy`
- Common options: `--release`, `--features`, `--target`, `--jobs`
- Conflict detection: `--lib` vs `--bin` vs `--bins`

### Leptos
- Commands: `leptos build`, `leptos serve`, `leptos watch`
- Framework options: `--bin-features`, `--lib-features`, `--hot-reload`
- Template support: `leptos new --template start-axum`

## Integration with Override Parser

The validation system integrates seamlessly with the raz-override parser:

```rust
use raz_override::parser::override_parser::OverrideParser;
use raz_validation::{ValidationConfig, ValidationLevel};

// Create parser with validation
let config = ValidationConfig::with_level(ValidationLevel::Strict);
let parser = OverrideParser::with_validation_config("build", config);

// Parse and validate
let parsed = parser.parse("--release --features serde").unwrap();
parser.validate(&parsed).unwrap(); // Will validate using the engine

// Get suggestions for typos
let suggestions = parser.suggest_option("--relase");
```

## Adding Custom Providers

```rust
use raz_validation::provider::{OptionProvider, OptionDef, ValueValidator};

struct MyFrameworkProvider;

impl OptionProvider for MyFrameworkProvider {
    fn name(&self) -> &str { "my-framework" }
    
    fn get_options(&self, command: &str) -> Vec<OptionDef> {
        vec![
            OptionDef::flag("--my-flag", "Custom flag option"),
            OptionDef::single("--my-value", "Custom value option", ValueValidator::Any),
        ]
    }
    
    fn validate(&self, command: &str, option: &str, value: Option<&str>) -> ValidationResult<()> {
        // Custom validation logic
        Ok(())
    }
    
    fn get_commands(&self) -> Vec<String> {
        vec!["my-framework build".to_string()]
    }
}

// Register the provider
let mut engine = ValidationEngine::new();
engine.register_provider(Box::new(MyFrameworkProvider));
```

## Error Handling

The validation system provides detailed error messages:

```rust
match engine.validate_option("build", "--relase", None) {
    Err(ValidationError::UnknownOption { command, option, suggestions }) => {
        println!("Unknown option '{}' for command '{}'", option, command);
        if !suggestions.is_empty() {
            println!("Did you mean: {}", suggestions.join(", "));
        }
    }
    Err(ValidationError::InvalidValue { option, value, reason }) => {
        println!("Invalid value '{}' for option '{}': {}", value, option, reason);
    }
    _ => {}
}
```

## Performance

- Lazy loading of option definitions
- Efficient fuzzy matching for suggestions
- Minimal overhead in permissive modes
- Caching of parsed help output (future feature)

## Examples

See `examples/basic_validation.rs` for a comprehensive demo:

```bash
cargo run -p raz-validation --example basic_validation
```