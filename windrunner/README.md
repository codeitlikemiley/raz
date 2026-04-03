# windrunner

The core build engine for the `raz` monorepo. Handles command generation, build-system detection, framework dispatch, and per-function override resolution for Cargo, Bazel, Rustc, and single-file-script targets.

## Architecture

```
windrunner/
└── crates/
    ├── core/          ← cargo-runner-core  (command engine, config, runners)
    └── cli/           ← cargo-runner-cli   (CLI front-end)
```

### Key subsystems

| Subsystem | Path | Description |
|-----------|------|-------------|
| **ResolverChain** | `crates/core/src/command/resolver/` | Composable resolver pipeline replacing legacy if/else dispatch |
| **CommandTemplate** | `crates/core/src/command/template/` | DSL-based template engine (`{target}`, `{test_filter}`, etc.) |
| **BazelCommandBuilder** | `crates/core/src/command/builder/bazel/` | Bazel command generation via `CommandTemplate::parse().render()` |
| **UnifiedRunner** | `crates/core/src/runners/unified_runner.rs` | Build-system & framework-aware dispatch |
| **DioxusRunner** | `crates/core/src/runners/dioxus_runner.rs` | Dioxus-specific `dx` command runner |
| **LeptosRunner** | `crates/core/src/runners/leptos_runner.rs` | Leptos-specific `cargo-leptos` runner |
| **Config** | `crates/core/src/config/` | v2 schema: `BazelConfig`, `BazelOverride`, `Override` |

---

## Configuration Reference

Configuration lives in `.cargo-runner.json` at your crate root.

### Top-level shape

```json
{
  "bazel": { ... },
  "cargo": { ... },
  "overrides": [ ... ]
}
```

### Bazel project config (`"bazel"`)

Used at the project level to configure how Bazel commands are built.

```json
{
  "bazel": {
    "workspace": "my_workspace",
    "test_framework": {
      "command": "bazel",
      "subcommand": "test",
      "target": "{target}",
      "args": ["--test_output", "streamed"],
      "test_args": ["--nocapture", "--exact", "{test_filter}"]
    },
    "binary_framework": {
      "command": "bazel",
      "subcommand": "run",
      "target": "{target}"
    },
    "benchmark_framework": {
      "command": "bazel",
      "subcommand": "test",
      "target": "{target}",
      "args": ["--test_output", "streamed", "--test_arg", "--bench"],
      "test_args": ["{bench_filter}"]
    }
  }
}
```

Supported template placeholders: `{target}`, `{test_filter}`, `{bench_filter}`, `{file_name}`.

### Per-function overrides (`"overrides"`)

The `overrides` array lets you customize commands for specific functions, tests, or files. Each entry has a `"match"` key and a command-type block.

#### Cargo override

```json
{
  "match": {
    "function_name": "my_slow_test",
    "package": "server"
  },
  "cargo": {
    "extra_args": ["--test-threads=1"],
    "env": { "RUST_BACKTRACE": "1" }
  }
}
```

#### Bazel override — **flat shape** (v2)

The `"bazel"` block inside an override is a **flat `BazelOverride`** — fields are promoted from the framework level directly to the override. You no longer need to nest `test_framework.test_args`.

```json
{
  "match": {
    "function_name": "it_works_too",
    "module_path": "tests",
    "package": "frontend"
  },
  "bazel": {
    "test_args": ["--nocapture"]
  }
}
```

All `BazelOverride` fields:

| Field | Type | Description |
|-------|------|-------------|
| `command` | `string` | Override the Bazel binary (e.g. `"bazelisk"`). Display/tooling only — not yet applied at runtime. |
| `subcommand` | `string` | Override the subcommand (`"test"`, `"run"`, `"build"`). |
| `target` | `string` | Override the target label. |
| `args` | `string[]` | Replace the base args block (after subcommand + target). |
| `extra_args` | `string[]` | Append verbatim args after the base args. |
| `test_args` | `string[]` | Inject as `--test_arg <value>` pairs. |
| `exec_args` | `string[]` | Append after `--` separator (for `bazel run`). |
| `extra_env` | `object` | Merge extra environment variables. |

> **Migration note**: The previous nested form `"bazel": { "test_framework": { "test_args": [...] } }` inside overrides **is no longer valid**. Update to the flat shape above.

---

## Build System & Framework Detection

