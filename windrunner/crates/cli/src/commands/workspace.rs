use anyhow::Result;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::config::bazel_workspace::find_cargo_workspace_root;

pub fn workspace_scan_roots(workspace_root: &Path) -> Result<Vec<PathBuf>> {
    let cargo_toml = workspace_root.join("Cargo.toml");
    let contents = fs::read_to_string(&cargo_toml)?;
    let value: toml::Value = contents
        .parse()
        .unwrap_or_else(|_| toml::Value::Table(Default::default()));

    let members = value
        .get("workspace")
        .and_then(|ws| ws.get("members"))
        .and_then(|members| members.as_array())
        .cloned()
        .unwrap_or_default();

    if members.is_empty() {
        return Ok(vec![workspace_root.to_path_buf()]);
    }

    let mut roots = BTreeSet::new();
    for member in members {
        let Some(member) = member.as_str() else {
            continue;
        };
        expand_workspace_member(workspace_root, member, &mut roots);
    }

    if roots.is_empty() {
        roots.insert(workspace_root.to_path_buf());
    }

    Ok(roots.into_iter().collect())
}

pub fn workspace_rs_files(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for root in roots {
        if !root.exists() {
            continue;
        }

        for entry in WalkDir::new(root)
            .follow_links(true)
            .into_iter()
            .filter_entry(|e| {
                if let Some(name) = e.file_name().to_str() {
                    if name.starts_with('.') || name == "target" || name.starts_with("bazel-") {
                        return false;
                    }
                }
                true
            })
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file()
                && entry.path().extension().and_then(|s| s.to_str()) == Some("rs")
            {
                files.push(entry.path().to_path_buf());
            }
        }
    }
    files
}

pub fn find_files_for_module_path(
    runner: &cargo_runner_core::UnifiedRunner,
    module_path: &str,
    cwd: &Path,
) -> Result<Vec<PathBuf>> {
    let workspace_root = find_cargo_workspace_root(cwd).unwrap_or_else(|| cwd.to_path_buf());
    let scan_roots = workspace_scan_roots(&workspace_root)?;

    let mut matches = BTreeSet::new();
    for path in workspace_rs_files(&scan_roots) {
        let Ok(runnables) = runner.detect_runnables(&path) else {
            continue;
        };

        if runnables.iter().any(|r| r.module_path == module_path) {
            matches.insert(path);
        }
    }

    Ok(matches.into_iter().collect())
}

pub fn resolve_module_path_to_file(
    runner: &cargo_runner_core::UnifiedRunner,
    module_path: &str,
    cwd: &Path,
) -> Result<PathBuf> {
    let matches = find_files_for_module_path(runner, module_path, cwd)?;

    match matches.len() {
        0 => Err(anyhow::anyhow!(
            "No file found for module path: {module_path}"
        )),
        1 => Ok(matches.into_iter().next().expect("Length is 1")),
        _ => {
            let paths = matches
                .into_iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            Err(anyhow::anyhow!(
                "Module path is ambiguous: {module_path}. Matches: {paths}"
            ))
        }
    }
}

fn expand_workspace_member(root: &Path, member: &str, roots: &mut BTreeSet<PathBuf>) {
    if member.ends_with("/*") || member.ends_with("/**") {
        let base = member.trim_end_matches("/**").trim_end_matches("/*");
        let base_dir = root.join(base);
        if let Ok(entries) = fs::read_dir(&base_dir) {
            let mut dirs: Vec<_> = entries
                .flatten()
                .filter(|e| e.path().join("Cargo.toml").exists())
                .collect();
            dirs.sort_by_key(|e| e.file_name());
            for entry in dirs {
                roots.insert(entry.path());
            }
        }
    } else {
        roots.insert(root.join(member));
    }
}
