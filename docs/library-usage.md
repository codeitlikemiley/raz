# Library Usage

RAZ Core can be embedded in your own applications for custom Rust command generation.

## Installation

Add RAZ Core to your `Cargo.toml`:

```toml
[dependencies]
raz-core = "0.1.2"
raz-override = "0.1.2"
raz-config = "0.1.2"
```

## Basic Usage

```rust
use raz_core::{RazCore, Position};
use raz_override::{SmartOverrideParser, OverrideSystem};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let raz = RazCore::new()?;
    let file_path = Path::new("src/lib.rs");
    let cursor = Some(Position { line: 24, column: 0 }); // 0-based
    
    // Generate commands with universal detection and automatic override loading
    let mut commands = raz.generate_universal_commands(file_path, cursor).await?;
    
    // Parse runtime overrides
    let parser = SmartOverrideParser::new("test");
    let overrides = parser.parse("RUST_BACKTRACE=full --release -- --exact");
    
    // Apply runtime overrides to the best command
    if let Some(cmd) = commands.first_mut() {
        // Apply environment variables
        for (key, value) in &overrides.env_vars {
            cmd.env.insert(key.clone(), value.clone());
        }
        
        // Apply options and args
        cmd.args.extend(overrides.options.clone());
        if !overrides.args.is_empty() {
            cmd.args.push("--".to_string());
            cmd.args.extend(overrides.args.clone());
        }
        
        println!("Running: {}", cmd.label);
        
        // Example: Implement deferred save pattern
        let workspace = Path::new(".");
        let mut override_system = OverrideSystem::new(workspace)?;
        
        // Execute command here - only save override if successful
        let success = execute_command(&cmd).await?;
        
        if success && should_save_override() {
            // Save override only after successful execution
            let function_context = override_system.get_function_context(file_path, 24, Some(0))?;
            let override_key = override_system.generate_key(&function_context)?;
            
            let mut command_override = raz_config::CommandOverride::new(cmd.command.clone());
            for (k, v) in &overrides.env_vars {
                command_override.env.insert(k.clone(), v.clone());
            }
            command_override.cargo_options.extend(overrides.options.clone());
            command_override.args.extend(overrides.args.clone());
            
            override_system.save_override_with_validation(
                override_key,
                command_override,
                &function_context,
                &cmd.command
            )?;
            
            println!("✓ Override saved after successful execution");
        }
    }
    
    Ok(())
}

async fn execute_command(cmd: &raz_core::Command) -> Result<bool, Box<dyn std::error::Error>> {
    // Your command execution logic here
    // Return true if command succeeded (exit code 0), false otherwise
    Ok(true)
}

fn should_save_override() -> bool {
    // Your logic to determine if override should be saved
    true
}
```

## Advanced Features

### File Detection

```rust
use raz_core::{FileDetector, FileExecutionContext};

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

### Override Management with Deferred Save

```rust
use raz_override::{OverrideSystem, FunctionContext};
use raz_config::CommandOverride;
use std::path::Path;

// Create override system for workspace
let mut override_system = OverrideSystem::new(&workspace_path)?;

// Create function context from file position
let function_context = override_system.get_function_context(
    Path::new("src/lib.rs"), 
    24, // 0-based line number
    Some(10) // optional column
)?;

// Generate stable key
let override_key = override_system.generate_key(&function_context)?;

// Create command override
let mut command_override = CommandOverride::new("test".to_string());
command_override.env.insert("RUST_BACKTRACE".to_string(), "1".to_string());
command_override.cargo_options.push("--exact".to_string());

// Save with validation (deferred save pattern)
override_system.save_override_with_validation(
    override_key.clone(),
    command_override,
    &function_context,
    "test" // command for validation
)?;

// Load saved overrides automatically
if let Some(saved_override) = override_system.resolve_override(&function_context)? {
    // Apply the override to your command
    println!("Found saved override with {} env vars", saved_override.env.len());
}
```

## API Reference

### Core Types

- `RazCore`: Main entry point for command generation
- `Position`: Cursor position (line, column) - 0-based indexing
- `FileExecutionContext`: Analysis result of a Rust file
- `ExecutableCommand`: Generated command with environment, args, and metadata

### Key Methods

#### Core Command Generation
- `generate_universal_commands()`: Main command generation method with automatic override loading
- `FileDetector::detect_context()`: Analyze file type and structure

#### Override System  
- `SmartOverrideParser::parse()`: Parse override strings into structured format
- `OverrideSystem::new()`: Create override system for workspace
- `OverrideSystem::get_function_context()`: Get function context from file position
- `OverrideSystem::generate_key()`: Generate stable keys for overrides
- `OverrideSystem::save_override_with_validation()`: Save override with validation (deferred save)
- `OverrideSystem::resolve_override()`: Load saved override for context

#### Validation Integration
- `raz_validation::validate_command()`: Validate command options
- `raz_validation::suggest_corrections()`: Get suggestions for invalid options

## Error Handling

RAZ uses standard Rust error handling patterns:

```rust
use raz_core::RazError;

match raz.generate_universal_commands(file_path, cursor).await {
    Ok(commands) => {
        // Handle success
    }
    Err(RazError::FileNotFound(path)) => {
        eprintln!("File not found: {}", path);
    }
    Err(RazError::InvalidContext(msg)) => {
        eprintln!("Invalid context: {}", msg);
    }
    Err(e) => {
        eprintln!("Error: {}", e);
    }
}
```