use std::path::Path;

pub fn determine_file_type(path: &Path) -> String {
    // Convert to absolute path for consistent checking
    let abs_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .ok()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|| path.to_path_buf())
    };

    let path_str = abs_path.to_str().unwrap_or("");

    // Check if it's in a Bazel project
    let has_bazel = abs_path.ancestors().any(|p| {
        p.join("BUILD.bazel").exists()
            || p.join("BUILD").exists()
            || p.join("WORKSPACE").exists()
            || p.join("WORKSPACE.bazel").exists()
    });

    if has_bazel {
        if path_str.ends_with("/src/main.rs") || path_str.ends_with("main.rs") {
            return "Binary (main.rs)".to_string();
        } else {
            return "Bazel Rust file".to_string();
        }
    }

    // Check if it's a standalone file (no Cargo.toml in parents)
    let has_cargo_toml = abs_path.ancestors().any(|p| p.join("Cargo.toml").exists());

    if !has_cargo_toml {
        // Check if it's a cargo script file
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Some(first_line) = content.lines().next() {
                if first_line.starts_with("#!")
                    && first_line.contains("cargo")
                    && first_line.contains("-Zscript")
                {
                    return "Cargo script file".to_string();
                }
            }
        }
        return "Standalone Rust file".to_string();
    }

    if path_str.ends_with("/src/lib.rs") || path_str == "src/lib.rs" {
        "Library (lib.rs)".to_string()
    } else if path_str.ends_with("/src/main.rs") || path_str == "src/main.rs" {
        "Binary (main.rs)".to_string()
    } else if path_str.contains("/src/bin/") {
        format!(
            "Binary '{}'",
            path.file_stem().unwrap_or_default().to_str().unwrap_or("")
        )
    } else if path_str.contains("/tests/") {
        format!(
            "Integration test '{}'",
            path.file_stem().unwrap_or_default().to_str().unwrap_or("")
        )
    } else if path_str.contains("/benches/") {
        format!(
            "Benchmark '{}'",
            path.file_stem().unwrap_or_default().to_str().unwrap_or("")
        )
    } else if path_str.contains("/examples/") {
        format!(
            "Example '{}'",
            path.file_stem().unwrap_or_default().to_str().unwrap_or("")
        )
    } else if path_str.contains("/src/") || path_str.starts_with("src/") {
        "Library module".to_string()
    } else {
        "Rust file".to_string()
    }
}

pub fn print_runnable_type(kind: &cargo_runner_core::RunnableKind) {
    match kind {
        cargo_runner_core::RunnableKind::Test {
            test_name,
            is_async,
        } => {
            print!("Test function '{}'", test_name);
            if *is_async {
                print!(" (async)");
            }
            println!();
        }
        cargo_runner_core::RunnableKind::DocTest {
            struct_or_module_name,
            method_name,
        } => {
            print!("Doc test for '{}'", struct_or_module_name);
            if let Some(method) = method_name {
                print!("::{}", method);
            }
            println!();
        }
        cargo_runner_core::RunnableKind::Benchmark { bench_name } => {
            println!("Benchmark '{}'", bench_name);
        }
        cargo_runner_core::RunnableKind::Binary { bin_name } => {
            print!("Binary");
            if let Some(name) = bin_name {
                print!(" '{}'", name);
            }
            println!();
        }
        cargo_runner_core::RunnableKind::ModuleTests { module_name } => {
            println!("Test module '{}'", module_name);
        }
        cargo_runner_core::RunnableKind::Standalone { has_tests } => {
            print!("Standalone Rust file");
            if *has_tests {
                print!(" (with tests)");
            }
            println!();
        }
        cargo_runner_core::RunnableKind::SingleFileScript { shebang } => {
            println!("Cargo script file");
            println!("   🔧 Shebang: {}", shebang);
        }
    }
}
