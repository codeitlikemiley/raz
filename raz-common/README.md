# raz-common

Common utilities and shared types for the RAZ project ecosystem.

## Features

- **Environment Management**: Parse and build environment variables
- **Error Handling**: Common error types with context support
- **Output Formatting**: Consistent output formatting across crates
- **Shell Commands**: Safe shell command execution utilities
- **Time Utilities**: Elapsed time tracking and formatting
- **Cargo Flags**: Common cargo flag definitions and parsing

## Usage

```rust
use raz_common::{EnvBuilder, ShellCommand, Result};

// Build environment variables
let env = EnvBuilder::new()
    .add("RUST_BACKTRACE", "1")
    .add("RUST_LOG", "debug")
    .build();

// Execute shell commands safely
let output = ShellCommand::new("cargo")
    .arg("test")
    .envs(&env)
    .execute()?;

// Use common error handling
use raz_common::{CommonError, ErrorContext};

fn process_file(path: &str) -> Result<()> {
    std::fs::read_to_string(path)
        .context("Failed to read file")?;
    Ok(())
}
```

## Components

### Environment Management
```rust
use raz_common::{EnvParser, EnvBuilder};

// Parse environment strings
let vars = EnvParser::parse("RUST_BACKTRACE=1 RUST_LOG=debug")?;

// Build environment programmatically
let env = EnvBuilder::new()
    .add("KEY", "value")
    .build();
```

### Error Handling
```rust
use raz_common::{CommonError, ErrorContext};

// Add context to errors
let result = std::fs::read("file.txt")
    .context("Failed to read configuration")?;

// Create custom errors
let err = CommonError::InvalidInput("Invalid format".to_string());
```

### Output Formatting
```rust
use raz_common::OutputFormatter;

let formatter = OutputFormatter::new();
formatter.success("Operation completed");
formatter.warning("This might cause issues");
formatter.error("Operation failed");
```

### Time Utilities
```rust
use raz_common::{Elapsed, TimeUtils};

let elapsed = Elapsed::start();
// ... do work ...
println!("Operation took: {}", elapsed.format());
```

## Integration

This crate is used throughout the RAZ ecosystem:
- `raz-core` uses it for error handling and environment management
- `raz-cli` uses it for output formatting and shell commands
- `raz-validation` uses it for error contexts
- `raz-override` uses it for time tracking

## License

MIT