use anyhow::{Context, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};

use crate::commands::workspace::resolve_module_path_to_file;
use crate::utils::parser::parse_filepath_with_line;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct RunnerContext {
    pub context_version: u32,
    pub cwd: String,
    pub project_root: Option<String>,
    pub file_path: Option<String>,
    pub line: Option<usize>,
    pub build_system: String,
    pub file_kind: String,
    pub runnable_kind: Option<String>,
    pub package_name: Option<String>,
    pub bins: Vec<String>,
    pub examples: Vec<String>,
    pub tests: Vec<String>,
    pub benches: Vec<String>,
    pub features: Vec<String>,
    pub profiles: Vec<String>,
    pub script_engine: Option<String>,
    pub recommended_target: Option<String>,
}

pub fn context_command(filepath_arg: Option<&str>, json: bool) -> Result<()> {
    let cwd = std::env::current_dir().context("failed to get current directory")?;
    let context = build_context(&cwd, filepath_arg)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&context)?);
    } else {
        print_human_context(&context);
    }

    Ok(())
}

fn build_context(cwd: &Path, filepath_arg: Option<&str>) -> Result<RunnerContext> {
    let mut runner = cargo_runner_core::UnifiedRunner::new()?;

    let (file_path, line) = if let Some(filepath_arg) = filepath_arg {
        let (path, line) = parse_filepath_with_line(filepath_arg);
        (
            Some(resolve_input_path(cwd, &path, &runner)?),
            line.map(|line| line + 1),
        )
    } else {
        (None, None)
    };
    let file_line = line.map(|line| line.saturating_sub(1) as u32);
    let command = match (file_path.as_deref(), file_line) {
        (Some(path), Some(line)) => runner
            .get_command_at_position_with_dir(path, Some(line))
            .ok(),
        (Some(path), None) => runner.get_file_command(path).ok().flatten(),
        _ => None,
    };

    let script_engine = file_path.as_deref().and_then(detect_script_engine);
    let project_root = detect_project_root(cwd, file_path.as_deref());
    let package_name = project_root
        .as_ref()
        .and_then(|root| runner.get_package_name_str(root).ok());
    let cargo_ctx = project_root
        .as_ref()
        .map(|root| collect_cargo_context(root, package_name.clone()));

    let file_kind = detect_file_kind(
        file_path.as_deref(),
        script_engine.as_deref(),
        command.as_ref(),
        cargo_ctx.as_ref(),
    );
    let build_system = detect_build_system(
        file_path.as_deref(),
        script_engine.as_deref(),
        command.as_ref(),
        &file_kind,
        &runner,
    );
    let recommended_target = detect_recommended_target(
        file_path.as_deref(),
        script_engine.as_deref(),
        command.as_ref(),
        cargo_ctx.as_ref(),
    );
    let runnable_kind = detect_runnable_kind(
        file_path.as_deref(),
        script_engine.as_deref(),
        command.as_ref(),
        cargo_ctx.as_ref(),
    );

    Ok(RunnerContext {
        context_version: 1,
        cwd: cwd.to_string_lossy().to_string(),
        project_root: project_root
            .as_ref()
            .map(|p| p.to_string_lossy().to_string()),
        file_path: file_path.map(|p| p.to_string_lossy().to_string()),
        line,
        build_system,
        file_kind,
        runnable_kind,
        package_name,
        bins: cargo_ctx
            .as_ref()
            .map(|ctx| ctx.bins.clone())
            .unwrap_or_default(),
        examples: cargo_ctx
            .as_ref()
            .map(|ctx| ctx.examples.clone())
            .unwrap_or_default(),
        tests: cargo_ctx
            .as_ref()
            .map(|ctx| ctx.tests.clone())
            .unwrap_or_default(),
        benches: cargo_ctx
            .as_ref()
            .map(|ctx| ctx.benches.clone())
            .unwrap_or_default(),
        features: cargo_ctx
            .as_ref()
            .map(|ctx| ctx.features.clone())
            .unwrap_or_default(),
        profiles: cargo_ctx
            .as_ref()
            .map(|ctx| ctx.profiles.clone())
            .unwrap_or_default(),
        script_engine,
        recommended_target,
    })
}

