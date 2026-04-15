use crate::config::Config;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContext {
    pub file_path: PathBuf,
    pub project_root: PathBuf,
    pub config: Arc<Config>,
    #[serde(default)]
    pub manifests: BTreeMap<String, PathBuf>,
}

impl ProjectContext {
    pub fn from_path(file_path: &Path, config: Arc<Config>) -> Self {
        let file_path = if file_path.is_absolute() {
            file_path.to_path_buf()
        } else {
            std::env::current_dir()
                .ok()
                .map(|cwd| cwd.join(file_path))
                .unwrap_or_else(|| file_path.to_path_buf())
        };

        let start = if file_path.is_file() {
            file_path.parent().unwrap_or(&file_path)
        } else {
            &file_path
        };

        let manifests = collect_manifests(start);
        let project_root = find_project_root(start).unwrap_or_else(|| start.to_path_buf());

        Self {
            file_path,
            project_root,
            config,
            manifests,
        }
    }

    pub fn has_manifest(&self, name: &str) -> bool {
        self.manifests.contains_key(name)
    }

    pub fn manifest(&self, name: &str) -> Option<&PathBuf> {
        self.manifests.get(name)
    }
}

const MANIFEST_NAMES: &[&str] = &[
    "MODULE.bazel",
    "BUILD.bazel",
    "BUILD",
    "Cargo.toml",
    "package.json",
    "go.mod",
    "Package.swift",
    "build.gradle.kts",
    "build.gradle",
    "pom.xml",
    "Dioxus.toml",
];

fn collect_manifests(start: &Path) -> BTreeMap<String, PathBuf> {
    let mut manifests = BTreeMap::new();

    for ancestor in start.ancestors() {
        for name in MANIFEST_NAMES {
            if manifests.contains_key(*name) {
                continue;
            }
            let candidate = ancestor.join(name);
            if candidate.exists() {
                manifests.insert(name.to_string(), candidate);
            }
        }
    }

    manifests
}

fn find_project_root(start: &Path) -> Option<PathBuf> {
    for ancestor in start.ancestors() {
        if MANIFEST_NAMES
            .iter()
            .any(|name| ancestor.join(name).exists())
        {
            return Some(ancestor.to_path_buf());
        }
    }

    None
}
