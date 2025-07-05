# RAZ VS Code Extension Troubleshooting

## Command Not Found Issues

If you're seeing errors like `command 'raz.runCommand' not found` or `command 'raz.runCommandWithOverride' not found`, try these steps:

### 1. Check Extension Installation
```bash
# List installed extensions
code --list-extensions | grep raz

# If not found, reinstall
code --install-extension raz-vscode-0.1.4.vsix --force
```

### 2. Verify Extension Activation

The extension should activate when:
- You open a Rust file (`.rs` extension)
- You run any RAZ command

**Check activation:**
1. Open VS Code
2. Open any `.rs` file (or create a test file: `test.rs`)
3. Open the Output panel (`View` → `Output`)
4. Select "RAZ" from the dropdown
5. You should see: `🚀 RAZ extension activated successfully!`

### 3. Manual Activation

If the extension isn't activating automatically:

1. **Open Command Palette**: `Cmd+Shift+P` (Mac) or `Ctrl+Shift+P` (Windows/Linux)
2. **Search for RAZ commands**: Type "RAZ" to see available commands
3. **Try running**: "RAZ: Run RAZ Command" to force activation

### 4. Check Keybindings

The default keybindings are:
- **Cmd+R** (Mac) / **Ctrl+R** (Windows/Linux): Run current file
- **Cmd+Shift+R** / **Ctrl+Shift+R**: Run with custom overrides

**If keybindings don't work:**
1. Go to `Code` → `Preferences` → `Keyboard Shortcuts`
2. Search for "raz"
3. Verify the keybindings are set correctly
4. Resolve any conflicts with other extensions

### 5. Reload VS Code

Sometimes a simple reload fixes activation issues:
1. **Command Palette**: `Cmd+Shift+P` / `Ctrl+Shift+P`
2. **Type**: "Developer: Reload Window"
3. **Press Enter**

### 6. Check for Conflicts

Other extensions might conflict with RAZ:
1. **Disable other Rust extensions temporarily**
2. **Test RAZ functionality**
3. **Re-enable extensions one by one** to identify conflicts

**⚠️ IMPORTANT**: Check for duplicate RAZ extensions:
```bash
# List all RAZ extensions
code --list-extensions --show-versions | grep -i raz

# If you see multiple versions, uninstall the old ones:
code --uninstall-extension raz.raz-vscode
```
Having multiple versions of RAZ installed will cause command conflicts!

### 7. Developer Debugging

For developers debugging the extension:

```bash
# Check extension logs
code --log-level trace

# Open developer console in VS Code
# Help → Toggle Developer Tools
# Check Console tab for errors
```

### 8. Common Issues

**Issue**: Extension activates but commands still not found
**Solution**: Check that the `main` field in `package.json` points to the correct compiled file:
```json
"main": "./out/extension"
```

**Issue**: Keybindings work but Command Palette commands don't
**Solution**: Verify all commands are listed in `package.json` `contributes.commands`

**Issue**: Extension works for some commands but not others
**Solution**: Check that all commands are registered in the `activate()` function

### 9. Reset Extension State

If all else fails, reset the extension:

```bash
# Uninstall
code --uninstall-extension masterustacean.raz-vscode

# Clear extension cache (varies by OS)
# macOS:
rm -rf ~/Library/Application\ Support/Code/User/workspaceStorage/*/masterustacean.raz-vscode

# Linux:
rm -rf ~/.config/Code/User/workspaceStorage/*/masterustacean.raz-vscode

# Windows:
# Delete %APPDATA%\Code\User\workspaceStorage\*\masterustacean.raz-vscode

# Reinstall
code --install-extension raz-vscode-0.1.4.vsix
```

### 10. Report Issues

If none of these steps work:

1. **Collect debug info**:
   - VS Code version: `code --version`
   - Extension version: Check in Extensions view
   - Operating system and version
   - Console errors from Developer Tools

