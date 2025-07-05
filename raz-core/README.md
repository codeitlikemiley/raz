# raz-core

Core library for RAZ (Rust Action Zapper) - Universal command generator for Rust projects.

## Features

- **Universal File Detection**: Analyze any Rust file to determine execution context
- **Stateless Operation**: Works from any directory without workspace context
- **Tree-sitter Powered**: AST-driven test, function, and doctest detection with rust-analyzer precision
- **Advanced Doctest Support**: Accurate detection with hidden lines, multi-block support, and precise association
- **Smart Test Detection**: Cursor-aware test discovery with module hierarchy support
- **Framework Providers**: Specialized support for Leptos, Dioxus, Tauri, Yew, and Bevy
- **Override System**: Smart parsing and persistence of command overrides
- **Configuration Management**: Function-level override persistence

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

## Documentation

For complete documentation and usage examples, visit the [main RAZ repository](https://github.com/codeitlikemiley/raz).

## CLI Tool

For a ready-to-use command-line interface, install the `raz-cli` crate:

```bash
cargo install raz-cli
```