use anyhow::Result;
use std::{env, fs};
use tracing::info;

pub fn unset_command(clean: bool) -> Result<()> {
    println!("🔧 Unsetting cargo-runner configuration...");

    // Get current PROJECT_ROOT if set
    let project_root = env::var("PROJECT_ROOT").ok();

    if let Some(root) = &project_root {
        println!("📍 Current PROJECT_ROOT: {}", root);

        if clean {
            println!("🧹 Cleaning .cargo-runner.json files...");

            use walkdir::WalkDir;
            let mut removed = 0;

            for entry in WalkDir::new(root)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_name() == ".cargo-runner.json" {
                    if let Err(e) = fs::remove_file(entry.path()) {
                        eprintln!("   ⚠️  Failed to remove {}: {}", entry.path().display(), e);
                    } else {
                        info!("Removed: {}", entry.path().display());
                        removed += 1;
                    }
                }
            }

            println!("   • Removed {} config files", removed);
        }
    } else {
        println!("ℹ️  PROJECT_ROOT is not currently set");
    }

    // Note: We can't actually unset the environment variable for the parent shell
    println!("\n📌 To unset PROJECT_ROOT, run in your shell:");
    println!("   unset PROJECT_ROOT");

    Ok(())
}
