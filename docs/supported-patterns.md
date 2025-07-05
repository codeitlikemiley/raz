# Supported Patterns

## All Rust Execution Patterns

| Pattern | Example | Command Generated |
|---------|---------|-------------------|
| **Main Binary** | `src/main.rs` | `cargo run --bin project` |
| **Library** | `src/lib.rs` | `cargo test --lib` |
| **Integration Test** | `tests/common.rs` | `cargo test --test common` |
| **Unit Test** | `src/lib.rs:25:1` | `cargo test -- tests::my_test --exact` |
| **Example** | `examples/demo.rs` | `cargo run --example demo` |
| **Benchmark** | `benches/perf.rs` | `cargo bench --bench perf` |
| **Build Script** | `build.rs` | Direct execution |
| **Cargo Script** | `script.rs` | `cargo +nightly -Zscript script.rs` |
| **Standalone** | `/tmp/hello.rs` | `rustc hello.rs && ./hello` |
| **Standalone Test** | `/tmp/test.rs:10:1` | `rustc --test test.rs && ./test my_test` |

## Test Macro Support

RAZ recognizes all common test macros:

```rust
#[test]                  // Standard test
#[tokio::test]          // Async test
#[async_std::test]      // async-std test  
#[test_case(1, 2, 3)]   // Parameterized test
#[bench]                // Benchmark
```

## Universal Execution Patterns

### Cargo Workspaces
Multi-package projects with proper member detection:
```bash
raz workspace/frontend/src/lib.rs:15:1    # Run test in workspace member
raz workspace/backend/src/main.rs         # Run backend binary
```

### Cargo Packages
Single package projects with bin/lib/test detection:
```bash
raz project/src/main.rs           # Run main binary
raz project/src/lib.rs:25:1       # Run specific test
raz project/tests/integration.rs  # Run integration tests
```

### Cargo Scripts
Files with embedded manifests:
```rust
#!/usr/bin/env -S cargo +nightly -Zscript
//! ```cargo
//! [dependencies]
//! serde = "1.0"
//! ```

use serde::Serialize;

#[derive(Serialize)]
struct Data {
    name: String,
}

fn main() {
    let data = Data { name: "RAZ".to_string() };
    println!("{}", serde_json::to_string(&data).unwrap());
}
```

### Single Files
Standalone Rust files compiled with rustc:
```bash
raz /tmp/hello.rs                 # Compile and run with rustc
raz example.rs:10:1               # Run test at line 10 with rustc --test
```

### Build Scripts and Special Files
```bash
raz build.rs                      # Detect and run build scripts
raz benches/my_bench.rs          # Run criterion benchmarks
```

## Intelligent Context Detection

### File Role Analysis
RAZ automatically detects:
- **Main binaries**: Files with `fn main()` in `src/main.rs` or `src/bin/`
- **Libraries**: Files in `src/lib.rs` with `pub` items
- **Tests**: Files in `tests/` directory or with `#[cfg(test)]` modules
- **Examples**: Files in `examples/` directory
- **Benchmarks**: Files in `benches/` directory

### Module Hierarchy
Tracks full module paths for accurate test targeting:
```rust
mod tests {
    #[test]
    fn unit_test() { }
}

mod integration_tests {
    #[test] 
    fn workflow_test() { }
}
```

RAZ generates:
- `tests::unit_test` for the first test
- `integration_tests::workflow_test` for the second test

### Entry Point Discovery
Finds main functions, tests, benchmarks with precise locations:
- **Main functions**: `fn main()` detection
- **Test functions**: Regex matching for `#[test]`, `#[tokio::test]`, etc.
- **Benchmark functions**: `#[bench]` detection
- **Doc tests**: Documentation test detection in `///` comments

### Framework Detection
Recognizes major Rust frameworks:
- **Leptos**: Web framework with SSR support
- **Dioxus**: Cross-platform GUI framework
- **Bevy**: Game engine
- **Tauri**: Desktop app framework
- **Yew**: WebAssembly frontend framework

## Advanced Test Capabilities

### Module-Aware Targeting
Correctly handles nested test modules:
```rust
#[cfg(test)]
mod tests {
    mod unit {
        #[test]
        fn basic_test() { }
    }
    
    mod integration {
        #[test]
        fn full_workflow() { }
    }
}
```

Generates: `tests::unit::basic_test` and `tests::integration::full_workflow`

### Proximity Detection
Finds closest test within 10 lines of cursor position.

### Test Filtering
Passes exact test names to test runners for precise execution:
```bash
cargo test -- tests::my_test --exact
```

### Doc Test Support
Advanced AST-driven doctest detection with tree-sitter integration:
```rust
/// Adds two numbers with comprehensive examples
/// 
/// ```
/// use mylib::add;
/// assert_eq!(add(2, 2), 4);
/// ```
/// 
/// # Hidden doctests (lines starting with #)
/// ```
/// # use mylib::add;
/// # fn setup() { /* setup code */ }
/// assert_eq!(add(1, 1), 2);
/// ```
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

/// Complex struct with associated doctests
/// ```
/// use mylib::Calculator;
/// let calc = Calculator::new();
/// assert_eq!(calc.compute(5), 10);
/// ```
pub struct Calculator {
    multiplier: i32,
}

impl Calculator {
    /// Creates a new calculator
    /// ```
    /// use mylib::Calculator;
    /// let calc = Calculator::new();
    /// assert!(calc.is_ready());
    /// ```
    pub fn new() -> Self {
        Self { multiplier: 2 }
    }
}
```

**Tree-sitter Features:**
- **Precise Association**: Uses AST to correctly link doctests to functions/structs
- **Multi-block Support**: Handles complex doctests with multiple code blocks
- **Hidden Line Detection**: Properly detects doctests with `# ` hidden lines
- **Context Awareness**: Avoids false positives in test modules
- **Cursor Position Accuracy**: Works anywhere within doctest comment ranges
- **Rust-analyzer Level Precision**: Same accuracy as professional IDE tools

## Framework-Specific Features

### Dioxus Projects
```bash
# Automatic platform detection
raz src/main.rs --platform web      # Web build
raz src/main.rs --platform desktop  # Desktop build
raz src/main.rs --device true       # Real device build
```

### Leptos Projects
```bash
# SSR and hydration features
raz src/app.rs --features ssr,hydrate
```

### Tauri Projects
```bash
# Cross-platform builds
raz src-tauri/src/main.rs --target universal-apple-darwin
```

### Bevy Projects
```bash
# Game-specific features automatically enabled
raz src/main.rs  # Includes bevy/default features
```