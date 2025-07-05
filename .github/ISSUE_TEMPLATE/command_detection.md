---
name: Command Detection Issue
about: Report problems with raz not detecting the right commands or detecting wrong project type
title: '[DETECTION] '
labels: detection, bug
assignees: ''
---

## Detection Issue Type
<!-- Check the type of detection issue -->

- [ ] Wrong project type detected (Cargo vs single file vs script)
- [ ] Missing expected commands
- [ ] Wrong commands generated
- [ ] Framework not detected (Dioxus, Leptos, Tauri, etc.)
- [ ] Test detection issues
- [ ] File role detection issues

## Current Behavior

**Command executed:**
```
*  Executing task: /Users/username/.cargo/bin/raz "/path/to/file.rs:line:column" 

ℹ Executing [Command Type]: [description]
> [actual command executed]

[Paste complete terminal output]
```

**Available commands seen:**
<!-- If using VS Code command palette or --dry-run -->
```
[List of commands shown]
```

## Expected Behavior
<!-- What commands did you expect to see? -->

**Expected command type:** [e.g., Test, Run, Build, Serve]
**Expected command:** 
```bash
[expected command here]
```

## Project Information

<details>
<summary>Project Structure</summary>

```
project-root/
├── Cargo.toml
├── src/
│   ├── main.rs
│   └── lib.rs
└── [other relevant files]
```
</details>

<details>
<summary>Cargo.toml</summary>

```toml
[paste complete Cargo.toml]
```
</details>

<details>
<summary>File being executed</summary>

**File path:** `/path/to/file.rs`
**Cursor position:** Line X, Column Y

```rust
[paste relevant file content, especially around cursor position]
```
</details>

## Environment
- OS: [e.g., macOS 14.5, Ubuntu 22.04, Windows 11]
- Raz Version: [Run `raz --version`]
- Rust Version: [Run `rustc --version`]
- Project Type: [What type of project is this?]

## Framework Information
<!-- If using a specific framework -->

**Framework:** [e.g., Dioxus, Leptos, Tauri, Bevy, Yew]
**Framework Version:** [check Cargo.toml dependencies]

**Framework-specific files:**
- [ ] Dioxus.toml
- [ ] Tauri.toml  
- [ ] index.html
- [ ] Other: [specify]

## Debug Information

**Dry run output:**
```bash
# Output of: raz --dry-run "/path/to/file.rs:line:column"
[paste output here]
```

**With verbose logging:**
```bash
# Output of: RUST_LOG=debug raz "/path/to/file.rs:line:column"
[paste debug output if available]
```