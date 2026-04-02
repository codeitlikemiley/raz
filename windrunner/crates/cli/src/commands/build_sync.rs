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
                        .map_or(false, |n| n.to_string_lossy() == filter)
            })
            .collect()
    } else {
        // Default: sync only the crate the user is inside of
        let local = all_crates
            .into_iter()
            .filter(|c| cwd.starts_with(&c.dir) || c.dir == cwd)
            .collect::<Vec<_>>();
        if local.is_empty() {
            anyhow::bail!(
                "Not inside a Bazel crate directory. Use --crate <dir> to specify one."
            );
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

fn process_crate(dir: &Path, crate_name: &str, repo_name: &str, dry_run: bool) -> Result<()> {
    println!("\n📁 Scanning: {}", dir.display());

    let build_path = dir.join("BUILD.bazel");

    // Read the existing file so we can skip already-defined targets
    let existing_content = if build_path.exists() {
        std::fs::read_to_string(&build_path)
            .with_context(|| format!("reading {}", build_path.display()))?
    } else {
        String::new()
    };

    // Collect names already defined *outside* the managed block
    let existing_names = names_outside_managed_block(&existing_content);

    let (targets, skipped) = infer_targets(dir, crate_name, repo_name, &existing_names);

    for name in &skipped {
        println!("   ~ skipping '{name}' (already defined)");
    }

    if targets.is_empty() {
        println!("   ✓ No new targets to add.");
        return Ok(());
    }

    let generated = render_managed_block(&targets);

    if dry_run {
        println!("   Would write to: {}", build_path.display());
        println!("   Generated block:\n{}", generated);
        return Ok(());
    }

    if !existing_content.is_empty() {
        let updated = splice_managed_block(&existing_content, &generated);
        std::fs::write(&build_path, updated)
            .with_context(|| format!("writing {}", build_path.display()))?;
    } else {
        // New file — write header + managed block
        let header = build_file_header(repo_name);
        std::fs::write(&build_path, format!("{}\n{}", header, generated))
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
enum BazelTarget {
    Library { name: String, repo_name: String },
    Binary { name: String, src: String, repo_name: String },
    TestSuite { name: String, repo_name: String },
    Example { name: String, src: String, repo_name: String },
    Bench { name: String, src: String, repo_name: String },
}

impl BazelTarget {
    /// The Bazel `name = "..."` value this target would produce.
    fn bazel_name(&self) -> String {
        match self {
            Self::Library { name, .. } => format!("{name}_lib"),
            Self::Binary { name, .. } => name.clone(),
            Self::TestSuite { .. } => "integration_tests".to_string(),
            Self::Example { name, .. } => format!("example_{name}"),
            Self::Bench { name, .. } => format!("bench_{name}"),
        }
    }

    fn description(&self) -> String {
        match self {
            Self::Library { name, .. } => format!("rust_library({name})"),
            Self::Binary { name, .. } => format!("rust_binary({name})"),
            Self::TestSuite { name, .. } => format!("rust_test_suite({name})"),
            Self::Example { name, .. } => format!("rust_binary(example_{name})"),
            Self::Bench { name, .. } => format!("rust_binary(bench_{name})"),
        }
    }

    fn render(&self) -> String {
        match self {
            Self::Library { name, repo_name: _ } => format!(
                r#"rust_library(
    name = "{name}_lib",
    srcs = glob(["src/**/*.rs"]),
    deps = all_crate_deps(),
    visibility = ["//visibility:public"],
    crate_name = "{name}",
)

rust_test(
    name = "unit_tests",
    crate = ":{name}_lib",
    deps = all_crate_deps(normal_dev = True),
)
"#,
            ),
            Self::Binary { name, src, repo_name: _ } => format!(
                r#"rust_binary(
    name = "{name}",
    srcs = ["{src}"],
    crate_root = "{src}",
    deps = all_crate_deps(normal = True),
)
"#,
            ),
            Self::TestSuite { name: _, repo_name: _ } => format!(
                r#"rust_test_suite(
    name = "integration_tests",
    srcs = glob(["tests/**/*.rs"]),
    deps = all_crate_deps(normal_dev = True),
)
"#,
            ),
            Self::Example { name, src, repo_name: _ } => format!(
                r#"rust_binary(
    name = "example_{name}",
    srcs = ["{src}"],
    crate_root = "{src}",
    deps = all_crate_deps(normal = True),
)
"#,
            ),
            Self::Bench { name, src, repo_name: _ } => format!(
                r#"rust_binary(
    name = "bench_{name}",
    srcs = ["{src}"],
    crate_root = "{src}",
    deps = all_crate_deps(normal_dev = True),
)
"#,
            ),
        }
    }
}

