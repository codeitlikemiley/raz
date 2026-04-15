//! `cargo runner build-sync [--crate <dir>] [--dry-run]`
//!
//! Scans a crate's `src/` layout and regenerates its `BUILD.bazel`,
//! adding targets for any new files that aren't yet covered.
//!
//! ## Safety contract
//!
//! The scaffolder ONLY touches lines between:
//!   ```text
//!   # BEGIN raz-managed
//!   ...
//!   # END raz-managed
//!   ```
//!
//! If those markers are absent, it appends a new managed block at the end of
//! the file (or creates the file if it doesn't exist). Hand-authored stanzas
//! above the managed block are never touched.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::config::bazel_workspace::find_bazel_crates;
use crate::config::{local_dependency_labels, rust_crate_name};

const MANAGED_BEGIN: &str = "# BEGIN raz-managed";
const MANAGED_END: &str = "# END raz-managed";

/// Run build-sync from `cwd`.
///
/// * `crate_filter` — limit to a specific crate directory or name
/// * `dry_run`      — print what would change without writing
pub fn build_sync_command(crate_filter: Option<&str>, dry_run: bool) -> Result<()> {
    let cwd = std::env::current_dir().context("failed to get current directory")?;

    // Find the bazel workspace root (needed for walkdir to have a bounded root)
    let root = find_workspace_root(&cwd).unwrap_or_else(|| cwd.clone());

    let all_crates =
        find_bazel_crates(&root).context("failed to scan workspace for Bazel crates")?;

    let crates_to_sync: Vec<_> = if let Some(filter) = crate_filter {
        all_crates
            .into_iter()
            .filter(|c| {
                c.name == filter
                    || c.dir
                        .file_name()
                        .is_some_and(|n| n.to_string_lossy() == filter)
            })
            .collect()
    } else {
        // Default: sync only the crate the user is inside of
        let local = all_crates
            .into_iter()
            .filter(|c| cwd.starts_with(&c.dir) || c.dir == cwd)
            .collect::<Vec<_>>();
        if local.is_empty() {
            anyhow::bail!("Not inside a Bazel crate directory. Use --crate <dir> to specify one.");
        }
        local
    };

    for krate in &crates_to_sync {
        process_crate(&krate.dir, &krate.name, &krate.repo_name, dry_run)?;
    }

    if dry_run {
        println!("\n(dry-run — no files were written)");
    } else {
        println!("\n✅ BUILD.bazel sync complete.");
    }

    Ok(())
}

// ── per-crate logic ───────────────────────────────────────────────────────────

pub(crate) fn process_crate(
    dir: &Path,
    crate_name: &str,
    repo_name: &str,
    dry_run: bool,
) -> Result<()> {
    println!("\n📁 Scanning: {}", dir.display());

    // Warn about build.rs — Bazel doesn't use Cargo build scripts
    if dir.join("build.rs").exists() {
        println!("   ⚠️  build.rs detected — Bazel ignores Cargo build scripts.");
        println!("      If your build.rs generates code or sets env vars, add a");
        println!("      `cargo_build_script()` rule to BUILD.bazel manually.");
        println!(
            "      See: https://bazelbuild.github.io/rules_rust/cargo.html#cargo_build_script"
        );
    }

    let build_path = dir.join("BUILD.bazel");

    // Read the existing file so we can skip already-defined targets
    let existing_content = if build_path.exists() {
        std::fs::read_to_string(&build_path)
            .with_context(|| format!("reading {}", build_path.display()))?
    } else {
        String::new()
    };

    let has_build_script = dir.join("build.rs").exists();
    let normalized_content = normalize_repo_header(&existing_content, repo_name, has_build_script);

    // Collect names already defined *outside* the managed block
    let existing_names = names_outside_managed_block(&normalized_content);

    let (targets, skipped) = infer_targets(
        dir,
        crate_name,
        repo_name,
        &existing_names,
        &normalized_content,
    );

    for name in &skipped {
        println!("   ~ skipping '{name}' (already defined)");
    }

    let header_changed = normalized_content != existing_content;

    if targets.is_empty() {
        if header_changed {
            if dry_run {
                println!("   Would update stale BUILD.bazel header:");
                println!("   load(\"@{repo_name}//:defs.bzl\", \"all_crate_deps\")");
            } else {
                std::fs::write(&build_path, normalized_content)
                    .with_context(|| format!("writing {}", build_path.display()))?;
                println!("   ✓ Updated stale BUILD.bazel header.");
            }
        } else {
            println!("   ✓ No new targets to add.");
        }
        return Ok(());
    }

    let generated = render_managed_block(&targets);

    if dry_run {
        println!("   Would write to: {}", build_path.display());
        println!("   Generated block:\n{generated}");
        return Ok(());
    }

    if !normalized_content.is_empty() {
        let updated = splice_managed_block(&normalized_content, &generated);
        std::fs::write(&build_path, updated)
            .with_context(|| format!("writing {}", build_path.display()))?;
    } else {
        // New file — write header + managed block
        let header = if has_build_script {
            build_file_header_with_build_script(repo_name)
        } else {
            build_file_header(repo_name)
        };
        std::fs::write(&build_path, format!("{header}\n{generated}"))
            .with_context(|| format!("creating {}", build_path.display()))?;
    }

    for t in &targets {
        println!("   + {}", t.description());
    }

    Ok(())
}

