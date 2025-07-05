# Override Debugging and Inspection Commands

The raz-override crate provides comprehensive debugging and inspection tools to help understand and manage command overrides.

## CLI Commands

### List Overrides

List all overrides in the workspace:
```bash
raz override list
```

List overrides for a specific file:
```bash
raz override list --file src/main.rs
```

### Inspect Override

Get detailed information about a specific override:
```bash
raz override inspect "src/main.rs:handle_request"
```

### Debug Resolution

Debug how overrides are resolved at a specific location:
```bash
raz override debug src/main.rs 42      # Debug at line 42
raz override debug src/main.rs 42 10   # Debug at line 42, column 10
```

This command shows:
- Detected function context
- All resolution candidates with scores
- The resolution strategy used
- Final resolved override

### Statistics

View statistics about your overrides:
```bash
raz override stats
```

Shows:
- Total number of overrides
- Overrides grouped by file
- Function-level vs file-level overrides
- Environment variables used

### Export/Import

Export overrides for backup or sharing:
```bash
raz override export                    # Print to stdout
raz override export -o overrides.toml  # Save to file
```

Import overrides from a file:
```bash
raz override import overrides.toml
```

### Delete Override

Remove a specific override:
```bash
raz override delete "src/main.rs:handle_request"
```

### Clear All

Remove all overrides (with confirmation):
```bash
raz override clear        # Will prompt for confirmation
raz override clear --force # Skip confirmation
```

## Programmatic API

The debugging functionality is also available through the Rust API:

```rust
use raz_override::{OverrideInspector, OverrideCli};
use std::path::Path;

// Create an inspector
let inspector = OverrideInspector::new(Path::new("/path/to/workspace"))?;

// List all overrides with details
inspector.list_all_detailed()?;

// Debug resolution at a specific location
inspector.debug_resolution(
    Path::new("src/main.rs"),
    42,  // line
    Some(10),  // column (optional)
)?;

// Show statistics
inspector.show_statistics()?;
```

## Resolution Strategies

When debugging resolution, you'll see these strategies:

1. **Exact Key Match** - Direct match on primary key
2. **Fallback Key** - Match on any fallback key
3. **Fuzzy Function Match** - Fuzzy string matching on function name
4. **Line Proximity** - Match based on line number proximity
5. **Hash Fallback** - Ultimate fallback using content hash

## Example Debug Output

```
Resolution Debug Information:

Context:
  File: src/main.rs
  Line: 42
  Column: 10
  Detected Function: handle_request

Found 3 resolution candidates:

1. src/main.rs:handle_request (score: 1.00)
   Strategy: Exact Key Match
   Function: handle_request
   Original Line: 42

2. src/main.rs:handle_req (score: 0.85)
   Strategy: Fuzzy Function Match
   Function: handle_req
   Original Line: 38

3. src/main.rs:40-50 (score: 0.70)
   Strategy: Line Proximity
   Function: N/A
   Original Line: 45

Final Resolution:
  Override Key: debug-test
  Strategy: Exact Key Match
```