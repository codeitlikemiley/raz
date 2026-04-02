//! `cargo runner bazel-init [OPTIONS]`
//!
//! Converts a plain `cargo new` project into a Bazel + Rust workspace by
//! generating all required files and running the initial sync.
//!
//! ## Files generated
//!
//! | File                    | Description                              |
//! |-------------------------|------------------------------------------|
//! | `MODULE.bazel`          | Bzlmod workspace with rules_rust wired   |
//! | `.bazelversion`         | Pins the Bazel version                   |
//! | `.bazelrc`              | Sensible build + test defaults           |
//! | `BUILD.bazel` (root)    | Empty visibility-allow-all root          |
//! | `BUILD.bazel` (crate)   | rust_binary/rust_library + rust_test     |
//! | `Cargo.lock`            | Generated via `cargo generate-lockfile`  |
//! | `.cargo-runner.json`    | cargo-runner Bazel config                |
//!
//! After writing all files, optionally runs `bazel sync` to pull deps.

use anyhow::{Context, Result};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

use crate::config::bazel_workspace::crate_repo_name;
use crate::config::templates::create_bazel_config;

// ── version pins ─────────────────────────────────────────────────────────────
const RULES_RUST_VERSION: &str = "0.63.0";
const BAZEL_VERSION: &str = "7.4.1";

pub fn bazel_init_command(
    cwd: Option<&str>,
    force: bool,
    skip_sync: bool,
    workspace_name: Option<&str>,
) -> Result<()> {
    let project_root = if let Some(c) = cwd {
        PathBuf::from(c)
            .canonicalize()
            .context("Failed to canonicalize --cwd path")?
    } else {
        std::env::current_dir().context("Failed to get current directory")?
    };

    // Guard: must be a Cargo project
    let cargo_toml = project_root.join("Cargo.toml");
    if !cargo_toml.exists() {
        anyhow::bail!(
            "No Cargo.toml found in {}.\n\
             Run `cargo new --bin <name>` first, then re-run `cargo runner bazel-init`.",
            project_root.display()
        );
    }

    // Guard: already a Bazel workspace?
    if project_root.join("MODULE.bazel").exists() && !force {
        anyhow::bail!(
            "MODULE.bazel already exists in {}.\n\
             Use --force to overwrite.",
            project_root.display()
        );
    }

    // Derive names
    let dir_name = project_root
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let ws_name = workspace_name.unwrap_or(&dir_name);
    let pkg_name = read_cargo_package_name(&cargo_toml).unwrap_or_else(|| ws_name.to_string());
    let repo = crate_repo_name(&pkg_name);
    let is_lib = project_root.join("src/lib.rs").exists()
        && !project_root.join("src/main.rs").exists();

    println!("🔥 Converting {} → Bazel workspace", pkg_name);
    println!("   rules_rust v{}  •  Bazel v{}", RULES_RUST_VERSION, BAZEL_VERSION);
    println!();

    // ── 1. MODULE.bazel ───────────────────────────────────────────────────────
    write_if_absent_or_forced(
        &project_root.join("MODULE.bazel"),
        &module_bazel_single_crate(ws_name, &pkg_name, &repo),
        force,
        "MODULE.bazel",
    )?;

    // ── 2. .bazelversion ──────────────────────────────────────────────────────
    write_if_absent_or_forced(
        &project_root.join(".bazelversion"),
        BAZEL_VERSION,
        force,
        ".bazelversion",
    )?;

    // ── 3. .bazelrc ───────────────────────────────────────────────────────────
    write_if_absent_or_forced(
        &project_root.join(".bazelrc"),
        BAZELRC_CONTENT,
        force,
        ".bazelrc",
    )?;

    // ── 4. Root BUILD.bazel ───────────────────────────────────────────────────
    write_if_absent_or_forced(
        &project_root.join("BUILD.bazel"),
        ROOT_BUILD_CONTENT,
        force,
        "BUILD.bazel (root)",
    )?;

    // ── 5. Cargo.lock ─────────────────────────────────────────────────────────
    let lockfile = project_root.join("Cargo.lock");
    if !lockfile.exists() {
        println!("📦 Generating Cargo.lock...");
        let status = Command::new("cargo")
            .arg("generate-lockfile")
            .current_dir(&project_root)
            .status()
            .context("Failed to run `cargo generate-lockfile`")?;
        if !status.success() {
            anyhow::bail!("`cargo generate-lockfile` failed");
        }
        println!("   ✅ Cargo.lock");
    } else {
        println!("   ~ Cargo.lock already exists — skipping");
    }

    // ── 6. Crate BUILD.bazel ─────────────────────────────────────────────────
    let crate_build = project_root.join("BUILD.bazel");
    // For single-crate repos the root BUILD.bazel IS the crate BUILD.bazel
    // We wrote a stub in step 4 — now overwrite it with the real targets.
    let crate_build_content = crate_build_content(&pkg_name, &repo, is_lib);
    write_if_absent_or_forced(&crate_build, &crate_build_content, true, "BUILD.bazel (crate targets)")?;

    // ── 7. .cargo-runner.json ─────────────────────────────────────────────────
    let runner_config = project_root.join(".cargo-runner.json");
    if !runner_config.exists() || force {
        let config = create_bazel_config(ws_name);
        std::fs::write(&runner_config, config)
            .with_context(|| format!("Failed to write {}", runner_config.display()))?;
        println!("   ✅ .cargo-runner.json");
    } else {
        println!("   ~ .cargo-runner.json already exists — skipping");
    }

    println!();
    println!("✅ Bazel workspace scaffold complete!");
    println!();

    // ── 8. Optional: bazel sync ───────────────────────────────────────────────
    if skip_sync {
        println!("⏭️  Skipping `bazel sync` (--skip-sync)");
        print_next_steps(&pkg_name);
        return Ok(());
    }

    println!("🔄 Running `bazel sync` to pull dependencies…");
    println!("   (This may take a few minutes on first run)");
    let status = Command::new("bazel")
        .arg("sync")
        .current_dir(&project_root)
        .status()
        .context("Failed to run `bazel sync` — is Bazel installed?")?;

    if status.success() {
        println!("   ✅ bazel sync complete");
    } else {
        println!("   ⚠️  bazel sync exited with non-zero status.");
        println!("      Check the output above and re-run manually: bazel sync");
    }

    print_next_steps(&pkg_name);
    Ok(())
}