// ── target inference ─────────────────────────────────────────────────────────

#[allow(dead_code)] // repo_name fields used for future per-crate load() generation
#[derive(Debug)]
pub(crate) enum BazelTarget {
    Library {
        name: String,
        repo_name: String,
        local_deps: Vec<String>,
    },
    Binary {
        name: String,
        src: String,
        repo_name: String,
        crate_name: String,
        has_local_lib: bool,
    },
    TestSuite {
        name: String,
        repo_name: String,
        crate_name: String,
        has_local_lib: bool,
    },
    Example {
        name: String,
        src: String,
        repo_name: String,
        crate_name: String,
        has_local_lib: bool,
    },
    Bench {
        name: String,
        src: String,
        repo_name: String,
        crate_name: String,
        has_local_lib: bool,
    },
    DocTest {
        crate_name: String,
    },
    BuildScript,
}

impl BazelTarget {
    /// The Bazel `name = "..."` value this target would produce.
    pub(crate) fn bazel_name(&self) -> String {
        match self {
            Self::Library { name, .. } => format!("{name}_lib"),
            Self::Binary { name, .. } => name.clone(),
            Self::TestSuite { .. } => "integration_tests".to_string(),
            Self::Example { name, .. } => format!("example_{name}"),
            Self::Bench { name, .. } => format!("bench_{name}"),
            Self::DocTest { .. } => "doc_tests".to_string(),
            Self::BuildScript => "build_script".to_string(),
        }
    }

    pub(crate) fn description(&self) -> String {
        match self {
            Self::Library { name, .. } => format!("rust_library({name})"),
            Self::Binary { name, .. } => format!("rust_binary({name})"),
            Self::TestSuite { name, .. } => format!("rust_test_suite({name})"),
            Self::Example { name, .. } => format!("rust_binary(example_{name})"),
            Self::Bench { name, .. } => format!("rust_binary(bench_{name})"),
            Self::DocTest { crate_name } => format!("rust_doc_test({crate_name})"),
            Self::BuildScript => "cargo_build_script(build_script)".to_string(),
        }
    }

