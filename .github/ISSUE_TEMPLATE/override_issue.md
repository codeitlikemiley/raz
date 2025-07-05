---
name: Override Issue
about: Report problems with saving, loading, or applying overrides
title: '[OVERRIDE] '
labels: override, bug
assignees: ''
---

## Override Issue Type
<!-- Check the type of override issue -->

- [ ] Override not saving correctly
- [ ] Override not loading/applying
- [ ] Override arguments in wrong position (before/after --)
- [ ] Override causing command to fail
- [ ] VS Code extension override dialog issues
- [ ] CLI override parsing issues

## Steps to Reproduce

### Saving Override (Cmd+Shift+R)
1. Open file: `path/to/file.rs` at line X
2. Press `Cmd+Shift+R` (or Ctrl+Shift+R)
3. Enter override text: `[paste exact text entered]`

**Terminal Output:**
```
*  Executing task: /Users/username/.cargo/bin/raz --save-override "/path/to/file.rs:line:column" [override args]

[Paste complete terminal output here]
```

### Loading Override (Cmd+R)
1. Position cursor at same location
2. Press `Cmd+R` (or Ctrl+R)

**Terminal Output:**
```
*  Executing task: /Users/username/.cargo/bin/raz "/path/to/file.rs:line:column" 

Found override: CommandOverride { ... }

ℹ Executing [Command Type]: [description]
> [actual command executed]

[Paste complete output including any errors]
```

## Expected vs Actual Command

**Expected command:**
```bash
cargo test --package my-project --release -- tests::my_test --exact
```

**Actual command being executed:**
```bash
cargo test --package my-project -- tests::my_test --exact --release
```

## Override Details

**Override text entered:** `[e.g., --release, RUST_BACKTRACE=1 --nocapture, etc.]`

**Current saved overrides:**
```bash
# Output of: raz override list
[paste output here]
```

**Specific override details:**
```bash
# Output of: raz override show [key] (if applicable)
[paste output here]
```

## Environment
- OS: [e.g., macOS 14.5, Ubuntu 22.04, Windows 11]
- VS Code Version: [e.g., 1.85.0]
- Raz Version: [Run `raz --version`]
- Extension Version: [Check VS Code extensions panel]

## Test Case
<!-- If possible, provide a minimal test case -->

<details>
<summary>Minimal test case</summary>

**File structure:**
```
my-project/
├── Cargo.toml
└── src/
    └── lib.rs
```

**Cargo.toml:**
```toml
[package]
name = "my-project"
version = "0.1.0"
edition = "2021"
```

**src/lib.rs:**
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn my_test() {
        assert_eq!(2 + 2, 4);
    }
}
```

**Cursor position:** Line X, Column Y (on the test function)
**Override text:** `[exact text entered]`

</details>