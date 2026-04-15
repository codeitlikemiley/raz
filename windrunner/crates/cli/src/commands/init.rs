use anyhow::{Context, Result};
use std::collections::HashSet;
use std::{
    env, fs,
    path::{Path, PathBuf},
};
use tracing::info;
use walkdir::WalkDir;

use crate::commands::build_sync::{
    BazelTarget, build_file_header, build_file_header_with_build_script, infer_targets,
    render_managed_block,
};
use crate::config::bazel_workspace::{
    cargo_workspace_repo_name_for_path, crate_repo_name, find_cargo_workspace_root,
};
use crate::config::generators::{
    create_default_config, create_root_config, create_workspace_config,
};
use crate::config::templates::{
    create_bazel_config, create_combined_config, create_rustc_config,
    create_single_file_script_config,
};
use crate::config::workspace::{
    get_package_name, is_workspace_only, local_dependency_labels, rust_crate_name,
};

pub fn init_command(
    cwd: Option<&str>,
    force: bool,
    rustc: bool,
    single_file_script: bool,
    bazel: bool,
    workspace_name: Option<&str>,
    skip_sync: bool,
) -> Result<()> {
    // Determine the project root
    let project_root = if let Some(cwd) = cwd {
        PathBuf::from(cwd)
    } else {
        env::current_dir().context("Failed to get current directory")?
    };

    let project_root = project_root
        .canonicalize()
        .context("Failed to canonicalize project root")?;

    // ── Handle --bazel ──────────────────────────────────────────────────────
    if bazel {
        return handle_bazel_init(&project_root, force, workspace_name, skip_sync);
    }

    // ── Handle --rustc / --single-file-script ──────────────────────────────
    if rustc || single_file_script {
        let config_path = project_root.join(".cargo-runner.json");

        if config_path.exists() && !force {
            println!("❌ Config already exists at: {}", config_path.display());
            println!("   Use --force to overwrite");
            return Ok(());
        }

        let config = if rustc && single_file_script {
            println!("🦀 Generating combined rustc and single-file-script configuration");
            create_combined_config()
        } else if rustc {
            println!("🦀 Generating rustc configuration for standalone files");
            create_rustc_config()
        } else {
            println!("📜 Generating single-file-script configuration");
            create_single_file_script_config()
        };

        fs::write(&config_path, config)
            .with_context(|| format!("Failed to write config to {}", config_path.display()))?;

        println!("✅ Created config: {}", config_path.display());

        if rustc {
            println!("\n📌 Example rustc config usage:");
            println!("   Add your rustc-specific settings to the 'rustc' section");
        } else {
            println!("\n📌 Example single-file-script config usage:");
            println!("   Add cargo script settings to the 'single_file_script' section");
        }

        return Ok(());
    }

    // ── Normal cargo project initialization ───────────────────────────────
    println!(
        "🚀 Initializing cargo-runner in: {}",
        project_root.display()
    );

    let env_file_path = project_root.join(".cargo-runner.env");
    let env_content = format!("export PROJECT_ROOT=\"{}\"", project_root.display());
    fs::write(&env_file_path, &env_content)
        .with_context(|| format!("Failed to write env file to {}", env_file_path.display()))?;

    println!("✅ Created environment file: {}", env_file_path.display());

    let mut cargo_tomls = Vec::new();

    for entry in WalkDir::new(&project_root)
        .follow_links(true)
        .into_iter()
        .filter_entry(|e| {
            if let Some(name) = e.file_name().to_str() {
                if name.starts_with("bazel-") {
                    return false;
                }
            }
            true
        })
        .filter_map(|e| e.ok())
    {
        if entry.file_name() == "Cargo.toml" {
            cargo_tomls.push(entry.path().to_path_buf());
        }
    }

    println!("📦 Found {} Cargo.toml files", cargo_tomls.len());

    let mut created = 0;
    let mut skipped = 0;

    let root_config_path = project_root.join(".cargo-runner.json");
    if !root_config_path.exists() || force {
        let root_config = create_root_config(&project_root, &cargo_tomls)?;
        fs::write(&root_config_path, root_config).with_context(|| {
            format!(
                "Failed to write root config to {}",
                root_config_path.display()
            )
        })?;
        info!("Created root config: {}", root_config_path.display());
        created += 1;
    } else {
        info!(
            "Skipping existing root config: {}",
            root_config_path.display()
        );
        skipped += 1;
    }

    for cargo_toml in &cargo_tomls {
        if cargo_toml == &project_root.join("Cargo.toml") {
            continue;
        }

        let project_dir = cargo_toml
            .parent()
            .context("Expected Cargo.toml to have a parent directory")?;
        let config_path = project_dir.join(".cargo-runner.json");

        if config_path.exists() && !force {
            info!("Skipping existing config: {}", config_path.display());
            skipped += 1;
            continue;
        }

        let config = if is_workspace_only(cargo_toml)? {
            create_workspace_config()
        } else {
            let package_name = get_package_name(cargo_toml)?;
            create_default_config(&package_name)
        };

        fs::write(&config_path, config)
            .with_context(|| format!("Failed to write config to {}", config_path.display()))?;

        info!("Created config: {}", config_path.display());
        created += 1;
    }

    println!("\n✅ Initialization complete!");
    println!("   • Created {created} config files");
    if skipped > 0 {
        println!("   • Skipped {skipped} existing configs (use --force to overwrite)");
    }

    println!("\n📌 To use PROJECT_ROOT in your current shell:");
    println!("   source {}", env_file_path.display());
    println!("\n   Or add to your shell profile (~/.bashrc, ~/.zshrc, etc.):");
    println!("   export PROJECT_ROOT=\"{}\"", project_root.display());

    Ok(())
}