    pub(crate) fn render(&self) -> String {
        match self {
            Self::Library {
                name,
                repo_name: _,
                local_deps,
            } => format!(
                r#"rust_library(
    name = "{name}_lib",
    srcs = glob(["src/**/*.rs"]),
    deps = {local_deps_expr}all_crate_deps(),
    visibility = ["//visibility:public"],
    crate_name = "{crate_name}",
)

rust_test(
    name = "unit_tests",
    crate = ":{name}_lib",
    deps = all_crate_deps(normal_dev = True),
)
"#,
                crate_name = rust_crate_name(name),
                local_deps_expr = deps_expr(local_deps),
            ),
            Self::Binary {
                name,
                src,
                repo_name: _,
                crate_name,
                has_local_lib,
            } => format!(
                r#"rust_binary(
    name = "{name}",
    srcs = ["{src}"],
    crate_root = "{src}",
{deps}
)
"#,
                deps = if *has_local_lib {
                    format!(r#"    deps = [":{crate_name}_lib"] + all_crate_deps(normal = True),"#)
                } else {
                    r#"    deps = all_crate_deps(normal = True),"#.to_string()
                },
            ),
            Self::TestSuite {
                name: _,
                repo_name: _,
                crate_name,
                has_local_lib,
            } => format!(
                r#"rust_test_suite(
    name = "integration_tests",
    srcs = glob(["tests/**/*.rs"]),
{deps}
)
"#,
                deps = if *has_local_lib {
                    format!(
                        r#"    deps = [":{crate_name}_lib"] + all_crate_deps(normal_dev = True),"#
                    )
                } else {
                    r#"    deps = all_crate_deps(normal_dev = True),"#.to_string()
                },
            ),
            Self::Example {
                name,
                src,
                repo_name: _,
                crate_name,
                has_local_lib,
            } => format!(
                r#"rust_binary(
    name = "example_{name}",
    srcs = ["{src}"],
    crate_root = "{src}",
{deps}
)
"#,
                deps = if *has_local_lib {
                    format!(r#"    deps = [":{crate_name}_lib"] + all_crate_deps(normal = True),"#)
                } else {
                    r#"    deps = all_crate_deps(normal = True),"#.to_string()
                },
            ),
            Self::Bench {
                name,
                src,
                repo_name: _,
                crate_name,
                has_local_lib,
            } => format!(
                r#"rust_binary(
    name = "bench_{name}",
    srcs = ["{src}"],
    crate_root = "{src}",
{deps}
)
"#,
                deps = if *has_local_lib {
                    format!(
                        r#"    deps = [":{crate_name}_lib"] + all_crate_deps(normal_dev = True),"#
                    )
                } else {
                    r#"    deps = all_crate_deps(normal_dev = True),"#.to_string()
                },
            ),
            Self::DocTest { crate_name } => format!(
                r#"rust_doc_test(
    name = "doc_tests",
    crate = ":{crate_name}_lib",
)
"#,
            ),
            Self::BuildScript => r#"cargo_build_script(
    name = "build_script",
    srcs = ["build.rs"],
)
"#
            .to_string(),
        }
    }
}

