# Release Scripts

## `release.sh`

Automated release script that handles the complete release process for RAZ.

### Usage

```bash
./scripts/release.sh <version> [--pre-release]
```

**Examples:**

**Stable Release:**
```bash
./scripts/release.sh 0.1.4
```

**Pre-Release (for testing):**
```bash
./scripts/release.sh 0.1.4-beta.1 --pre-release
```

### Prerequisites

1. **Git**: Clean working directory (no uncommitted changes)
2. **Rust**: `cargo` must be available and you must be logged in to crates.io
3. **Node.js**: `npm` must be available
4. **VS Code Extension CLI**: Install `vsce` globally:
   ```bash
   npm install -g @vscode/vsce
   ```
5. **VS Code PAT**: Set the `VSCODE_PAT` environment variable with your VS Code marketplace Personal Access Token

### What the script does

#### For Stable Releases (`./scripts/release.sh 0.1.4`)

1. **Validation**: Checks git status and version format
2. **Backup**: Creates a git stash backup before starting
3. **Version Updates**: Updates all `Cargo.toml` files and `package.json` with the new version
4. **Validation**: Runs `cargo check` to ensure everything compiles
5. **Git Operations**: Commits changes and creates a version tag
6. **Crate Publishing**: Publishes all crates to crates.io in dependency order:
   - `raz-common`
   - `raz-config`
   - `raz-validation`
   - `raz-override`
   - `raz-core`
   - `raz-adapters/cli`
7. **VS Code Extension**: Builds and publishes the VS Code extension to marketplace
8. **Git Push**: Pushes changes and tags to the repository
9. **CI Trigger**: The version tag push triggers the GitHub Actions release workflow to build cross-platform binaries

#### For Pre-Releases (`./scripts/release.sh 0.1.4-beta.1 --pre-release`)

1. **Validation**: Checks git status and pre-release version format
2. **Backup**: Creates a git stash backup before starting
3. **Version Updates**: Updates all `Cargo.toml` files and `package.json` with the new version
4. **Validation**: Runs `cargo check` to ensure everything compiles
5. **Git Operations**: Commits changes and creates a version tag
6. **Crate Publishing**: ⚠️ **SKIPPED** - Pre-releases are not published to crates.io
7. **VS Code Extension**: Builds and publishes the VS Code extension as **PRE-RELEASE** to marketplace
8. **Git Push**: Pushes changes and tags to the repository
9. **CI Trigger**: The version tag push triggers the GitHub Actions release workflow to build cross-platform binaries

### Environment Variables

- `VSCODE_PAT`: Your VS Code marketplace Personal Access Token (required for extension publishing)

### Error Handling

The script includes comprehensive error handling:
- Git backup before making changes
- Cargo check validation before publishing
- Individual crate validation with dry-run
- Automatic cleanup of backup files
- Clear error messages with next steps

### Safety Features

- Validates version format (semantic versioning)
- Checks for clean git working directory
- Creates backup before making changes
- Runs dry-run before actual publishing
- Publishes crates in correct dependency order
- Waits between crate publications for registry updates

### Output

The script provides colored output with clear step indicators:
- 🔵 **[STEP]**: Current operation
- 🟢 **[SUCCESS]**: Completed successfully
- 🟡 **[WARNING]**: Non-critical issues
- 🔴 **[ERROR]**: Critical failures

### CI Integration

The script integrates with GitHub Actions:
- Creates and pushes version tags (`v*`) 
- Tag push triggers the `.github/workflows/release.yml` workflow
- CI builds cross-platform binaries for all supported targets
- CI creates a GitHub release with downloadable artifacts
- CI can optionally publish VS Code extension to marketplace

### Example Run

```bash
$ ./scripts/release.sh 0.1.4
[STEP] Starting release process for version 0.1.4
[STEP] Creating backup of current state
[STEP] Updating versions in all Cargo.toml files
[STEP] Updating workspace dependencies
[STEP] Updating raz-common version to 0.1.4
[SUCCESS] Updated raz-common
[STEP] Updating raz-config version to 0.1.4
[SUCCESS] Updated raz-config
# ... continues for all crates ...
[STEP] Publishing crates to crates.io
[STEP] Publishing raz-common
[SUCCESS] Published raz-common 0.1.4
# ... continues for all crates ...
[STEP] Publishing VS Code extension
[SUCCESS] VS Code extension published successfully
[SUCCESS] 🎉 Release 0.1.4 completed successfully!
```

### Recovery

If something goes wrong during the release process:

1. **Git Recovery**: The script creates a backup stash before starting
2. **Manual Rollback**: If needed, you can manually revert:
   ```bash
   git reset --hard HEAD~1  # Remove version commit
   git tag -d v0.1.4        # Remove tag if created
   ```
3. **Crate Yanking**: If a crate was published with issues:
   ```bash
   cargo yank --crate raz-core --version 0.1.4
   ```

### Troubleshooting

**Common Issues:**

1. **"vsce command not found"**
   ```bash
   npm install -g @vscode/vsce
   ```

2. **"VSCODE_PAT not set"**
   ```bash
   export VSCODE_PAT=your_personal_access_token
   ```

3. **"Not logged in to crates.io"**
   ```bash
   cargo login
   ```

4. **"Git working directory not clean"**
   ```bash
   git status
   git add . && git commit -m "your changes"
   # or
   git stash
   ```

### Manual Steps (if needed)

If you need to run parts of the release process manually:

```bash
# Update versions manually
sed -i 's/version = "0.1.3"/version = "0.1.4"/' */Cargo.toml

# Publish specific crate
cd raz-core && cargo publish

# Build and publish VS Code extension
cd raz-adapters/vscode
npm run bundle
vsce publish --pat $VSCODE_PAT
```