// ── Bazel scaffold (absorbed from bazel-init) ─────────────────────────────────
//
// Called when `cargo runner init --bazel` is run.
// Decision tree:
//   - MODULE.bazel exists + !force  → update .cargo-runner.json only
//   - MODULE.bazel missing OR force  → full workspace scaffold + .cargo-runner.json
//
fn handle_bazel_init(
    project_root: &Path,
    force: bool,
    workspace_name: Option<&str>,
    skip_sync: bool,
) -> Result<()> {
    let module_bazel = project_root.join("MODULE.bazel");
    let already_bazel = module_bazel.exists();

    let ws_name = workspace_name.map(|s| s.to_string()).unwrap_or_else(|| {
        project_root
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    });

    // Detect Cargo workspace members
    let cargo_toml_path = project_root.join("Cargo.toml");
    let workspace_members = if cargo_toml_path.exists() {
        parse_workspace_members(&cargo_toml_path)?
    } else {
        Vec::new()
    };
    let is_workspace = !workspace_members.is_empty();

    if already_bazel && !force {
        // Already a Bazel workspace — re-sync BUILD.bazel targets + config
        println!("🔥 Bazel workspace already initialised: {ws_name}");
        println!("   Re-scanning source files to update BUILD.bazel targets…\n");

        // Run build-sync logic across all detected crates
        let crates =
            crate::config::bazel_workspace::find_bazel_crates(project_root).unwrap_or_default();

        if crates.is_empty() {
            // No BUILD.bazel files yet — fall through to full scaffold
            println!("   No existing BUILD.bazel files found. Running full scaffold…\n");
        } else {
            for krate in &crates {
                crate::commands::build_sync::process_crate(
                    &krate.dir,
                    &krate.name,
                    &krate.repo_name,
                    false,
                )?;
            }

            write_bazel_runner_config(project_root, &ws_name, false)?;
            println!("\n✅ BUILD.bazel targets up to date.");
            println!(
                "\n📌 Tip: use --force to regenerate all scaffolding files (MODULE.bazel, .bazelrc, etc.)"
            );
            return Ok(());
        }
    }

    // Full scaffold
    if is_workspace {
        println!(
            "🚀 Scaffolding Bazel workspace (Cargo workspace with {} members): {}",
            workspace_members.len(),
            ws_name
        );
    } else {
        println!("🚀 Scaffolding Bazel workspace: {ws_name}");
    }
    println!("   Directory: {}", project_root.display());
    println!();

    let cargo_tomls = discover_cargo_tomls(project_root);
    let cargo_workspace_blocks = collect_cargo_workspace_blocks(project_root, &cargo_tomls);

    // ── Generate MODULE.bazel ─────────────────────────────────────────────
    // For mixed Bazel/Cargo trees: group Cargo manifests by their Cargo
    // workspace root so crate_universe sees one workspace per `from_cargo`.
    write_file_if(
        project_root,
        "MODULE.bazel",
        &module_bazel_content(&ws_name, &cargo_workspace_blocks),
        force,
    )?;
    write_file_if(project_root, ".bazelversion", BAZEL_VERSION, force)?;
    write_file_if(project_root, ".bazelrc", BAZELRC_CONTENT, force)?;

    if is_workspace {
        // ── Workspace mode: scaffold each member ─────────────────────────
        // Root BUILD.bazel is empty (no Rust sources at root)
        write_file_if(project_root, "BUILD.bazel", EMPTY_BUILD_CONTENT, force)?;

        for member_rel in &workspace_members {
            let member_dir = project_root.join(member_rel);
            if !member_dir.exists() {
                println!("   ⚠️  member '{member_rel}' not found, skipping");
                continue;
            }

            let member_name = read_cargo_package_name(&member_dir).unwrap_or_else(|| {
                member_dir
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string()
            });
            let member_repo = cargo_workspace_repo_name_for_path(&member_dir)
                .unwrap_or_else(|| crate_repo_name(&member_name));

            println!("\n📁 {member_rel}/");

            // Generate BUILD.bazel for this member using build-sync inference
            let build_path = member_dir.join("BUILD.bazel");
            if build_path.exists() && !force {
                println!("   ~ skipping BUILD.bazel (already exists, use --force)");
            } else {
                let existing_names = HashSet::new();
                let (targets, _skipped) =
                    infer_targets(&member_dir, &member_name, &member_repo, &existing_names, "");

                if targets.is_empty() {
                    println!("   ⚠️  no targets inferred for {member_name}");
                } else {
                    let _local_deps = local_dependency_labels(&member_dir).unwrap_or_default();
                    let has_build_script = targets
                        .iter()
                        .any(|t| matches!(t, BazelTarget::BuildScript));
                    let header = if has_build_script {
                        build_file_header_with_build_script(&member_repo)
                    } else {
                        build_file_header(&member_repo)
                    };
                    let managed = render_managed_block(&targets);
                    let content = format!("{header}\n{managed}");
                    fs::write(&build_path, content)
                        .with_context(|| format!("Failed to write {}", build_path.display()))?;
                    for t in &targets {
                        println!("   ✅ {}", t.description());
                    }
                }
            }

            // Warn about build.rs
            if member_dir.join("build.rs").exists() {
                println!(
                    "   ⚠️  build.rs detected — review the generated cargo_build_script() rule."
                );
                println!(
                    "      See: https://bazelbuild.github.io/rules_rust/cargo.html#cargo_build_script"
                );
            }
        }
    } else {
        // ── Single-crate mode (existing behavior) ────────────────────────
        let has_main = project_root.join("src/main.rs").exists();
        let has_lib = project_root.join("src/lib.rs").exists();
        let pkg_name = read_cargo_package_name(project_root).unwrap_or_else(|| ws_name.clone());
        let repo_name = crate_repo_name(&pkg_name);
        let local_deps = local_dependency_labels(project_root).unwrap_or_default();
        write_file_if(
            project_root,
            "BUILD.bazel",
            &crate_build_content(&pkg_name, &repo_name, has_lib && !has_main, &local_deps),
            force,
        )?;
    }

    // ── Ensure Cargo.lock exists for each Cargo workspace root ───────────
    if is_workspace {
        for block in &cargo_workspace_blocks {
            let lockfile = block.workspace_root.join("Cargo.lock");
            if !lockfile.exists() {
                println!("\n📦 Generating Cargo.lock for {} ...", block.repo_name);
                let status = std::process::Command::new("cargo")
                    .arg("generate-lockfile")
                    .current_dir(&block.workspace_root)
                    .status()
                    .with_context(|| {
                        format!(
                            "Failed to run `cargo generate-lockfile` in {}",
                            block.workspace_root.display()
                        )
                    })?;
                if !status.success() {
                    anyhow::bail!(
                        "`cargo generate-lockfile` failed in {}",
                        block.workspace_root.display()
                    );
                }
            }
        }
    } else {
        let lockfile = project_root.join("Cargo.lock");
        if !lockfile.exists() {
            println!("\n📦 Generating Cargo.lock ...");
            let status = std::process::Command::new("cargo")
                .arg("generate-lockfile")
                .current_dir(project_root)
                .status()
                .context("Failed to run `cargo generate-lockfile`")?;
            if !status.success() {
                anyhow::bail!("`cargo generate-lockfile` failed");
            }
        }
    }

    // ── Write .cargo-runner.json ──────────────────────────────────────────
    write_bazel_runner_config(project_root, &ws_name, force)?;

    println!();
    println!("✅ Workspace scaffolded:");
    println!("   MODULE.bazel   — bzlmod deps");
    println!("   .bazelversion  — pins Bazel {BAZEL_VERSION}");
    println!("   .bazelrc       — build flags + shared caches");
    if is_workspace {
        println!(
            "   BUILD.bazel    — per-member targets ({} members)",
            workspace_members.len()
        );
    } else {
        let has_main = project_root.join("src/main.rs").exists();
        println!(
            "   BUILD.bazel    — {} target(s)",
            if has_main {
                "rust_binary"
            } else {
                "rust_library"
            }
        );
    }
    println!("   Cargo.lock     — required by crate_universe");
    println!("   .cargo-runner.json");

    if !skip_sync {
        println!();
        println!(
            "⏳ Running bazel sync (first run downloads ~1 GB of toolchain — cached forever after) …"
        );
        let status = std::process::Command::new("bazel")
            .arg("sync")
            .current_dir(project_root)
            .status()
            .context("Failed to run `bazel sync`")?;
        if !status.success() {
            println!(
                "⚠️  bazel sync had errors — workspace files are ready but deps may be incomplete."
            );
            println!("   Run `bazel sync` manually to retry.");
        } else {
            println!("✅ bazel sync complete.");

            // Validate BUILD files without compiling — catches config errors early
            print!("🔍 Validating BUILD files… ");
            let validate = std::process::Command::new("bazel")
                .args(["build", "--nobuild", "//..."])
                .current_dir(project_root)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped())
                .output();
            match validate {
                Ok(out) if out.status.success() => println!("✅ all targets valid."),
                Ok(out) => {
                    println!("⚠️  some targets have errors:");
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    // Show only the error lines, not the progress noise
                    for line in stderr
                        .lines()
                        .filter(|l| l.contains("ERROR") || l.contains("error"))
                    {
                        println!("   {line}");
                    }
                    println!("   Run `bazel build --nobuild //...` for full details.");
                }
                Err(_) => println!("⚠️  could not run validation (bazel not found?)"),
            }
        }
    } else {
        println!();
        println!("⏭️  Skipped bazel sync (--skip-sync). Run manually:");
        println!("   bazel sync");
    }

    println!();
    println!("📌 Next:");
    println!("   cargo runner run        — build + run via Bazel");
    println!("   cargo runner add <crate> — add a dep + sync in one step");
    if is_workspace {
        println!("   cargo runner build-sync — update BUILD.bazel after adding new files");
    }

    Ok(())
}

