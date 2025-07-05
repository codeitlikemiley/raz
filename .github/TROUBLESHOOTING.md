# RAZ Troubleshooting Guide

This guide helps you diagnose and fix common issues with RAZ.

## 🚨 Before Reporting an Issue

### 1. Check Your Setup
```bash
# Verify raz is installed and working
raz --version

# Test with a simple file
raz "/path/to/simple/file.rs:1:1" --dry-run
```

### 2. Enable Debug Logging
```bash
# Run with debug output to see what's happening
RUST_LOG=debug raz "/path/to/file.rs:line:column"
```

### 3. Check for Recent Changes
- Did you recently update raz, Rust, or VS Code?
- Did you change your project configuration?
- Did you add new dependencies?

## 🔧 Common Issues and Solutions

### Issue: "Command not working in VS Code"

**Symptoms:**
- Cmd+R or Cmd+Shift+R does nothing
- Extension seems inactive

**Solutions:**
1. **Check extension is installed and enabled:**
   - Go to Extensions panel
   - Search for "RAZ"
   - Ensure it's installed and enabled

2. **Check VS Code output:**
   - View → Output → Select "RAZ" from dropdown
   - Look for error messages

3. **Check keybindings:**
   - File → Preferences → Keyboard Shortcuts
   - Search for "raz"
   - Ensure shortcuts aren't conflicting

4. **Try binary directly:**
   ```bash
   # Test if the underlying binary works
   raz "/full/path/to/file.rs:line:column"
   ```

### Issue: "Override not saving or wrong command generated"

**Symptoms:**
- Entered `--release` but command shows `--release` after `--`
- Override not persisting between sessions

**Debug Steps:**
1. **Check saved overrides:**
   ```bash
   raz override list
   ```

2. **Test override saving:**
   ```bash
   # Save with explicit flag
   raz --save-override "/path/to/file.rs:line:column" --release
   ```

3. **Check override format:**
   ```bash
   # Should show --release in cargo_options, not args
   raz override show [key]
   ```

**Solutions:**
- Ensure you're using the latest version of raz
- Check cursor position is on the correct function
- Verify override syntax matches expected format

### Issue: "Wrong project type detected"

**Symptoms:**
- Single file commands for Cargo project
- Missing framework-specific commands

**Debug Steps:**
1. **Check project structure:**
   ```bash
   # Verify Cargo.toml exists and is valid
   cargo check
   ```

2. **Test detection:**
   ```bash
   # See what raz detects
   RUST_LOG=debug raz "/path/to/file.rs:line:column" --dry-run 2>&1 | grep -i "detect"
   ```

**Solutions:**
- Ensure Cargo.toml is in the project root
- Check file paths are correct
- Verify framework dependencies in Cargo.toml

### Issue: "Performance problems / slow execution"

**Symptoms:**
- Commands take > 5 seconds to execute
- VS Code becomes unresponsive

**Debug Steps:**
1. **Profile execution:**
   ```bash
   time raz "/path/to/file.rs:line:column"
   ```

2. **Check project size:**
   ```bash
   find . -name "*.rs" | wc -l  # Count Rust files
   du -sh target/              # Check target directory size
   ```

**Solutions:**
- Clean build artifacts: `cargo clean`
- Reduce dependencies if possible
- Check for infinite loops in code
- Use `--dry-run` to test without execution

### Issue: "Binary not found or download failed"

**Symptoms:**
- "raz: command not found"
- Extension can't download binary

**Solutions:**
1. **Manual installation:**
   ```bash
   # Install via cargo
   cargo install raz-cli
   
   # Or download from releases
   curl -L https://github.com/raz-rs/raz/releases/latest/download/raz-[platform] -o ~/.cargo/bin/raz
   chmod +x ~/.cargo/bin/raz
   ```

2. **Set custom path in VS Code:**
   ```json
   {
     "raz.path": "/full/path/to/raz"
   }
   ```

3. **Check PATH:**
   ```bash
   echo $PATH
   which raz
   ```

## 🐛 Collecting Debug Information

When reporting an issue, please include:

### 1. Environment Information
```bash
# System info
uname -a                    # OS details
raz --version              # RAZ version
rustc --version            # Rust version
code --version             # VS Code version (if applicable)
```

### 2. Complete Terminal Output
```bash
# Run the failing command and copy ALL output
raz "/full/path/to/file.rs:line:column"
```

### 3. Debug Logs
```bash
# Enable debug logging and copy output
RUST_LOG=debug raz "/full/path/to/file.rs:line:column" 2>&1 | tee debug.log
```

### 4. Override Information
```bash
# If override-related issue
raz override list
raz override show [key]  # for specific override
```

### 5. Project Context
- Share relevant Cargo.toml
- Describe project structure
- Specify file being executed and cursor position

## 📋 Issue Template Checklist

When creating an issue, use the appropriate template:

- 🐛 **Bug Report** - General bugs and unexpected behavior
- ⚡ **Override Issue** - Problems with saving/loading overrides  
- 🔍 **Command Detection** - Wrong commands or project detection
- 🆚 **VS Code Extension** - Extension-specific problems
- 🐌 **Performance Issue** - Slow performance
- ✨ **Feature Request** - New features

Each template asks for specific information needed to debug that type of issue.

## 🆘 Getting Help

### Quick Help Options

1. **Check existing issues:** [GitHub Issues](https://github.com/raz-rs/raz/issues)
2. **Read documentation:** [README](../README.md) and [docs](../docs/)
3. **Ask in discussions:** [GitHub Discussions](https://github.com/raz-rs/raz/discussions)

### Escalation Path

1. Try troubleshooting steps above
2. Search existing issues
3. Create new issue with appropriate template
4. Provide complete information requested in template

---

**Remember:** The more information you provide, the faster we can help! 🚀