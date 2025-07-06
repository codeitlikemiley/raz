# RAZ VS Code Extension - Installation Guide

## Quick Installation

The RAZ VS Code extension has been packaged with a simplified, cleaner UI for better user experience.

### Installation Steps

1. **Install the VSIX file:**
   ```bash
   code --install-extension raz-vscode-0.2.2.vsix
   ```

2. **Restart VS Code** to ensure the extension loads properly.

3. **Open a Rust project** to activate the extension.

## What's New & Fixed

### Override Commands
- ✅ **List All Overrides** - Shows picker to select project when multiple .raz directories exist
- ✅ **Show Override Statistics** - Properly detects project context from active file or shows picker
- ✅ **Clear Saved Overrides** - Shows picker to select which project to clear overrides from

### Simplified Tree View
- ✅ **Cleaner UI** - Less nesting, shows only what matters
- ✅ **Direct Function Names** - Shows just the function name without file details
- ✅ **One-Line Override Display** - Shows `▶  -- --test-threads=1` format
- ✅ **Simple Interactions**:
  - **Click** the item to run with the override
  - **Right-click** → Edit to modify the override
- ✅ **Workspace Aggregation** - Workspaces show all member crate overrides directly

## Using the Simplified UI

### Tree View Structure
```
⚡ RAZ: OVERRIDES
  taxman (2 overrides)              ← Workspace total count, not expandable
    > taxman-domain (1 override)    ← Member crate, expandable
    v taxman (1 override)           ← Member crate, expandable
      v test_create_transaction_simple
        ▶  -- --test-threads=1
```

### How to Use
1. Click the ⚡ (RAZ) icon in the activity bar
2. Expand projects to see function overrides
3. Expand functions to see their override settings
4. **Click** `▶  -- --test-threads=1` to run
5. **Right-click** → Edit to modify the override

### Key Improvements
- **Flattened workspace structure** - Workspace shows total count, member crates as siblings
- **No duplicate nesting** - Clean hierarchy without redundant folders
- **Clean function names** - Just shows the function name
- **Single action line** - Play button and args in one line
- **Direct execution** - Click to run, right-click to edit

## Configuration Hierarchy

The extension supports hierarchical configuration:
- **Project** (.raz in project root) - Highest priority
- **Workspace** (.raz in workspace root) - Medium priority  
- **Global** (~/.raz) - Lowest priority

## Tips

- The `▶` indicates a runnable override
- Hover over items to see tooltips with full details
- The tree automatically refreshes when override files change

## Troubleshooting

If you don't see your overrides:
1. Make sure you're in a Rust project with saved overrides
2. Check that `.raz/overrides.toml` exists in your project
3. Try refreshing the tree view with the refresh button
4. Check the Output panel (View → Output → RAZ) for any errors

## Need Help?

- Check the [README](readme.md) for detailed usage instructions
- Report issues at: https://github.com/codeitlikemiley/raz/issues