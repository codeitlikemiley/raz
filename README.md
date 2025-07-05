# RAZ - Rust Action Zapper

<div align="center">

![RAZ Cover](raz-github-cover.svg)

[![CI](https://github.com/codeitlikemiley/raz/actions/workflows/ci.yml/badge.svg)](https://github.com/codeitlikemiley/raz/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/raz-cli.svg)](https://crates.io/crates/raz-cli)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**Universal command runner for Rust - Run any Rust file from anywhere with smart test detection**

[Installation](#installation) • [Quick Start](#quick-start) • [Documentation](docs/) • [VS Code Extension](#vs-code-extension)

</div>

## What is RAZ?

RAZ is a universal command runner that executes any Rust file from any directory without requiring workspace context. It provides intelligent, cursor-aware test detection and persistent command overrides.

### Key Features

- 🎯 **Run from Anywhere** - Execute any Rust file without `cd`ing into directories
- 📍 **Cursor-Aware** - Automatically detects and runs the test/function at your cursor position
- 💾 **Smart Overrides** - Save command flags per function, only persisted after successful execution
- 🛡️ **Safe by Default** - Failed commands never save bad configurations
- 🔧 **IDE Integration** - Full VS Code support with Cmd+R hotkeys
- 🚀 **Zero Config** - Works instantly with any Rust project

## Installation

### CLI
```bash
# Install from crates.io
cargo install raz-cli

# Install with cargo-binstall (downloads pre-built binary)
cargo binstall raz-cli
```

### VS Code Extension
```bash
# Install from marketplace (search "RAZ")
# Or download .vsix from GitHub releases
code --install-extension raz-vscode-*.vsix
```

## Quick Start

```bash
# Run any Rust file
raz /path/to/file.rs

# Run test at specific line
raz src/lib.rs:25:1

# Save override (only saved after success)
raz --save-override src/lib.rs:25:1 RUST_BACKTRACE=1 --exact

# Use saved override automatically
raz src/lib.rs:25:1  # Uses RUST_BACKTRACE=1 --exact

# Manage overrides
raz override list      # Show saved overrides
raz override rollback  # Restore last working state
```

**VS Code**: Press `Cmd+R` to run, `Cmd+Shift+R` to run with overrides

## How It Works

1. **Smart Detection** - Uses tree-sitter AST parsing to understand your code structure
2. **Deferred Save** - Overrides only persist after successful execution
3. **Universal Persistence** - Saved overrides work across all editors and terminals
4. **Automatic Context** - Detects workspace, package, or standalone files automatically

## Project Structure

| Crate | Description |
|-------|-------------|
| [`raz-cli`](raz-adapters/cli/) | Command-line interface |
| [`raz-core`](raz-core/) | Core command generation library |
| [`raz-validation`](raz-validation/) | Smart options validation |
| [`raz-override`](raz-override/) | Override management system |

## Documentation

- [Advanced Usage](docs/advanced-usage.md) - Override system, deferred save
- [Validation Guide](docs/validation-guide.md) - Framework support and validation
- [Override Management](docs/override-management.md) - Complete override CLI reference
- [Library Usage](docs/library-usage.md) - Embed RAZ in your applications

## Why RAZ?

**Without RAZ**: Remember and retype complex cargo commands, navigate to project directories, manually manage test flags

**With RAZ**: Run any file from anywhere, save working configurations automatically, never lose your test setup

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and guidelines.

## License

MIT - see [LICENSE](LICENSE) for details.

---

<div align="center">

**The universal command runner for Rust** 🦀⚡

[Report Bug](https://github.com/codeitlikemiley/raz/issues) • [Request Feature](https://github.com/codeitlikemiley/raz/issues) • [Full Documentation](docs/)

</div>