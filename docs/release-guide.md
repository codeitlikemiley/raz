# Release Guide

This guide covers the complete release process for RAZ, including pre-release testing and stable releases.

## Overview

RAZ uses an automated release script that handles:
- Version bumping across all crates
- Publishing to crates.io
- VS Code extension publishing
- Git tagging and CI triggering
- Cross-platform binary building

## Release Types

### Pre-Release (Testing)

Use pre-releases to test new versions before making them stable:

```bash
./scripts/release.sh 0.1.4-beta.1 --pre-release
```

**What happens:**
- ✅ Updates all versions to `0.1.4-beta.1`
- ✅ Creates git tag `v0.1.4-beta.1`
- ✅ Publishes VS Code extension as **PRE-RELEASE** to marketplace
- ✅ CI builds cross-platform binaries for testing
- ⚠️ **SKIPS** crates.io publishing (testing only)

### Stable Release

After thorough testing of pre-release:

```bash
./scripts/release.sh 0.1.4
```

**What happens:**
- ✅ Updates all versions to `0.1.4`
- ✅ Publishes all crates to crates.io
- ✅ Publishes VS Code extension as stable release
- ✅ CI builds and creates GitHub release with artifacts

## Prerequisites

Before running any release:

### 1. Environment Setup

```bash
# Install VS Code extension CLI
npm install -g @vscode/vsce

# Set VS Code marketplace token
export VSCODE_PAT=your_personal_access_token

# Ensure cargo is logged in to crates.io
cargo login
```

### 2. Repository State

- Clean git working directory (no uncommitted changes)
- All tests passing: `cargo test --workspace`
- All lints passing: `cargo clippy --workspace`
- Documentation builds: `cargo doc --workspace`

### 3. Version Planning