fn deps_expr(local_deps: &[String]) -> String {
    if local_deps.is_empty() {
        String::new()
    } else {
        format!(
            "[{}] + ",
            local_deps
                .iter()
                .map(|d| format!("\"{d}\""))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

/// Returns all `name = "..."` values found anywhere in the file.
/// This ensures `build-sync` is idempotent — targets already in the managed
/// block are treated the same as hand-authored ones and are never duplicated.
fn names_outside_managed_block(content: &str) -> std::collections::HashSet<String> {
    let re = regex::Regex::new(r#"name\s*=\s*"([^"]+)""#).expect("Valid regex");
    re.captures_iter(content)
        .map(|c| c[1].to_string())
        .collect()
}

/// Returns `(new_targets, skipped_names)` where `skipped_names` are targets
/// that already exist in the hand-authored part of the file.
///
/// Target inference uses a **combined** strategy:
///   1. If `Cargo.toml` has explicit `[[bin]]`/`[[test]]`/`[[bench]]`/`[[example]]`
///      sections, those definitions win (names, paths, harness settings).
///   2. Otherwise, fall back to Rust filesystem conventions **but verify that
///      files in `src/bin/` and `examples/` actually contain `fn main()`**
///      before treating them as binary targets—a `.rs` file without `fn main()`
///      is just a helper module.
///   3. `tests/*.rs` never need a `fn main()` check (the test harness provides
///      the entry point).
///   4. `benches/*.rs` are detected by convention; `harness = false` in
///      `Cargo.toml` `[[bench]]` is noted so the Bazel rule can match.
pub(crate) fn infer_targets(
    dir: &Path,
    crate_name: &str,
    repo_name: &str,
    existing_names: &std::collections::HashSet<String>,
    existing_content: &str,
) -> (Vec<BazelTarget>, Vec<String>) {
    let mut candidates: Vec<BazelTarget> = Vec::new();
    let local_deps = local_dependency_labels(dir).unwrap_or_default();

    // Parse Cargo.toml for explicit target definitions
    let cargo_toml = dir.join("Cargo.toml");
    let cargo_content = std::fs::read_to_string(&cargo_toml).unwrap_or_default();
    let explicit_bins = parse_cargo_targets(&cargo_content, "bin");
    let explicit_tests = parse_cargo_targets(&cargo_content, "test");
    let explicit_benches = parse_cargo_targets(&cargo_content, "bench");
    let explicit_examples = parse_cargo_targets(&cargo_content, "example");

    // ── src/lib.rs → rust_library + unit_tests + doc_tests ──────────────────
    if dir.join("src/lib.rs").exists() {
        candidates.push(BazelTarget::Library {
            name: crate_name.to_string(),
            repo_name: repo_name.to_string(),
            local_deps: local_deps.clone(),
        });
        candidates.push(BazelTarget::DocTest {
            crate_name: crate_name.to_string(),
        });
    }

    // ── Binaries ────────────────────────────────────────────────────────────
    if !explicit_bins.is_empty() {
        // Cargo.toml has explicit [[bin]] definitions → use those
        for target in &explicit_bins {
            let path = target.path.as_deref().unwrap_or_else(|| {
                if target.name == crate_name {
                    "src/main.rs"
                } else {
                    ""
                }
            });
            if !path.is_empty() {
                candidates.push(BazelTarget::Binary {
                    name: target.name.clone(),
                    src: path.to_string(),
                    repo_name: repo_name.to_string(),
                    crate_name: crate_name.to_string(),
                    has_local_lib: dir.join("src/lib.rs").exists(),
                });
            }
        }
    } else {
        // Convention: src/main.rs → binary named <crate>_bin
        if dir.join("src/main.rs").exists() {
            candidates.push(BazelTarget::Binary {
                name: format!("{crate_name}_bin"),
                src: "src/main.rs".to_string(),
                repo_name: repo_name.to_string(),
                crate_name: crate_name.to_string(),
                has_local_lib: dir.join("src/lib.rs").exists(),
            });
        }

        // Convention: src/bin/*.rs → one binary per file WITH fn main()
        scan_rs_dir_with_main_check(dir, "src/bin", &mut candidates, |stem, src| {
            BazelTarget::Binary {
                name: stem,
                src,
                repo_name: repo_name.to_string(),
                crate_name: crate_name.to_string(),
                has_local_lib: dir.join("src/lib.rs").exists(),
            }
        });

        // Convention: src/bin/*/main.rs → subdirectory binaries
        if let Ok(entries) = std::fs::read_dir(dir.join("src/bin")) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && path.join("main.rs").exists() {
                    let stem = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    candidates.push(BazelTarget::Binary {
                        name: stem.clone(),
                        src: format!("src/bin/{stem}/main.rs"),
                        repo_name: repo_name.to_string(),
                        crate_name: crate_name.to_string(),
                        has_local_lib: dir.join("src/lib.rs").exists(),
                    });
                }
            }
        }
    }

    // ── Tests ───────────────────────────────────────────────────────────────
    if !explicit_tests.is_empty() {
        // Cargo.toml has explicit [[test]] definitions
        // Still use a single test_suite — Bazel handles the glob
        candidates.push(BazelTarget::TestSuite {
            name: "integration_tests".to_string(),
            repo_name: repo_name.to_string(),
            crate_name: crate_name.to_string(),
            has_local_lib: dir.join("src/lib.rs").exists(),
        });
    } else if dir.join("tests").exists() {
        // Convention: tests/ directory exists → rust_test_suite
        // No fn main() check needed — test harness provides it
        let has_rs_files = std::fs::read_dir(dir.join("tests"))
            .map(|entries| {
                entries
                    .flatten()
                    .any(|e| e.path().extension().is_some_and(|ext| ext == "rs"))
            })
            .unwrap_or(false);

        if has_rs_files {
            candidates.push(BazelTarget::TestSuite {
                name: "integration_tests".to_string(),
                repo_name: repo_name.to_string(),
                crate_name: crate_name.to_string(),
                has_local_lib: dir.join("src/lib.rs").exists(),
            });
        }
    }

    // ── Examples ────────────────────────────────────────────────────────────
    if !explicit_examples.is_empty() {
        // Cargo.toml has explicit [[example]] definitions → use those
        for target in &explicit_examples {
            let default_path = format!("examples/{}.rs", target.name);
            let path = target.path.as_deref().unwrap_or(&default_path);
            candidates.push(BazelTarget::Example {
                name: target.name.clone(),
                src: path.to_string(),
                repo_name: repo_name.to_string(),
                crate_name: crate_name.to_string(),
                has_local_lib: dir.join("src/lib.rs").exists(),
            });
        }
    } else {
        // Convention: examples/*.rs → one binary per file WITH fn main()
        scan_rs_dir_with_main_check(dir, "examples", &mut candidates, |stem, src| {
            BazelTarget::Example {
                name: stem,
                src,
                repo_name: repo_name.to_string(),
                crate_name: crate_name.to_string(),
                has_local_lib: dir.join("src/lib.rs").exists(),
            }
        });

        // Convention: examples/*/main.rs → subdirectory examples
        if let Ok(entries) = std::fs::read_dir(dir.join("examples")) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && path.join("main.rs").exists() {
                    let stem = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    candidates.push(BazelTarget::Example {
                        name: stem.clone(),
                        src: format!("examples/{stem}/main.rs"),
                        repo_name: repo_name.to_string(),
                        crate_name: crate_name.to_string(),
                        has_local_lib: dir.join("src/lib.rs").exists(),
                    });
                }
            }
        }
    }

    // ── Benches ─────────────────────────────────────────────────────────────
    if !explicit_benches.is_empty() {
        // Cargo.toml has explicit [[bench]] definitions → use those
        for target in &explicit_benches {
            let default_path = format!("benches/{}.rs", target.name);
            let path = target.path.as_deref().unwrap_or(&default_path);
            candidates.push(BazelTarget::Bench {
                name: target.name.clone(),
                src: path.to_string(),
                repo_name: repo_name.to_string(),
                crate_name: crate_name.to_string(),
                has_local_lib: dir.join("src/lib.rs").exists(),
            });
        }
    } else {
        // Convention: benches/*.rs → check for fn main() (criterion-style)
        scan_rs_dir_with_main_check(dir, "benches", &mut candidates, |stem, src| {
            BazelTarget::Bench {
                name: stem,
                src,
                repo_name: repo_name.to_string(),
                crate_name: crate_name.to_string(),
                has_local_lib: dir.join("src/lib.rs").exists(),
            }
        });
    }

    // ── build.rs → cargo_build_script ───────────────────────────────────────
    if dir.join("build.rs").exists() {
        candidates.push(BazelTarget::BuildScript);
    }

    // Partition into new vs already-defined
    let mut targets = Vec::new();
    let mut skipped = Vec::new();
    for target in candidates {
        let bazel_name = target.bazel_name();

        // For Library targets, init.rs generates `name = "<crate>"` but
        // infer_targets generates `name = "<crate>_lib"`. Check both forms.
        let is_existing = existing_names.contains(&bazel_name)
            || match &target {
                BazelTarget::Library { name, .. } => existing_names.contains(name),
                BazelTarget::DocTest { .. } => {
                    // Skip if ANY rust_doc_test rule exists in the file,
                    // regardless of its name attribute.
                    existing_content.contains("rust_doc_test(")
                }
                _ => false,
            };

        if is_existing {
            skipped.push(bazel_name);
        } else {
            targets.push(target);
        }
    }

    (targets, skipped)
}

