# raz-core

Core library for RAZ (Rust Action Zapper) - Universal command generator for Rust projects.


## Features

- **Universal File Detection**: Analyze any Rust file to determine execution context
- **Stateless Operation**: Works from any directory without workspace context
- **Tree-sitter Powered**: AST-driven test and function detection
- **Smart Test Detection**: Cursor-aware test discovery with module hierarchy
- **Framework Providers**: Specialized support for multiple frameworks
- **Override Integration**: Automatic loading of saved command configurations
- **Validation Support**: Smart command-line option validation

## Quick Start

```rust
use raz_core::{RazCore, Position};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let raz = RazCore::new()?;
    let file_path = Path::new("src/lib.rs");
    let cursor = Some(Position { line: 24, column: 0 }); // 0-based
    
    // Generate commands with universal detection
    let commands = raz.generate_universal_commands(file_path, cursor).await?;
    
    for command in commands {
        println!("Command: {}", command.label);
        println!("Executable: {}", command.command);
        println!("Args: {:?}", command.args);
    }
    
    Ok(())
}
```

## Advanced Usage

### File Detection

```rust
use raz_core::{FileDetector, FileExecutionContext, FileType};

let context = FileDetector::detect_context(&file_path, cursor)?;
match context.file_type {
    FileType::CargoPackage => {
        // Handle cargo package
    }
    FileType::SingleFile => {
        // Handle standalone file
    }
    // ... other types
}
```

### Framework Providers

```rust
use raz_core::providers::{ProviderRegistry, CargoProvider, DioxusProvider};

let mut registry = ProviderRegistry::new();
registry.register(Box::new(CargoProvider::new()));
registry.register(Box::new(DioxusProvider::new()));

let commands = registry.generate_commands(&context, cursor)?;
```

### Complete Integration Example

This example shows how raz-core integrates with other RAZ crates:

```rust
use raz_core::{RazCore, Position};
use raz_override::{OverrideSystem, SmartOverrideParser};
use raz_validation::{ValidationEngine, ValidationLevel};
use raz_config::{GlobalConfig, EffectiveConfig};
use std::path::Path;

#[tokio::main] 
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = GlobalConfig::load()?;
    
    // Initialize core with config
    let raz = RazCore::new()?;
    
    // Generate commands
    let file_path = Path::new("src/lib.rs");
    let cursor = Some(Position { line: 24, column: 0 });
    let commands = raz.generate_universal_commands(file_path, cursor).await?;
    
    // Get the first command
    if let Some(mut cmd) = commands.into_iter().next() {
        // Apply saved overrides
        let override_system = OverrideSystem::new(Path::new("."))?;
        let context = override_system.get_function_context(file_path, 24, Some(0))?;
        
        if let Some(saved_override) = override_system.resolve_override(&context)? {
            cmd.apply_override(&saved_override);
        }
        
        // Validate options
        let validator = ValidationEngine::new();
        for arg in &cmd.args {
            if let Err(e) = validator.validate_option(&cmd.command, arg, None) {
                eprintln!("Warning: {}", e);
            }
        }
        
        println!("Executing: {} {}", cmd.command, cmd.args.join(" "));
    }
    
    Ok(())
}
```

### Crate Dependencies

- **[raz-common](https://crates.io/crates/raz-common)**: Common utilities and error handling
- **[raz-config](https://crates.io/crates/raz-config)**: Configuration management
- **[raz-validation](https://crates.io/crates/raz-validation)**: Command-line option validation  
- **[raz-override](https://crates.io/crates/raz-override)**: Override persistence and management

## API Reference

### Core Types

- `RazCore`: Main entry point for command generation
- `Position`: Cursor position (line, column) - 0-based indexing
- `FileExecutionContext`: Analysis result of a Rust file
- `ExecutableCommand`: Generated command with environment, args, and metadata

### Key Methods

- `generate_universal_commands()`: Main command generation with automatic override loading
- `FileDetector::detect_context()`: Analyze file type and structure
- `SmartOverrideParser::parse()`: Parse override strings
- `OverrideSystem::resolve_override()`: Load saved overrides

## Supported Execution Patterns

RAZ can detect and execute any Rust file pattern:

| Pattern | Example | Command Generated |
|---------|---------|-------------------|
| **Main Binary** | `src/main.rs` | `cargo run --bin project` |
| **Library** | `src/lib.rs` | `cargo test --lib` |
| **Integration Test** | `tests/common.rs` | `cargo test --test common` |
| **Unit Test** | `src/lib.rs:25:1` | `cargo test -- tests::my_test --exact` |
| **Example** | `examples/demo.rs` | `cargo run --example demo` |
| **Benchmark** | `benches/perf.rs` | `cargo bench --bench perf` |
| **Build Script** | `build.rs` | Direct execution |
| **Cargo Script** | `script.rs` | `cargo +nightly -Zscript script.rs` |
| **Standalone** | `/tmp/hello.rs` | `rustc hello.rs && ./hello` |
| **Standalone Test** | `/tmp/test.rs:10:1` | `rustc --test test.rs && ./test my_test` |

### Test Macro Support

RAZ recognizes all common test macros:
- `#[test]` - Standard test
- `#[tokio::test]` - Async runtime tests
- `#[async_std::test]` - Alternative async tests
- `#[test_case(...)]` - Parameterized tests
- `#[bench]` - Benchmarks

### Universal Detection

RAZ works with:
- **Cargo Workspaces**: Multi-package projects with proper member detection
- **Cargo Packages**: Single package projects with bin/lib/test detection
- **Cargo Scripts**: Files with `#!/usr/bin/env -S cargo +nightly -Zscript`
- **Single Files**: Standalone Rust files compiled with rustc
- **Any Directory**: No need to be in project root - works from anywhere
