use anyhow::{Context, Result};
use std::{env, fs, path::PathBuf};
use tracing::info;
use walkdir::WalkDir;

use crate::config::generators::{
    create_default_config, create_root_config, create_workspace_config,
};
use crate::config::templates::{
    create_bazel_config, create_combined_config, create_rustc_config,
    create_single_file_script_config,
};
use crate::config::workspace::{get_package_name, is_workspace_only};

pub fn init_command(
    cwd: Option<&str>,
    force: bool,
    rustc: bool,
    single_file_script: bool,
    bazel: bool,
    workspace_name: Option<&str>,
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
        let config_path = project_root.join(".cargo-runner.json");
        if config_path.exists() && !force {
            println!("❌ Config already exists at: {}", config_path.display());
            println!("   Use --force to overwrite");
            return Ok(());
        }
        let ws_name = workspace_name
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                project_root
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string()
            });
        println!("🔥 Generating Bazel .cargo-runner.json for workspace: {}", ws_name);
        let config = create_bazel_config(&ws_name);
        fs::write(&config_path, config)
            .with_context(|| format!("Failed to write config to {}", config_path.display()))?;
        println!("✅ Created: {}", config_path.display());
        println!("\n📌 Next steps:");
        println!("   1. Set your Bazel target labels in the config if needed");
        println!("   2. Run: cargo runner sync  (after any cargo add)");
        println!("   3. Run: cargo runner build-sync  (after adding new source files)");
        return Ok(());
    }

    // ── Handle --rustc / --single-file-script ──────────────────────────────
    if rustc || single_file_script {
        // Generate a single config file in the current directory
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

        // Print example usage
        if rustc {
            println!("\n📌 Example rustc config usage:");
            println!("   Add your rustc-specific settings to the 'rustc' section");
            println!("   Configure test_framework, binary_framework, etc.");
        } else {
            println!("\n📌 Example single-file-script config usage:");
            println!("   Add cargo script settings to the 'single_file_script' section");
            println!("   Configure extra_args, extra_env, etc.");
        }

        return Ok(());
    }

    // Normal cargo project initialization
    println!(
        "🚀 Initializing cargo-runner in: {}",
        project_root.display()
    );

    // Create a .cargo-runner.env file for easy sourcing
    let env_file_path = project_root.join(".cargo-runner.env");
    let env_content = format!("export PROJECT_ROOT=\"{}\"", project_root.display());
    fs::write(&env_file_path, &env_content)
        .with_context(|| format!("Failed to write env file to {}", env_file_path.display()))?;

    println!("✅ Created environment file: {}", env_file_path.display());

    // Find all Cargo.toml files recursively, excluding bazel directories
    let mut cargo_tomls = Vec::new();

    for entry in WalkDir::new(&project_root)
        .follow_links(true)
        .into_iter()
        .filter_entry(|e| {
            // Skip bazel-generated directories
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

    // Generate .cargo-runner.json for each project
    let mut created = 0;
    let mut skipped = 0;

    // Create root config with linkedProjects
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

    // Generate configs for each sub-project
    for cargo_toml in &cargo_tomls {
        // Skip if this is the root Cargo.toml
        if cargo_toml == &project_root.join("Cargo.toml") {
            continue;
        }

        let project_dir = cargo_toml.parent().unwrap();
        let config_path = project_dir.join(".cargo-runner.json");

        // Check if config already exists
        if config_path.exists() && !force {
            info!("Skipping existing config: {}", config_path.display());
            skipped += 1;
            continue;
        }

        // Check if this is a workspace-only Cargo.toml
        let config = if is_workspace_only(cargo_toml)? {
            // Create workspace config (no package name)
            create_workspace_config()
        } else {
            // Read package name from Cargo.toml
            let package_name = get_package_name(cargo_toml)?;
            // Create package configuration
            create_default_config(&package_name)
        };

        // Write configuration file
        fs::write(&config_path, config)
            .with_context(|| format!("Failed to write config to {}", config_path.display()))?;

        info!("Created config: {}", config_path.display());
        created += 1;
    }

    println!("\n✅ Initialization complete!");
    println!("   • Created {} config files", created);
    if skipped > 0 {
        println!(
            "   • Skipped {} existing configs (use --force to overwrite)",
            skipped
        );
    }

    // Print instructions for using PROJECT_ROOT
    println!("\n📌 To use PROJECT_ROOT in your current shell:");
    println!("   source {}", env_file_path.display());
    println!("\n   Or add to your shell profile (~/.bashrc, ~/.zshrc, etc.):");
    println!("   export PROJECT_ROOT=\"{}\"", project_root.display());

    Ok(())
}