fn write_bazel_runner_config(project_root: &Path, ws_name: &str, force: bool) -> Result<()> {
    let config_path = project_root.join(".cargo-runner.json");
    if config_path.exists() && !force {
        println!("   ℹ️  .cargo-runner.json already exists (--force to overwrite)");
        return Ok(());
    }
    let config = create_bazel_config(ws_name);
    fs::write(&config_path, config)
        .with_context(|| format!("Failed to write {}", config_path.display()))?;
    println!("   ✅ .cargo-runner.json");
    Ok(())
}

fn write_file_if(root: &Path, name: &str, content: &str, force: bool) -> Result<()> {
    let path = root.join(name);
    if path.exists() && !force {
        println!("   ~ skipping {name} (already exists, use --force)");
        return Ok(());
    }
    fs::write(&path, content).with_context(|| format!("Failed to write {}", path.display()))?;
    println!("   ✅ {name}");
    Ok(())
}

fn read_cargo_package_name(root: &Path) -> Option<String> {
    let content = fs::read_to_string(root.join("Cargo.toml")).ok()?;
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
            if let Some(val) = trimmed.split_once('=').map(|x| x.1) {
                return Some(val.trim().trim_matches('"').trim_matches('\'').to_string());
            }
        }
    }
    None
}

// ── Scaffolding templates ─────────────────────────────────────────────────────

