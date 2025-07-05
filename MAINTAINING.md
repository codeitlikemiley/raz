# Maintaining RAZ

This guide covers how to maintain, update, and publish the RAZ project components.

## Table of Contents
- [Version Management](#version-management)
- [Publishing to Crates.io](#publishing-to-cratesio)
- [Publishing VS Code Extension](#publishing-vs-code-extension)
- [Release Workflow](#release-workflow)

## Version Management

RAZ follows semantic versioning (MAJOR.MINOR.PATCH):
- **MAJOR**: Breaking API changes
- **MINOR**: New features, backward compatible
- **PATCH**: Bug fixes, backward compatible

### Updating Version Numbers

1. **Update workspace Cargo.toml files**:
   ```bash
   # Update version in these files:
   # - /raz-core/Cargo.toml
   # - /raz-adapters/cli/Cargo.toml
   ```

2. **Keep versions in sync**:
   - When updating raz-core, also update its version reference in raz-cli
   - Example: If raz-core goes to 0.2.0, update raz-cli's dependency:
     ```toml
     raz-core = { version = "0.2.0", path = "../../raz-core" }
     ```

## Publishing to Crates.io

### Prerequisites
- Crates.io account with API token configured
- Run `cargo login` if not already authenticated

### Publishing raz-core

1. **Update version** in `/raz-core/Cargo.toml`

2. **Create standalone version** (required for workspace packages):
   ```bash
   # Create temp directory
   mkdir -p temp-publish/raz-core
   cp -r raz-core/* temp-publish/raz-core/
   cd temp-publish/raz-core
   ```

3. **Replace workspace dependencies** in `Cargo.toml`:
   ```toml
   # Change from:
   serde = { workspace = true }
   
   # To:
   serde = { version = "1.0", features = ["derive"] }
   ```

4. **Add workspace table** to prevent workspace detection:
   ```toml
   [workspace]
   # This is intentionally empty to make this a standalone package
   ```

5. **Test and publish**:
   ```bash
   # Test the package
   cargo test
   cargo publish --dry-run
   
   # Publish for real
   cargo publish
   ```

### Publishing raz-cli

1. **Wait for raz-core** to be indexed on crates.io (usually 1-5 minutes)

2. **Update version** in `/raz-adapters/cli/Cargo.toml`

3. **Create standalone version**:
   ```bash
   mkdir -p temp-publish/raz-cli
   cp -r raz-adapters/cli/* temp-publish/raz-cli/
   cd temp-publish/raz-cli
   ```

4. **Update dependencies**:
   ```toml
   # Change from:
   raz-core = { version = "0.2.0", path = "../../raz-core" }
   clap = { workspace = true }
   
   # To:
   raz-core = "0.2.0"
   clap = { version = "4.0", features = ["derive"] }
   ```

5. **Add workspace table and publish**:
   ```toml
   [workspace]
   ```
   
   ```bash
   cargo test
   cargo publish --dry-run
   cargo publish
   ```

### Post-Publishing Cleanup

After publishing, restore local development setup:
```bash
# In raz-adapters/cli/Cargo.toml, restore:
raz-core = { version = "0.2.0", path = "../../raz-core" }

# Clean up temp directories
rm -rf temp-publish/
```

## Publishing VS Code Extension

### Prerequisites
- Node.js and npm installed
- VS Code Extension Manager (vsce) installed:
  ```bash
  npm install -g @vscode/vsce
  ```
- Visual Studio Marketplace publisher account

### Building the Extension

1. **Navigate to extension directory**:
   ```bash
   cd vscode
   ```

2. **Install dependencies**:
   ```bash
   npm install
   ```

3. **Update version** in `package.json`:
   ```json
   {
     "version": "0.2.0"
   }
   ```

4. **Build the extension**:
   ```bash
   # This creates the .vsix file
   vsce package
   ```

### Publishing to VS Code Marketplace

1. **Login to vsce**:
   ```bash
   vsce login <publisher-name>
   ```

2. **Publish the extension**:
   ```bash
   vsce publish
   
   # Or publish with version bump:
   vsce publish minor  # 0.1.0 -> 0.2.0
   vsce publish patch  # 0.1.0 -> 0.1.1
   ```

### Testing Locally

Before publishing, test the extension locally:
```bash
# Package the extension
vsce package

# Install in VS Code
code --install-extension raz-*.vsix
```

## Release Workflow

### 1. Pre-Release Checklist

- [ ] All tests passing: `cargo test --all-features`
- [ ] Clippy clean: `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] Formatted: `cargo fmt --check`
- [ ] Documentation updated
- [ ] CHANGELOG.md updated

### 2. Version Bump

```bash
# Update versions in:
# - raz-core/Cargo.toml
# - raz-adapters/cli/Cargo.toml  
# - raz-adapters/vscode/package.json
```

### 3. Create Git Tag

```bash
git add .
git commit -m "chore: Release v0.2.0"
git tag v0.2.0
git push origin main --tags
```

### 4. Publish Sequence

1. Publish raz-core to crates.io
2. Wait for indexing
3. Publish raz-cli to crates.io
4. Publish VS Code extension

### 5. Post-Release

1. Create GitHub Release with changelog
2. Update installation instructions if needed
3. Announce on social media/forums

## Troubleshooting

### Common Publishing Issues

1. **"no matching package named `raz-core` found"**
   - raz-core not yet indexed on crates.io
   - Wait 1-5 minutes and try again

2. **"workspace dependencies not allowed"**
   - Ensure you're using the standalone version
   - All `{ workspace = true }` replaced with explicit versions

3. **VS Code extension validation errors**
   - Check `vsce ls` to see what files are included
   - Ensure all required files are not in .vscodeignore

### Version Mismatch

If local development breaks after publishing:
```bash
# Restore local path dependency
cd raz-adapters/cli
# Edit Cargo.toml to add back path:
# raz-core = { version = "0.2.0", path = "../../raz-core" }
```

## CI/CD Integration

The GitHub Actions workflow automatically:
- Runs tests on all platforms
- Checks code formatting
- Runs clippy lints
- Generates code coverage

For automated releases, consider adding:
```yaml
# .github/workflows/release.yml
on:
  push:
    tags:
      - 'v*'
```

## Maintenance Schedule

- **Weekly**: Update dependencies with `cargo update`
- **Monthly**: Review and merge dependabot PRs
- **Quarterly**: Major feature releases
- **As needed**: Security patches

## Getting Help

- GitHub Issues: Bug reports and feature requests
- Discussions: General questions and ideas
- Discord/Matrix: Real-time chat (if available)