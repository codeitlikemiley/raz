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

## Deferred Save System

The override system implements a deferred save pattern - overrides are only persisted after successful command execution. This prevents failed configurations from polluting the override store.

### How It Works
1. Override data is prepared but not saved immediately
2. Command is executed with the override applied
3. **Only if the command succeeds** is the override saved to config
4. Failed commands leave no persistent configuration

## Usage Examples

### Library Usage
```rust
use raz_override::{SmartOverrideParser, OverrideSystem};
use raz_config::CommandOverride;

// Parse runtime overrides
let parser = SmartOverrideParser::new("test");
let overrides = parser.parse("RUST_BACKTRACE=full --release -- --exact");

// Create override system for workspace
let mut override_system = OverrideSystem::new(&workspace_path)?;

// Get function context and save override (deferred save pattern)
let function_context = override_system.get_function_context(file_path, 24, Some(0))?;
let override_key = override_system.generate_key(&function_context)?;

// Only save after successful execution
if command_succeeded {
    override_system.save_override_with_validation(
        override_key,
        command_override,
        &function_context,
        "test"
    )?;
}
```

## Library API

The crate provides comprehensive APIs for override management:
- List and inspect overrides
- Debug override resolution
- Export/import functionality
- Migration from legacy formats
- Statistics and reporting

For CLI usage, see the [raz-cli](https://crates.io/crates/raz-cli) documentation.

## Migration from Legacy Overrides

The crate includes migration functionality for converting legacy cursor position-based overrides to the new stable key format. Migration can be performed programmatically through the API.

See [migration-guide.md](docs/migration-guide.md) for the migration format details.