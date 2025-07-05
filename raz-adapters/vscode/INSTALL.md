# RAZ VS Code Extension - Installation Guide

This guide covers how to build, package, and install the RAZ VS Code extension locally.

## Prerequisites

- Node.js and npm installed
- Rust toolchain installed
- VS Code installed

## Building the Extension

### 1. Install Dependencies

```bash
cd vscode
npm install
```

### 2. Install VSCE (VS Code Extension Manager)

```bash
npm install -g @vscode/vsce
```

### 3. Build the Extension

```bash
# Compile TypeScript
npm run compile

# Build RAZ binaries for your platform
npm run build:binaries

# Or do both at once
npm run vscode:prepublish
```

### 4. Package the Extension

```bash
vsce package
```

This creates a `.vsix` file (e.g., `raz-vscode-0.1.0.vsix`) in the vscode directory.

## Installing the Extension

### Method 1: Using Command Palette (Recommended)

1. Open VS Code
2. Press `Cmd+Shift+P` (Mac) or `Ctrl+Shift+P` (Windows/Linux)
3. Type: `Extensions: Install from VSIX...`
4. Navigate to and select: `/path/to/raz/raz-adapters/vscode/raz-vscode-0.1.0.vsix`
5. Click "Install"
6. Reload VS Code when prompted

### Method 2: Using Command Line

```bash
code --install-extension /path/to/raz/raz-adapters/vscode/raz-vscode-0.1.0.vsix
```

### Method 3: Using Extensions View

1. Open Extensions view (`Cmd+Shift+X` or click Extensions icon)
2. Click the "..." menu (three dots) at the top
3. Select "Install from VSIX..."
4. Browse to and select the `.vsix` file

## Verifying Installation

1. **Check Extensions List**
   - Open Extensions view (`Cmd+Shift+X`)
   - Search for "RAZ"
   - You should see "RAZ - Rust Action Zapper"

2. **Test the Extension**
   - Open any `.rs` file
   - Press `Cmd+R` (Mac) or `Ctrl+R` (Windows/Linux)
   - RAZ should analyze the file and run the appropriate command

## Uninstalling

To uninstall the extension:

1. Open Extensions view
2. Search for "RAZ"
3. Click the gear icon next to the extension
4. Select "Uninstall"

## Troubleshooting

### Extension Not Working

1. **Check Output Channel**
   - View → Output
   - Select "RAZ" from dropdown
   - Look for error messages

2. **Verify Binary Permissions**
   ```bash
   ls -la raz-adapters/vscode/bin/*/raz
   # Should show executable permissions (x)
   ```

3. **Check Activation**
   - Extension only activates for Rust files (`.rs`)
   - Check Developer Tools: Help → Toggle Developer Tools

### Building Issues

1. **Missing Binaries**
   - Ensure `cargo build -p raz-cli --release` succeeds
   - Check that binaries are copied to `raz-adapters/vscode/bin/`

2. **TypeScript Errors**
   - Run `npm run compile` to see specific errors
   - Ensure all dependencies are installed

### Platform-Specific Notes

- **macOS**: May need to allow the binary in System Preferences → Security & Privacy
- **Windows**: Ensure `.exe` extension is present for the binary
- **Linux**: Binary needs executable permissions

## VS Code Profiles

The extension is installed per-profile. If you use multiple VS Code profiles, you'll need to install the extension in each profile where you want to use it.

## Development Mode

For development, you can run the extension without packaging:

1. Open the vscode folder in VS Code
2. Press `F5` to launch a new VS Code window with the extension loaded
3. Make changes and reload (`Cmd+R` in the extension development window)

## Configuration

After installation, configure the extension:

1. Open Settings (`Cmd+,`)
2. Search for "raz"
3. Available settings:
   - `raz.showOutput`: Show output in terminal (default: true)
   - `raz.useTaskRunner`: Use VS Code tasks for better concurrency (default: true)