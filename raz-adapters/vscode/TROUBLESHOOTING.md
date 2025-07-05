# RAZ VS Code Extension Troubleshooting

## Command Not Found Issues

If you're seeing errors like `command 'raz.runCommand' not found` or `command 'raz.runCommandWithOverride' not found`, try these steps:

### 1. Check Extension Installation
```bash
# List installed extensions
code --list-extensions | grep raz

# If not found, reinstall
code --install-extension raz-vscode-0.1.2.vsix --force
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
code --install-extension raz-vscode-0.1.2.vsix
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