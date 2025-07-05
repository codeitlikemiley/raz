# Raz Validation Documentation

The `raz-validation` crate provides intelligent command-line option validation for the Raz ecosystem.

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Getting Started](#getting-started)
- [Built-in Providers](#built-in-providers)
- [Creating Custom Providers](#creating-custom-providers)
- [Validation Levels](#validation-levels)
- [Error Handling](#error-handling)
- [Integration Patterns](#integration-patterns)
- [Performance Considerations](#performance-considerations)
- [API Reference](#api-reference)

## Overview

Raz Validation solves the problem of providing immediate, helpful feedback for command-line options without waiting for tool execution. Instead of generic "unknown option" errors, users get contextual suggestions and framework-aware validation.

### Key Benefits

- **Immediate Feedback** - Catch errors before cargo/tool execution
- **Context-Aware** - Knows which options work with which commands
- **Framework Support** - Built-in knowledge of Cargo, Leptos, Dioxus
- **Smart Suggestions** - "Did you mean --release?" error messages
- **Extensible** - Easy to add new tools and frameworks

## Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│ ValidationEngine│────│ValidationRegistry│────│  OptionProvider │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         │              ┌─────────────────┐              │
         └──────────────│OptionSuggester │              │
                        └─────────────────┘              │
                                                         │
                                              ┌─────────────────┐
                                              │ CargoProvider   │
                                              │ LeptosProvider  │
                                              │ CustomProvider  │
                                              └─────────────────┘
```

### Core Components

- **ValidationEngine** - Main API entry point
- **ValidationRegistry** - Manages multiple providers
- **OptionProvider** - Interface for tool-specific validation
- **OptionSuggester** - Fuzzy matching for suggestions

## Getting Started

### Basic Usage

```rust
use raz_validation::{ValidationEngine, ValidationConfig, ValidationLevel};

// Create engine with default providers
let engine = ValidationEngine::new();

// Validate options
match engine.validate_option("build", "--release", None) {
    Ok(_) => println!("Valid option"),
    Err(e) => println!("Error: {}", e),
}

// Get suggestions for typos
let suggestions = engine.suggest_option("build", "--relase");
// suggestions: ["--release"]
```

### With Custom Configuration

```rust
let config = ValidationConfig::with_level(ValidationLevel::Strict)
    .add_provider("dioxus")
    .with_suggestion_threshold(50);

let engine = ValidationEngine::with_config(config);
```

### Integration with Override Parser

```rust
use raz_override::parser::override_parser::OverrideParser;

let parser = OverrideParser::with_validation_config("build", config);
let parsed = parser.parse("--release --features serde").unwrap();

// Validation happens automatically
parser.validate(&parsed)?;
```

## Built-in Providers

### CargoProvider

Supports all major cargo commands with comprehensive option definitions:

**Commands:**
- `build`, `test`, `run`, `check`, `clippy`, `doc`, `clean`, `bench`, `install`, `publish`

**Key Options:**
- `--release`, `--dev` - Build modes
- `--features`, `--all-features`, `--no-default-features` - Feature flags
- `--target` - Target triple
- `--bin`, `--lib`, `--bins` - Target selection (with conflict detection)
- `--jobs`, `-j` - Parallel jobs (with number validation)

**Example:**
```rust
// Valid cargo options
engine.validate_option("build", "--release", None)?;
engine.validate_option("build", "--features", Some("serde,tokio"))?;
engine.validate_option("test", "--nocapture", None)?;

// Conflict detection
engine.validate_options("build", &HashMap::from([
    ("--lib".to_string(), None),
    ("--bin".to_string(), Some("myapp".to_string())), // Error: conflicts with --lib
]))?;
```

### LeptosProvider

Supports Leptos framework commands with specialized options:

**Commands:**
- `leptos build`, `leptos serve`, `leptos watch`, `leptos new`, `leptos end-to-end`

**Key Options:**
- `--bin-features`, `--lib-features` - Separate feature sets for binary and library
- `--hot-reload`, `--reload-port` - Development features
- `--precompress` - Asset optimization
- `--template` - Project templates

**Example:**
```rust
// Leptos-specific options
engine.validate_option("leptos build", "--bin-features", Some("ssr"))?;
engine.validate_option("leptos serve", "--hot-reload", None)?;
engine.validate_option("leptos new", "--template", Some("start-axum"))?;
```

## Creating Custom Providers

### Basic Provider

```rust
use raz_validation::provider::{OptionProvider, OptionDef, ValueValidator};

struct MyToolProvider;

impl OptionProvider for MyToolProvider {
    fn name(&self) -> &str {
        "mytool"
    }

    fn get_options(&self, command: &str) -> Vec<OptionDef> {
        match command {
            "mytool build" => vec![
                OptionDef::flag("--verbose", "Enable verbose output"),
                OptionDef::single("--output", "Output file", ValueValidator::FilePath),
                OptionDef::multiple("--exclude", "Exclude patterns", ValueValidator::Any),
            ],
            _ => vec![],
        }
    }

    fn validate(&self, command: &str, option: &str, value: Option<&str>) -> ValidationResult<()> {
        // Custom validation logic
        Ok(())
    }

    fn get_commands(&self) -> Vec<String> {
        vec!["mytool build".to_string(), "mytool test".to_string()]
    }
}
```

### Advanced Provider with Value Validation

```rust
impl OptionProvider for AdvancedProvider {
    fn validate(&self, command: &str, option: &str, value: Option<&str>) -> ValidationResult<()> {
        match (option, value) {
            ("--port", Some(port)) => {
                let port_num: u16 = port.parse()
                    .map_err(|_| ValidationError::invalid_value(option, port, "must be a number"))?;
                
                if port_num < 1024 {
                    return Err(ValidationError::invalid_value(
                        option, port, "port must be >= 1024 for non-root users"
                    ));
                }
                Ok(())
            }
            ("--level", Some(level)) => {
                if !["debug", "info", "warn", "error"].contains(&level) {
                    return Err(ValidationError::invalid_value(
                        option, level, "must be one of: debug, info, warn, error"
                    ));
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}
```

### Registering Custom Providers

```rust
let mut engine = ValidationEngine::new();
engine.register_provider(Box::new(MyToolProvider));

// Now can validate mytool options
engine.validate_option("mytool build", "--verbose", None)?;
```

## Validation Levels

### ValidationLevel::Off
- **Behavior**: No validation performed
- **Use Case**: Maximum compatibility, performance-critical paths
- **Returns**: Always `Ok(())`

```rust
let config = ValidationConfig::with_level(ValidationLevel::Off);
let engine = ValidationEngine::with_config(config);

// Always passes
engine.validate_option("any-command", "--any-option", Some("any-value"))?;
```

### ValidationLevel::Normal (Default)
- **Behavior**: Validate known options, allow unknown ones
- **Use Case**: Development, helpful feedback without blocking
- **Returns**: Errors for invalid values, warnings for unknown options

```rust
// Known option with invalid value -> Error
let result = engine.validate_option("build", "--jobs", Some("abc"));
assert!(result.is_err());

// Unknown option -> Warning (but still Ok)
let result = engine.validate_option("build", "--unknown-flag", None);
assert!(result.is_ok());
```

### ValidationLevel::Strict
- **Behavior**: Reject unknown options with suggestions
- **Use Case**: Production, CI/CD, strict environments
- **Returns**: Errors for any unrecognized options

```rust
let config = ValidationConfig::with_level(ValidationLevel::Strict);
let engine = ValidationEngine::with_config(config);

// Unknown option -> Error with suggestions
let result = engine.validate_option("build", "--relase", None);
assert!(result.is_err());
// Error message: "Unknown option '--relase' for command 'build'. Did you mean: --release"
```

## Error Handling

### Error Types

```rust
use raz_validation::ValidationError;

match engine.validate_option("build", "--invalid", Some("value")) {
    Err(ValidationError::UnknownOption { command, option, suggestions }) => {
        println!("Unknown option '{}' for command '{}'", option, command);
        if !suggestions.is_empty() {
            println!("Did you mean: {}", suggestions.join(", "));
        }
    }
    Err(ValidationError::InvalidValue { option, value, reason }) => {
        println!("Invalid value '{}' for option '{}': {}", value, option, reason);
    }
    Err(ValidationError::OptionConflict { option, conflicts_with }) => {
        println!("Option '{}' conflicts with '{}'", option, conflicts_with);
    }
    Ok(_) => println!("Valid option"),
}
```

### Formatted Error Messages

```rust
let error = ValidationError::unknown_option("build", "--relase", vec!["--release".to_string()]);
println!("{}", error.format_with_suggestions());
// Output: "Unknown option '--relase' for command 'build'\nDid you mean: --release"
```

## Integration Patterns

### With Override System

```rust
use raz_override::parser::override_parser::OverrideParser;
use raz_validation::{ValidationConfig, ValidationLevel};

// Create parser with validation
let config = ValidationConfig::with_level(ValidationLevel::Normal);
let parser = OverrideParser::with_validation_config("build", config);

// Parse and validate in one step
let parsed = parser.parse("--release --features serde,tokio")?;
parser.validate(&parsed)?;

// Get suggestions for errors
if let Err(e) = parser.validate(&parsed) {
    let suggestions = parser.suggest_option("--relase");
    println!("Error: {}. Suggestions: {}", e, suggestions.join(", "));
}
```

### With CLI Applications

```rust
use clap::Parser;
use raz_validation::{ValidationEngine, ValidationLevel};

#[derive(Parser)]
struct Args {
    #[clap(long)]
    command: String,
    
    #[clap(long)]
    options: Vec<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let engine = ValidationEngine::new();
    
    // Validate each option
    for option in &args.options {
        engine.validate_option(&args.command, option, None)?;
    }
    
    println!("All options are valid!");
    Ok(())
}
```

### Batch Validation

```rust
use std::collections::HashMap;

let options = HashMap::from([
    ("--release".to_string(), None),
    ("--features".to_string(), Some("serde,tokio".to_string())),
    ("--target".to_string(), Some("x86_64-unknown-linux-gnu".to_string())),
]);

// Validate all options at once
engine.validate_options("build", &options)?;
```

## Performance Considerations

### Lazy Loading
- Option definitions are loaded only when needed
- Providers are initialized on first use
- Suggestion matching is optimized for common cases

### Caching Strategy
```rust
// Engine reuse is recommended
let engine = ValidationEngine::new(); // Initialize once

// Reuse for multiple validations
for option in options {
    engine.validate_option("build", option, None)?;
}
```

### Memory Usage
- Minimal overhead in `Off` and `Minimal` modes
- Option definitions cached per provider
- Fuzzy matcher uses efficient algorithms

### Benchmarks
```bash
# Run benchmarks
cargo bench -p raz-validation

# Typical performance:
# - Option validation: ~1-5μs
# - Suggestion generation: ~10-50μs
# - Provider lookup: ~100ns
```

## API Reference

### ValidationEngine

```rust
impl ValidationEngine {
    pub fn new() -> Self;
    pub fn with_config(config: ValidationConfig) -> Self;
    
    pub fn validate_option(&self, command: &str, option: &str, value: Option<&str>) -> ValidationResult<()>;
    pub fn validate_options(&self, command: &str, options: &HashMap<String, Option<String>>) -> ValidationResult<()>;
    pub fn suggest_option(&self, command: &str, option: &str) -> Vec<String>;
    
    pub fn register_provider(&mut self, provider: Box<dyn OptionProvider>);
    pub fn get_options(&self, command: &str) -> Vec<OptionDef>;
}
```

### ValidationConfig

```rust
impl ValidationConfig {
    pub fn with_level(level: ValidationLevel) -> Self;
    pub fn add_provider(self, provider: impl Into<String>) -> Self;
    pub fn with_suggestion_threshold(self, threshold: i64) -> Self;
    
    pub fn is_provider_enabled(&self, provider: &str) -> bool;
}
```

### OptionProvider Trait

```rust
pub trait OptionProvider: Send + Sync {
    fn name(&self) -> &str;
    fn get_options(&self, command: &str) -> Vec<OptionDef>;
    fn validate(&self, command: &str, option: &str, value: Option<&str>) -> ValidationResult<()>;
    fn get_commands(&self) -> Vec<String>;
    fn supports_command(&self, command: &str) -> bool;
}
```

### OptionDef Builder

```rust
impl OptionDef {
    pub fn flag(name: impl Into<String>, description: impl Into<String>) -> Self;
    pub fn single(name: impl Into<String>, description: impl Into<String>, validator: ValueValidator) -> Self;
    pub fn multiple(name: impl Into<String>, description: impl Into<String>, validator: ValueValidator) -> Self;
    
    pub fn conflicts_with(self, options: Vec<String>) -> Self;
    pub fn requires(self, options: Vec<String>) -> Self;
    pub fn deprecated(self, message: impl Into<String>) -> Self;
    pub fn repeatable(self) -> Self;
}
```