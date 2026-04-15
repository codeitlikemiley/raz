use std::path::{Path, PathBuf};

use crate::config::bazel_workspace::find_cargo_workspace_root;
use crate::config::workspace::{find_cargo_package_root, resolve_default_run_for_package};
use crate::commands::workspace::workspace_scan_roots;

/// Resolve an optional filepath argument to a concrete path string.
///
/// If `arg` is `None`, auto-detect the best Rust entry point from cwd:
///   1. `src/main.rs`          — standard binary crate
///   2. `src/bin/<first>.rs`   — named binary (no main.rs)
///   3. Workspace member `src/main.rs` when standing at a workspace root
///   4. `src/lib.rs`           — library-only crate (maps to `cargo test`)
///   5. First `*.rs` in `src/` — fallback for non-standard layouts
///   6. First `*.rs` in cwd    — last resort outside workspace roots
pub fn resolve_filepath_arg(arg: Option<String>) -> anyhow::Result<String> {
    if let Some(path) = arg {
        return Ok(path);
    }

    let cwd = std::env::current_dir()?;

    if let Some(entrypoint) = resolve_default_run_entrypoint(&cwd)? {
        return Ok(entrypoint.to_string_lossy().into_owned());
    }

    // 1. src/main.rs
    let main_rs = cwd.join("src/main.rs");
    if main_rs.exists() {
        return Ok(main_rs.to_string_lossy().into_owned());
    }

    // 2. src/bin/*.rs — named binaries (sorted for determinism)
    let bin_dir = cwd.join("src/bin");
    if let Ok(entries) = std::fs::read_dir(&bin_dir) {
        let mut bins: Vec<_> = entries
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == "rs"))
            .collect();
        bins.sort_by_key(|e| e.file_name());
        if let Some(first) = bins.first() {
            return Ok(first.path().to_string_lossy().into_owned());
        }
    }

    if find_cargo_workspace_root(&cwd).as_deref() == Some(cwd.as_path()) {
        if let Some(entrypoint) = resolve_workspace_binary_entrypoint(&cwd)? {
            return Ok(entrypoint.to_string_lossy().into_owned());
        }

        anyhow::bail!(
            "No default binary entry point found in this workspace root.\n\
             Pass an explicit file or module path, e.g. `cargo runner run crates/cli/src/main.rs`."
        );
    }

    // 3. src/lib.rs — library crate; cargo runner maps this to `cargo test`
    let lib_rs = cwd.join("src/lib.rs");
    if lib_rs.exists() {
        return Ok(lib_rs.to_string_lossy().into_owned());
    }

    // 4. Any other .rs in src/
    if let Ok(entries) = std::fs::read_dir(cwd.join("src")) {
        let mut rs_files: Vec<_> = entries
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == "rs"))
            .collect();
        rs_files.sort_by_key(|e| e.file_name());
        if let Some(first) = rs_files.first() {
            return Ok(first.path().to_string_lossy().into_owned());
        }
    }

    // 5. Any .rs in cwd itself
    if let Ok(entries) = std::fs::read_dir(&cwd) {
        let mut rs_files: Vec<_> = entries
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == "rs"))
            .collect();
        rs_files.sort_by_key(|e| e.file_name());
        if let Some(first) = rs_files.first() {
            return Ok(first.path().to_string_lossy().into_owned());
        }
    }

    anyhow::bail!(
        "No Rust entry point found in {}.\n\
         Hint: pass a file explicitly, e.g.  cargo runner run src/main.rs",
        cwd.display()
    )
}

fn resolve_workspace_binary_entrypoint(cwd: &Path) -> anyhow::Result<Option<PathBuf>> {
    let Some(workspace_root) = find_cargo_workspace_root(cwd) else {
        return Ok(None);
    };
    if workspace_root != cwd {
        return Ok(None);
    }

    let scan_roots = workspace_scan_roots(&workspace_root)?;
    let mut candidates = Vec::new();

    for root in scan_roots {
        let main_rs = root.join("src/main.rs");
        if main_rs.exists() {
            candidates.push(main_rs);
        }
    }

    match candidates.len() {
        0 => Ok(None),
        1 => Ok(candidates.into_iter().next()),
        _ => Err(anyhow::anyhow!(
            "Workspace root has multiple binary entry points. Pass one explicitly:\n{}",
            candidates
                .into_iter()
                .map(|p| format!("  - {}", p.display()))
                .collect::<Vec<_>>()
                .join("\n")
        )),
    }
}

fn resolve_default_run_entrypoint(cwd: &Path) -> anyhow::Result<Option<PathBuf>> {
    if let Some(package_root) = find_cargo_package_root(cwd) {
        return resolve_default_run_for_package(&package_root);
    }

    let Some(workspace_root) = find_cargo_workspace_root(cwd) else {
        return Ok(None);
    };
    if workspace_root != cwd {
        return Ok(None);
    }

    let scan_roots = workspace_scan_roots(&workspace_root)?;
    let mut candidates = Vec::new();

    for root in scan_roots {
        match resolve_default_run_for_package(&root) {
            Ok(Some(entrypoint)) => candidates.push(entrypoint),
            Ok(None) => {}
            Err(err) => return Err(err),
        }
    }

    match candidates.len() {
        0 => Ok(None),
        1 => Ok(candidates.into_iter().next()),
        _ => Err(anyhow::anyhow!(
            "Multiple workspace packages declare default-run. Pass a file explicitly:\n{}",
            candidates
                .into_iter()
                .map(|p| format!("  - {}", p.display()))
                .collect::<Vec<_>>()
                .join("\n")
        )),
    }
}