const RULES_RUST_VERSION: &str = "0.63.0";
const BAZEL_VERSION: &str = "7.4.1";

/// Parse `[workspace] members = ["a", "b"]` from a Cargo.toml.
///
/// Supports both inline arrays and multi-line arrays. Returns an empty vec if
/// the file is not a workspace or has no members.
fn parse_workspace_members(cargo_toml_path: &std::path::Path) -> Result<Vec<String>> {
    let content = fs::read_to_string(cargo_toml_path)
        .with_context(|| format!("Failed to read {}", cargo_toml_path.display()))?;

    if !content.contains("[workspace]") {
        return Ok(Vec::new());
    }

    let mut in_workspace = false;
    let mut in_members = false;
    let mut members = Vec::new();
    let mut members_line_buf = String::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Track sections
        if trimmed == "[workspace]" || trimmed.starts_with("[workspace]") {
            in_workspace = true;
            continue;
        }
        if trimmed.starts_with('[') && trimmed != "[workspace]" {
            in_workspace = false;
            in_members = false;
            continue;
        }

        if !in_workspace {
            continue;
        }

        // Look for `members = [...]`
        if trimmed.starts_with("members") {
            if let Some(rhs) = trimmed.split_once('=').map(|x| x.1) {
                let rhs = rhs.trim();
                if rhs.contains('[') && rhs.contains(']') {
                    // Single-line: members = ["a", "b"]
                    members_line_buf = rhs.to_string();
                } else if rhs.contains('[') {
                    // Multi-line start: members = [
                    in_members = true;
                    members_line_buf = rhs.to_string();
                    continue;
                }
            }
        } else if in_members {
            members_line_buf.push_str(trimmed);
            if trimmed.contains(']') {
                in_members = false;
            } else {
                continue;
            }
        } else {
            continue;
        }

        // Parse the collected buffer
        if !members_line_buf.is_empty() {
            // Extract strings between quotes
            let mut in_quote = false;
            let mut current = String::new();
            for ch in members_line_buf.chars() {
                match ch {
                    '"' => {
                        if in_quote && !current.is_empty() {
                            members.push(current.clone());
                            current.clear();
                        }
                        in_quote = !in_quote;
                    }
                    _ if in_quote => current.push(ch),
                    _ => {}
                }
            }
            members_line_buf.clear();
        }
    }

    // Expand globs like "crates/*"
    let mut expanded = Vec::new();
    let parent = cargo_toml_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    for member in members {
        if member.ends_with("/*") || member.ends_with("/**") {
            let base = member.trim_end_matches("/**").trim_end_matches("/*");
            let base_dir = parent.join(base);
            if let Ok(entries) = std::fs::read_dir(&base_dir) {
                let mut dirs: Vec<_> = entries
                    .flatten()
                    .filter(|e| e.path().join("Cargo.toml").exists())
                    .collect();
                dirs.sort_by_key(|e| e.file_name());
                for entry in dirs {
                    let rel = format!("{}/{}", base, entry.file_name().to_string_lossy());
                    expanded.push(rel);
                }
            }
        } else {
            expanded.push(member);
        }
    }

    Ok(expanded)
}

