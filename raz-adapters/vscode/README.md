# RAZ - Rust Action Zapper

⚡ Run ANY Rust file from ANYWHERE instantly. Smart test detection. Zero config. Just hit Cmd+R and watch it fly 🚀

## VS Code Features

- **`Cmd+R` to Run**: Press `Cmd+R` (Mac) or `Ctrl+R` (Windows/Linux) in any Rust file to instantly run it
- **`Cmd+Shift+R` to Override**: Run with custom environment variables, options, and arguments
- **Auto-Breakpoint Debug Mode**: Set breakpoints and press `Cmd+R` - switches to debug mode automatically
- **Binary Auto-Download**: Downloads correct binary for your platform automatically
- **Task Runner Integration**: Better concurrency with VS Code's task system
- **Override Persistence**: Custom command configurations persist across sessions
- **Extension Dependencies**: Auto-installs rust-analyzer and CodeLLDB
- **Smart Test Detection**: Automatically detects and runs the test at your cursor position
- **Framework Aware**: Specialized commands for Leptos, Dioxus, Tauri, Bevy, Yew, and more
- **Cross-IDE Override Persistence**: Save overrides in VS Code, use them in terminal, Vim, IntelliJ, or anywhere
- **Zero Configuration**: Works immediately without any setup

## Usage

### Basic Usage (Cmd+R)

1. Open any Rust file in VS Code
2. Press **Cmd+R** (Mac) or **Ctrl+R** (Windows/Linux)
3. RAZ will:
   - Detect the file type and project context
   - **Check for breakpoints** in the current symbol's range
   - **Automatically switch to debug mode** if breakpoints are detected
   - Use rust-analyzer's debug codelens for debugging or fallback to RAZ execution

### VS Code Debugging Integration

**Auto-Breakpoint Debug Mode**: Set breakpoints and press `Cmd+R` - switches to debug mode automatically

1. **Set breakpoints** in your Rust code by clicking in the gutter
2. **Press `Cmd+R`** as normal - RAZ automatically detects breakpoints
3. **Debug mode activates** when breakpoints are found in the current symbol
4. **Run mode** is used when no breakpoints are present

#### How It Works
- **Symbol Detection**: RAZ finds the symbol at your cursor position
- **Breakpoint Scanning**: Checks for breakpoints within that symbol's range  
- **Smart Switching**: 
  - ✅ **Breakpoints found** → Executes rust-analyzer's "Debug" codelens command
  - ⚡ **No breakpoints** → Uses fast RAZ execution
- **Zero Configuration**: Leverages existing rust-analyzer + CodeLLDB setup

### Override Usage (Cmd+Shift+R)

Press **Cmd+Shift+R** to run with custom overrides and optionally save them for future use. Enter overrides in the input field:

#### Input Syntax
```
RUST_BACKTRACE=full --release --device true -- --exact --nocapture
```

The extension automatically parses:
- **Environment variables**: `SCREAMING_CASE=value` (e.g., `RUST_BACKTRACE=full`)
- **Cargo options**: Common cargo flags like `--release`, `--features`, etc. are automatically recognized
- **Test arguments**: Test-specific flags like `--nocapture`, `--show-output` are placed after `--`
- **Manual separator**: Use `--` to explicitly separate cargo options from test arguments

#### Examples

**Run with backtrace:**
```
RUST_BACKTRACE=full
```

**Run in release mode:**
```
--release
```
Note: `--release` is automatically recognized as a cargo option

**Run tests with output:**
```
--nocapture --show-output
```
Note: Test arguments are automatically placed after `--`

**Run specific test in release mode:**
```
--release --exact --nocapture
```

**Combined example:**
```
RUST_BACKTRACE=full RUST_LOG=debug --release --platform web -- --test-threads 1
```

### Command Selection

To see all available commands:

- Use **Cmd+Shift+P** → "RAZ: Select and Run Command"

### Other Commands

Additional commands available through the Command Palette:
- **"RAZ: Open RAZ Settings"**: Open VS Code settings for RAZ
- **"RAZ: Setup RAZ Binary"**: Guided setup for RAZ binary installation
- **"RAZ: Toggle Breakpoint Detection"**: Enable/disable automatic breakpoint detection
- **"RAZ: Show Debug Info"**: Show debugging information for current cursor position

