# Advanced Usage

## Smart Override Parsing

RAZ automatically parses your command overrides without requiring special flags:

```bash
# All of these are automatically detected:
raz file.rs RUST_BACKTRACE=1               # Environment variable
raz file.rs --release                      # Cargo option
raz file.rs --platform web                 # Framework option
raz file.rs -- --exact                     # Test arguments

# Complex example with mixed types:
raz src/lib.rs:25:1 RUST_LOG=debug RUST_BACKTRACE=full --release --features ssr,hydrate --platform web -- --exact --nocapture --test-threads 1
```

**Smart Detection Rules**:
- `SCREAMING_CASE=value` → Environment variable
- `--flag` or `--option value` → Command option (validated against cargo/framework commands)
- Everything after `--` → Arguments passed to the binary/test runner
- Quoted values are handled correctly: `RUST_FLAGS="-A warnings -D unused"`

## VS Code Extension

RAZ includes a full-featured VS Code extension for seamless IDE integration:

**Installation**:
```bash
# Build and install the extension
cd vscode
npm install
npm run vscode:prepublish
code --install-extension raz-vscode-*.vsix
```

**Key Features**:
- **Cmd+R** (Mac) / **Ctrl+R** (Windows/Linux): Run RAZ command for current file
- **Cmd+Shift+R** / **Ctrl+Shift+R**: Run with custom overrides and optionally save them
- **Task Runner**: Background execution for long-running commands (serve, watch, etc.)
- **Override Persistence**: Save overrides per function and auto-apply them
- **Concurrent Execution**: Run multiple commands simultaneously

**Usage**:
1. Open any Rust file in VS Code
2. Place cursor on a test function or main function
3. Press **Cmd+R** to run (uses saved overrides if any)
4. Press **Cmd+Shift+R** to run with new overrides and save them

**Advanced Operator Syntax** (VS Code extension only):
```bash
+--features new_feature    # Add feature
---default-features        # Remove default features
!--target wasm32           # Force target override
```

**Override Management with Deferred Save**:
- Overrides are saved to `.raz/overrides.toml` in your workspace **only after successful execution**
- Failed commands in VS Code do NOT create persistent overrides
- Use "RAZ: Override Statistics" command to view saved configurations
- Each function can have its own saved override configuration
- Automatic backups protect against override corruption

## Cursor-Aware Test Execution

RAZ uses cursor position to intelligently select which test to run:

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_addition() {      // Line 4
        assert_eq!(2 + 2, 4);
    }

    #[test] 
    fn test_subtraction() {   // Line 9
        assert_eq!(5 - 3, 2);
    }
}

#[cfg(test)]
mod integration_tests {
    #[test]
    fn test_workflow() {      // Line 16
        // Complex integration test
    }
}
```

```bash
# Cursor on line 4 - runs tests::test_addition
raz src/lib.rs:4:1
> cargo test --package myproject -- tests::test_addition --exact

# Cursor on line 16 - runs integration_tests::test_workflow  
raz src/lib.rs:16:1
> cargo test --package myproject -- integration_tests::test_workflow --exact
```

## Framework Integration

RAZ automatically detects and provides framework-specific commands:

```bash
# Leptos projects
raz src/app.rs           # Runs leptos serve for frontend files

# Tauri projects  
raz src-tauri/src/main.rs    # Runs tauri dev

# Bevy projects
raz src/main.rs          # Runs with proper game features
```

## Override Persistence with Deferred Save

RAZ features a **deferred save mechanism** that prevents failed overrides from persisting, ensuring only working configurations are saved:

```bash
# Save an override - only saves AFTER successful execution
raz --save-override src/lib.rs:25:1 RUST_BACKTRACE=1 --quiet -- --exact --nocapture
# ✓ Success Override saved after successful execution
# Key: src/lib.rs:test_function

# Failed commands do NOT save overrides (prevents persistence of bad options)
raz --save-override test.rs --invalid-flag
# Command failed with exit code: 1
# ⚠ Warning Command failed. Override was NOT saved.

# Future runs automatically apply the saved override
raz src/lib.rs:25:1  # Automatically uses RUST_BACKTRACE=1 --quiet -- --exact --nocapture
```

**How Deferred Save Works**:
1. Override data is prepared but not saved immediately
2. Command is executed with the override applied
3. **Only if the command succeeds** is the override saved to config
4. Failed commands leave no persistent configuration

**Override Storage**:
- Overrides are saved to `.raz/overrides.toml` in your workspace  
- Keys are function-specific: `src/lib.rs:function_name` or `src/lib.rs:L25` for line-based
- Different test functions can have different saved configurations
- Automatic loading when you run commands
- Automatic backups created before changes

## Override Management Commands

RAZ provides comprehensive override management through CLI commands:

### List and Inspect Overrides
```bash
# List all saved overrides
raz override list

# List overrides for a specific file
raz override list --file src/lib.rs

# Inspect a specific override
raz override inspect src/lib.rs:test_function

# Show override statistics
raz override stats
```

### Backup and Recovery
```bash
# Create backup and rollback to previous state
raz override rollback

# Force rollback without confirmation
raz override rollback --force

# List available backups
raz override list-backups

# Export overrides for backup
raz override export --output backup.toml

# Import overrides from backup
raz override import backup.toml
```

### Debug and Troubleshooting
```bash
# Debug override resolution at a specific location
raz override debug src/lib.rs 25 10

# Update execution status manually
raz override update-status src/lib.rs:test_func success

# Delete a specific override
raz override delete src/lib.rs:test_function

# Clear all overrides (with confirmation)
raz override clear
```

### Migration from Legacy Formats
```bash
# Auto-detect and migrate legacy overrides
raz override migrate --auto

# Migrate from specific file
raz override migrate --file old_config.toml --dry-run

# Preview migration without changes
raz override migrate --file old_config.toml --dry-run
```

**Additional CLI flags**:
```bash
raz --dry-run file.rs:10:1        # Preview command without executing
raz --save-override file.rs opt   # Save override (deferred until success)
raz --verbose file.rs             # Show detailed execution info
```

## Supported File Patterns

### 1. Cargo Projects
```bash
raz workspace/member/src/lib.rs:20:1    # Workspace member test
raz project/src/main.rs                 # Package binary
raz project/examples/demo.rs            # Package example
```

### 2. Cargo Scripts
```rust
#!/usr/bin/env -S cargo +nightly -Zscript

fn main() {
    println!("Hello from cargo script!");
}
```
```bash
raz script.rs    # Automatically detected and run with cargo
```

### 3. Standalone Files
```rust
fn main() {
    println!("Hello, world!");
}

#[test]
fn test_something() {
    assert_eq!(2 + 2, 4);
}
```
```bash
raz standalone.rs        # Runs main function
raz standalone.rs:7:1    # Runs test_something
```

## Configuration (Optional)

While RAZ works without configuration, you can optimize it for your projects:

```bash
# Initialize project-specific configuration
raz init --template web      # For Leptos, Dioxus projects
raz init --template game     # For Bevy projects  
raz init --template library  # For library development
raz init --template desktop  # For Tauri, Egui projects
```