/// Returns all `name = "..."` values found anywhere in the file.
/// This ensures `build-sync` is idempotent — targets already in the managed
/// block are treated the same as hand-authored ones and are never duplicated.
fn names_outside_managed_block(content: &str) -> std::collections::HashSet<String> {
    let re = regex::Regex::new(r#"name\s*=\s*"([^"]+)""#).unwrap();
    re.captures_iter(content)
        .map(|c| c[1].to_string())
        .collect()
}

/// Returns `(new_targets, skipped_names)` where `skipped_names` are targets
/// that already exist in the hand-authored part of the file.
fn infer_targets(
    dir: &Path,
    crate_name: &str,
    repo_name: &str,
    existing_names: &std::collections::HashSet<String>,
) -> (Vec<BazelTarget>, Vec<String>) {
    let mut candidates: Vec<BazelTarget> = Vec::new();

    // src/lib.rs → rust_library + unit_tests
    if dir.join("src/lib.rs").exists() {
        candidates.push(BazelTarget::Library {
            name: crate_name.to_string(),
            repo_name: repo_name.to_string(),
        });
    }

    // src/main.rs → rust_binary named <crate>_bin
    if dir.join("src/main.rs").exists() {
        candidates.push(BazelTarget::Binary {
            name: format!("{}_bin", crate_name),
            src: "src/main.rs".to_string(),
            repo_name: repo_name.to_string(),
        });
    }

    // src/bin/*.rs → individual rust_binary per file
    if let Ok(entries) = std::fs::read_dir(dir.join("src/bin")) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "rs") {
                let stem = path.file_stem().unwrap().to_string_lossy().to_string();
                candidates.push(BazelTarget::Binary {
                    name: stem.clone(),
                    src: format!("src/bin/{}.rs", stem),
                    repo_name: repo_name.to_string(),
                });
            }
        }
    }

    // tests/ → rust_test_suite
    if dir.join("tests").exists() {
        candidates.push(BazelTarget::TestSuite {
            name: "integration_tests".to_string(),
            repo_name: repo_name.to_string(),
        });
    }

    // examples/*.rs → rust_binary per file
    if let Ok(entries) = std::fs::read_dir(dir.join("examples")) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "rs") {
                let stem = path.file_stem().unwrap().to_string_lossy().to_string();
                candidates.push(BazelTarget::Example {
                    name: stem.clone(),
                    src: format!("examples/{}.rs", stem),
                    repo_name: repo_name.to_string(),
                });
            }
        }
    }

    // benches/*.rs → rust_binary per file
    if let Ok(entries) = std::fs::read_dir(dir.join("benches")) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "rs") {
                let stem = path.file_stem().unwrap().to_string_lossy().to_string();
                candidates.push(BazelTarget::Bench {
                    name: stem.clone(),
                    src: format!("benches/{}.rs", stem),
                    repo_name: repo_name.to_string(),
                });
            }
        }
    }

    // Partition into new vs already-defined
    let mut targets = Vec::new();
    let mut skipped = Vec::new();
    for target in candidates {
        let bazel_name = target.bazel_name();
        if existing_names.contains(&bazel_name) {
            skipped.push(bazel_name);
        } else {
            targets.push(target);
        }
    }

    (targets, skipped)
}

// ── rendering ─────────────────────────────────────────────────────────────────

fn build_file_header(repo_name: &str) -> String {
    format!(
        r#"load("@{repo_name}//:defs.bzl", "all_crate_deps")
load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_library", "rust_test", "rust_test_suite")
"#
    )
}

fn render_managed_block(targets: &[BazelTarget]) -> String {
    let body: String = targets.iter().map(|t| t.render()).collect::<Vec<_>>().join("\n");
    format!("{MANAGED_BEGIN}\n{body}\n{MANAGED_END}\n")
}

/// Replace the existing managed block in `existing` with `new_block`, or
/// append the block if no markers are found.
fn splice_managed_block(existing: &str, new_block: &str) -> String {
    if let (Some(start), Some(end)) = (
        existing.find(MANAGED_BEGIN),
        existing.find(MANAGED_END),
    ) {
        let end_pos = end + MANAGED_END.len();
        format!("{}{}{}", &existing[..start], new_block, &existing[end_pos..])
    } else {
        // Append
        format!("{}\n{}", existing.trim_end(), new_block)
    }
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
