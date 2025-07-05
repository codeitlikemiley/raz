# VS Code Extension: Deferred Override Save

## Problem Fixed

Previously, when using `Cmd+Shift+R` to run with override:
1. The override was saved IMMEDIATELY before execution
2. If the command failed, the bad override was already saved
3. Pressing `Cmd+R` would use the bad saved override and fail again

## New Behavior

Now, when using `Cmd+Shift+R`:
1. Override is NOT saved immediately - it's stored as "pending"
2. Command executes with the override
3. **If successful**: Override is saved automatically
4. **If failed**: Override is discarded, never saved

## Benefits

- **No bad overrides persist**: Failed overrides are never saved
- **Cmd+R always safe**: Won't use a bad override from a previous failure
- **Automatic validation**: Only working overrides get saved
- **Better UX**: No need to manually rollback failed overrides

## How It Works

### When you press `Cmd+Shift+R`:
```
1. Enter override: --nocapture (missing --)
2. Command executes: cargo test --nocapture ❌ FAILS
3. Result: Override NOT saved, error shown
4. Press Cmd+R: Runs original command (no bad override)
```

### With correct override:
```
1. Enter override: -- --nocapture
2. Command executes: cargo test -- --nocapture ✅ SUCCESS
3. Result: Override saved automatically
4. Press Cmd+R: Uses saved override
```

## Technical Details

- Pending overrides stored in `workspaceState`
- Task execution monitoring via `onDidEndTaskProcess`
- Save only triggered on exit code 0 (success)
- Failed overrides automatically discarded

## Messages

**On Success**:
- "Override saved after successful execution for [filename]"

**On Failure**:
- "Command failed with exit code X. Override was not saved."

This ensures that only validated, working overrides are persisted!