# RAZ Documentation

Welcome to the comprehensive RAZ documentation! This directory contains detailed guides for all aspects of using and extending RAZ.

## Getting Started

- **[Installation & Quick Start](../README.md#installation)** - Get RAZ up and running quickly
- **[Basic Usage](../README.md#quick-start)** - Learn the fundamental commands and patterns

## Detailed Guides

### Core Features
- **[Advanced Usage](advanced-usage.md)** - Override parsing, VS Code extension, cursor-aware execution
- **[Override Management](override-management.md)** - Complete guide to the deferred save system and CLI commands
- **[Validation Guide](validation-guide.md)** - Smart options validation and framework support
- **[Supported Patterns](supported-patterns.md)** - Complete reference of all Rust execution patterns RAZ supports

### IDE Integration
- **[Vim/Neovim Integration](vim-integration.md)** - Complete setup guide with utility functions and keymaps

### For Developers  
- **[Library Usage](library-usage.md)** - Embed RAZ Core in your own applications
- **[Release Guide](release-guide.md)** - Complete guide for releasing new versions of RAZ
- **[Contributing Guide](../CONTRIBUTING.md)** - How to contribute to RAZ development

## Key Concepts

### Universal Execution
RAZ can run any Rust file from any directory without requiring workspace context. This means you can:
- Run files from `/tmp/` or any random location
- Execute tests without being in the project root
- Work with multiple projects simultaneously

### Override Persistence with Deferred Save
Save command configurations per function with automatic validation - only working overrides are saved:
```bash
# Save once (only saves if command succeeds)
raz --save-override src/lib.rs:25:1 RUST_BACKTRACE=1 --exact

# Use forever  
raz src/lib.rs:25:1  # Automatically applies saved settings

# Manage saved overrides
raz override list              # View all saved overrides  
raz override rollback          # Restore from backup
```

### Smart Context Detection
RAZ analyzes your Rust files to determine:
- File type (binary, library, test, example, etc.)
- Module structure for accurate test targeting
- Framework detection (Leptos, Dioxus, Tauri, etc.)
- Entry points (main functions, tests, benchmarks)

### Intelligent Validation
RAZ provides smart validation for command-line options:
- Context-aware validation (knows which options work with which commands)
- Framework-specific option support (Cargo, Leptos, Dioxus)
- Helpful error messages with suggestions ("Did you mean --release?")
- Configurable validation levels from permissive to strict

## Architecture Overview

RAZ consists of several main components:

1. **`raz-core`** - Core library with universal command generation logic
2. **`raz-validation`** - Smart options validation system with framework support
3. **`raz-override`** - Override management and persistence system
4. **`raz-config`** - Configuration management for the entire system
5. **`raz-adapters/cli`** - Command-line interface with override persistence  
6. **`raz-adapters/vscode/`** - VS Code extension with IDE integration

Each component can be used independently or together for a complete development experience.

## Quick Reference

### Common Commands
```bash
raz file.rs                     # Run file
raz file.rs:line:col           # Run with cursor position  
raz --save-override file.rs    # Save command configuration
raz --dry-run file.rs          # Preview command

# Override management
raz override list               # List all saved overrides
raz override stats              # Show override statistics  
raz override rollback           # Rollback to last backup
raz override clear              # Clear all overrides
```

### Override Syntax
```bash
# Environment variables
raz file.rs RUST_BACKTRACE=1 RUST_LOG=debug

# Command options  
raz file.rs --release --features ssr

# Test arguments
raz file.rs:25:1 -- --exact --nocapture

# Combined
raz file.rs:25:1 RUST_BACKTRACE=1 --release -- --exact
```

## Need Help?

- Check the specific guide for your use case
- Look at the examples in each documentation file
- Visit the [main repository](https://github.com/codeitlikemiley/raz) for issues and discussions
- See the [Contributing Guide](../CONTRIBUTING.md) if you want to help improve RAZ