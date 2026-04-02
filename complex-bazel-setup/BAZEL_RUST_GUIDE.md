# Bazel + Rust Guide

A practical reference for working with Rust in this Bazel monorepo. This guide reflects the **actual structure** of `complex-bazel-setup/` as it exists today.

---

## Table of Contents

1. [Mental Model: What Changed from Pure Cargo](#mental-model)
2. [Workspace Isolation: The [workspace] Marker Rule](#workspace-isolation)
3. [Adding External Dependencies (cargo add)](#external-deps)
4. [Using Local Crates (No Path Deps)](#local-crates)
5. [Creating New Crates](#creating-new-crates)
6. [BUILD.bazel Patterns Reference](#build-patterns)
7. [MODULE.bazel Patterns Reference](#module-patterns)
8. [IDE Setup: Generating rust-project.json](#ide-setup)
9. [Known Limitations & Fallbacks](#limitations)
10. [Quick Reference](#quick-reference)

---

## Mental Model

Bazel takes over as the **build graph authority**. Cargo is demoted to **dependency resolver only**.

| Role | Cargo | Bazel |
|------|-------|-------|
| External deps (crates.io) | ✅ `Cargo.toml` | reads from Cargo.toml via `crate_universe` |
| Local crate deps | ❌ No path deps! | `"//crate:target"` in BUILD.bazel |
| Build targets | ❌ implicit | ✅ explicit `rust_binary`, `rust_library`, etc. |
| Test running | `cargo test` | `bazel test //pkg:target` |
| IDE support | automatic | `bazel run @rules_rust//tools/rust_analyzer:gen_rust_project -- //...` |

---

## Workspace Isolation

### The Problem

`cargo-bazel` (crate_universe) uses Cargo workspaces to splice dependencies. If `corex` or `server` are accidentally absorbed into the root `raz` monorepo workspace, their crates.io deps become unavailable to Bazel.

### The Rule

Any **standalone Bazel crate** that is NOT a member of a sub-workspace must have a `[workspace]` marker in its `Cargo.toml`:

```toml
# corex/Cargo.toml — standalone crate, isolated from raz monorepo
[workspace]

[package]
name = "corex"
version = "0.1.0"
edition = "2024"
```

```toml
# server/Cargo.toml — standalone crate, isolated from raz monorepo
[workspace]

[package]
name = "server"
version = "0.1.0"
edition = "2024"
```

### Sub-Workspace Members (combos)

Crates that ARE members of a sub-workspace (`combos/`) do **not** need the `[workspace]` marker — they inherit from `combos/Cargo.toml`:

```toml
# combos/Cargo.toml — THE workspace definition
[workspace]
resolver = "2"
members = ["backend", "frontend"]

# combos/backend/Cargo.toml — NO [workspace] marker here
[package]
name = "backend"
```

### Summary

| Crate | Has `[workspace]` marker? | Why |
|-------|--------------------------|-----|
| `corex` | ✅ Yes | Standalone, must not be absorbed |
| `server` | ✅ Yes | Standalone, must not be absorbed |
| `combos` | ✅ Yes (owns the workspace) | Defines workspace |
| `combos/backend` | ❌ No | Member of combos workspace |
| `combos/frontend` | ❌ No | Member of combos workspace |

---

## External Dependencies

### Standard Workflow

```bash
# 1. Navigate to the crate
cd corex

# 2. Add the dependency (same as pure Cargo)
cargo add serde --features derive
cargo add tokio --features full

# 3. Update Cargo.lock
cargo update

# 4. No BUILD.bazel change needed — deps are auto-included via all_crate_deps()

# 5. Sync Bazel's crate universe (picks up the new dep)
cd ..
bazel sync --only=corex_crates   # or just: bazel build //corex/...

# 6. Regenerate IDE files
bazel run @rules_rust//tools/rust_analyzer:gen_rust_project -- //...
```

### Why No BUILD.bazel Change?

`all_crate_deps()` introspects the `Cargo.toml` at build time via `crate_universe`. Any dep in `Cargo.toml` is automatically available in BUILD.bazel through this macro.

```python
# BUILD.bazel — deps are implicit via all_crate_deps()
rust_library(
    name = "corex_lib",
    srcs = ["src/lib.rs"],
    deps = all_crate_deps(),  # ← auto-includes serde, tokio, etc.
)
```

### Dev Dependencies

```bash
cargo add --dev criterion      # bench
cargo add --dev proptest       # property testing
```

Access in BUILD.bazel:
```python
rust_test(
    name = "corex_tests",
    crate = ":corex_lib",
    deps = all_crate_deps(normal_dev = True),  # ← includes dev-deps
)
```

---

## Local Crate Dependencies

> ⚠️ **Critical**: Never use `path = "../corex"` in `Cargo.toml` when using Bazel. It breaks `crate_universe`.

### Correct Pattern: BUILD.bazel Only

If `server` needs to import from `corex`:

```python
# server/BUILD.bazel
rust_binary(
    name = "server_bin",
    srcs = ["src/main.rs"],
    deps = all_crate_deps(normal = True) + [
        "//corex:corex_lib",    # ← local dep via Bazel label
    ],
    crate_root = "src/main.rs",
)
```

In Rust code:
```rust
// server/src/main.rs
use corex::Calculator;  // works because crate_name = "corex" in BUILD.bazel
```

The library must be publicly visible:
```python
# corex/BUILD.bazel
rust_library(
    name = "corex_lib",
    crate_name = "corex",                       # what `use corex::...` maps to
    visibility = ["//visibility:public"],        # allows //server:server_bin to dep on it
    ...
)
```

### Dependency Matrix

| Dependency type | Where to add | Example |
|----------------|--------------|---------|
| crates.io crate | `Cargo.toml` + `bazel sync` | `cargo add tokio` |
| Local Bazel target | `BUILD.bazel` deps only | `"//corex:corex_lib"` |
| Dev-only (crates.io) | `Cargo.toml` `[dev-dependencies]` | `cargo add --dev criterion` |
| ❌ Path dep | Never | `path = "../corex"` → breaks Bazel |

---

## Creating New Crates

### Option A: Standalone Crate (e.g., `payments`)

```bash
mkdir -p payments/src
```

**`payments/Cargo.toml`**:
```toml
[workspace]        # ← REQUIRED for standalone crates

[package]
name = "payments"
version = "0.1.0"
edition = "2024"

[dependencies]
```

**`payments/Cargo.lock`**:
```bash
cd payments && cargo generate-lockfile
```

**`payments/BUILD.bazel`**:
```python
load("@payments_crates//:defs.bzl", "all_crate_deps")
load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")

rust_library(
    name = "payments_lib",
    srcs = ["src/lib.rs"],
    deps = all_crate_deps(),
    visibility = ["//visibility:public"],
    crate_name = "payments",
)

rust_test(
    name = "payments_tests",
    crate = ":payments_lib",
    deps = all_crate_deps(normal_dev = True),
)
```

**`MODULE.bazel`** — add a new `crate.from_cargo` block:
```python
crate.from_cargo(
    name = "payments_crates",
    manifests = ["//payments:Cargo.toml"],
    cargo_lockfile = ["//payments:Cargo.lock"],
)
use_repo(crate, "payments_crates")
```

---

### Option B: Sub-workspace Member (e.g., adding `combos/auth`)

```bash
mkdir -p combos/auth/src
```

**`combos/auth/Cargo.toml`** — NO `[workspace]` marker:
```toml
[package]
name = "auth"
version = "0.1.0"
edition = "2024"

[dependencies]
```

**`combos/Cargo.toml`** — add to members:
```toml
[workspace]
resolver = "2"
members = ["backend", "frontend", "auth"]   # ← add here
```

**`combos/auth/BUILD.bazel`**:
```python
load("@combos_crates//:defs.bzl", "all_crate_deps")
load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")

rust_library(
    name = "auth_lib",
    srcs = ["src/lib.rs"],
    deps = all_crate_deps(),
    visibility = ["//visibility:public"],
    crate_name = "auth",
)

rust_test(
    name = "auth_tests",
    crate = ":auth_lib",
    deps = all_crate_deps(normal_dev = True),
)
```

**`MODULE.bazel`** — add the new manifest to the existing `combos_crates` block:
```python
crate.from_cargo(
    name = "combos_crates",
    manifests = [
        "//combos:Cargo.toml",
        "//combos/backend:Cargo.toml",
        "//combos/frontend:Cargo.toml",
        "//combos/auth:Cargo.toml",          # ← add here
    ],
    cargo_lockfile = "//combos:Cargo.lock",
)
```

---

## BUILD.bazel Patterns Reference

### Pattern: Binary

```python
rust_binary(
    name = "server_bin",
    srcs = ["src/main.rs"],
    crate_root = "src/main.rs",
    deps = all_crate_deps(normal = True),
)
```

### Pattern: Binary in `src/bin/`

```python
rust_binary(
    name = "proxy",
    srcs = ["src/bin/proxy.rs"],
    crate_root = "src/bin/proxy.rs",
    deps = all_crate_deps(normal = True),
)

rust_test(
    name = "proxy_test",
    srcs = ["src/bin/proxy.rs"],
    crate_root = "src/bin/proxy.rs",
    deps = all_crate_deps(normal_dev = True),
)
```

### Pattern: Library

```python
rust_library(
    name = "corex_lib",
    srcs = glob(["src/**/*.rs"]),
    deps = all_crate_deps(),
    visibility = ["//visibility:public"],
    crate_name = "corex",
)
```

### Pattern: Unit Tests (from library)

```python
rust_test(
    name = "unit_tests",
    crate = ":corex_lib",
    deps = all_crate_deps(normal_dev = True),
)
```

### Pattern: Integration Tests (`tests/` dir)

```python
rust_test_suite(
    name = "integration_tests",
    srcs = glob(["tests/**/*.rs"]),
    deps = all_crate_deps(normal_dev = True) + [":corex_lib"],
)
```

### Pattern: Example

```python
rust_binary(
    name = "example_basic",
    srcs = ["examples/basic.rs"],
    crate_root = "examples/basic.rs",
    deps = all_crate_deps(normal = True) + [":corex_lib"],
)
```

### Pattern: Benchmark

```python
rust_binary(
    name = "bench_performance",
    srcs = ["benches/performance.rs"],
    crate_root = "benches/performance.rs",
    deps = all_crate_deps(normal_dev = True) + [":corex_lib"],
)
```

### Pattern: `build.rs`

```python
load("@rules_rust//cargo:defs.bzl", "cargo_build_script")

cargo_build_script(
    name = "build_script",
    srcs = ["build.rs"],
    deps = all_crate_deps(build = True),
)

rust_library(
    name = "mylib",
    ...
    build_script = ":build_script",
)
```

### Pattern: Doc-tests

> ⚠️ **Limited support**. Use `rust_doc_test` only for simple cases. For complex doc-tests (with external deps), fall back to `cargo test --doc`.

```python
load("@rules_rust//rust:defs.bzl", "rust_doc_test")

rust_doc_test(
    name = "corex_doc_test",
    crate = ":corex_lib",
    deps = all_crate_deps(normal_dev = True),
)
```

---

## MODULE.bazel Patterns Reference

### Current `MODULE.bazel` (as of this workspace)

```python
# MODULE.bazel
bazel_dep(name = "rules_rust", version = "0.63.0")

crate = use_extension(
    "@rules_rust//crate_universe:extensions.bzl",
    "crate",
)

# combos sub-workspace (backend + frontend)
crate.from_cargo(
    name = "combos_crates",
    manifests = [
        "//combos:Cargo.toml",
        "//combos/backend:Cargo.toml",
        "//combos/frontend:Cargo.toml",
    ],
    cargo_lockfile = "//combos:Cargo.lock",
)

# corex standalone crate
crate.from_cargo(
    name = "corex_crates",
    manifests = ["//corex:Cargo.toml"],
    cargo_lockfile = "//corex:Cargo.lock",
)

# server standalone crate
crate.from_cargo(
    name = "server_crates",
    manifests = ["//server:Cargo.toml"],
    cargo_lockfile = "//server:Cargo.lock",
)

use_repo(crate, "combos_crates", "corex_crates", "server_crates")
```

Each `crate.from_cargo(name = "<X>_crates", ...)` creates an importable `@<X>_crates//:defs.bzl` for the `all_crate_deps()` macro.

---

## IDE Setup: Generating rust-project.json

rust-analyzer needs `rust-project.json` to understand Bazel-built crates.

```bash
bazel run @rules_rust//tools/rust_analyzer:gen_rust_project -- //...
```

When to regenerate:
- After adding a dependency (cargo add)
- After adding a new crate to the workspace
- After modifying BUILD.bazel targets
- After changing `MODULE.bazel`
- When rust-analyzer shows "unresolved import" for deps that exist

The generated file contains all crate roots, dependency edges, and proc-macro paths. Commit it to git for shared IDE setup.

---

## Known Limitations & Fallbacks

| Scenario | Limitation | Use instead |
|----------|-----------|-------------|
| Doc-tests with complex deps | `rust_doc_test` may fail | `cargo test --doc -p <crate>` |
| Path dependencies in Cargo.toml | Breaks crate_universe splicing | Use Bazel labels in BUILD.bazel |
| `cargo publish` | Not supported | Use `cargo publish` directly |
| Hot-reload dev servers | Bazel doesn't support incremental watch-rebuild | `cargo leptos watch`, `dx serve`, etc. |
| `cargo expand` (macro expansion) | Not available | `cargo expand` in the crate dir |
| `cargo audit` / `cargo deny` | Bazel doesn't run these | `cargo audit` from crate dir |
| Standalone `.rs` files without Cargo.toml | No Bazel target | `rustc file.rs` directly |

`cargo runner run` automatically falls back to the correct tool based on context.

---

## Quick Reference

```bash
# ─── Build ────────────────────────────────────────────
bazel build //...                             # Build everything
bazel build //server:server_bin              # Build one target

# ─── Test ─────────────────────────────────────────────
bazel test //...                             # Test everything
bazel test //corex:unit_tests               # Test one target
bazel test //corex:unit_tests \
  --test_output=streamed \
  --test_arg=--exact \
  --test_arg=test_calculator_add            # Run specific test

# ─── Run ──────────────────────────────────────────────
bazel run //server:server_bin               # Run binary
bazel run //combos/backend:backend_bin

# ─── Dependencies ─────────────────────────────────────
cd corex && cargo add serde --features derive    # Add external dep
cd corex && cargo update                         # Update lock file
bazel sync --only=corex_crates                   # Sync Bazel crate universe

# ─── IDE Support ──────────────────────────────────────
bazel run @rules_rust//tools/rust_analyzer:gen_rust_project -- //...

# ─── Via cargo runner (recommended) ──────────────────
cargo runner run corex/src/lib.rs:67        # Run test at line 67
cargo runner run server/src/main.rs        # Run binary
cargo runner run corex/benches/perf.rs     # Run benchmark
cargo runner run --dry-run server/src/main.rs  # Preview command

# ─── Diagnostic ───────────────────────────────────────
bazel query //...:all                       # List all targets
bazel query 'deps(//server:server_bin)'    # Show dependency graph
bazel info output_base                     # Show Bazel cache location
```

---

## Troubleshooting

| Error | Cause | Fix |
|-------|-------|-----|
| `target X is not visible from Y` | Missing `visibility` | Add `visibility = ["//visibility:public"]` to the library |
| `unresolved import corex` | `crate_name` mismatch | Check `crate_name` in BUILD.bazel matches your `use` statement |
| `failed to load manifest for dependency` | Path dep in Cargo.toml | Remove `path = "../..."`, use Bazel label in BUILD.bazel instead |
| `workspace splicing error` | Missing `[workspace]` marker | Add `[workspace]` to the standalone crate's Cargo.toml |
| IDE shows wrong paths in rust-project.json | Stale file | Re-run `gen_rust_project` |
| `all_crate_deps()` undefined | Wrong `@repo` in load | Check the `name =` in MODULE.bazel matches `@<name>_crates` in BUILD.bazel load |
| New dep not picked up | `bazel sync` not run | Run `bazel sync --only=<crate>_crates` after `cargo update` |