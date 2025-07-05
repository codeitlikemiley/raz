# raz-cli

Universal command runner for Rust - Run any Rust file from anywhere with cursor-aware test targeting and override persistence.

## Installation

```bash
cargo install raz-cli
```

## Quick Start

```bash
# Run any Rust file
raz file.rs

# Run with cursor position for test targeting
raz file.rs:line:column

# Run with overrides
raz file.rs RUST_BACKTRACE=full --release -- --exact

# Save overrides for reuse
raz --save-override src/lib.rs:25:1 RUST_LOG=debug --quiet
```

## Features

- 🎯 **Stateless Operation**: Works from any directory - just provide a file path
- 📍 **Cursor-Aware**: Smart test detection based on cursor position  
- 💾 **Override Persistence**: Save command overrides per function automatically
- ⚡ **Zero Configuration**: No setup required - works instantly
- 🚀 **Universal Support**: Handles all Rust execution patterns seamlessly

## Usage Examples

### Basic Execution
```bash
# Run main function
raz src/main.rs

# Run specific test at line 25
raz src/lib.rs:25:1

# Run integration test
raz tests/integration.rs:40:5
```

### Smart Override Parsing
```bash
# Environment variables (automatically detected)
raz file.rs RUST_BACKTRACE=1 RUST_LOG=debug

# Command options (passed to cargo/framework tools)
raz file.rs --release --features ssr

# Test arguments (passed after -- to test runner)
raz file.rs:25:1 -- --exact --nocapture

# Combined example
raz src/lib.rs:45:1 RUST_BACKTRACE=1 --release --platform web -- --test-threads 1
```

### Override Persistence
```bash
# Save command configuration for a specific test function
raz --save-override src/lib.rs:test_fn RUST_BACKTRACE=1 --exact

# Future runs automatically apply saved settings
raz src/lib.rs:test_fn  # Uses saved RUST_BACKTRACE=1 --exact
```

## Supported Patterns

- **Cargo Workspaces**: Multi-package projects with proper member detection
- **Cargo Packages**: Single package projects with bin/lib/test detection  
- **Cargo Scripts**: Files with `#!/usr/bin/env -S cargo +nightly -Zscript`
- **Single Files**: Standalone Rust files compiled with rustc
- **Build Scripts**: Special handling for build.rs files

## Override Management

RAZ provides comprehensive override management with deferred save - overrides are only saved after successful execution.

### Basic Commands
```bash
# Save override (only saved after success)
raz --save-override src/lib.rs:test_fn RUST_BACKTRACE=1 --exact

# List and inspect
raz override list
raz override list --file src/main.rs
raz override inspect "src/main.rs:handle_request"
raz override stats

# Backup and recovery
raz override rollback
raz override export --output backup.toml
raz override import backup.toml

# Debug and management
raz override debug src/lib.rs 25 10
raz override delete src/lib.rs:test_fn
raz override clear [--force]

# Migration from legacy format
raz override migrate --auto
raz override migrate --file legacy.toml --dry-run
```

## Template Commands

```bash
# Initialize project with template
raz init --template web      # Leptos, Dioxus projects
raz init --template game     # Bevy projects  
raz init --template library  # Library development
raz init --template desktop  # Tauri, Egui projects

# Template management (planned features)
raz template list
raz template show my-framework
raz template validate .raz/templates/my-framework.toml
raz template init my-framework
raz template export --output my-project-template.toml
```

## Framework Integration

RAZ automatically detects and provides framework-specific commands:

```bash
# Leptos projects
raz src/app.rs           # Runs leptos serve

# Tauri projects  
raz src-tauri/src/main.rs    # Runs tauri dev

# Bevy projects
raz src/main.rs          # Runs with proper game features
```

## Documentation

For complete documentation, examples, and VS Code integration, visit the [main RAZ repository](https://github.com/codeitlikemiley/raz).

## Library Usage

For embedding RAZ functionality in your own applications, see the `raz-core` crate.