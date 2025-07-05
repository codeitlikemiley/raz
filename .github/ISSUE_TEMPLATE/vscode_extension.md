---
name: VS Code Extension Issue
about: Report problems specific to the VS Code extension
title: '[VSCODE] '
labels: vscode, bug
assignees: ''
---

## Extension Issue Type
<!-- Check the type of extension issue -->

- [ ] Keybindings not working (Cmd+R, Cmd+Shift+R)
- [ ] Extension not loading/activating
- [ ] Task runner issues
- [ ] Override dialog problems
- [ ] Binary download/management issues
- [ ] Output channel issues
- [ ] Command palette integration

## Steps to Reproduce
1. Open VS Code
2. Open Rust file: `path/to/file.rs`
3. [Specific steps that cause the issue]

## VS Code Output
<!-- Check VS Code's output channels -->

### RAZ Output Channel
<!-- View → Output → Select "RAZ" from dropdown -->
```
[Paste RAZ output channel content]
```

### Tasks Output
<!-- If using task runner -->
```
*  Executing task: [command]

[Paste complete task output]
```

### Extension Host Log
<!-- If extension fails to load -->
<!-- Help → Toggle Developer Tools → Console tab -->
```
[Paste any relevant error messages]
```

## Keyboard Shortcuts
<!-- Test the keybindings -->

**Cmd+R (or Ctrl+R):**
- [ ] Works correctly
- [ ] Does nothing
- [ ] Shows error
- [ ] Triggers wrong command

**Cmd+Shift+R (or Ctrl+Shift+R):**
- [ ] Shows override dialog
- [ ] Does nothing  
- [ ] Shows error
- [ ] Triggers wrong command

## Extension Settings
<!-- File → Preferences → Settings → Search "raz" -->

```json
{
  "raz.showOutput": true/false,
  "raz.useTaskRunner": true/false,
  "raz.path": "/custom/path/to/raz" // if set
}
```

## Environment
- OS: [e.g., macOS 14.5, Ubuntu 22.04, Windows 11]
- VS Code Version: [e.g., 1.85.0]
- RAZ Extension Version: [Check Extensions panel]
- RAZ Binary Version: [If installed, run `raz --version`]

## Binary Management
<!-- If related to binary download/management -->

**Binary location:** [e.g., ~/.vscode/extensions/raz/bin/raz]
**Download status:** 
- [ ] Automatic download succeeded
- [ ] Automatic download failed
- [ ] Using custom path
- [ ] Binary not found

**Binary test:**
```bash
# Output of running the binary directly
/path/to/raz --version
```

## Workspace Information
- **Workspace type:** [Single folder, Multi-root, No folder]
- **Rust project type:** [Cargo workspace, package, single file]
- **Other extensions:** [List any Rust or task-related extensions]

## Additional Context

<details>
<summary>File being executed</summary>

**File path:** `/path/to/file.rs`
**File content (if small):**
```rust
[paste relevant source code]
```
</details>

<details>
<summary>Extension logs</summary>

<!-- If you can find extension logs -->
```
[paste any extension-specific logs]
```
</details>

## Workarounds
<!-- Have you found any workarounds? -->

- [ ] Running `raz` directly from terminal works
- [ ] Restarting VS Code fixes it temporarily
- [ ] Disabling other extensions helps
- [ ] Other: [describe]