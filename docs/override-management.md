# Override Management Guide

This guide covers RAZ's comprehensive override management system, including the deferred save mechanism, backup/rollback functionality, and all CLI commands for managing saved overrides.

## Table of Contents

- [Understanding Deferred Save](#understanding-deferred-save)
- [Basic Override Commands](#basic-override-commands)
- [Listing and Inspecting Overrides](#listing-and-inspecting-overrides)
- [Backup and Recovery](#backup-and-recovery)
- [Debug and Troubleshooting](#debug-and-troubleshooting)
- [Migration from Legacy Formats](#migration-from-legacy-formats)
- [Configuration and Settings](#configuration-and-settings)
- [Best Practices](#best-practices)

## Understanding Deferred Save

RAZ's deferred save mechanism is a key feature that prevents invalid or failed overrides from persisting in your configuration.

### How It Works

```bash
# Traditional immediate save (problematic)
some_tool --save config --invalid-flag
# ❌ Config saved immediately, even if command fails

# RAZ deferred save (safe)
raz --save-override test.rs --invalid-flag
# 1. Override prepared but not saved
# 2. Command executed with override applied
# 3. Command fails (exit code 1)
# 4. ⚠ Warning Command failed. Override was NOT saved.
```

### Benefits

1. **No Bad Persistence**: Failed commands don't create persistent configurations
2. **Clean State**: Your `.raz/overrides.toml` only contains working configurations
3. **Automatic Validation**: Only proven-working overrides are saved
4. **VS Code Integration**: Failed tasks don't clutter your override config

### Successful Save Example

```bash
raz --save-override src/lib.rs:25:1 RUST_BACKTRACE=1 --exact
# 1. Override prepared: RUST_BACKTRACE=1 --exact
# 2. Command executed: cargo test -- test_name --exact
# 3. Command succeeds (exit code 0)
# 4. ✓ Success Override saved after successful execution
# 5. Key: src/lib.rs:test_function_name
```

## Basic Override Commands

### Saving Overrides

```bash
# Save with environment variables
raz --save-override src/lib.rs:25:1 RUST_BACKTRACE=full RUST_LOG=debug

# Save with cargo options
raz --save-override src/main.rs --release --features web

# Save with test arguments
raz --save-override src/lib.rs:42:10 -- --exact --nocapture --test-threads 1

# Combined example
raz --save-override src/integration.rs:15:5 RUST_BACKTRACE=1 --quiet -- --exact --show-output
```

### Running with Saved Overrides

```bash
# Automatically applies saved overrides
raz src/lib.rs:25:1

# Check what would be executed
raz --dry-run src/lib.rs:25:1

# Verbose output shows override application
raz --verbose src/lib.rs:25:1
```

## Listing and Inspecting Overrides

### List All Overrides

```bash
# Show all saved overrides
raz override list

# Example output:
# Override: src/lib.rs:test_auth
#   Created: 2024-01-15 10:30:00 UTC
#   Function: test_auth
#   Last execution: Success (2024-01-15 11:45:00 UTC)
#   Environment: RUST_BACKTRACE=1
#   Options: --quiet
#   Args: --exact, --nocapture
```

### List Overrides for Specific File

```bash
# Show overrides for specific file
raz override list --file src/lib.rs

# Show overrides for current directory files
raz override list --file .
```

### Inspect Specific Override

```bash
# Detailed view of specific override
raz override inspect src/lib.rs:test_auth

# Example output:
# Override Details: src/lib.rs:test_auth
# =====================================
# Primary Key: src/lib.rs:test_auth
# Fallback Keys: src/lib.rs:L25, src/lib.rs:42
# 
# Command Configuration:
#   Environment Variables:
#     RUST_BACKTRACE = "1"
#     RUST_LOG = "debug"
#   Cargo Options: --quiet, --release
#   Arguments: --exact, --nocapture
# 
# Metadata:
#   Created: 2024-01-15T10:30:00Z
#   Modified: 2024-01-15T11:45:00Z
#   File: src/lib.rs
#   Function: test_auth
#   Line: 25
#   
# Execution History:
#   Last Execution: 2024-01-15T11:45:00Z (Success)
#   Failure Count: 0
#   Status: Validated
```

### Show Override Statistics

```bash
# Overview of all overrides
raz override stats

# Example output:
# Override Statistics
# ==================
# Total Overrides: 15
# Successful: 12 (80%)
# Failed: 2 (13%)
# Pending: 1 (7%)
# 
# Most Used Files:
#   src/lib.rs: 8 overrides
#   src/auth.rs: 4 overrides
#   tests/integration.rs: 3 overrides
# 
# Recent Activity:
#   src/lib.rs:test_login (2 hours ago)
#   src/auth.rs:validate_token (1 day ago)
```

## Backup and Recovery

RAZ automatically creates backups before modifying override configurations, enabling safe recovery from corruption or mistakes.

### Automatic Backups

```bash
# Backups are created automatically when:
# 1. Saving new overrides
# 2. Modifying existing overrides
# 3. Importing override configurations

# Backup location: .raz/backups/
# Format: override_backup_YYYY-MM-DD_HH-MM-SS.toml
```

### List Available Backups

```bash
# Show all available backups
raz override list-backups

# Example output:
# Available backups:
#   1. 2024-01-15 14:30:15 - 12 overrides (1.2 KB)
#   2. 2024-01-15 11:22:08 - 11 overrides (1.1 KB)
#   3. 2024-01-14 16:45:33 - 8 overrides (0.9 KB)
```

### Rollback to Previous State

```bash
# Interactive rollback (asks for confirmation)
raz override rollback

# Force rollback without confirmation
raz override rollback --force

# Example interaction:
# Are you sure you want to rollback to the last backup? [y/N] y
# ✓ Success Successfully rolled back to last backup
# Restored 11 overrides from 2024-01-15 11:22:08
```

### Export and Import

```bash
# Export current overrides for backup
raz override export --output my_overrides_backup.toml

# Export to stdout (for piping)
raz override export

# Import overrides from backup file
raz override import my_overrides_backup.toml

# Example import output:
# ✓ Success Imported 12 overrides from my_overrides_backup.toml
# Note: Existing overrides with same keys were replaced
```

## Debug and Troubleshooting

### Debug Override Resolution

```bash
# Debug resolution at specific location
raz override debug src/lib.rs 25 10

# Example output:
# Debug Override Resolution
# ========================
# File: src/lib.rs
# Position: Line 25, Column 10
# Function Context: test_auth (detected)
# 
# Generated Keys:
#   Primary: src/lib.rs:test_auth
#   Fallbacks: src/lib.rs:L25
# 
# Resolution Candidates:
#   1. src/lib.rs:test_auth (exact match, score: 1.0)
#   2. src/lib.rs:L25 (line fallback, score: 0.8)
# 
# Selected Override: src/lib.rs:test_auth
# Strategy: ExactFunctionMatch
```

### Update Execution Status

```bash
# Mark override as successful
raz override update-status src/lib.rs:test_auth success

# Mark override as failed
raz override update-status src/lib.rs:test_auth failure

# Example output for failure:
# ⚠ Warning Override marked as failed
# ⚠ Warning This override has failed 3 times and may need attention
```

### Delete Specific Override

```bash
# Delete specific override
raz override delete src/lib.rs:test_auth

# Example output:
# ✓ Success Deleted override: src/lib.rs:test_auth
```

### Clear All Overrides

```bash
# Clear all overrides (asks for confirmation)
raz override clear

# Force clear without confirmation
raz override clear --force

# Example interaction:
# This will delete ALL saved overrides. Are you sure? [y/N] y
# ✓ Success Cleared all overrides (15 deleted)
# Backup created: .raz/backups/override_backup_2024-01-15_15-30-45.toml
```

## Migration from Legacy Formats

RAZ provides tools to migrate from older override formats and configurations.

### Auto-Migration

```bash
# Automatically detect and migrate legacy overrides
raz override migrate --auto

# Example output:
# ✓ Migration completed!
# 
# Migration Summary:
# =================
# Legacy file: .raz/config.toml
# Migrated: 8 overrides
# Skipped: 2 (invalid format)
# 
# New format: .raz/overrides.toml
# Backup created: .raz/backups/legacy_backup_2024-01-15_15-45-12.toml
```

### Manual Migration

```bash
# Preview migration without changes
raz override migrate --file old_config.toml --dry-run

# Perform migration from specific file
raz override migrate --file old_config.toml

# Example dry-run output:
# ⚠ Warning Dry run mode - no changes will be made
# 
# Migration Plan:
# ===============
# Source: old_config.toml
# Would migrate:
#   old_key_format_1 → src/lib.rs:test_function
#   old_key_format_2 → src/auth.rs:validate_user
# 
# Would skip:
#   invalid_key_format (unsupported format)
```

## Configuration and Settings

### Global Configuration

Override behavior can be configured in your global RAZ config:

```toml
# ~/.config/raz/config.toml

[overrides]
# Number of automatic backups to keep (default: 5)
backup_count = 10

# Auto-rollback threshold - prompt for rollback after N failures (default: 3)
auto_rollback_threshold = 5

# Validation level for new overrides (default: "warning")
validation_level = "strict"  # "off", "warning", "strict"
```

### Workspace Configuration

```toml
# .raz/config.toml (workspace-specific)

[overrides]
# Override global settings for this workspace
auto_rollback_threshold = 2
validation_level = "warning"
```

## Best Practices

### 1. Regular Backups

```bash
# Create manual backups before major changes
raz override export --output project_overrides_$(date +%Y%m%d).toml
```

### 2. Use Descriptive Override Keys

```bash
# Good: Function-specific overrides
raz --save-override src/auth.rs:test_login_success -- --exact

# Less ideal: Line-based overrides (harder to maintain)
raz --save-override src/auth.rs:42:1 -- --exact
```

### 3. Test Before Saving

```bash
# Always test your override before saving
raz src/lib.rs:25:1 RUST_BACKTRACE=1 --quiet

# If it works, then save it
raz --save-override src/lib.rs:25:1 RUST_BACKTRACE=1 --quiet
```

### 4. Monitor Override Health

```bash
# Regularly check override statistics
raz override stats

# Clean up failed or unused overrides
raz override list | grep "Failed\|Never used"
```

### 5. Version Control Considerations

```bash
# Consider tracking override config in git for team projects
git add .raz/overrides.toml

# Or ignore if overrides are developer-specific
echo ".raz/overrides.toml" >> .gitignore
```

### 6. Documentation

Document project-specific overrides in your README:

```markdown
## Development Overrides

Common RAZ overrides for this project:

```bash
# Run tests with full output
raz --save-override src/lib.rs:test_integration RUST_BACKTRACE=full -- --nocapture

# Run with database features
raz --save-override src/main.rs --features database,migrations
```

This guide provides comprehensive coverage of RAZ's override management capabilities. For more specific use cases or advanced configurations, see the [Advanced Usage Guide](advanced-usage.md).