2. **Create a minimal reproduction**:
   - Create a simple `.rs` file
   - Try the exact steps that fail
   - Note any error messages

3. **Report the issue** at: https://github.com/codeitlikemiley/raz/issues

Include all the debug info and reproduction steps in your report.

---

## Debugging Integration Issues (v0.1.4+)

### Debugging Integration Issues

If RAZ's debugging integration isn't working properly:

#### How RAZ Debugging Works
RAZ v0.1.4 uses rust-analyzer's existing codelens commands for debugging. When breakpoints are detected, RAZ simply executes the "Debug" codelens command that rust-analyzer provides. This means:

1. **No binary management**: RAZ doesn't manage debug binaries - rust-analyzer handles everything
2. **Direct command execution**: RAZ just calls `vscode.commands.executeCommand` with the codelens command
3. **Zero configuration**: If rust-analyzer's Debug codelens works, RAZ's debugging works

#### Common Issues

**1. Debug Codelens Not Available**
If rust-analyzer isn't showing Debug codelens:
- Ensure rust-analyzer extension is installed and enabled
- Check that your code compiles (`cargo check`)
- Verify you're in a Rust function/test that can be debugged

**2. Check rust-analyzer Settings**
Ensure rust-analyzer is properly configured:
1. Open VS Code settings (`Cmd+,`)
2. Search for "rust-analyzer"
3. Verify these settings:
   - `rust-analyzer.cargo.buildScripts.enable`: true
   - `rust-analyzer.check.command`: "check" or "clippy"

**3. Test rust-analyzer Debug Codelens Directly**
Before troubleshooting RAZ, verify rust-analyzer's Debug codelens works:
1. Open a Rust file with a test function
2. Look for "Debug" codelens above the function
3. Click it directly - if this fails, the issue is with rust-analyzer, not RAZ

**4. Disable Breakpoint Detection Temporarily**
If debugging integration causes issues, you can disable it:
```json
{
  "raz.enableBreakpointDetection": false
}
```
This will make RAZ always use fast execution instead of debugging.

**5. Use RAZ's Debug Info Command**
To understand what RAZ is detecting:
1. Set a breakpoint in your code
2. Place cursor on the function
3. Command Palette → "RAZ: Show Debug Info"
4. This shows what symbol and breakpoints RAZ found

**6. Force RAZ Execution**
You can bypass debugging integration:
1. Remove all breakpoints temporarily
2. Press `Cmd+R` - RAZ will use fast execution
3. Add breakpoints back when debugging is needed

### Debugging Integration Not Working

**Check rust-analyzer Extension**
The debugging integration requires rust-analyzer to be installed and working:
```bash
# Verify rust-analyzer is installed
code --list-extensions | grep rust-analyzer
```

**Check Symbol Detection**
1. Use "RAZ: Show Debug Info" command
2. Verify it finds the correct symbol at your cursor
3. Check if symbol types match your configuration in `raz.prioritySymbolKinds`

**Check Breakpoint Detection**
1. Set a breakpoint in VS Code
2. The breakpoint should appear as a red dot in the gutter
3. Use "RAZ: Show Debug Info" to verify RAZ detects it

### Configuration for Debugging

**Recommended Settings**:
```json
{
  "raz.enableBreakpointDetection": true,
  "raz.useRustAnalyzerCodeLens": true,
  "raz.prioritySymbolKinds": ["Function", "Enum", "Struct", "Object", "Module"],
  "raz.logLevel": "debug"  // For troubleshooting only
}
```

**If You Don't Want Debugging Integration**:
```json
{
  "raz.enableBreakpointDetection": false
}
```
This makes RAZ always use fast execution.

---

## Quick Verification Test

Create this test file to verify the extension works:

**test.rs**:
```rust
fn main() {
    println!("Hello RAZ!");
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
```

1. Save the file
2. Press `Cmd+R` (Mac) or `Ctrl+R` (Windows/Linux)
3. You should see RAZ execute the file

If this doesn't work, follow the troubleshooting steps above.