// ── helpers for target inference ──────────────────────────────────────────────

/// Simple representation of a `[[bin]]`, `[[test]]`, `[[bench]]`, or
/// `[[example]]` entry parsed from Cargo.toml.
#[derive(Debug)]
struct CargoTarget {
    name: String,
    path: Option<String>,
}

/// Parse `[[<kind>]]` sections from Cargo.toml text.
///
/// Extracts `name` and `path` fields from each section. Uses simple line-based
/// scanning (no TOML crate dependency).
fn parse_cargo_targets(content: &str, kind: &str) -> Vec<CargoTarget> {
    let header = format!("[[{kind}]]");
    let mut targets = Vec::new();
    let mut current_name: Option<String> = None;
    let mut current_path: Option<String> = None;
    let mut in_section = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed == header {
            // Save previous section
            if let Some(name) = current_name.take() {
                targets.push(CargoTarget {
                    name,
                    path: current_path.take(),
                });
            }
            current_path = None;
            in_section = true;
            continue;
        }

        // Any other section header ends our section
        if trimmed.starts_with('[') {
            if let Some(name) = current_name.take() {
                targets.push(CargoTarget {
                    name,
                    path: current_path.take(),
                });
            }
            current_path = None;
            in_section = false;
            continue;
        }

        if !in_section {
            continue;
        }

        if trimmed.starts_with("name") {
            if let Some(val) = extract_string_value(trimmed) {
                current_name = Some(val);
            }
        } else if trimmed.starts_with("path") {
            if let Some(val) = extract_string_value(trimmed) {
                current_path = Some(val);
            }
        }
    }

    // Flush last section
    if let Some(name) = current_name {
        targets.push(CargoTarget {
            name,
            path: current_path,
        });
    }

    targets
}

