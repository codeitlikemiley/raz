---
name: Performance Issue
about: Report slow performance, hangs, or resource usage problems
title: '[PERFORMANCE] '
labels: performance, bug
assignees: ''
---

## Performance Issue Type
<!-- Check the type of performance issue -->

- [ ] Slow command detection/generation
- [ ] Extension feels sluggish
- [ ] High CPU usage
- [ ] High memory usage
- [ ] Long startup time
- [ ] File parsing taking too long
- [ ] Override resolution slow

## Description
<!-- Describe the performance problem -->

**What feels slow:** [e.g., pressing Cmd+R, opening override dialog, etc.]
**Expected duration:** [e.g., < 1 second]
**Actual duration:** [e.g., 5-10 seconds]

## Performance Data

### Timing Information
```bash
# Run with timing: time raz "/path/to/file.rs:line:column"
[paste timing output]
```

### System Resources
<!-- While the issue is occurring -->

**CPU Usage:** [e.g., 80% for 10 seconds]
**Memory Usage:** [e.g., 500MB RAM]
**Disk Usage:** [e.g., high I/O during operation]

### VS Code Performance
<!-- If applicable -->

**Extension Host CPU:** [Check VS Code's Developer Tools]
**Extension Memory:** [Check VS Code's process explorer]

## Project Information

**Project size:**
- Files: [approximate number of .rs files]
- Lines of code: [approximate]
- Dependencies: [number in Cargo.toml]
- Workspace members: [if applicable]

**Project type:** [Cargo workspace, large monorepo, many dependencies, etc.]

<details>
<summary>Cargo.toml (dependencies section)</summary>

```toml
[dependencies]
[paste dependencies - especially if many/large ones]
```
</details>

## Reproduction Steps
1. [Steps to reproduce the slow behavior]
2. [Be specific about timing]

## Environment
- OS: [e.g., macOS 14.5, Ubuntu 22.04, Windows 11]
- Hardware: [e.g., M1 Mac, Intel i7, 16GB RAM]
- Raz Version: [Run `raz --version`]
- VS Code Version: [if applicable]
- Rust toolchain: [Run `rustc --version`]

## Debug Information

**With verbose logging:**
```bash
# Output of: time RUST_LOG=debug raz "/path/to/file.rs:line:column"
[paste output focusing on timing information]
```

**Profile information:**
<!-- If you can generate profiles -->
```bash
# Any profiling data if available
```

## Expected Performance
<!-- What performance would you expect? -->

**Expected behavior:** [e.g., Command detection should be < 1 second]
**Comparison:** [e.g., "Other similar tools take X seconds"]

## Additional Context
- **When did this start:** [Always slow, recent regression, specific project]
- **Workarounds:** [Anything that makes it faster]
- **Pattern:** [Consistent slow, gets slower over time, first run slow, etc.]