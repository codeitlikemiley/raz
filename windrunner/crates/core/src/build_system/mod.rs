use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BuildSystem {
    Cargo,
    Bazel,
}

pub trait BuildSystemDetector {
    fn detect(project_path: &Path) -> Option<BuildSystem>;
}

pub struct DefaultBuildSystemDetector;

impl BuildSystemDetector for DefaultBuildSystemDetector {
    fn detect(project_path: &Path) -> Option<BuildSystem> {
        tracing::debug!(
            "DefaultBuildSystemDetector::detect checking path: {:?}",
            project_path
        );

        // Check for Bazel first since a project might have both.
        // For Bazel detection, we require BUILD files, not just MODULE.bazel/WORKSPACE.
        // This prevents standalone files from being detected as Bazel just because
        // they're under a directory tree with a MODULE.bazel file far up the hierarchy.
        let build_bazel = project_path.join("BUILD.bazel");
        let build = project_path.join("BUILD");

        if build_bazel.exists() {
            tracing::info!("Found BUILD.bazel at: {:?}", build_bazel);
            return Some(BuildSystem::Bazel);
        }

        if build.exists() {
            tracing::info!("Found BUILD at: {:?}", build);
            return Some(BuildSystem::Bazel);
        }

        // Check for Cargo
        let cargo_toml = project_path.join("Cargo.toml");
        if cargo_toml.exists() {
            tracing::debug!("Found Cargo.toml at: {:?}", cargo_toml);
            return Some(BuildSystem::Cargo);
        }

        tracing::debug!("No build system found at: {:?}", project_path);
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_detect_cargo_project() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        fs::write(&cargo_toml, "[package]\nname = \"test\"").unwrap();

        let build_system = DefaultBuildSystemDetector::detect(temp_dir.path());
        assert_eq!(build_system, Some(BuildSystem::Cargo));
    }

    #[test]
    fn test_detect_bazel_project_with_build_bazel_file() {
        let temp_dir = TempDir::new().unwrap();
        let build_file = temp_dir.path().join("BUILD.bazel");
        fs::write(&build_file, "rust_binary(name = \"test\")").unwrap();

        let build_system = DefaultBuildSystemDetector::detect(temp_dir.path());
        assert_eq!(build_system, Some(BuildSystem::Bazel));
    }

    #[test]
    fn test_detect_bazel_project_with_plain_build_file() {
        let temp_dir = TempDir::new().unwrap();
        let build_file = temp_dir.path().join("BUILD");
        fs::write(&build_file, "rust_binary(name = \"test\")").unwrap();

        let build_system = DefaultBuildSystemDetector::detect(temp_dir.path());
        assert_eq!(build_system, Some(BuildSystem::Bazel));
    }

    /// MODULE.bazel alone (without BUILD.bazel) does NOT trigger Bazel detection.
    /// This is intentional: prevents false positives when a file is deep in a Bazel
    /// workspace but the immediate directory has no BUILD target file.
    /// Detection requires BUILD.bazel/BUILD to be present at the checked directory.
    #[test]
    fn test_module_bazel_alone_does_not_trigger_detection() {
        let temp_dir = TempDir::new().unwrap();
        let module_file = temp_dir.path().join("MODULE.bazel");
        fs::write(&module_file, "module(name = \"test\")").unwrap();

        // Should NOT detect as Bazel — no BUILD file present
        let build_system = DefaultBuildSystemDetector::detect(temp_dir.path());
        assert_eq!(build_system, None);
    }

    /// WORKSPACE alone (without BUILD.bazel) does NOT trigger Bazel detection.
    /// Presence of a workspace root file is not sufficient to route a directory
    /// through the Bazel runner; we require an actual BUILD target file.
    #[test]
    fn test_workspace_alone_does_not_trigger_detection() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_file = temp_dir.path().join("WORKSPACE");
        fs::write(&workspace_file, "workspace(name = \"test\")").unwrap();

        // Should NOT detect as Bazel — no BUILD file present
        let build_system = DefaultBuildSystemDetector::detect(temp_dir.path());
        assert_eq!(build_system, None);
    }

    /// Real Bazel projects have both MODULE.bazel (workspace root) AND BUILD.bazel
    /// (per-package build file). Combined, they correctly trigger Bazel detection.
    #[test]
    fn test_module_bazel_plus_build_file_detects_bazel() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(
            temp_dir.path().join("MODULE.bazel"),
            "module(name = \"test\")",
        )
        .unwrap();
        fs::write(
            temp_dir.path().join("BUILD.bazel"),
            "rust_binary(name = \"bin\")",
        )
        .unwrap();

        let build_system = DefaultBuildSystemDetector::detect(temp_dir.path());
        assert_eq!(build_system, Some(BuildSystem::Bazel));
    }

    #[test]
    fn test_prefer_bazel_over_cargo() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        let build_file = temp_dir.path().join("BUILD.bazel");
        fs::write(&cargo_toml, "[package]\nname = \"test\"").unwrap();
        fs::write(&build_file, "rust_binary(name = \"test\")").unwrap();

        let build_system = DefaultBuildSystemDetector::detect(temp_dir.path());
        assert_eq!(build_system, Some(BuildSystem::Bazel));
    }

    #[test]
    fn test_no_build_system() {
        let temp_dir = TempDir::new().unwrap();

        let build_system = DefaultBuildSystemDetector::detect(temp_dir.path());
        assert_eq!(build_system, None);
    }
}
