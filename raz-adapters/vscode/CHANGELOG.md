# Changelog

All notable changes to the RAZ VS Code extension will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2026-04-02

### Added

#### Bazel Integration
- **Bazel-aware CodeLens** — CodeLens now detects Bazel workspaces and renders contextually correct labels:
  - `▶ Run (Bazel)` / `⚡ Test (Bazel)` instead of generic Cargo labels
  - `⚠️ Doc-tests not supported in Bazel` inline warning for doc-test positions
  - Inferred Bazel target label shown in CodeLens description (e.g. `//server:unit_tests`)
- **Dynamic status bar badge** — the bottom-status-bar item now shows:
  - `$(flame) Bazel` for Bazel projects
  - `$(package) Cargo` for standard Cargo projects
  - Clicking opens the `.cargo-runner.json` config file
- **Bazel Config section in Override Tree** — when a project is Bazel-based, the override tree panel shows a collapsible **Bazel Config** section with:
  - Inferred target label (`//package:target`)
  - Active test runner (`bazel test` by default)
  - ⚠️ Doc-test limitation notice
  - Click-to-open `.cargo-runner.json`

#### Override Tree UX
- **Flat `BazelOverride` config shape** — overrides no longer require `"bazel": { "test_framework": { ... } }` nesting. Write fields directly:
  ```json
  { "match": { "function_name": "my_test" }, "bazel": { "test_args": ["--nocapture"] } }
  ```
- Override tree reads `bazel.test_framework.command + subcommand` from the top-level project config to show the active runner in the Bazel Config tree item.

#### Project Generation
- `raz.generateRustProject` command exposed to the activity bar — runs `bazel run @rules_rust//tools/rust_analyzer:gen_rust_project -- //...` for Bazel workspaces.

---

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