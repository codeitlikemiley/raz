# Smart Test Default Flags in RAZ

RAZ now intelligently handles default test flags like `--exact` and `--show-output` to avoid duplication when users specify their own overrides.

## The Problem

Previously, when RAZ generated test commands, it would always add default flags like `--exact` and `--show-output`. If a user added an override that also included these flags, they would be duplicated in the final command:

```bash
# Before: Duplicated flags
cargo test --package mypackage -- my_test --exact --show-output --exact --nocapture
```

## The Solution

The improved override system now:

1. **Checks for existing flags** before adding them in append mode
2. **Avoids duplication** of test arguments after the `--` separator
3. **Preserves user intent** while maintaining sensible defaults

```bash
# After: No duplication
cargo test --package mypackage -- my_test --exact --show-output --nocapture
```

## How It Works

### 1. Default Test Commands

When RAZ generates test commands, it adds sensible defaults:
- `--exact` for precise test matching
- `--show-output` to see test output

### 2. Smart Override Application

When applying overrides in append mode:
- The system checks existing arguments after the `--` separator
- It only adds override arguments that aren't already present
- This prevents duplication of flags

### 3. Replace Mode

In replace mode, all arguments after `--` are replaced with the override arguments, giving users full control.

## Example Usage

```bash
# RAZ generates a test command with defaults:
cargo test --package mypackage -- my_test --exact --show-output

# User adds an override with:
raz override test -- --exact --nocapture

# Result (no duplication of --exact):
cargo test --package mypackage -- my_test --exact --show-output --nocapture
```

## Benefits

1. **No more duplicate flags** in test commands
2. **Cleaner command output** that's easier to read
3. **Better user experience** when customizing test behavior
4. **Preserves defaults** when they make sense
5. **Respects user overrides** without redundancy