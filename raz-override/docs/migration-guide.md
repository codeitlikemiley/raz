# Migration Guide: Legacy to New Override Format

This guide explains how to migrate from the old cursor position-based override format to the new function-based override system.

## Overview

The new override system provides more stable keys that don't break when code changes:

- **Old format**: `"src/main.rs:42:10"` (cursor position-based)
- **New format**: `"src/main.rs:handle_request"` (function-based) or `"src/main.rs:L42"` (line-based fallback)

## Migration Methods

### 1. Automatic Migration

The simplest way to migrate is using the auto-detect feature:

```bash
raz override migrate --auto
```

This will:
- Search for legacy overrides in `.raz/overrides.toml`
- Create a backup at `.raz/overrides.toml.backup`
- Migrate all overrides to the new format
- Show a detailed migration report

### 2. Manual File Migration

To migrate a specific legacy file:

```bash
raz override migrate --file /path/to/legacy-overrides.toml
```

### 3. Dry Run Preview

To see what changes would be made without actually migrating:

```bash
raz override migrate --file legacy.toml --dry-run
```

This shows you exactly what the new keys would be without making any changes.

## Migration Process

The migration tool follows this process for each override:

1. **Parse Legacy Key**: Extract file path, line, and column from `"file:line:column"` format
2. **Function Detection**: Try to detect the function at that position using tree-sitter
3. **Key Generation**: Generate a new stable key:
   - Function-based: `"file:function_name"` (preferred)
   - Line-based: `"file:L<line>"` (fallback when function detection fails)
4. **Save Override**: Save the override with the new key and all original configuration

## Migration Results

The migration report shows three types of results:

### ✅ Success
- Function was detected successfully
- Override migrated to function-based key
- Most stable format

### ⚠️ Warning  
- Function detection failed (file moved, syntax errors, etc.)
- Override migrated to line-based key
- Still more stable than cursor position

### ❌ Failed
- Critical error in migration process
- Override not migrated
- Manual intervention required

## Example Migration Report

```
Migration Report:

Total overrides: 5
Successful: 3
Warnings: 2
Failed: 0

Details:
1. ✓ src/main.rs:42:10 → src/main.rs:handle_request
   Migrated to function-based key

2. ⚠ src/lib.rs:100:5 → src/lib.rs:L100  
   Could not detect function, migrated to line-based key

3. ✓ tests/test.rs:25:8 → tests/test.rs:test_user_auth
   Migrated to function-based key

4. ✓ src/utils.rs:15:1 → src/utils.rs:parse_config
   Migrated to function-based key

5. ⚠ old/removed.rs:50:10 → old/removed.rs:L50
   Could not detect function (file not found), migrated to line-based key
```

## Legacy Override Format

The old format stored overrides like this:

```toml
[overrides."src/main.rs:42:10"]
key = "debug-build"
mode = "replace"
cargo_options = ["--features", "debug"]
rustc_options = []
args = []
created_at = "2024-01-01T00:00:00Z"

[overrides."src/main.rs:42:10".env]
RUST_LOG = "debug"
DEBUG = "1"
```

## New Override Format

After migration, the same override becomes:

```toml
version = 1

[[overrides]]
key = "src/main.rs:handle_request"
override_config = { key = "debug-build", mode = "replace", cargo_options = ["--features", "debug"], rustc_options = [], args = [], env = { RUST_LOG = "debug", DEBUG = "1" }, created_at = "2024-01-01T00:00:00Z" }

[overrides.metadata]
created_at = "2024-01-01T00:00:00Z"
modified_at = "2024-01-01T00:00:00Z"
file_path = "src/main.rs"
function_name = "handle_request"
original_line = 42
```

## Benefits of Migration

1. **Stability**: Overrides survive code refactoring and line changes
2. **Function-based**: More intuitive keys based on actual functions
3. **Fallback Resolution**: Multiple strategies for finding overrides
4. **Better Metadata**: Rich metadata for debugging and management
5. **Debugging Tools**: Comprehensive inspection and debugging commands

## Troubleshooting

### Function Detection Failed
If function detection fails during migration:
- File might have moved or been deleted
- Syntax errors in the source file
- Complex function structures not recognized

The migration will still succeed with a line-based key as fallback.

### Migration Errors
If migration fails entirely:
- Check file permissions
- Ensure `.raz` directory is writable
- Verify legacy file format is valid TOML

### Backup and Recovery
Always backup your overrides before migration:
```bash
cp .raz/overrides.toml .raz/overrides.toml.manual-backup
```

The migration tool automatically creates backups, but manual backups provide extra safety.

## Post-Migration

After successful migration:

1. **Test Overrides**: Verify your overrides still work as expected
2. **Remove Backup**: Clean up the `.toml.backup` file when confident
3. **Update Documentation**: Update any documentation referencing old keys
4. **Use New Tools**: Take advantage of the new debugging commands:
   ```bash
   raz override list
   raz override debug src/main.rs 42
   raz override stats
   ```