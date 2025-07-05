# Contributing to RAZ

Thank you for your interest in contributing to RAZ! This guide will help you report issues and contribute effectively.

## 🐛 Reporting Issues

We have specific issue templates to help you provide the right information:

### Choose the Right Template

- **🐛 [Bug Report](https://github.com/raz-rs/raz/issues/new?template=bug_report.md)** - General bugs and unexpected behavior
- **⚡ [Override Issue](https://github.com/raz-rs/raz/issues/new?template=override_issue.md)** - Problems with saving, loading, or applying overrides
- **🔍 [Command Detection](https://github.com/raz-rs/raz/issues/new?template=command_detection.md)** - Wrong commands generated or project type detection issues
- **🆚 [VS Code Extension](https://github.com/raz-rs/raz/issues/new?template=vscode_extension.md)** - Extension-specific problems
- **🐌 [Performance Issue](https://github.com/raz-rs/raz/issues/new?template=performance.md)** - Slow performance or resource usage problems
- **✨ [Feature Request](https://github.com/raz-rs/raz/issues/new?template=feature_request.md)** - New features or enhancements

### Essential Information for Bug Reports

When reporting any issue, please include:

1. **Complete terminal output** - Copy the entire output including the command being executed
2. **Exact reproduction steps** - We need to be able to reproduce the issue
3. **Environment details** - OS, VS Code version, Rust version, etc.
4. **Project context** - Cargo.toml, file structure, cursor position

### 📋 Terminal Output Format

Always include the complete terminal output in this format:

```
*  Executing task: /Users/username/.cargo/bin/raz "/path/to/file.rs:line:column" 

[Complete output including any errors, warnings, and the actual command executed]
```

This output contains crucial debugging information:
- The exact command being executed
- File path and cursor position
- Any detected overrides
- Error messages and exit codes

## 🔧 Getting Help

### Before Opening an Issue

1. **Check existing issues** - Your issue might already be reported
2. **Read the documentation** - Check the [README](../README.md) and [docs](../docs/)
3. **Try with latest version** - Run `raz --version` and compare with latest release
4. **Test with minimal example** - Can you reproduce with a simple test case?

### Quick Debugging Steps

1. **Test the binary directly:**
   ```bash
   raz --version
   raz "/path/to/file.rs:line:column" --dry-run
   ```

2. **Check saved overrides:**
   ```bash
   raz override list
   ```

3. **Enable debug logging:**
   ```bash
   RUST_LOG=debug raz "/path/to/file.rs:line:column"
   ```

4. **VS Code troubleshooting:**
   - Check the RAZ output channel (View → Output → RAZ)
   - Try restarting VS Code
   - Test with other extensions disabled

## 🏗️ Development

### Setting Up Development Environment

1. **Clone the repository:**
   ```bash
   git clone https://github.com/raz-rs/raz.git
   cd raz
   ```

2. **Install dependencies:**
   ```bash
   # Rust toolchain (if not already installed)
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   
   # Build the project
   cargo build
   ```

3. **Run tests:**
   ```bash
   cargo test
   ```

4. **Install locally for testing:**
   ```bash
   cargo install --path raz-adapters/cli
   ```

### Project Structure

```
raz/
├── raz-common/          # Shared utilities
├── raz-config/          # Configuration management
├── raz-core/            # Core command generation logic
├── raz-override/        # Override system
├── raz-validation/      # Input validation
├── raz-adapters/
│   ├── cli/            # Command-line interface
│   └── vscode/         # VS Code extension
├── docs/               # Documentation
└── tests/              # Integration tests
```

### Testing Your Changes

1. **Unit tests:**
   ```bash
   cargo test --workspace
   ```

2. **Integration tests:**
   ```bash
   cargo test --package raz-core
   ```

3. **Manual testing:**
   ```bash
   # Build and install locally
   cargo build --release
   cp target/release/raz ~/.cargo/bin/
   
   # Test with various project types
   raz "/path/to/test/file.rs:line:column"
   ```

4. **VS Code extension testing:**
   - Open the `raz-adapters/vscode` folder in VS Code
   - Press F5 to launch Extension Development Host
   - Test the extension in the new window

### Pull Request Guidelines

1. **Create focused PRs** - One feature or fix per PR
2. **Add tests** - Include tests for new functionality
3. **Update documentation** - Update README or docs if needed
4. **Follow code style** - Run `cargo fmt` and `cargo clippy`
5. **Write good commit messages** - Be descriptive and concise

### Code Style

- **Rust code:** Follow standard Rust conventions
- **Use `cargo fmt`** - Format code before committing
- **Use `cargo clippy`** - Fix any warnings
- **Document public APIs** - Add doc comments for public functions
- **Add tests** - Especially for new features and bug fixes

## 🤝 Community

- **Discussions:** [GitHub Discussions](https://github.com/raz-rs/raz/discussions)
- **Issues:** Use the appropriate issue template
- **Discord/Chat:** [Link if available]

## 📄 License

By contributing to RAZ, you agree that your contributions will be licensed under the same license as the project.

---

Thank you for helping make RAZ better! 🚀