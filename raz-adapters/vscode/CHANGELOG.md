# Changelog

All notable changes to the RAZ VS Code extension will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.6] - 2025-07-05

### Added
- Synchronized versioning across all crates and extensions
- Automated version bumping script

### Fixed
- Version conflicts with marketplace

## [0.1.5] - 2025-07-05

### Added
- Open VSX Registry publishing support
- Automated multi-registry publishing (VS Code Marketplace + Open VSX)

### Fixed
- GitHub Actions workflow permissions for release creation
- Cross-platform binary builds with macOS 14 runners

## [0.1.4-pre] - 2025-01-05 (Pre-release)

### 🐛 Added - Intelligent Debugging Integration

- **Breakpoint-Driven Debugging**: Automatic detection of breakpoints in symbol ranges
- **Seamless rust-analyzer Integration**: Uses rust-analyzer's debug codelens when breakpoints are detected
- **Smart Mode Switching**: Automatically switches between debug and run modes based on breakpoint presence
- **Symbol-Aware Detection**: Intelligently finds the most relevant symbol at cursor position
- **Zero Configuration**: No debug setup required - just set breakpoints and run

### 🔧 New Features

- **New Commands**:
  - `raz.toggleBreakpointDetection` - Toggle automatic breakpoint detection
  - `raz.showDebugInfo` - Show debugging information for current cursor position
- **New Configuration Options**:
  - `raz.enableBreakpointDetection` - Enable/disable breakpoint detection (default: true)
  - `raz.useRustAnalyzerCodeLens` - Use rust-analyzer codelens for debugging (default: true)
  - `raz.prioritySymbolKinds` - Configure symbol types for debugging context
  - `raz.logLevel` - Set logging level for debugging features

### 🚀 Improvements

- **Enhanced User Experience**: Single `Cmd+R` now intelligently chooses between debug and run modes
- **Performance**: Fast RAZ execution when no breakpoints are present
- **Reliability**: Fallback to RAZ execution if rust-analyzer debugging fails
- **Symbol Priority**: Configurable symbol type priorities for better context detection

### 🧠 Technical Details

- Implemented breakpoint range detection using VSCode's debugging API
- Added symbol traversal with priority-based filtering
- Integrated with rust-analyzer's codelens system for debugging
- Added comprehensive error handling and fallback mechanisms

### 📚 Documentation

- Updated README.md with debugging integration documentation
- Enhanced INSTALL.md with debugging setup instructions
- Added configuration examples for debugging features

## [0.1.3] - Previous Release

### Features
- Override system with deferred save mechanism
- Task runner integration
- Cross-IDE override persistence
- Smart test detection
- Framework-aware command generation

## [0.1.2] - Previous Release

### Features
- Basic command execution
- Binary management
- Configuration system

## [0.1.1] - Previous Release

### Features
- Initial VS Code extension release
- Basic Rust file execution

---

For more details, see the [README.md](README.md) and [project repository](https://github.com/codeitlikemiley/raz).