fn cargo_manifest_labels(project_root: &Path, cargo_tomls: &[PathBuf]) -> Vec<String> {
    let mut labels = Vec::new();
    for cargo_toml in cargo_tomls {
        if let Ok(rel) = cargo_toml.strip_prefix(project_root) {
            let rel = rel.to_string_lossy().replace('\\', "/");
            if rel == "Cargo.toml" {
                labels.push("//:Cargo.toml".to_string());
            } else if let Some(parent) = cargo_toml
                .parent()
                .and_then(|p| p.strip_prefix(project_root).ok())
            {
                let parent = parent.to_string_lossy().replace('\\', "/");
                labels.push(format!("//{parent}:Cargo.toml"));
            } else {
                labels.push("//:Cargo.toml".to_string());
            }
        }
    }

    if labels.is_empty() {
        labels.push("//:Cargo.toml".to_string());
    }

    labels.sort();
    labels.dedup();
    labels
}

#[derive(Debug, Clone)]
struct CargoWorkspaceBlock {
    workspace_root: PathBuf,
    repo_name: String,
    manifests: Vec<String>,
    lockfile_label: String,
}

fn collect_cargo_workspace_blocks(
    project_root: &Path,
    cargo_tomls: &[PathBuf],
) -> Vec<CargoWorkspaceBlock> {
    let mut groups: std::collections::BTreeMap<PathBuf, Vec<PathBuf>> =
        std::collections::BTreeMap::new();

    for cargo_toml in cargo_tomls {
        let workspace_root = find_cargo_workspace_root(
            cargo_toml
                .parent()
                .unwrap_or_else(|| std::path::Path::new(project_root)),
        )
        .unwrap_or_else(|| cargo_toml.parent().unwrap_or(project_root).to_path_buf());
        groups
            .entry(workspace_root)
            .or_default()
            .push(cargo_toml.clone());
    }

    let mut blocks = Vec::new();
    for (workspace_root, mut manifests) in groups {
        manifests.sort();
        manifests.dedup();

        let root_name = workspace_root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("workspace");
        let repo_name = crate_repo_name(root_name);
        let manifests = cargo_manifest_labels(project_root, &manifests);

        let lockfile_label = workspace_root_label(project_root, &workspace_root);
        blocks.push(CargoWorkspaceBlock {
            workspace_root,
            repo_name,
            manifests,
            lockfile_label,
        });
    }

    blocks
}

