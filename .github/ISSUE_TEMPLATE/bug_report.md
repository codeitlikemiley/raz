---
name: Bug report
about: Report a bug or unexpected behavior in raz
title: '[BUG] '
labels: bug
assignees: ''
---

## Description
<!-- A clear and concise description of what the bug is -->

## Steps to Reproduce
<!-- Please provide the exact steps to reproduce the issue -->

1. Open file: `path/to/file.rs`
2. Press keyboard shortcut: (e.g., Cmd+R, Cmd+Shift+R)
3. Enter override (if applicable): 
4. See error

## Terminal Output
<!-- IMPORTANT: Please paste the complete terminal output including the command being executed -->

### First attempt (if applicable):
```
*  Executing task: /Users/username/.cargo/bin/raz "/path/to/file.rs:line:column" 

[Paste complete terminal output here]
```

### Second attempt / When error occurs:
```
*  Executing task: /Users/username/.cargo/bin/raz "/path/to/file.rs:line:column" 

[Paste complete terminal output including error messages]
```

## Expected Behavior
<!-- What did you expect to happen? -->

## Actual Behavior
<!-- What actually happened? -->

## Environment
<!-- Please complete the following information -->

- OS: [e.g., macOS 14.5, Ubuntu 22.04, Windows 11]
- VS Code Version: [e.g., 1.85.0]
- Raz Version: [Run `raz --version`]
- Rust Version: [Run `rustc --version`]
- Project Type: [e.g., Cargo workspace, single file, Cargo.toml package]

## Additional Context
<!-- Add any other context about the problem here -->

### Relevant Files
<!-- If applicable, provide relevant file contents -->

<details>
<summary>Cargo.toml (if applicable)</summary>

```toml
[paste Cargo.toml content]
```
</details>

<details>
<summary>Source file content (if small)</summary>

```rust
[paste relevant source code]
```
</details>

### Saved Overrides
<!-- If the issue involves overrides, please include: -->

Output of `raz override list`:
```
[paste output here]
```

### Debug Information
<!-- If you can reproduce with debug logging -->

Run with debug logging: `RUST_LOG=debug raz [your command]`
```
[paste debug output if available]
```