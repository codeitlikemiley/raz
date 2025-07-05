# Changelog

All notable changes to the RAZ VS Code extension will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.1] - 2025-07-05

### Added
- Auto-breakpoint debug mode: Set breakpoints and press `Cmd+R` - switches to debug mode automatically
- Binary auto-download: Downloads correct binary for your platform automatically
- Task runner integration: Better concurrency with VS Code's task system
- Override persistence: Custom command configurations persist across sessions
- Extension dependencies: Auto-installs rust-analyzer and CodeLLDB
- Override system with deferred save mechanism
- Cross-IDE override persistence
- Smart test detection
- Framework-aware command generation
- Basic command execution
- Binary management
- Configuration system
- Open VSX Registry publishing support
- Automated multi-registry publishing (VS Code Marketplace + Open VSX)
- Migration system to clean up old binary management files
- Weekly update notifications with intelligent update method detection
- Breakpoint-driven debugging with automatic detection
- Seamless rust-analyzer integration
- Symbol-aware detection with priority filtering
- Zero configuration debug setup
- Commands: `raz.toggleBreakpointDetection`, `raz.showDebugInfo`
- Configuration options: `raz.enableBreakpointDetection`, `raz.useRustAnalyzerCodeLens`, `raz.prioritySymbolKinds`, `raz.logLevel`

### Features
- Single `Cmd+R` intelligently chooses between debug and run modes
- Fast RAZ execution when no breakpoints are present
- Fallback to RAZ execution if rust-analyzer debugging fails
- Properly quoted RAZ binary paths for spaces
- Cross-platform binary builds
- Synchronized versioning across all crates and extensions

---

For more details, see the [README.md](README.md) and [project repository](https://github.com/codeitlikemiley/raz).