fn discover_cargo_tomls(project_root: &Path) -> Vec<PathBuf> {
    WalkDir::new(project_root)
        .follow_links(true)
        .into_iter()
        .filter_entry(|e| {
            if let Some(name) = e.file_name().to_str() {
                if name.starts_with("bazel-") {
                    return false;
                }
            }
            true
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name() == "Cargo.toml")
        .map(|e| e.path().to_path_buf())
        .collect()
}

fn module_bazel_content(ws_name: &str, blocks: &[CargoWorkspaceBlock]) -> String {
    let blocks = blocks
        .iter()
        .map(|block| {
            let manifests_block = block
                .manifests
                .iter()
                .map(|m| format!(r#"    "{m}","#))
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                r#"crate.from_cargo(
    name = "{repo}",
    manifests = [
{manifests_block}
    ],
    cargo_lockfile = "{lockfile}",
)

use_repo(crate, "{repo}")
"#,
                repo = block.repo_name,
                manifests_block = manifests_block,
                lockfile = block.lockfile_label,
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        r#"# MODULE.bazel — generated by `cargo runner init --bazel`
module(name = "{ws_name}")

bazel_dep(name = "rules_rust", version = "{RULES_RUST_VERSION}")

crate = use_extension(
    "@rules_rust//crate_universe:extensions.bzl",
    "crate",
)

{blocks}
"#
    )
}

fn workspace_root_label(project_root: &Path, workspace_root: &Path) -> String {
    if workspace_root == project_root {
        "//:Cargo.lock".to_string()
    } else if let Ok(rel) = workspace_root.strip_prefix(project_root) {
        let rel = rel.to_string_lossy().replace('\\', "/");
        format!("//{rel}:Cargo.lock")
    } else {
        "//:Cargo.lock".to_string()
    }
}

fn crate_build_content(pkg_name: &str, repo: &str, is_lib: bool, local_deps: &[String]) -> String {
    let local_deps_expr = if local_deps.is_empty() {
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
    };
    if is_lib {
        format!(
            r#"load("@{repo}//:defs.bzl", "all_crate_deps")
load("@rules_rust//rust:defs.bzl", "rust_doc_test", "rust_library", "rust_test")

rust_library(
    name = "{pkg_name}",
    srcs = glob(["src/**/*.rs"]),
    deps = {local_deps_expr}all_crate_deps(normal = True),
    visibility = ["//visibility:public"],
    crate_name = "{crate_name}",
)

rust_test(
    name = "{pkg_name}_test",
    crate = ":{pkg_name}",
    deps = {local_deps_expr}all_crate_deps(normal = True, normal_dev = True),
)

rust_doc_test(
    name = "doc_tests",
    crate = ":{pkg_name}",
)
"#,
            crate_name = rust_crate_name(pkg_name),
            local_deps_expr = local_deps_expr,
        )
    } else {
        format!(
            r#"load("@{repo}//:defs.bzl", "all_crate_deps")
load("@rules_rust//rust:defs.bzl", "rust_binary")

rust_binary(
    name = "{pkg_name}",
    srcs = glob(["src/**/*.rs"]),
    deps = {local_deps_expr}all_crate_deps(normal = True),
    visibility = ["//visibility:public"],
)
"#,
        )
    }
}

/// Root BUILD.bazel for workspaces — empty since sources live in member crates.
const EMPTY_BUILD_CONTENT: &str = "# BUILD.bazel — workspace root (no Rust sources here)\n\
# See member directories for individual crate targets.\n";

const BAZELRC_CONTENT: &str = r#"# .bazelrc — generated by `cargo runner init --bazel`
#
# Download minimization
# ---------------------
# Bazel downloads the Rust toolchain + all crates on first run (~1–2 GB).
# Everything is cached in ~/.cache/bazel after that — subsequent runs are fast.

# ── Disk cache (reused across workspaces on the same machine) ─────────────────
build --disk_cache=~/.cache/bazel-disk
fetch --disk_cache=~/.cache/bazel-disk

# ── Repository cache (share downloaded archives across Bazel workspaces) ──────
common --repository_cache=~/.cache/bazel-repo

# ── Build defaults ────────────────────────────────────────────────────────────
build --jobs=auto
build --keep_going

# ── Test defaults ─────────────────────────────────────────────────────────────
test --test_output=errors
test --keep_going

# ── Rust / rules_rust ─────────────────────────────────────────────────────────
build --@rules_rust//:extra_rustc_flags=-Dwarnings

# ── macOS / Apple Silicon ─────────────────────────────────────────────────────
# Uncomment if building on Apple Silicon and Bazel picks the wrong CPU:
# build --cpu=darwin_arm64
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // ── module_bazel_content ──────────────────────────────────────

    #[test]
    fn module_bazel_has_workspace_name() {
        let blocks = vec![CargoWorkspaceBlock {
            workspace_root: PathBuf::from("/tmp/project/server"),
            repo_name: "my_repo".to_string(),
            manifests: vec!["//server:Cargo.toml".to_string()],
            lockfile_label: "//server:Cargo.lock".to_string(),
        }];
        let content = module_bazel_content("my_workspace", &blocks);
        assert!(content.contains("module(name = \"my_workspace\")"));
    }

    #[test]
    fn module_bazel_has_repo_refs() {
        let blocks = vec![CargoWorkspaceBlock {
            workspace_root: PathBuf::from("/tmp/project/server"),
            repo_name: "my_deps".to_string(),
            manifests: vec!["//:Cargo.toml".to_string()],
            lockfile_label: "//server:Cargo.lock".to_string(),
        }];
        let content = module_bazel_content("ws", &blocks);
        assert!(content.contains("name = \"my_deps\""));
        assert!(content.contains("use_repo(crate, \"my_deps\")"));
        assert!(content.contains("cargo_lockfile = \"//server:Cargo.lock\""));
    }

    #[test]
    fn module_bazel_has_rules_rust_version() {
        let blocks = vec![CargoWorkspaceBlock {
            workspace_root: PathBuf::from("/tmp/project/server"),
            repo_name: "repo".to_string(),
            manifests: vec!["//:Cargo.toml".to_string()],
            lockfile_label: "//:Cargo.lock".to_string(),
        }];
        let content = module_bazel_content("ws", &blocks);
        assert!(content.contains(RULES_RUST_VERSION));
    }

    #[test]
    fn cargo_manifest_labels_use_workspace_members() {
        let root = PathBuf::from("/tmp/project");
        let labels = cargo_manifest_labels(
            &root,
            &[
                root.join("server/Cargo.toml"),
                root.join("corex/Cargo.toml"),
            ],
        );
        assert_eq!(
            labels,
            vec![
                "//corex:Cargo.toml".to_string(),
                "//server:Cargo.toml".to_string()
            ]
        );
    }

    #[test]
    fn workspace_root_label_uses_member_lockfile() {
        let project_root = PathBuf::from("/tmp/project");
        let workspace_root = PathBuf::from("/tmp/project/combos");
        assert_eq!(
            workspace_root_label(&project_root, &workspace_root),
            "//combos:Cargo.lock"
        );
    }

    // ── crate_build_content ───────────────────────────────────────

    #[test]
    fn build_content_library_has_lib_targets() {
        let content = crate_build_content("mylib", "repo", true, &[]);
        assert!(content.contains("rust_library"));
        assert!(content.contains("rust_test"));
        assert!(content.contains("rust_doc_test"));
        assert!(content.contains("name = \"mylib\""));
    }

    #[test]
    fn build_content_binary_has_bin_target() {
        let content = crate_build_content("mycli", "repo", false, &[]);
        assert!(content.contains("rust_binary"));
        assert!(!content.contains("rust_library"));
        assert!(!content.contains("rust_doc_test"));
        assert!(content.contains("name = \"mycli\""));
    }

    #[test]
    fn build_content_uses_repo_for_deps() {
        let content = crate_build_content("pkg", "custom_repo", true, &[]);
        assert!(content.contains("@custom_repo"));
        assert!(content.contains("all_crate_deps"));
    }

    #[test]
    fn bazel_workspace_member_build_uses_module_repo_name() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("windrunner");
        fs::create_dir(&root).unwrap();

        fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/cli\"]\n",
        )
        .unwrap();

        let cli = root.join("crates/cli");
        fs::create_dir_all(cli.join("src")).unwrap();
        fs::write(
            cli.join("Cargo.toml"),
            r#"[package]
name = "cargo-runner"
version = "0.1.0"
"#,
        )
        .unwrap();
        fs::write(cli.join("src/main.rs"), "fn main() {}\n").unwrap();

        handle_bazel_init(&root.to_path_buf(), true, Some("windrunner"), true).unwrap();

        let build = fs::read_to_string(cli.join("BUILD.bazel")).unwrap();
        assert!(build.contains("load(\"@windrunner_crates//:defs.bzl\""));
        assert!(!build.contains("load(\"@cargo_runner_crates//:defs.bzl\""));
    }
}