`UnifiedRunner` checks for framework-specific CLIs first, then falls back to build-system detection:

```
┌─ Framework detection (highest priority) ──────────────────────┐
│  Dioxus.toml in ancestor dirs    →  DioxusRunner  (dx CLI)    │
│  "leptos" in Cargo.toml          →  LeptosRunner  (cargo-leptos)
└───────────────────────────────────────────────────────────────┘
         │ (no framework detected)
         ▼
┌─ Build system detection ──────────────────────────────────────┐
│  MODULE.bazel present            →  BazelRunner               │
│  Cargo.toml present              →  CargoRunner               │
│  (none)                          →  RustcRunner (standalone)   │
└───────────────────────────────────────────────────────────────┘
```

### Framework vs Bazel — design boundary

Framework-managed projects (Dioxus, Leptos, Tauri) **always use their native CLI**, never Bazel. These frameworks orchestrate WASM compilation, asset bundling, hot-reload dev servers, and platform-specific builds internally — capabilities that Bazel cannot replicate.

Bazel support targets **pure Rust projects**: API servers, CLI tools, libraries, and monorepos with shared dependency graphs.

| Framework | CLI | Bazel support? |
|-----------|-----|----------------|
| Dioxus | `dx serve / dx build` | ❌ Not supported — use `dx` |
| Leptos | `cargo leptos watch / build` | ❌ Not supported — use `cargo-leptos` |
| Tauri | `cargo tauri dev / build` | ❌ Not supported — use Tauri CLI |
| Pure Rust (lib, bin, tests) | `cargo` or `bazel` | ✅ Fully supported |

---

## Bazel — One-Command Workflow

> **Goal**: Use a Bazel-managed Rust workspace as if it were plain Cargo — no manual Bazel bookkeeping.

### Prerequisites

```bash
cargo install cargo-runner  # installs the cargo-runner binary
```

### `cargo runner init --bazel`

This is the **single entry point** for all Bazel scaffolding. It handles both initial setup and subsequent syncs.

#### First run — scaffolds the workspace

```bash
cargo runner init --bazel
```

Generates:
- `MODULE.bazel` — bzlmod dependency graph via `crate.from_cargo()`
- `.bazelversion` — pins Bazel 7.4.1
- `.bazelrc` — build flags + shared disk/repo caches
- `BUILD.bazel` — targets for each crate (see below)
- `Cargo.lock` — required by `crate_universe`
- `.cargo-runner.json` — framework defaults

Then runs:
1. `bazel sync` — downloads toolchain + resolves crate deps
2. `bazel build --nobuild //...` — validates all BUILD files without compiling

#### Re-run — idempotent sync

```bash
cargo runner init --bazel   # safe to re-run anytime
```

Re-scans source files, adds missing targets, skips existing ones. The single command replaces the old `build-sync` workflow.

#### Workspace support

For Cargo workspaces, `init --bazel` automatically:
- Parses `[workspace] members` (supports explicit lists and globs)
- Generates per-member `BUILD.bazel` files
- Creates a unified `MODULE.bazel` at the root

### Target inference

`init --bazel` uses a **combined** strategy for discovering Bazel targets:

| Source | Strategy | Target generated |
|--------|----------|-----------------|
| `src/lib.rs` | Always | `rust_library` + `rust_test` (unit tests) + `rust_doc_test` |
| `src/main.rs` | Always | `rust_binary` |
| `src/bin/*.rs` | Only if `fn main()` present | `rust_binary` per file |
| `src/bin/*/main.rs` | Subdirectory binaries | `rust_binary` per dir |
| `tests/*.rs` | Always (harness provides entry) | `rust_test_suite` |
| `examples/*.rs` | Only if `fn main()` present | `rust_binary` |
| `benches/*.rs` | Only if `fn main()` present | `rust_binary` |
| `build.rs` | Always | `cargo_build_script` + warning |
| `Cargo.toml` `[[bin]]` | Explicit definitions win | `rust_binary` per entry |
| `Cargo.toml` `[[test]]` | Explicit definitions | `rust_test_suite` |
| `Cargo.toml` `[[bench]]` | Explicit definitions | `rust_binary` per entry |
| `Cargo.toml` `[[example]]` | Explicit definitions | `rust_binary` per entry |

**Priority**: Explicit `Cargo.toml` definitions always win over filesystem convention.

