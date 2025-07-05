# RAZ - Rust Action Zapper

<div align="center">

![RAZ Cover](raz-github-cover.svg)

[![CI](https://github.com/codeitlikemiley/raz/actions/workflows/ci.yml/badge.svg)](https://github.com/codeitlikemiley/raz/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/codeitlikemiley/raz/branch/main/graph/badge.svg)](https://codecov.io/gh/codeitlikemiley/raz)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.88.0-orange.svg)](rust-toolchain.toml)

**Universal, stateless command generator for Rust - Run any Rust file from anywhere**

[![crates.io](https://img.shields.io/crates/v/raz-cli.svg)](https://crates.io/crates/raz-cli)
[![Downloads](https://img.shields.io/crates/d/raz-cli.svg)](https://crates.io/crates/raz-cli)

[Installation](#installation) • [Quick Start](#quick-start) • [Documentation](docs/) • [Contributing](CONTRIBUTING.md) • [Maintaining](MAINTAINING.md)

</div>

## Why RAZ?

Ever found yourself repeatedly typing the same complex cargo commands? Frustrated by having to `cd` into project directories just to run a single test? Tired of losing your carefully crafted test flags when switching between functions? Worried about saving bad command configurations that break your workflow?

RAZ solves these real developer pain points with revolutionary deferred save technology:

**🎯 The Problem**: You're debugging a specific test with `RUST_BACKTRACE=1 --exact --nocapture`, but every time you run it, you have to remember and retype those flags.

**✨ RAZ Solution**: `raz --save-override src/lib.rs:25:1 RUST_BACKTRACE=1 --exact --nocapture` - saves it once, uses it forever, **but only if the command succeeds**.

**🎯 The Problem**: You accidentally save a bad override with invalid flags, and now every `Cmd+R` in VS Code fails with confusing errors.

**✨ RAZ Solution**: **Deferred save** - overrides are only saved AFTER successful execution. Bad commands never persist, keeping your workflow clean.

**🎯 The Problem**: You have a script in `/tmp/` you want to test, but you're deep in a project directory and don't want to lose your place.

**✨ RAZ Solution**: `raz /tmp/script.rs:10:1` - runs from anywhere, finds the right test automatically.

**🎯 The Problem**: You saved some overrides weeks ago but can't remember what they were, and now you want to clean them up.

**✨ RAZ Solution**: Comprehensive override management - `raz override list`, `raz override rollback`, `raz override stats` - full control over your saved configurations.

**🎯 The Problem**: You work in multiple IDEs but have to set up different run configurations in each one.

**✨ RAZ Solution**: **Universal override persistence** - save once in VS Code, use everywhere! Your `RUST_BACKTRACE=1 --exact` override works in VS Code, Vim, IntelliJ, terminal, or any future IDE integration.

## What is RAZ?

RAZ is the first universal command runner for Rust that can execute any Rust file from any directory without requiring workspace context. It provides intelligent, cursor-aware command generation with revolutionary **deferred save technology** that ensures only working configurations are preserved:

- 🎯 **Stateless Operation**: Works from any directory - just provide a file path
- 📍 **Cursor-Aware**: Smart test detection based on cursor position using tree-sitter
- 💾 **Deferred Save Overrides**: Save command overrides per function - **only after successful execution**
- 🛡️ **Automatic Rollback**: Failed overrides don't persist, with comprehensive backup/rollback system
- 🔧 **IDE Integration**: Full integration with VS Code/Codium with Cmd+R/Cmd+Shift+R support
- ✅ **Smart Validation**: Framework-aware option validation with helpful error messages and suggestions
- 📊 **Override Management**: Complete CLI for managing, inspecting, and cleaning up saved configurations
- 🏗️ **Framework Detection**: Specialized support for Leptos, Dioxus, Tauri, Bevy, Yew, and more
- ⚡ **Zero Configuration**: No setup required - works instantly with any Rust project

Instead of remembering complex cargo commands and project structures, RAZ analyzes your code and runs exactly what you need.

## Core Features

### 🔐 **Deferred Save Technology**
- **Safe Override Persistence**: Overrides only saved after successful command execution
- **No Bad Configurations**: Failed commands never create persistent overrides
- **Automatic Validation**: Built-in validation prevents invalid option combinations

### 🛡️ **Comprehensive Backup & Rollback**
- **Automatic Backups**: Every override change creates a timestamped backup
- **One-Command Rollback**: `raz override rollback` restores to last working state  
- **Backup Management**: `raz override list-backups` shows all recovery points

### 📊 **Advanced Override Management**
- **Complete CLI**: `list`, `stats`, `rollback`, `clear`, `inspect`, `debug` commands
- **Function-Level Precision**: Override specific tests/functions, not just files
- **Smart Key Generation**: Automatic fallback from function to line-based keys
- **Rich Metadata**: Track creation time, execution history, failure counts

### 🎯 **Universal Execution Engine**
- **Truly Stateless**: Run any Rust file from any directory without `cd`
- **Smart Context Detection**: Automatically detects workspace, package, or standalone files
- **Framework Intelligence**: Specialized support for Leptos, Dioxus, Tauri, Bevy, Yew
- **Tree-sitter Powered**: AST-driven test, function, and doctest detection with rust-analyzer precision
- **Advanced Doctest Support**: Accurate detection of doc comments with hidden lines, multi-block support

### 🔧 **Seamless IDE Integration**
- **VS Code Extension**: Cmd+R to run, Cmd+Shift+R to save overrides
- **Real-time Feedback**: Clear success/failure indicators in IDE
- **Task System Integration**: Non-blocking execution for long-running commands
- **Cross-platform**: Windows, macOS, and Linux support
- **🔥 Universal Override Persistence**: Saved overrides work across ALL IDEs! Save in VS Code, use in Vim, IntelliJ, or terminal
- **🚀 Coming Soon**: JetBrains IDEs (IntelliJ IDEA, RustRover), Zed, and other popular editors
- **📝 Vim/Neovim Ready**: Full integration guide available - see [Vim Integration](docs/vim-integration.md)
- **🤝 Community Driven**: We're actively accepting PRs for other IDE integrations

### ✅ **Intelligent Validation System**
- **Framework-Aware**: Knows which options work with which commands
- **Smart Suggestions**: "Did you mean --release?" for typos
- **Context Validation**: Different validation rules for cargo run vs test vs bench
- **Error Prevention**: Catches invalid combinations before execution

## Project Structure

RAZ is organized as a workspace with multiple crates:

| Crate | Description | Version |
|-------|-------------|---------|
| **[`raz-core`](raz-core/)** | Core library for command generation | [![crates.io](https://img.shields.io/crates/v/raz-core.svg)](https://crates.io/crates/raz-core) |
| **[`raz-cli`](raz-adapters/cli/)** | Command-line interface | [![crates.io](https://img.shields.io/crates/v/raz-cli.svg)](https://crates.io/crates/raz-cli) |
| **[`raz-validation`](raz-validation/)** | Smart options validation system | [![crates.io](https://img.shields.io/crates/v/raz-validation.svg)](https://crates.io/crates/raz-validation) |
| **[`raz-override`](raz-override/)** | Override management system | [![crates.io](https://img.shields.io/crates/v/raz-override.svg)](https://crates.io/crates/raz-override) |
| **[`raz-config`](raz-config/)** | Configuration management | [![crates.io](https://img.shields.io/crates/v/raz-config.svg)](https://crates.io/crates/raz-config) |
| **[`raz-common`](raz-common/)** | Common utilities and shared types | [![crates.io](https://img.shields.io/crates/v/raz-common.svg)](https://crates.io/crates/raz-common) |
| **[`vscode`](raz-adapters/vscode/)** | VS Code extension | [Install from VSIX](raz-adapters/vscode/README.md) |

## Supported Editors & IDEs

RAZ works universally with any editor through the CLI, and provides enhanced integrations for popular editors:

| Editor/IDE | Status | Integration Type | Installation |
|------------|---------|------------------|--------------|
| **VS Code** | ✅ **Full Integration** | Native Extension | [Download VSIX](raz-adapters/vscode/README.md) |
| **Vim/Neovim** | ✅ **Ready** | Utility Functions | [Setup Guide](docs/vim-integration.md) |
| **Terminal/CLI** | ✅ **Native** | Direct CLI Usage | `cargo install raz-cli` |
| **JetBrains IDEs** | 🚧 **Coming Soon** | Plugin Planned | IntelliJ IDEA, RustRover, CLion |
| **Zed** | 🚧 **Coming Soon** | Extension Planned | Modern performance editor |
| **Emacs** | 📝 **Community Welcome** | Elisp Functions | PRs welcome! |
| **Sublime Text** | 📝 **Community Welcome** | Plugin/Commands | PRs welcome! |
| **Helix** | 📝 **Community Welcome** | Custom Commands | PRs welcome! |

### Universal Override Persistence

All editors share the same saved overrides! Configure once in any editor, use everywhere:
- Save `RUST_BACKTRACE=1 --exact` in VS Code
- Automatically available in Vim, terminal, and any future IDE integrations
- Stored in `.raz/overrides.toml` with function-level precision

## Demo

```bash
# Run any Rust file from anywhere - no cd required!
raz /path/to/any/file.rs

# Cursor-aware test execution
raz src/lib.rs:25:1          # Runs the test at line 25
raz tests/integration.rs:45:5 # Runs integration test at line 45

# Save overrides with deferred save (only saves after successful execution)
$ raz --save-override src/lib.rs:25:1 RUST_BACKTRACE=1 --exact
✓ Success Override saved after successful execution
$ raz src/lib.rs:25:1  # Uses RUST_BACKTRACE=1 --exact automatically

# Override management commands
raz override list            # List all saved overrides
raz override rollback        # Rollback to last backup
raz override stats           # Show override statistics
```

## Installation

### CLI Tool
```bash
# Install from crates.io
cargo install raz-cli

# Or build from source
git clone https://github.com/codeitlikemiley/raz.git
cd raz
cargo install raz-cli
```

### VS Code Extension
```bash
# Install from VSIX file (see releases)
# Download latest .vsix from GitHub releases and install:
# code --install-extension raz-*.vsix

# Or build from source
git clone https://github.com/codeitlikemiley/raz.git
cd raz/raz-adapters/vscode
npm install
npm run package
code --install-extension raz-*.vsix
```

### Vim/Neovim Integration
```bash
# Install RAZ CLI first
cargo install raz-cli

# Add integration to your config - see full guide:
# https://github.com/codeitlikemiley/raz/blob/main/docs/vim-integration.md
```

## Quick Start

```bash
# Test CLI
raz --help

# Test with a simple file
echo 'fn main() { println!("Hello RAZ!"); }' > test.rs
raz test.rs

# Run with overrides
raz file.rs RUST_BACKTRACE=full --release -- --exact

# Save overrides with deferred save (only saves after successful execution)
raz --save-override src/lib.rs:25:1 RUST_LOG=debug --quiet
# ✓ Success Override saved after successful execution

# Failed commands don't save overrides (prevents persistence of bad options)
raz --save-override test.rs --invalid-flag
# ⚠ Warning Command failed. Override was NOT saved.

# Get helpful suggestions for typos
raz build --relase
# Error: Unknown option '--relase' for command 'build'
# Did you mean: --release

# Manage saved overrides
raz override list               # Show all saved overrides
raz override rollback           # Rollback to last backup
raz override stats              # Show execution statistics
```

**VS Code Usage**:
- **Cmd+R** (Mac) / **Ctrl+R** (Windows/Linux): Run current file
- **Cmd+Shift+R** / **Ctrl+Shift+R**: Run with custom overrides

**🔥 Pro Tip**: Your saved overrides are universal! Set up `RUST_BACKTRACE=1 --exact` in VS Code, then use the same settings in terminal, Vim, or any other IDE. One configuration, everywhere!

## 🌳 Tree-sitter Powered Detection

RAZ uses tree-sitter AST parsing for rust-analyzer level accuracy in code analysis:

### **Advanced Doctest Support**
```rust
/// Complex doctest example that works perfectly with RAZ
/// 
/// ```
/// use mylib::calculate;
/// # fn setup() { /* hidden setup */ }
/// # let config = Config::default();
/// assert_eq!(calculate(5, 3), 8);
/// ```
/// 
/// Multiple code blocks are supported:
/// ```
/// # use mylib::*;
/// let result = calculate(10, 20);
/// assert_eq!(result, 30);
/// ```
pub fn calculate(a: i32, b: i32) -> i32 { a + b }
```

**Key Improvements:**
- ✅ **Accurate Function Association**: Uses AST to link doctests to correct functions/structs
- ✅ **Hidden Line Support**: Properly handles `# ` hidden lines in doctests  
- ✅ **Multi-block Detection**: Supports complex doctests with multiple code examples
- ✅ **Context Awareness**: Avoids false positives in test modules
- ✅ **Cursor Position Accuracy**: Works anywhere within doctest comment ranges
- ✅ **No More Line Arithmetic**: Eliminated flawed line-based guessing

### **Precise Test Detection**
- **AST-based Analysis**: Uses tree-sitter to understand code structure
- **Module Hierarchy**: Correctly handles nested test modules
- **Function Boundaries**: Knows exactly where functions start/end
- **Cursor Context**: Understands what code the cursor is actually in

This provides the same level of accuracy you'd expect from rust-analyzer or other professional IDE tools.

## Documentation

📚 **[Complete Documentation](docs/)** - Detailed guides for all RAZ features

Key guides:
- **[Advanced Usage](docs/advanced-usage.md)** - Override system, deferred save, backup/rollback
- **[Validation Guide](docs/validation-guide.md)** - Smart options validation and framework support
- **[Supported Patterns](docs/supported-patterns.md)** - Complete reference of execution patterns  
- **[Library Usage](docs/library-usage.md)** - Embed RAZ in your applications
- **[Override Management Guide](docs/override-management.md)** - Complete guide to override commands
- **[Release Guide](docs/release-guide.md)** - Complete guide for releasing new versions

## Comparison

| Tool | Scope | Override Persistence | Universal Execution | Deferred Save | Cross-IDE Support |
|------|-------|---------------------|-------------------|---------------|------------------|
| **RAZ** | ✅ Any Rust file | ✅ Function-level saves | ✅ Any directory | ✅ Only saves on success | ✅ **Works across ALL IDEs** |
| **rust-analyzer CodeLens** | Workspace only | ❌ No persistence | ❌ Requires workspace | ❌ N/A | ⚠️ IDE-specific |
| **cargo test** | Package/workspace | ❌ Manual flags | ❌ Requires project root | ❌ N/A | ❌ Terminal only |
| **VS Code tasks** | Project-specific | ⚠️ Manual config | ❌ Project dependent | ❌ N/A | ❌ VS Code only |

## 🐛 Issues and Support

### Reporting Issues

We have specific issue templates to help you provide the right information:

| Template | When to Use | Quick Link |
|----------|------------|------------|
| 🐛 **Bug Report** | General bugs and unexpected behavior | [Report Bug](https://github.com/raz-rs/raz/issues/new?template=bug_report.md) |
| ⚡ **Override Issue** | Problems with saving/loading overrides | [Override Issue](https://github.com/raz-rs/raz/issues/new?template=override_issue.md) |
| 🔍 **Command Detection** | Wrong commands or project detection | [Detection Issue](https://github.com/raz-rs/raz/issues/new?template=command_detection.md) |
| 🆚 **VS Code Extension** | Extension-specific problems | [VS Code Issue](https://github.com/raz-rs/raz/issues/new?template=vscode_extension.md) |
| 🐌 **Performance** | Slow performance or resource usage | [Performance Issue](https://github.com/raz-rs/raz/issues/new?template=performance.md) |
| ✨ **Feature Request** | New features or enhancements | [Request Feature](https://github.com/raz-rs/raz/issues/new?template=feature_request.md) |

### Essential Information

When reporting issues, always include:

1. **Complete terminal output** - Copy the entire output including the command being executed:
   ```
   *  Executing task: /Users/username/.cargo/bin/raz "/path/to/file.rs:line:column" 
   
   [Complete output including any errors and the actual command executed]
   ```

2. **Environment details** - OS, VS Code version, Rust version
3. **Exact reproduction steps** - We need to be able to reproduce the issue
4. **Project context** - Cargo.toml, file structure, cursor position

### Quick Troubleshooting

Before reporting an issue, try these steps:

```bash
# Check version
raz --version

# Test with dry run
raz "/path/to/file.rs:line:column" --dry-run

# Enable debug logging
RUST_LOG=debug raz "/path/to/file.rs:line:column"

# Check saved overrides
raz override list
```

📋 **[Full Troubleshooting Guide](.github/TROUBLESHOOTING.md)** - Complete debugging steps

### Getting Help

- 📖 **[Documentation](docs/)** - Detailed usage guides
- 💬 **[Discussions](https://github.com/raz-rs/raz/discussions)** - Ask questions and share ideas
- 🐛 **[Issues](https://github.com/raz-rs/raz/issues)** - Report bugs using templates above
- 🤝 **[Contributing](.github/CONTRIBUTING.md)** - Help improve RAZ

## Maintaining

For maintainers and contributors, see our **[Maintenance Guide](MAINTAINING.md)** which covers:
- 📦 Publishing updates to crates.io
- 🎨 Publishing VS Code extension
- 🔄 Version management
- 🚀 Release workflow

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

- [tree-sitter](https://tree-sitter.github.io/) for AST parsing
- The Rust community for excellent tooling and frameworks
- All contributors who made this universal runner possible

---

<div align="center">

**The first universal command runner for Rust** 🦀⚡

[Report Bug](https://github.com/raz-rs/raz/issues/new?template=bug_report.md) • [Request Feature](https://github.com/raz-rs/raz/issues/new?template=feature_request.md) • [Documentation](docs/)

</div>