/// Extract the string value from a line like `name = "foo"`.
fn extract_string_value(line: &str) -> Option<String> {
    let rhs = line.split_once('=')?.1.trim();
    let val = rhs.trim_matches('"').trim_matches('\'');
    if val.is_empty() {
        None
    } else {
        Some(val.to_string())
    }
}

/// Scan a directory for `.rs` files that contain `fn main()`, and push
/// targets via the provided constructor. Files without `fn main()` are
/// silently skipped (they're helper modules, not entry points).
fn scan_rs_dir_with_main_check<F>(
    crate_dir: &Path,
    rel_dir: &str,
    candidates: &mut Vec<BazelTarget>,
    make_target: F,
) where
    F: Fn(String, String) -> BazelTarget,
{
    let abs_dir = crate_dir.join(rel_dir);
    let entries = match std::fs::read_dir(&abs_dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() || path.extension().is_none_or(|e| e != "rs") {
            continue;
        }

        if !file_has_fn_main(&path) {
            continue;
        }

        let stem = path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        let src = format!("{rel_dir}/{stem}.rs");
        candidates.push(make_target(stem, src));
    }
}

/// Quick check whether a file contains a `fn main()` declaration.
///
/// This is a best-effort heuristic — it looks for `fn main()` or `fn main ()`
/// anywhere on a non-comment line. It won't be fooled by `// fn main()` but
/// could theoretically match inside a string literal (acceptable trade-off for
/// a scaffolding tool).
fn file_has_fn_main(path: &Path) -> bool {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return false,
    };

    for line in content.lines() {
        let trimmed = line.trim();
        // Skip single-line comments
        if trimmed.starts_with("//") {
            continue;
        }
        // Look for fn main with optional whitespace variations
        if trimmed.contains("fn main()") || trimmed.contains("fn main ()") {
            return true;
        }
    }

    false
}

// ── rendering ─────────────────────────────────────────────────────────────────

pub(crate) fn build_file_header(repo_name: &str) -> String {
    format!(
        r#"load("@{repo_name}//:defs.bzl", "all_crate_deps")
load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_doc_test", "rust_library", "rust_test", "rust_test_suite")
"#
    )
}

/// Build header that also includes the `cargo_build_script` load.
pub(crate) fn build_file_header_with_build_script(repo_name: &str) -> String {
    format!(
        r#"load("@{repo_name}//:defs.bzl", "all_crate_deps")
load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_doc_test", "rust_library", "rust_test", "rust_test_suite")
load("@rules_rust//cargo:defs.bzl", "cargo_build_script")
"#
    )
}

pub(crate) fn render_managed_block(targets: &[BazelTarget]) -> String {
    let body: String = targets
        .iter()
        .map(|t| t.render())
        .collect::<Vec<_>>()
        .join("\n");
    format!("{MANAGED_BEGIN}\n{body}\n{MANAGED_END}\n")
}

