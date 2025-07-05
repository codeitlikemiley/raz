# Raz Validation System Guide

This guide explains how to use and extend the Raz validation system for intelligent command-line option validation.

## Quick Start

The validation system is automatically integrated with the override parser. No additional setup is required for basic usage.

```bash
# These commands now provide intelligent validation
raz build --release --features serde
raz leptos build --bin-features ssr
raz test --nocapture

# Typos get helpful suggestions
raz build --relase
# Error: Unknown option '--relase' for command 'build'
# Did you mean: --release
```

## Validation in Override System

When using overrides, validation happens automatically:

```bash
# Save override with validation
raz build --save-override --release --features web

# Invalid options are caught immediately
raz build --save-override --invalid-option
# Error: Unknown option '--invalid-option' for command 'build'

# Framework-specific options work correctly
raz leptos build --save-override --bin-features ssr --lib-features hydrate
```

## Supported Frameworks

### Cargo (Built-in)
- **Commands**: build, test, run, check, clippy, doc, clean, bench
- **Options**: --release, --features, --target, --bin, --lib, --jobs, etc.
- **Validation**: Conflict detection (--lib vs --bin), value validation (numeric --jobs)

### Leptos (Built-in)
- **Commands**: leptos build, leptos serve, leptos watch, leptos new
- **Options**: --bin-features, --lib-features, --hot-reload, --port, --template
- **Validation**: Port numbers, template names, feature lists

### Adding New Frameworks

The system is designed to be easily extensible. Here's how to add support for a new framework:

#### 1. Create Provider File
```rust
// In your project or as a contribution to raz-validation
use raz_validation::provider::{OptionProvider, OptionDef, ValueValidator};

pub struct DioxusProvider {
    options: HashMap<String, Vec<OptionDef>>,
}

impl OptionProvider for DioxusProvider {
    fn name(&self) -> &str { "dioxus" }
    
    fn get_options(&self, command: &str) -> Vec<OptionDef> {
        match command {
            "dx build" => vec![
                OptionDef::flag("--release", "Build in release mode"),
                OptionDef::single("--platform", "Target platform", 
                    ValueValidator::Enum(vec!["web".into(), "desktop".into(), "mobile".into()])),
                OptionDef::multiple("--features", "Feature list", ValueValidator::Any),
            ],
            "dx serve" => vec![
                OptionDef::flag("--hot-reload", "Enable hot reloading"),
                OptionDef::single("--port", "Server port", ValueValidator::Number),
            ],
            _ => vec![],
        }
    }
    
    fn validate(&self, command: &str, option: &str, value: Option<&str>) -> ValidationResult<()> {
        // Custom validation logic here
        Ok(())
    }
    
    fn get_commands(&self) -> Vec<String> {
        vec!["dx build".into(), "dx serve".into(), "dx bundle".into()]
    }
}
```

#### 2. Register Provider
```rust
// In your initialization code
use raz_validation::ValidationEngine;

let mut engine = ValidationEngine::new();
engine.register_provider(Box::new(DioxusProvider::new()));
```

## Configuration Options

### Validation Levels

You can configure how strict validation should be:

```toml
# In your raz config
[validation]
level = "normal"  # "off", "minimal", "normal", "strict"
providers = ["cargo", "leptos", "dioxus"]
suggestion_threshold = 30
```

**Levels explained:**
- `off` - No validation (maximum compatibility)
- `minimal` - Only basic conflict checking
- `normal` - Validate known options, allow unknown (default)
- `strict` - Reject all unknown options with suggestions

### Per-Command Configuration

```toml
[validation.commands]
"cargo build" = { level = "strict" }
"leptos build" = { level = "normal" }
"custom-tool" = { level = "off" }
```

### Custom Options

Add support for your own tools without writing a full provider:

```toml
[validation.custom."my-tool"]
options = [
    { name = "--verbose", type = "flag", description = "Enable verbose output" },
    { name = "--config", type = "file", description = "Configuration file" },
    { name = "--level", type = "enum", values = ["debug", "info", "warn", "error"] },
]
```

## Integration Examples

### With VS Code Extension

The validation system automatically enhances the VS Code experience:

- **Real-time validation** as you type overrides
- **IntelliSense suggestions** for available options
- **Error highlighting** for invalid options
- **Quick fixes** for common typos

### With CLI Tool

```rust
use raz_validation::{ValidationEngine, ValidationConfig, ValidationLevel};

fn validate_user_input(command: &str, options: &[&str]) -> Result<(), String> {
    let config = ValidationConfig::with_level(ValidationLevel::Strict);
    let engine = ValidationEngine::with_config(config);
    
    for option in options {
        if let Err(e) = engine.validate_option(command, option, None) {
            // Get suggestions for errors
            let suggestions = engine.suggest_option(command, option);
            if !suggestions.is_empty() {
                return Err(format!("{}\nDid you mean: {}", e, suggestions.join(", ")));
            }
            return Err(e.to_string());
        }
    }
    
    Ok(())
}
```

### Batch Validation

```rust
use std::collections::HashMap;

let options = HashMap::from([
    ("--release".to_string(), None),
    ("--features".to_string(), Some("web,ssr".to_string())),
    ("--target".to_string(), Some("wasm32-unknown-unknown".to_string())),
]);

// Validate all at once, including conflict checking
engine.validate_options("build", &options)?;
```

## Troubleshooting

### Common Issues

**Q: "My custom tool's options aren't recognized"**
A: Make sure you've registered a provider or added custom options to the config. The validation system only knows about explicitly defined options.

**Q: "Validation is too strict/permissive"**
A: Adjust the validation level in your config. Use `normal` for development and `strict` for production/CI.

**Q: "Getting false positives for valid options"**
A: The built-in providers might not cover all possible options. Consider contributing to the provider or using a custom provider.

### Debugging

Enable debug output to see what the validation system is doing:

```bash
RUST_LOG=raz_validation=debug raz build --your-options
```

This will show:
- Which providers are being consulted
- How options are being matched
- Why certain validations pass or fail

### Performance

The validation system is designed to be fast, but if you're experiencing performance issues:

1. **Use appropriate validation levels** - `off` or `minimal` for performance-critical paths
2. **Reuse ValidationEngine instances** - Don't create new engines for each validation
3. **Consider lazy provider loading** - Only register providers you actually use

## Contributing

### Adding Built-in Provider Support

We welcome contributions for new framework providers:

1. **Research the framework** - Document all command-line options
2. **Create comprehensive provider** - Include all commands and options
3. **Add thorough tests** - Cover edge cases and validation scenarios
4. **Update documentation** - Add examples and usage patterns

### Improving Existing Providers

- **Add missing options** - Cargo and other tools add new options regularly
- **Improve value validation** - More specific validation for option values
- **Better conflict detection** - Identify more mutually exclusive options
- **Enhanced suggestions** - Better fuzzy matching for typos

### Testing

```bash
# Run validation-specific tests
cargo test -p raz-validation

# Run integration tests
cargo test --test test_validation_integration

# Run benchmarks
cargo bench -p raz-validation
```

## Future Roadmap

### Planned Features

- **Dynamic provider loading** - Load providers from plugins
- **Help text generation** - Generate help from option definitions
- **Completion support** - Shell completion integration
- **Online option database** - Fetch latest options from tool documentation
- **IDE integration** - Language server protocol support

### Framework Support Roadmap

- **Tauri** - Cross-platform app framework
- **Yew** - WebAssembly frontend framework
- **Bevy** - Game engine
- **Custom project tools** - Support for project-specific tooling

The validation system is designed to grow with the Rust ecosystem while maintaining backward compatibility and performance.