## Override Persistence with Deferred Save

RAZ features a **deferred save mechanism** that only saves override configurations after successful command execution. This prevents failed or invalid overrides from persisting and causing future execution problems.

### How Deferred Save Works

1. **Cmd+Shift+R**: Enter overrides and run the command
2. **Command Execution**: RAZ runs the command with your overrides applied
3. **Success**: If the command succeeds (exit code 0), the override is saved
4. **Failure**: If the command fails, the override is **NOT saved** and you see: "⚠ Warning Command failed. Override was NOT saved."
5. **Future Runs**: **Cmd+R** automatically applies saved overrides for that specific function

### Benefits

- **No Bad Overrides**: Failed commands with invalid flags don't create persistent configurations
- **Clean Configurations**: Only working overrides are saved to your workspace
- **Automatic Cleanup**: No need to manually remove failed override attempts
- **VS Code Integration**: Failed tasks in VS Code don't create persistent overrides

### Storage

Overrides are stored in `.raz/overrides.toml` with automatic backups and function-specific keys like `src/lib.rs:function_name`

### Example Config Structure

```toml
[[overrides]]
version = 1

[overrides."src/lib.rs:test_foo"]
key = { primary = "src/lib.rs:test_foo", fallbacks = ["src/lib.rs:L25"] }
override_config = { key = "test", env = { RUST_BACKTRACE = "1" }, cargo_options = ["--quiet"], args = ["--exact", "--nocapture"] }
metadata = { created_at = "2024-01-15T10:30:00Z", file_path = "src/lib.rs", function_name = "test_foo" }
```

### Managing Overrides

Use VS Code commands to manage your saved overrides:
- **"RAZ: Override Statistics"**: View all saved override configurations
- **"RAZ: List All Overrides"**: List all saved override configurations
- **"RAZ: Clear Saved Overrides"**: Clear all saved overrides
- **"RAZ: Debug Override Resolution"**: Debug override resolution for current file
- **Command Palette**: Access override management through Cmd+Shift+P