fn resolve_input_path(
    cwd: &Path,
    path: &str,
    runner: &cargo_runner_core::UnifiedRunner,
) -> Result<PathBuf> {
    let candidate = Path::new(path);
    let resolved = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        cwd.join(candidate)
    };

    if resolved.exists() {
        return Ok(resolved.canonicalize().unwrap_or(resolved));
    }

    if path.contains("::") {
        let module_path = resolve_module_path_to_file(runner, path, cwd)?;
        return Ok(module_path.canonicalize().unwrap_or(module_path));
    }

    Err(anyhow::anyhow!("File not found: {}", resolved.display()))
}

fn print_human_context(context: &RunnerContext) {
    println!("cwd: {}", context.cwd);
    println!("build system: {}", context.build_system);
    println!("file kind: {}", context.file_kind);
    if let Some(ref root) = context.project_root {
        println!("project root: {root}");
    }
    if let Some(ref file) = context.file_path {
        println!("file: {file}");
    }
    if let Some(line) = context.line {
        println!("line: {line}");
    }
    if let Some(ref package) = context.package_name {
        println!("package: {package}");
    }
    if let Some(ref runnable) = context.runnable_kind {
        println!("runnable kind: {runnable}");
    }
    if let Some(ref target) = context.recommended_target {
        println!("recommended target: {target}");
    }
    if let Some(ref engine) = context.script_engine {
        println!("script engine: {engine}");
    }
}

fn find_cargo_root(start: &Path) -> Option<PathBuf> {
    let mut current = if start.is_file() {
        start.parent()?.to_path_buf()
    } else {
        start.to_path_buf()
    };

    loop {
        if current.join("Cargo.toml").exists() {
            return Some(current);
        }
        if !current.pop() {
            break;
        }
    }

    None
}

#[derive(Debug, Clone)]
struct CargoProjectContext {
    package_name: Option<String>,
    bins: Vec<String>,
    examples: Vec<String>,
    tests: Vec<String>,
    benches: Vec<String>,
    features: Vec<String>,
    profiles: Vec<String>,
}

fn detect_project_root(cwd: &Path, file_path: Option<&Path>) -> Option<PathBuf> {
    file_path
        .and_then(find_cargo_root)
        .or_else(|| find_cargo_root(cwd))
}

fn collect_cargo_context(root: &Path, package_name: Option<String>) -> CargoProjectContext {
    let mut bins = Vec::new();
    let mut examples = Vec::new();
    let mut tests = Vec::new();
    let mut benches = Vec::new();

    let src = root.join("src");
    let bin_dir = src.join("bin");
    if bin_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&bin_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        bins.push(stem.to_string());
                    }
                } else if path.is_dir() && path.join("main.rs").exists() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        bins.push(name.to_string());
                    }
                }
            }
        }
    }
    if src.join("main.rs").exists() {
        if let Some(name) = package_name.clone() {
            bins.push(name);
        }
    }

    let examples_dir = root.join("examples");
    if examples_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&examples_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        examples.push(stem.to_string());
                    }
                } else if path.is_dir() && path.join("main.rs").exists() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        examples.push(name.to_string());
                    }
                }
            }
        }
    }

    let tests_dir = root.join("tests");
    if tests_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&tests_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        tests.push(stem.to_string());
                    }
                }
            }
        }
    }

    let benches_dir = root.join("benches");
    if benches_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&benches_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        benches.push(stem.to_string());
                    }
                }
            }
        }
    }

    bins.sort();
    bins.dedup();
    examples.sort();
    examples.dedup();
    tests.sort();
    tests.dedup();
    benches.sort();
    benches.dedup();

    let cargo_toml = root.join("Cargo.toml");
    let (features, profiles) = std::fs::read_to_string(&cargo_toml)
        .ok()
        .and_then(|content| content.parse::<toml::Value>().ok())
        .map(|manifest| {
            let mut features = Vec::new();
            if let Some(table) = manifest.get("features").and_then(|f| f.as_table()) {
                for key in table.keys() {
                    if key != "default" {
                        features.push(key.clone());
                    }
                }
            }
            features.sort();
            features.dedup();

            let mut profiles = Vec::new();
            if let Some(table) = manifest.get("profile").and_then(|p| p.as_table()) {
                for key in table.keys() {
                    profiles.push(key.clone());
                }
            }
            for profile in ["dev", "release", "test", "bench"] {
                if !profiles.iter().any(|p| p == profile) {
                    profiles.push(profile.to_string());
                }
            }
            profiles.sort();
            profiles.dedup();

            (features, profiles)
        })
        .unwrap_or_else(|| {
            (
                Vec::new(),
                vec![
                    "bench".to_string(),
                    "dev".to_string(),
                    "release".to_string(),
                    "test".to_string(),
                ],
            )
        });

    CargoProjectContext {
        package_name,
        bins,
        examples,
        tests,
        benches,
        features,
        profiles,
    }
}