/// Replace the existing managed block in `existing` with `new_block`, or
/// append the block if no markers are found.
fn splice_managed_block(existing: &str, new_block: &str) -> String {
    if let (Some(start), Some(end)) = (existing.find(MANAGED_BEGIN), existing.find(MANAGED_END)) {
        let end_pos = end + MANAGED_END.len();
        format!(
            "{}{}{}",
            &existing[..start],
            new_block,
            &existing[end_pos..]
        )
    } else {
        // Append
        format!("{}\n{}", existing.trim_end(), new_block)
    }
}

/// Replace a stale generated repo header with the current workspace repo name.
///
/// Existing BUILD.bazel files may have been generated before the workspace-wide
/// repo naming fix, so `sync` needs to normalize the header before it splices
/// the managed block. This preserves any hand-authored content outside the
/// managed section while fixing the `load("@*_crates//:defs.bzl", ...)` line.
fn normalize_repo_header(existing: &str, repo_name: &str, has_build_script: bool) -> String {
    let desired = if has_build_script {
        build_file_header_with_build_script(repo_name)
    } else {
        build_file_header(repo_name)
    };

    let expected_first_line = "load(\"@";
    let Some(start) = existing.find(expected_first_line) else {
        return existing.to_string();
    };

    let lines: Vec<&str> = existing[start..].lines().collect();
    if lines.is_empty() {
        return existing.to_string();
    }

    let mut end = start;
    let mut seen_header_line = false;
    for line in lines {
        let trimmed = line.trim();
        if trimmed.starts_with("load(\"@") {
            seen_header_line = true;
            end += line.len() + 1;
            continue;
        }
        if seen_header_line {
            break;
        }
        return existing.to_string();
    }

    let mut result = String::with_capacity(existing.len() + desired.len());
    result.push_str(&existing[..start]);
    result.push_str(&desired);
    result.push_str(&existing[end..]);
    result
}

