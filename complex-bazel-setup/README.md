# complex-bazel-setup

A reference Bazel + Rust monorepo workspace used to validate the `raz` / `windrunner` Bazel integration. Contains real multi-crate targets (library, binary, integration tests, examples, benchmarks) to exercise the full `cargo runner` command generation path.

## Structure

```
complex-bazel-setup/
├── MODULE.bazel              ← Bzlmod module definition
├── WORKSPACE                 ← Legacy workspace bootstrap (for compat)
├── Cargo.toml                ← Virtual workspace (no [package])
├── Cargo.lock
├── .cargo-runner.json        ← Top-level raz config (v2 schema)
│
├── corex/                    ← Library crate
│   ├── BUILD.bazel
│   ├── Cargo.toml            ← [workspace] marker (isolated from parent)
│   └── src/lib.rs
│
├── server/                   ← Binary crate with axum
│   ├── BUILD.bazel
│   ├── Cargo.toml            ← [workspace] marker (isolated from parent)
│   ├── src/
│   │   ├── main.rs
│   │   └── bin/proxy.rs
│   ├── examples/axum.rs
│   ├── benches/fibonacci_benchmark.rs
│   └── tests/just_test.rs
│
└── combos/                   ← Virtual sub-workspace
    ├── Cargo.toml            ← [workspace] with members: [backend, frontend]
    ├── backend/
    │   ├── BUILD.bazel
    │   ├── Cargo.toml        ← member of combos workspace (no [workspace] marker)
    │   └── src/main.rs
    └── frontend/
        ├── BUILD.bazel
        ├── Cargo.toml        ← member of combos workspace (no [workspace] marker)
        ├── src/main.rs
        └── .cargo-runner.json   ← per-crate config with flat BazelOverride
```

## Workspace Isolation

Each standalone Bazel crate (`corex`, `server`) has a `[workspace]` marker in `Cargo.toml` to prevent `cargo-bazel` from absorbing them into the parent `raz` monorepo workspace during splicing.

The `combos/` directory is a sub-workspace with `combos/Cargo.toml` owning `backend` and `frontend` as virtual members — those two **do not** have `[workspace]` markers.

## Regenerating rust-project.json

```bash
cd complex-bazel-setup
bazel run @rules_rust//tools/rust_analyzer:gen_rust_project -- //...
```

This produces `rust-project.json` with 8 workspace members, all pointing to the correct absolute paths under this directory.

## Configuration (`.cargo-runner.json`)

### Schema version

All config files have been migrated to the **v2 schema**.

### Top-level config example (`complex-bazel-setup/.cargo-runner.json`)

```json
{
  "bazel": {
    "workspace": "complex_bazel_setup",
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
    }
  },
  "overrides": []
}
```

### Per-function override example (`combos/frontend/.cargo-runner.json`)

The `"bazel"` block inside overrides uses the **flat `BazelOverride` shape** — fields are promoted directly, no need to nest inside `test_framework`:

```json
{
  "bazel": { ... },
  "overrides": [
    {
      "match": {
        "file_path": ".../combos/frontend/src/main.rs",
        "function_name": "it_works_too",
        "module_path": "tests",
        "package": "frontend"
      },
      "bazel": {
        "test_args": ["--nocapture"]
      }
    }
  ]
}
```

> ⚠️ The old nested form `"bazel": { "test_framework": { "test_args": [...] } }` inside overrides is no longer valid. Use the flat shape above.

## Using `cargo runner`

```bash
# Run any file — detects target type automatically
cargo runner run corex/src/lib.rs

# Run at a specific line (test detection)
cargo runner run corex/src/lib.rs:67
# → bazel test //corex:unit_tests --test_output streamed --test_arg --exact --test_arg test_calculator_add

# Dry-run (preview command without executing)
cargo runner run --dry-run server/src/main.rs
# → bazel run //server:server_bin

# Run binary in src/bin/
cargo runner run server/src/bin/proxy.rs
# → bazel run //server:proxy

# Run integration test
cargo runner run corex/tests/integration_test.rs:25
# → bazel test //corex:test_integration_test --test_output streamed ...

# Run benchmark
cargo runner run corex/benches/performance.rs
# → bazel run //corex:bench_performance
```

## Quick Bazel Commands

```bash
# Build everything
bazel build //...

# Test everything
bazel test //...

# Run specific binary
bazel run //server:server_bin
bazel run //combos/backend:backend_bin

# Run specific test target
bazel test //corex:unit_tests --test_output=streamed

# Regenerate IDE config
bazel run @rules_rust//tools/rust_analyzer:gen_rust_project -- //...
```

## Known Limitations

- **Doc-tests**: Not supported in Bazel. Convert doc-tests to `#[test]` unit tests within the same module.
- **Path dependencies in Cargo.toml**: Not supported — use Bazel `BUILD.bazel` deps instead.
- **Cargo.lock updates**: Run `cargo update` inside the individual crate directory, then re-run `gen_rust_project`.

For more details, see [BAZEL_RUST_GUIDE.md](./BAZEL_RUST_GUIDE.md).