fn detect_script_engine(file_path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(file_path).ok()?;
    let first_line = content.lines().next()?;
    if !(first_line.starts_with("#!") && content.contains("fn main(")) {
        return None;
    }

    if first_line.contains("rust-script") {
        Some("rust-script".to_string())
    } else if first_line.contains("cargo") && first_line.contains("-Zscript") {
        Some("cargo +nightly -Zscript".to_string())
    } else {
        None
    }
}

fn detect_file_kind(
    file_path: Option<&Path>,
    script_engine: Option<&str>,
    command: Option<&cargo_runner_core::Command>,
    cargo_ctx: Option<&CargoProjectContext>,
) -> String {
    if script_engine.is_some() {
        return "single_file_script".to_string();
    }

    if let Some(command) = command {
        if matches!(command.strategy, cargo_runner_core::CommandStrategy::Rustc) {
            return "standalone".to_string();
        }
    }

    let Some(file_path) = file_path else {
        return if cargo_ctx.is_some() {
            "cargo_project".to_string()
        } else {
            "standalone".to_string()
        };
    };

    let normalized = file_path.to_string_lossy().replace('\\', "/");

    if normalized.ends_with("build.rs") {
        "build_script".to_string()
    } else if normalized.contains("/tests/")
        || normalized.starts_with("tests/")
        || normalized.contains("/benches/")
        || normalized.starts_with("benches/")
        || normalized.contains("/examples/")
        || normalized.starts_with("examples/")
        || normalized.ends_with("/src/main.rs")
        || normalized.ends_with("src/main.rs")
        || normalized.contains("/src/bin/")
        || normalized.starts_with("src/bin/")
        || normalized.ends_with("/src/lib.rs")
        || normalized.ends_with("src/lib.rs")
        || cargo_ctx.is_some()
    {
        "cargo_project".to_string()
    } else {
        "standalone".to_string()
    }
}

fn detect_build_system(
    file_path: Option<&Path>,
    script_engine: Option<&str>,
    command: Option<&cargo_runner_core::Command>,
    file_kind: &str,
    runner: &cargo_runner_core::UnifiedRunner,
) -> String {
    if let Some(engine) = script_engine {
        return if engine == "rust-script" {
            "rust-script".to_string()
        } else {
            "cargo".to_string()
        };
    }

    if let Some(command) = command {
        return match command.strategy {
            cargo_runner_core::CommandStrategy::Rustc => "rustc".to_string(),
            cargo_runner_core::CommandStrategy::CargoScript => "cargo".to_string(),
            cargo_runner_core::CommandStrategy::Shell => {
                if command
                    .args
                    .first()
                    .map(|arg| arg == "rust-script")
                    .unwrap_or(false)
                {
                    "rust-script".to_string()
                } else {
                    "shell".to_string()
                }
            }
            cargo_runner_core::CommandStrategy::Bazel => "bazel".to_string(),
            cargo_runner_core::CommandStrategy::Cargo => "cargo".to_string(),
        };
    }

    if file_kind == "standalone" {
        return "rustc".to_string();
    }

    let Some(file_path) = file_path else {
        return "cargo".to_string();
    };

    match runner.detect_build_system_with_fallback(file_path) {
        cargo_runner_core::build_system::BuildSystem::Cargo => "cargo".to_string(),
        cargo_runner_core::build_system::BuildSystem::Bazel => "bazel".to_string(),
    }
}

