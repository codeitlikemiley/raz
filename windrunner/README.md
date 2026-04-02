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
| **UnifiedRunner** | `crates/core/src/runners/unified_runner.rs` | Framework-aware dispatch (Dioxus → Leptos → Cargo) |
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

## Build System Detection

`UnifiedRunner` dispatches to framework-specific runners before falling back to generic Cargo:

```
detect Dioxus.toml in ancestor dirs  →  DioxusRunner
Cargo.toml mentions leptos            →  LeptosRunner
default                               →  CargoRunner
```

Separate from framework detection, **build system** detection:

```
BUILD.bazel or BUILD present  →  Bazel
Cargo.toml present            →  Cargo
(none)                        →  Rustc (standalone files)
```

---

## ResolverChain

The resolver pipeline evaluates in priority order:

1. `IntegrationTestResolver` — files under `tests/`
2. `BinResolver` — files in `src/bin/`
3. _(more resolvers as needed)_
4. Generic fallback

---

## CommandTemplate DSL

Templates use `{placeholder}` syntax:

```
bazel test {target} --test_output streamed --test_arg --exact --test_arg {test_filter}
```

Render:
```rust
let cmd = CommandTemplate::parse(template_str)?.render(&ctx)?;
```

The `BazelCommandBuilder::expand_template` method now uses this engine (with a `legacy_expand()` fallback if parsing fails).

---

## Testing

```bash
# Run full test suite (139 tests)
cargo test -p cargo-runner-core

# Run a specific test module
cargo test -p cargo-runner-core bazel_builder
```

---

## Bazel Transparent-Proxy CLI (Phase 1)

> **Goal**: use a Bazel-managed Rust workspace as if it were plain Cargo — no manual Bazel bookkeeping.

### Prerequisites

```bash
cargo install cargo-runner  # installs the cargo-runner binary
```

### Commands

| Command | What it does |
|---------|-------------|
| `cargo runner init --bazel [--workspace-name <name>]` | Generate `.cargo-runner.json` pre-populated with Bazel framework defaults |
| `cargo runner add <crate> [--features f] [--dev] [--crate-dir <dir>]` | `cargo add` + `cargo update` + `bazel sync` + `gen_rust_project` in one shot |
| `cargo runner sync [--crate <name>] [--skip-ide]` | Sync Bazel crate-universe after any `Cargo.toml` edit |
| `cargo runner build-sync [--crate <name>] [--dry-run]` | Scaffold / update `BUILD.bazel` targets from the crate's `src/` layout |

### Typical first-time workflow

```bash
# 1. Generate the config
cargo runner init --bazel --workspace-name my_workspace

# 2. Add dependencies the same way you would with plain Cargo
cargo runner add tokio --features full
cargo runner add serde --features derive

# 3. After adding new source files, refresh BUILD.bazel
cargo runner build-sync

# 4. Preview BUILD.bazel changes without writing
cargo runner build-sync --dry-run
```

### `build-sync` safety model

`build-sync` only modifies lines inside a **managed block** — anything outside
the fences is left untouched:

```python
# BEGIN raz-managed — do not edit this block manually
rust_library(...)
rust_test_suite(...)
# END raz-managed
```

Hand-authored targets outside the block are never touched.

### Fallback matrix

| Scenario | Behaviour |
|----------|-----------|
| `cargo test --doc` | Falls back to `cargo test` via `UnifiedRunner` (Bazel has no doc-test support) |
| `cargo runner watch` | Phase 2 — file watcher TBD |
| Non-Bazel workspace | All four commands operate in no-op / warning mode |

---

### VSCode Extension integration

The four commands are also accessible from the IDE:

| Command palette entry | VS Code command ID |
|-----------------------|--------------------|
| RAZ: Sync Bazel Crate-Universe | `raz.bazelSync` |
| RAZ: Add Crate (Bazel) | `raz.bazelAdd` |
| RAZ: Scaffold / Update BUILD.bazel Targets | `raz.buildSync` |
| RAZ: Init Bazel Config (.cargo-runner.json) | `raz.initBazel` |

Clicking the **`$(flame) Bazel`** status-bar badge on any `.rs` file in a Bazel
workspace opens an action quick-pick that exposes all four commands directly.

A **Cargo.toml watcher** (`raz.bazelAutoSync` setting, default `true`) detects
saves in Bazel-managed crates and prompts you to run `cargo runner sync`
automatically.

---

## License

MIT or Apache-2.0, at your option.