fn find_workspace_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        if current.join("MODULE.bazel").exists() {
            return Some(current);
        }
        match current.parent() {
            Some(p) => current = p.to_path_buf(),
            None => return None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // ── extract_string_value ──────────────────────────────────────

    #[test]
    fn extract_string_double_quotes() {
        assert_eq!(
            extract_string_value("name = \"foo\""),
            Some("foo".to_string())
        );
    }

    #[test]
    fn extract_string_single_quotes() {
        assert_eq!(
            extract_string_value("name = 'bar'"),
            Some("bar".to_string())
        );
    }

    #[test]
    fn extract_string_with_spaces() {
        assert_eq!(
            extract_string_value("name  =  \"spaced\""),
            Some("spaced".to_string())
        );
    }

    #[test]
    fn extract_string_empty() {
        assert_eq!(extract_string_value("name = \"\""), None);
    }

    #[test]
    fn extract_string_no_equals() {
        assert_eq!(extract_string_value("name"), None);
    }

    // ── parse_cargo_targets ───────────────────────────────────────

    #[test]
    fn parse_single_bin() {
        let toml = "[[bin]]\nname = \"my-cli\"\npath = \"src/main.rs\"";
        let targets = parse_cargo_targets(toml, "bin");
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].name, "my-cli");
        assert_eq!(targets[0].path, Some("src/main.rs".to_string()));
    }

    #[test]
    fn parse_multiple_bins() {
        let toml = "[[bin]]\nname = \"a\"\n\n[[bin]]\nname = \"b\"\npath = \"src/b.rs\"";
        let targets = parse_cargo_targets(toml, "bin");
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].name, "a");
        assert_eq!(targets[0].path, None);
        assert_eq!(targets[1].name, "b");
        assert_eq!(targets[1].path, Some("src/b.rs".to_string()));
    }

    #[test]
    fn parse_tests() {
        let toml = "[[test]]\nname = \"integration\"\npath = \"tests/it.rs\"";
        let targets = parse_cargo_targets(toml, "test");
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].name, "integration");
    }

    #[test]
    fn parse_no_matching_sections() {
        let toml = "[package]\nname = \"foo\"\n\n[dependencies]\ntokio = \"1\"";
        let targets = parse_cargo_targets(toml, "bin");
        assert!(targets.is_empty());
    }

    #[test]
    fn parse_bin_stops_at_other_section() {
        let toml = "[[bin]]\nname = \"cli\"\n\n[dependencies]\ntokio = \"1\"";
        let targets = parse_cargo_targets(toml, "bin");
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].name, "cli");
    }

    // ── splice_managed_block ──────────────────────────────────────

    #[test]
    fn splice_replaces_existing_block() {
        let existing = "# hand authored\nrust_binary(name = \"foo\")\n\n# BEGIN raz-managed\nold stuff\n# END raz-managed\n";
        let new_block = "# BEGIN raz-managed\nnew stuff\n# END raz-managed\n";
        let result = splice_managed_block(existing, new_block);
        assert!(result.contains("new stuff"));
        assert!(!result.contains("old stuff"));
        assert!(result.contains("hand authored"));
    }

    #[test]
    fn splice_appends_when_no_markers() {
        let existing = "# hand authored\nrust_binary(name = \"foo\")\n";
        let new_block = "# BEGIN raz-managed\nnew stuff\n# END raz-managed\n";
        let result = splice_managed_block(existing, new_block);
        assert!(result.contains("hand authored"));
        assert!(result.contains("new stuff"));
    }

    #[test]
    fn normalize_repo_header_updates_stale_repo_name() {
        let existing = r#"load("@server_crates//:defs.bzl", "all_crate_deps")
load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_doc_test", "rust_library", "rust_test", "rust_test_suite")

# BEGIN raz-managed
rust_binary(
    name = "server_bin",
    srcs = ["src/main.rs"],
    crate_root = "src/main.rs",
    deps = all_crate_deps(normal = True),
)
# END raz-managed
"#;

        let result = normalize_repo_header(existing, "complex_bazel_setup", false);
        assert!(result.contains("@complex_bazel_setup//:defs.bzl"));
        assert!(!result.contains("@server_crates//:defs.bzl"));
        assert!(result.contains("server_bin"));
    }

    #[test]
    fn process_crate_updates_header_even_without_new_targets() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let server = root.join("server");
        fs::create_dir(&server).unwrap();

        fs::write(
            server.join("Cargo.toml"),
            r#"[package]
name = "server"
version = "0.1.0"
"#,
        )
        .unwrap();
        fs::write(
            server.join("BUILD.bazel"),
            r#"load("@server_crates//:defs.bzl", "all_crate_deps")
load("@rules_rust//rust:defs.bzl", "rust_binary")

# BEGIN raz-managed
rust_binary(
    name = "server_bin",
    srcs = ["src/main.rs"],
    crate_root = "src/main.rs",
    deps = all_crate_deps(normal = True),
)
# END raz-managed
"#,
        )
        .unwrap();

        process_crate(&server, "server", "complex_bazel_setup", false).unwrap();

        let build = fs::read_to_string(server.join("BUILD.bazel")).unwrap();
        assert!(build.contains("@complex_bazel_setup//:defs.bzl"));
        assert!(!build.contains("@server_crates//:defs.bzl"));
    }

    // ── names_outside_managed_block ───────────────────────────────

    #[test]
    fn names_extracts_all_names() {
        let content = "rust_binary(name = \"foo\")\nrust_test(name = \"bar\")\n";
        let names = names_outside_managed_block(content);
        assert!(names.contains("foo"));
        assert!(names.contains("bar"));
    }

    #[test]
    fn names_empty_content() {
        let names = names_outside_managed_block("");
        assert!(names.is_empty());
    }

    // ── build_file_header ─────────────────────────────────────────

    #[test]
    fn header_contains_repo_name() {
        let header = build_file_header("my_crate_deps");
        assert!(header.contains("@my_crate_deps"));
        assert!(header.contains("rust_binary"));
        assert!(header.contains("rust_library"));
    }

    #[test]
    fn header_with_build_script() {
        let header = build_file_header_with_build_script("repo");
        assert!(header.contains("cargo_build_script"));
        assert!(header.contains("@repo"));
    }
}