Choose appropriate version numbers following [Semantic Versioning](https://semver.org/):
- **Patch** (0.1.3 → 0.1.4): Bug fixes, minor improvements
- **Minor** (0.1.4 → 0.2.0): New features, backward compatible
- **Major** (0.2.0 → 1.0.0): Breaking changes

## Step-by-Step Release Process

### Phase 1: Pre-Release Testing

1. **Prepare Pre-Release Version**
   ```bash
   # Choose next version with pre-release suffix
   ./scripts/release.sh 0.1.4-beta.1 --pre-release
   ```

2. **Monitor CI Build**
   - Go to [GitHub Actions](https://github.com/codeitlikemiley/raz/actions)
   - Wait for release workflow to complete
   - Verify cross-platform binaries are built

3. **Test Pre-Release**
   - Install VS Code pre-release extension
   - Download binaries from GitHub releases
   - Test core functionality across platforms
   - Verify binary auto-download in VS Code works

4. **User Testing** (Optional)
   - Share pre-release with beta testers
   - Gather feedback on new features
   - Document any issues found

### Phase 2: Stable Release

1. **Fix Any Issues**
   - Address bugs found during testing
   - Update documentation
   - Commit all fixes

2. **Release Stable Version**
   ```bash
   # Use stable version number (no suffix)
   ./scripts/release.sh 0.1.4
   ```

3. **Verify Release**
   - Check [crates.io](https://crates.io/crates/raz-cli) for new version
   - Check VS Code marketplace for stable extension
   - Verify GitHub release has all artifacts

4. **Post-Release Tasks**
   - Update main README if needed
   - Announce release (social media, Discord, etc.)
   - Close related GitHub issues
   - Update project boards/milestones

## Release Script Usage

### Basic Usage

```bash
# Stable release
./scripts/release.sh 0.1.4

# Pre-release
./scripts/release.sh 0.1.4-beta.1 --pre-release
```

### Script Features

- **Automatic Version Updates**: Updates all `Cargo.toml` and `package.json` files
- **Safety Checks**: Validates git status and version format
- **Backup Creation**: Creates git stash backup before changes
- **Dependency Order**: Publishes crates in correct dependency order
- **Error Handling**: Comprehensive error messages and rollback
- **CI Integration**: Automatically triggers release workflow

### What Gets Published

#### Stable Release (`0.1.4`)
| Component | Destination | Status |
|-----------|------------|--------|
| `raz-common` | crates.io | ✅ Published |
| `raz-config` | crates.io | ✅ Published |
| `raz-validation` | crates.io | ✅ Published |
| `raz-override` | crates.io | ✅ Published |
| `raz-core` | crates.io | ✅ Published |
| `raz-cli` | crates.io | ✅ Published |
| VS Code Extension | Marketplace | ✅ Stable Release |
| Cross-platform Binaries | GitHub Releases | ✅ CI Built |

#### Pre-Release (`0.1.4-beta.1`)
| Component | Destination | Status |
|-----------|------------|--------|
| `raz-common` | crates.io | ⚠️ **SKIPPED** |
| `raz-config` | crates.io | ⚠️ **SKIPPED** |
| `raz-validation` | crates.io | ⚠️ **SKIPPED** |
| `raz-override` | crates.io | ⚠️ **SKIPPED** |
| `raz-core` | crates.io | ⚠️ **SKIPPED** |
| `raz-cli` | crates.io | ⚠️ **SKIPPED** |
| VS Code Extension | Marketplace | ✅ Pre-Release |
| Cross-platform Binaries | GitHub Releases | ✅ CI Built |

## Binary Distribution

The VS Code extension automatically downloads the correct binary:

1. **Version Detection**: Reads version from `package.json`
2. **Platform Detection**: Detects OS and architecture
3. **Download**: Fetches from GitHub releases
4. **Verification**: Validates binary integrity
5. **Installation**: Places in extension directory

### Supported Platforms

| Platform | Architecture | Binary Name |
|----------|-------------|-------------|
| Windows | x64 | `raz-win32-x64.tar.gz` |
| macOS | x64 (Intel) | `raz-darwin-x64.tar.gz` |
| macOS | ARM64 (Apple Silicon) | `raz-darwin-arm64.tar.gz` |
| Linux | x64 | `raz-linux-x64.tar.gz` |
| Linux | ARM64 | `raz-linux-arm64.tar.gz` |

## Version Synchronization

The release script ensures all components use the same version:

- **Workspace Dependencies**: Updates internal crate references
- **VS Code Extension**: Updates `package.json` version
- **Cross-References**: Updates any version-specific documentation

## Troubleshooting

### Common Issues

#### "vsce command not found"
```bash
npm install -g @vscode/vsce
```

#### "VSCODE_PAT not set"
```bash
export VSCODE_PAT=your_personal_access_token
```

#### "Not logged in to crates.io"
```bash
cargo login
```

#### "Git working directory not clean"
```bash
git status
git add . && git commit -m "pre-release cleanup"
```

#### "Cargo publish failed"
- Check if version already exists on crates.io
- Ensure dependencies are published first
- Verify `Cargo.toml` syntax

#### "VS Code extension publish failed"
- Verify `VSCODE_PAT` has correct permissions
- Check marketplace account status
- Ensure extension builds successfully

### Recovery Procedures

#### Rollback Failed Release
```bash
# Remove local tag
git tag -d v0.1.4

# Reset to previous commit
git reset --hard HEAD~1

# If tag was pushed
git push origin :refs/tags/v0.1.4
```

#### Yank Published Crate
```bash
cargo yank --crate raz-core --version 0.1.4
```

#### Unpublish VS Code Extension
Use VS Code marketplace web interface to unpublish if needed.

## Best Practices

### Before Release
- [ ] Run full test suite
- [ ] Update CHANGELOG.md
- [ ] Review documentation
- [ ] Check for security vulnerabilities
- [ ] Verify examples work

### During Release
- [ ] Monitor CI builds
- [ ] Test pre-release thoroughly
- [ ] Document any issues
- [ ] Keep release notes updated

### After Release
- [ ] Verify all artifacts available
- [ ] Test installation procedures
- [ ] Update related documentation
- [ ] Announce to community

## Automation

The release process includes several automated checks:

1. **Pre-flight Validation**
   - Git status check
   - Version format validation
   - Dependency verification

2. **Build Validation**
   - `cargo check` across workspace
   - TypeScript compilation
   - Extension packaging

3. **Publication Validation**
   - Dry-run before publishing
   - Incremental publishing with delays
   - Error handling and rollback

4. **Post-Release Verification**
   - CI artifact generation
   - GitHub release creation
   - Marketplace publication confirmation

## Security Considerations

### Token Management
- Store `VSCODE_PAT` securely
- Rotate tokens periodically
- Use minimal required permissions

### Crate Security
- Review dependencies before release
- Run `cargo audit` before publishing
- Follow security advisories

### Binary Integrity
- CI builds from tagged source
- Checksums provided in releases
- Reproducible builds where possible

## Release Checklist

### Pre-Release Checklist
- [ ] All tests passing
- [ ] Documentation updated
- [ ] CHANGELOG.md updated
- [ ] Version number chosen
- [ ] Environment variables set
- [ ] Git working directory clean

### Release Checklist
- [ ] Pre-release tested thoroughly
- [ ] No critical issues found
- [ ] Stable version number confirmed
- [ ] Release script executed successfully
- [ ] All artifacts published correctly

### Post-Release Checklist
- [ ] Installation verified on multiple platforms
- [ ] Documentation reflects new version
- [ ] Community notified
- [ ] Issues/PRs updated
- [ ] Next version planning started

---

For questions about the release process, check our [troubleshooting guide](../scripts/README.md#troubleshooting) or open an issue on GitHub.