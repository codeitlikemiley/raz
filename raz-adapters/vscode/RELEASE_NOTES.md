# RAZ VS Code Extension v0.1.4-pre Release Notes

## 🚨 Pre-release Version

This is a **pre-release** version to test the new features, especially the binary auto-download functionality. Please report any issues at [GitHub Issues](https://github.com/codeitlikemiley/raz/issues).

## 🎯 Highlights

### 🐛 Intelligent Debugging Integration (NEW!)
- **Automatic Breakpoint Detection**: Set a breakpoint and press `Cmd+R` - RAZ automatically switches to debug mode
- **Seamless Mode Switching**: No breakpoints? Get instant RAZ execution. With breakpoints? Launch the debugger
- **Zero Configuration**: Works out of the box with rust-analyzer and CodeLLDB
- **Opt-in Feature**: Can be disabled via `raz.enableBreakpointDetection` setting

### 📦 Binary Auto-Download (NEEDS TESTING!)
- **Automatic Platform Detection**: Downloads the correct binary for your OS and architecture
- **No Manual Setup**: Extension manages RAZ binary installation automatically
- **Fallback Options**: Can still use system-installed RAZ or custom binary path
- **Lightweight Extension**: No bundled binaries means smaller download size

### 🔧 Enhanced Features
- **Task Runner Integration**: Better concurrency with VS Code's task system
- **Override Persistence**: Save custom command configurations that persist across sessions
- **Deferred Save**: Overrides only saved after successful execution
- **Extension Dependencies**: Automatically installs rust-analyzer and CodeLLDB

## 📋 Testing Checklist for Pre-release

Please help test these features:

### Binary Auto-Download
- [ ] Extension downloads binary on first use
- [ ] Platform detection works correctly (Linux/macOS/Windows)
- [ ] Binary has correct permissions
- [ ] Fallback to system RAZ works
- [ ] Custom binary path setting works

### Debugging Integration
- [ ] Breakpoint detection works inside functions
- [ ] Debug mode launches with breakpoints
- [ ] Run mode executes without breakpoints
- [ ] Cursor position detection is accurate
- [ ] Works with tests, benchmarks, and regular functions

### General Functionality
- [ ] `Cmd+R` executes correctly
- [ ] `Cmd+Shift+R` shows override dialog
- [ ] Task runner executes commands
- [ ] Override persistence works
- [ ] Error messages are helpful

## 🔄 Upgrade Instructions

1. **Backup your settings** if you have custom RAZ configurations
2. **Uninstall previous version** (optional but recommended)
3. **Install pre-release**:
   ```bash
   code --install-extension raz-vscode-0.1.4.vsix
   ```
4. **Test the features** and report issues

## ⚙️ Configuration

### New Settings
```json
{
  // Enable automatic debugging when breakpoints detected (default: true)
  "raz.enableBreakpointDetection": true,
  
  // Path to RAZ binary (leave empty for auto-download)
  "raz.path": "",
  
  // Use VS Code tasks for better concurrency (default: true)
  "raz.useTaskRunner": true,
  
  // Show output in terminal (default: true)
  "raz.showOutput": true
}
```

## 🐞 Known Issues

1. **Binary auto-download not tested in production** - This is why it's a pre-release
2. **Debugging may not work outside functions** - By design, but may be confusing
3. **Some codelens edge cases** - Complex macro-generated tests might not detect properly

## 📝 Feedback

Please report issues with:
- Your OS and architecture
- VS Code version
- Rust toolchain version
- Specific error messages
- Steps to reproduce

Report at: https://github.com/codeitlikemiley/raz/issues

## 🙏 Thank You

Thank you for testing the pre-release! Your feedback helps make RAZ better for everyone.

---

**Note**: Once testing is complete and issues are resolved, we'll release the stable v0.1.4.