For CLI-based management, see the [Override Management Guide](https://github.com/codeitlikemiley/raz/blob/main/docs/override-management.md).

## Configuration

```json
{
  // Show terminal output (default: true)
  "raz.showOutput": true,
  
  // Use VS Code tasks for better concurrency (default: true)
  "raz.useTaskRunner": true,
  
  // Enable automatic breakpoint detection for debugging (default: true)
  "raz.enableBreakpointDetection": true,
  
  // Custom path to RAZ binary (optional)
  // If not set, the extension will automatically download and manage the binary
  "raz.path": "/usr/local/bin/raz"
}
```

### Binary Management

RAZ automatically downloads and manages the binary for your platform:

- **Automatic Download**: On first use, RAZ downloads the latest binary from GitHub releases
- **Platform Detection**: Automatically detects your OS and architecture (Linux, macOS, Windows)
- **Custom Path**: Set `raz.path` to use your own RAZ installation
- **Lightweight Extension**: No bundled binaries - smaller download size

### Debugging Integration

RAZ seamlessly integrates with rust-analyzer and CodeLLDB for debugging:

- **Direct Codelens Execution**: Simply executes rust-analyzer's "Debug" codelens command when breakpoints are found
- **Automatic Detection**: Detects breakpoints within symbol ranges automatically  
- **Zero Configuration**: Leverages your existing rust-analyzer + CodeLLDB setup
- **Fallback Support**: Falls back to fast RAZ execution when no breakpoints are detected
- **Extension Dependencies**: Automatically installs rust-analyzer and CodeLLDB extensions

## Common Overrides

### For Dioxus Projects
- `--platform web` - Build for web platform
- `--platform desktop` - Build for desktop platform  
- `--device true` - Build for real device (mobile)
- `--device false` - Build for simulator (mobile)
- `--release` - Build in release mode
- `--open` - Open browser after starting server

### For Tests
- `-- --exact` - Run exact test match
- `-- --nocapture` - Show println! output
- `-- --test-threads 1` - Run tests sequentially
- `-- --ignored` - Run ignored tests

### Environment Variables
- `RUST_BACKTRACE=full` - Full backtrace on panic
- `RUST_BACKTRACE=1` - Short backtrace on panic
- `RUST_LOG=debug` - Enable debug logging
- `RUST_FLAGS="-A warnings"` - Suppress warnings

## Examples

### Running Tests
Place your cursor on a test function and press Cmd+R:
```rust
#[test]
fn test_auth() { // <- Cursor here + Cmd+R + Breakpoint = Debug mode!
    assert!(true);  // <- Set breakpoint here for automatic debugging
}
// Without breakpoints: cargo test -- tests::test_auth --exact
// With breakpoints: rust-analyzer debug mode with full debugging support
```

### Running Binaries
Open any main.rs and press Cmd+R:
```rust
fn main() {
    println!("Hello, world!");
}
// Executes: cargo run
```

### Running Single Files
Open a standalone Rust file and press Cmd+R:
```rust
// /tmp/hello.rs
fn main() {
    println!("Hello from single file!");
}
// Executes: rustc /tmp/hello.rs && /tmp/hello
```

## Task Runner

The extension uses VS Code's task system for better command management:

### Benefits
- **Concurrent Execution**: Run multiple commands simultaneously (e.g., dev server + tests)
- **Non-blocking**: Long-running commands (like `cargo leptos watch`) don't block the terminal
- **Better Management**: Stop, restart, and monitor tasks through VS Code's UI
- **Dedicated Terminals**: Each task runs in its own terminal panel

### Usage
When you run a command with Cmd+R, it executes as a VS Code task. If you try to run the same command again, you'll get options:
- **Show Task**: Switch to the existing task's terminal
- **Start New**: Create a new instance
- **Cancel**: Cancel the operation

### Long-Running Commands
These commands are automatically detected as background tasks:
- `cargo leptos watch`
- `trunk serve` (Yew)
- `cargo tauri dev`
- `dx serve` (Dioxus)
- `cargo watch`

To disable the task runner and use traditional terminal execution:
```json
{
  "raz.useTaskRunner": false
}
```

## Installation

See [INSTALL.md](INSTALL.md) for detailed installation instructions.

## Building from Source

### For Installation

1. Clone the repository
2. Install dependencies:
   ```bash
   cd raz-adapters/vscode
   npm install
   ```
3. Bundle the extension:
   ```bash
   npm run bundle
   ```
4. Install the extension:
   ```bash
   code --install-extension raz-vscode-x.x.x.vsix
   ```

### For Development/Contributing

1. Clone the repository
2. Install dependencies:
   ```bash
   cd raz-adapters/vscode
   npm install
   ```
3. Compile TypeScript:
   ```bash
   npm run compile
   ```
4. Press F5 in VS Code to launch the extension development host

## Requirements

- VS Code 1.89.0 or higher
- Rust toolchain (cargo, rustc)
- Optional: Framework-specific tools (cargo-leptos, dx, trunk, etc.)

## 🐛 Issues and Support

### Reporting Extension Issues

Use our specialized **[VS Code Extension Issue Template](https://github.com/codeitlikemiley/raz/issues/new?template=vscode_extension.md)** for:
- Keybinding problems (Cmd+R, Cmd+Shift+R not working)
- Extension loading issues
- Task runner problems
- Override dialog issues

### Quick Debugging

1. **Check the RAZ output channel:**
   - View → Output → Select "RAZ" from dropdown

2. **Test the binary directly:**
   ```bash
   raz --version
   raz "/path/to/file.rs:line:column" --dry-run
   ```

3. **Common solutions:**
   - Restart VS Code
   - Check keybinding conflicts in Command Palette
   - Verify binary is installed: `which raz`

📋 **[All Issue Templates](https://github.com/codeitlikemiley/raz/issues/new/choose)** | 📖 **[Troubleshooting Guide](https://github.com/codeitlikemiley/raz/blob/main/.github/TROUBLESHOOTING.md)**

## License

MIT