# raz-override

Override management system for raz with stable key generation that doesn't depend on exact cursor position.

## Features

- **Stable Key Generation**: Uses function signatures and AST analysis for consistent override keys
- **Fuzzy Function Detection**: Intelligently detects function boundaries even with imprecise cursor positions
- **Fallback Resolution**: Multi-level key resolution for finding the best matching override
- **Tree-sitter Integration**: Accurate AST-based function signature extraction
- **Debug Commands**: Comprehensive tools for inspecting and managing overrides
- **Override Preview**: See changes before applying them
- **Export/Import**: Backup and share override configurations

## Key Components

### Override Key Generation
- Function signature-based keys (e.g., `fn:test_my_function`)
- File + line range keys for non-function contexts
- Hierarchical fallback resolution

### Function Detection
- Tree-sitter based AST analysis
- Fuzzy boundary detection within tolerance
- Support for nested functions and closures

### Override Resolution
1. Try exact function signature match
2. Try file + expanded line range
3. Try file-level override
4. Try workspace default

## Debugging and Inspection

The crate includes comprehensive debugging tools:

### CLI Commands
- `raz override list` - List all overrides
- `raz override inspect <key>` - Inspect specific override
- `raz override debug <file> <line> [column]` - Debug resolution
- `raz override stats` - Show statistics
- `raz override export/import` - Backup and restore
- `raz override migrate` - Migrate legacy overrides

See [debugging-commands.md](docs/debugging-commands.md) for detailed usage.

## Migration from Legacy Overrides

If you have existing overrides using the old cursor position-based format, use the migration tool:

```bash
# Auto-detect and migrate legacy overrides
raz override migrate --auto

# Preview migration changes
raz override migrate --file legacy.toml --dry-run

# Migrate specific file
raz override migrate --file legacy.toml
```

See [migration-guide.md](docs/migration-guide.md) for complete migration instructions.