fn detect_runnable_kind(
    file_path: Option<&Path>,
    script_engine: Option<&str>,
    command: Option<&cargo_runner_core::Command>,
    cargo_ctx: Option<&CargoProjectContext>,
) -> Option<String> {
    if script_engine.is_some() {
        return Some("single_file_script".to_string());
    }

    if let Some(command) = command {
        match command.strategy {
            cargo_runner_core::CommandStrategy::Rustc => return Some("standalone".to_string()),
            cargo_runner_core::CommandStrategy::CargoScript => {
                return Some("single_file_script".to_string());
            }
            cargo_runner_core::CommandStrategy::Shell => {
                if command
                    .args
                    .first()
                    .map(|arg| arg == "rust-script")
                    .unwrap_or(false)
                {
                    return Some("single_file_script".to_string());
                }
            }
            cargo_runner_core::CommandStrategy::Bazel => return Some("cargo_project".to_string()),
            cargo_runner_core::CommandStrategy::Cargo => {}
        }

        if let Some(subcommand) = command.args.first() {
            return match subcommand.as_str() {
                "run" => Some("binary".to_string()),
                "test" => {
                    if command.args.iter().any(|arg| arg == "--lib") {
                        Some("module_tests".to_string())
                    } else {
                        Some("test".to_string())
                    }
                }
                "bench" => Some("benchmark".to_string()),
                "build" | "check" => Some("cargo_project".to_string()),
                _ => None,
            };
        }
    }

    let Some(file_path) = file_path else {
        return cargo_ctx.map(|_| "cargo_project".to_string());
    };
    let normalized = file_path.to_string_lossy().replace('\\', "/");

    if normalized.ends_with("build.rs") {
        Some("build_script".to_string())
    } else if normalized.contains("/benches/") || normalized.starts_with("benches/") {
        Some("benchmark".to_string())
    } else if normalized.contains("/tests/") || normalized.starts_with("tests/") {
        Some("test".to_string())
    } else if normalized.contains("/examples/")
        || normalized.starts_with("examples/")
        || normalized.ends_with("/src/main.rs")
        || normalized.ends_with("src/main.rs")
        || normalized.contains("/src/bin/")
        || normalized.starts_with("src/bin/")
    {
        Some("binary".to_string())
    } else if normalized.ends_with("/src/lib.rs") || normalized.ends_with("src/lib.rs") {
        Some("module_tests".to_string())
    } else if cargo_ctx.is_some() {
        Some("cargo_project".to_string())
    } else {
        Some("standalone".to_string())
    }
}

fn detect_recommended_target(
    file_path: Option<&Path>,
    script_engine: Option<&str>,
    command: Option<&cargo_runner_core::Command>,
    cargo_ctx: Option<&CargoProjectContext>,
) -> Option<String> {
    if script_engine.is_some() {
        return file_path.map(|path| path.to_string_lossy().to_string());
    }

    if let Some(command) = command {
        for flag in ["--bin", "--example", "--test", "--bench"] {
            if let Some(pos) = command.args.iter().position(|arg| arg == flag) {
                if let Some(value) = command.args.get(pos + 1) {
                    return Some(value.clone());
                }
            }
        }

        if let Some(pos) = command.args.iter().position(|arg| arg == "-p") {
            if let Some(value) = command.args.get(pos + 1) {
                return Some(value.clone());
            }
        }
    }

    if file_path.is_none() {
        return cargo_ctx.and_then(|ctx| ctx.package_name.clone());
    }

    let file_path = file_path?;
    let stem = file_path.file_stem().and_then(|s| s.to_str())?.to_string();
    let normalized = file_path.to_string_lossy().replace('\\', "/");

    if normalized.ends_with("/src/main.rs") || normalized.ends_with("src/main.rs") {
        return cargo_ctx
            .and_then(|ctx| ctx.package_name.clone())
            .or(Some(stem));
    }

    if normalized.contains("/src/bin/") || normalized.starts_with("src/bin/") {
        return Some(stem);
    }

    if normalized.contains("/examples/") || normalized.starts_with("examples/") {
        return Some(stem);
    }

    if normalized.contains("/tests/") || normalized.starts_with("tests/") {
        return Some(stem);
    }

    if normalized.contains("/benches/") || normalized.starts_with("benches/") {
        return Some(stem);
    }

    if normalized.ends_with("build.rs") {
        return Some("build".to_string());
    }

    Some(stem)
}