**`fn main()` heuristic**: Files in `src/bin/` and `examples/` are only scaffolded as binaries if they contain `fn main()` — helper modules are silently skipped.

### Doctests

Library crates (`src/lib.rs`) automatically get a `rust_doc_test` target:

```python
rust_doc_test(
    name = "doc_tests",
    crate = ":my_lib",
)
```

Doctests use Bazel natively. There is **no cargo fallback** — if it's a Bazel project, everything goes through Bazel.

### BUILD.bazel safety model

`build-sync` only modifies lines inside a **managed block** — anything outside the fences is left untouched:

```python
# Hand-authored rules above are NEVER touched

# BEGIN raz-managed — do not edit this block manually
rust_library(...)
rust_test(...)
rust_doc_test(...)
# END raz-managed
```

Deduplication is name-aware and content-aware:
- Exact name matches are skipped
- Any existing `rust_doc_test(` rule (regardless of name) prevents duplicate doc test targets

### Other commands

| Command | What it does |
|---------|-------------|
| `cargo runner add <crate> [--features f] [--dev]` | `cargo add` + `cargo update` + `bazel sync` + `gen_rust_project` in one shot |
| `cargo runner sync [--crate <name>] [--skip-ide]` | Sync Bazel crate-universe after any `Cargo.toml` edit |
| `cargo runner build-sync [--crate <name>] [--dry-run]` | Update `BUILD.bazel` targets (also runs as part of `init --bazel`) |
| `cargo runner clean` | Context-aware clean: `bazel clean` (Bazel) or `cargo clean` (Cargo) |
| `cargo runner watch` | Context-aware file watcher: `ibazel` (Bazel) or `cargo watch` (Cargo) |
| `cargo runner run <file>:<line>` | Scope-based execution: detects build system and runs the target at the given line |

---

## Scoped Execution

`cargo runner run path/to/file.rs:25` works identically for both Cargo and Bazel projects:

```
1. Parse file:line → find the smallest scope containing that line
2. Detect build system (Bazel or Cargo)
3. Generate the right command with test filter / bench filter
```

| What you cursor into | Cargo generates | Bazel generates |
|---------------------|-----------------|-----------------|
| `#[test] fn test_add()` | `cargo test test_add --exact` | `bazel test //:unit_tests --test_arg="test_add"` |
| `mod tests { }` block | `cargo test tests::` | `bazel test //:unit_tests --test_arg="tests::"` |
| `/// ``` doctest` | `cargo test --doc add` | `bazel test //:doc_tests` |
| `fn main()` binary | `cargo run --bin name` | `bazel run //:name` |
| Benchmark function | `cargo bench name` | `bazel run //:bench_name -c opt` |

---

## ResolverChain

The resolver pipeline evaluates in priority order:

1. `IntegrationTestResolver` — files under `tests/`
2. `BinResolver` — files in `src/bin/`
3. _(more resolvers as needed)_
4. Generic fallback

---

## CommandTemplate DSL

Templates use `{placeholder}` syntax with conditionals:

```
{cmd?bazel} test {target} {?test_output:--test_output={test_output}} {?test_filter:--test_arg=--exact --test_arg={test_filter}}
```

Render:
```rust
let cmd = CommandTemplate::parse(template_str)?.render(&ctx)?;
```

---

## Testing

```bash
# Run full test suite
cargo test -p cargo-runner-core

# Run a specific test module
cargo test -p cargo-runner-core bazel_builder
```

---

### VSCode Extension integration

Commands are accessible from the IDE:

| Command palette entry | VS Code command ID |
|-----------------------|--------------------|
| RAZ: Init Bazel Config | `raz.initBazel` |
| RAZ: Sync Bazel Crate-Universe | `raz.bazelSync` |
| RAZ: Add Crate (Bazel) | `raz.bazelAdd` |
| RAZ: Scaffold / Update BUILD.bazel Targets | `raz.buildSync` |

Clicking the **`$(flame) Bazel`** status-bar badge on any `.rs` file in a Bazel
workspace opens an action quick-pick that exposes all commands directly.

A **Cargo.toml watcher** (`raz.bazelAutoSync` setting, default `true`) detects
saves in Bazel-managed crates and prompts you to run `cargo runner sync`
automatically.

---

## License

MIT or Apache-2.0, at your option.