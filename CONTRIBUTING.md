# Contributing to RAZ

Thank you for your interest in contributing to RAZ! This guide will help you get started.

## Code of Conduct

By participating in this project, you agree to abide by our Code of Conduct. Please be respectful and welcoming to all contributors.

## How to Contribute

### Reporting Issues

1. Check if the issue already exists
2. Include a clear description
3. Provide steps to reproduce
4. Include relevant error messages and logs
5. Mention your OS, Rust version, and RAZ version

### Suggesting Features

1. Check if the feature has been requested
2. Clearly describe the use case
3. Explain why existing features don't solve the problem
4. Provide examples of how it would work

### Pull Requests

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Add tests for new functionality
5. Ensure all tests pass (`cargo test`)
6. Run clippy (`cargo clippy -- -D warnings`)
7. Format your code (`cargo fmt`)
8. Commit with a descriptive message
9. Push to your fork
10. Open a Pull Request

## Development Setup

```bash
# Clone your fork
git clone https://github.com/your-username/raz.git
cd raz

# Add upstream remote
git remote add upstream https://github.com/raz-rs/raz.git

# Install development dependencies
rustup component add clippy rustfmt

# Build the project
cargo build

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run --bin raz -- list
```

## Project Structure

- `raz-core/` - Core library
  - `src/context.rs` - Project analysis
  - `src/providers/` - Command providers
  - `src/ast.rs` - Tree-sitter integration
  - `src/filters.rs` - Command filtering
  
- `raz-adapters/` - IDE integrations
  - `cli/` - Command-line interface
  - `raz-adapters/vscode/` - VS Code extension
  
- `docs/` - Documentation

## Adding a New Provider

1. Create a new file in `raz-core/src/providers/`
2. Implement the `CommandProvider` trait
3. Add tests in the same file
4. Register in `providers.rs`
5. Update documentation

Example:
```rust
use async_trait::async_trait;
use crate::{CommandProvider, Command, ProjectContext, RazResult};

pub struct MyProvider;

#[async_trait]
impl CommandProvider for MyProvider {
    fn name(&self) -> &str { "my-provider" }
    
    async fn commands(&self, context: &ProjectContext) -> RazResult<Vec<Command>> {
        // Implementation
    }
}
```

## Testing

### Unit Tests
```bash
cargo test
```

### Integration Tests
```bash
cargo test --test '*'
```

### Test Coverage
```bash
cargo tarpaulin --out Html
```

### Manual Testing
```bash
# Test with example projects
cargo run --bin raz -- --directory test-leptos list
cargo run --bin raz -- --directory test-dioxus list
```

## Documentation

- Update code documentation with `///` comments
- Update README.md for user-facing changes
- Add examples for new features
- Update CLI help text

## Style Guide

### Rust Code
- Follow standard Rust naming conventions
- Use `rustfmt` for formatting
- Keep functions focused and small
- Prefer explicit over implicit
- Document public APIs

### Error Handling
- Use `RazResult<T>` for fallible operations
- Provide helpful error messages
- Handle edge cases gracefully

### Performance
- Profile before optimizing
- Prefer lazy evaluation
- Cache expensive computations
- Use parallel operations where beneficial

## Commit Messages

Follow conventional commits:
- `feat:` New features
- `fix:` Bug fixes
- `docs:` Documentation changes
- `test:` Test additions or fixes
- `refactor:` Code refactoring
- `perf:` Performance improvements
- `chore:` Maintenance tasks

Example:
```
feat: add Bevy game engine provider

- Implements commands for game development
- Adds asset management support
- Includes platform-specific builds
```

## Release Process

1. Update version in Cargo.toml files
2. Update CHANGELOG.md
3. Create a PR with version bump
4. After merge, tag the release
5. GitHub Actions will publish to crates.io

## Getting Help

- Join our Discord server
- Ask questions in GitHub Discussions
- Check existing issues and PRs
- Read the documentation

## Recognition

Contributors will be recognized in:
- The README.md file
- Release notes
- Our website (when launched)

Thank you for contributing to RAZ!