// ── template generators ───────────────────────────────────────────────────────

// For a single-crate repo (the output of `cargo new`) the repo root IS the crate.
// Manifest path is //Cargo.toml, not //<subdir>:Cargo.toml.
fn module_bazel_single_crate(ws_name: &str, _pkg_name: &str, repo: &str) -> String {
    format!(
        r#"# MODULE.bazel — generated by `cargo runner bazel-init`
module(name = "{ws_name}")

bazel_dep(name = "rules_rust", version = "{RULES_RUST_VERSION}")

crate = use_extension(
    "@rules_rust//crate_universe:extensions.bzl",
    "crate",
)

crate.from_cargo(
    name = "{repo}",
    manifests = ["//Cargo.toml"],
    cargo_lockfile = "//Cargo.lock",
)

use_repo(crate, "{repo}")
"#
    )
}

fn crate_build_content(pkg_name: &str, repo: &str, is_lib: bool) -> String {
    if is_lib {
        format!(
            r#"load("@{repo}//:defs.bzl", "all_crate_deps")
load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")

rust_library(
    name = "{pkg_name}",
    srcs = glob(["src/**/*.rs"]),
    deps = all_crate_deps(normal = True),
    visibility = ["//visibility:public"],
)

rust_test(
    name = "{pkg_name}_test",
    crate = ":{pkg_name}",
    deps = all_crate_deps(normal_dev = True),
)
"#
        )
    } else {
        format!(
            r#"load("@{repo}//:defs.bzl", "all_crate_deps")
load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_test")

rust_binary(
    name = "{pkg_name}",
    srcs = ["src/main.rs"],
    deps = all_crate_deps(normal = True),
    visibility = ["//visibility:public"],
)

rust_test(
    name = "{pkg_name}_test",
    crate = ":{pkg_name}",
    deps = all_crate_deps(normal_dev = True),
)
"#
        )
    }
}

const ROOT_BUILD_CONTENT: &str = r#"# Root BUILD.bazel — generated by `cargo runner bazel-init`
package(default_visibility = ["//visibility:public"])
"#;

const BAZELRC_CONTENT: &str = r#"# .bazelrc — generated by `cargo runner bazel-init`

# ── Build defaults ────────────────────────────────────────────────────────────
build --jobs=auto
build --keep_going

# ── Test defaults ─────────────────────────────────────────────────────────────
test --test_output=errors
test --keep_going

# ── Rust / rules_rust ─────────────────────────────────────────────────────────
build --@rules_rust//:extra_rustc_flags=-Dwarnings

# ── macOS / Apple Silicon ─────────────────────────────────────────────────────
# Uncomment if building on Apple Silicon:
# build --cpu=darwin_arm64
"#;

// ── helpers ───────────────────────────────────────────────────────────────────

fn write_if_absent_or_forced(
    path: &Path,
    content: &str,
    force: bool,
    label: &str,
) -> Result<()> {
    if path.exists() && !force {
        println!("   ~ {} already exists — skipping", label);
        return Ok(());
    }
    std::fs::write(path, content)
        .with_context(|| format!("Failed to write {}", path.display()))?;
    println!("   ✅ {}", label);
    Ok(())
}

fn read_cargo_package_name(cargo_toml: &Path) -> Option<String> {
    let content = std::fs::read_to_string(cargo_toml).ok()?;
    let mut in_package = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[package]" {
            in_package = true;
            continue;
        }
        if trimmed.starts_with('[') {
            in_package = false;
        }
        if in_package && trimmed.starts_with("name") {
            if let Some(val) = trimmed.splitn(2, '=').nth(1) {
                let name = val.trim().trim_matches('"').trim_matches('\'').to_string();
                if !name.is_empty() {
                    return Some(name);
                }
            }
        }
    }
    None
}

fn print_next_steps(pkg_name: &str) {
    println!();
    println!("📌 Next steps:");
    println!("   bazel build //...                  # build everything");
    println!("   bazel run //:{pkg_name}             # run the binary");
    println!("   bazel test //...                   # run all tests");
    println!("   cargo runner add <crate>           # add a dependency");
    println!("   cargo runner build-sync            # update BUILD.bazel after new files");
    println!("   cargo runner run                   # run via cargo-runner");
}
