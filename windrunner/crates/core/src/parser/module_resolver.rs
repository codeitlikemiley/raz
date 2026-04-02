use crate::{
    error::{Error, Result},
    types::{Scope, ScopeKind},
};
use std::path::{Path, PathBuf};

pub struct ModuleResolver {
    package_name: Option<String>,
}

impl Default for ModuleResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleResolver {
    pub fn new() -> Self {
        Self { package_name: None }
    }

    pub fn with_package_name(package_name: String) -> Self {
        Self {
            package_name: Some(package_name),
        }
    }

    pub fn resolve_module_path(
        &self,
        file_path: &Path,
        scopes: &[Scope],
        target_scope: &Scope,
    ) -> Result<String> {
        // Special handling for functions inside impl blocks
        if matches!(
            target_scope.kind,
            ScopeKind::Function | ScopeKind::Test | ScopeKind::Benchmark
        ) {
            // Check if this function is inside an impl block
            if let Some(impl_scope) = scopes
                .iter()
                .filter(|s| matches!(s.kind, ScopeKind::Impl))
                .find(|s| s.contains_line(target_scope.start.line))
            {
                // Extract the type name from the impl block
                if let Some(impl_name) = &impl_scope.name {
                    let type_name = if impl_name.starts_with("impl ") {
                        impl_name
                            .strip_prefix("impl ")
                            .unwrap_or(impl_name)
                            .split(" for ")
                            .last()
                            .unwrap_or(impl_name)
                            .trim()
                    } else {
                        impl_name
                    };

                    // Return Type::method format
                    if let Some(method_name) = &target_scope.name {
                        return Ok(format!("{type_name}::{method_name}"));
                    }
                }
            }
        }

        // Normal module path resolution for non-impl items
        let mut path_components = Vec::new();

        // For test functions and modules, we want a simpler path without package name
        let is_test_or_module = matches!(target_scope.kind, ScopeKind::Test | ScopeKind::Module);

        // Check if this is inside a test module
        let is_in_test_module = scopes
            .iter()
            .filter(|s| matches!(s.kind, ScopeKind::Module))
            .filter(|s| s.contains_line(target_scope.start.line))
            .any(|s| s.name.as_deref() == Some("tests"));

        // Skip package name for test functions, modules, and items in test modules
        let should_include_package = !is_test_or_module && !is_in_test_module;

        if let Some(ref pkg) = self.package_name {
            if should_include_package {
                path_components.push(pkg.clone());
            }
        }

        // Determine module path from file location
        let file_module_path = self.get_file_module_path(file_path)?;
        if !file_module_path.is_empty() {
            path_components.extend(file_module_path);
        }

        // Add inline module hierarchy
        let inline_modules = self.get_inline_module_hierarchy(scopes, target_scope);
        path_components.extend(inline_modules);

        // Don't add the function/test name to the module path - it will be added by the command builder
        // Only add names for non-executable items (like structs)
        if matches!(target_scope.kind, ScopeKind::Struct | ScopeKind::Impl) {
            if let Some(ref name) = target_scope.name {
                path_components.push(name.clone());
            }
        }

        Ok(path_components.join("::"))
    }

    fn get_file_module_path(&self, file_path: &Path) -> Result<Vec<String>> {
        let mut components = Vec::new();

        // Get the path relative to src/
        let path_str = file_path
            .to_str()
            .ok_or_else(|| Error::ParseError("Invalid file path".to_string()))?;

        // Check for /src/ or if path starts with src/
        let (src_index, offset) = if let Some(idx) = path_str.find("/src/") {
            (Some(idx), 5)
        } else if path_str.starts_with("src/") {
            (Some(0), 4)
        } else {
            (None, 0)
        };

        if let Some(src_index) = src_index {
            let relative_path = &path_str[src_index + offset..];
            let path_without_ext = relative_path.trim_end_matches(".rs");

            // Handle special cases
            if path_without_ext == "main" || path_without_ext == "lib" {
                return Ok(components);
            }
            
            // Handle binary files in src/bin/
            if path_without_ext.starts_with("bin/") {
                // For files in src/bin/, we don't include the bin prefix in the module path
                // The file src/bin/proxy.rs has an implicit module path of just the inline modules
                return Ok(components);
            }

            // Split path into module components
            for component in path_without_ext.split('/') {
                if component == "mod" {
                    // Skip mod.rs
                    continue;
                }
                components.push(component.to_string());
            }
        } else if let Some(src_index) = path_str.find("/tests/") {
            // Handle test files
            let relative_path = &path_str[src_index + 7..];
            let path_without_ext = relative_path.trim_end_matches(".rs");

            components.push("tests".to_string());
            for component in path_without_ext.split('/') {
                if component != "mod" && !component.is_empty() {
                    components.push(component.to_string());
                }
            }
        } else if let Some(src_index) = path_str.find("/benches/") {
            // Handle benchmark files
            let relative_path = &path_str[src_index + 9..];
            let path_without_ext = relative_path.trim_end_matches(".rs");

            components.push("benches".to_string());
            for component in path_without_ext.split('/') {
                if component != "mod" && !component.is_empty() {
                    components.push(component.to_string());
                }
            }
        }

        Ok(components)
    }

    fn get_inline_module_hierarchy(&self, scopes: &[Scope], target_scope: &Scope) -> Vec<String> {
        let mut modules = Vec::new();

        // Find all module scopes that contain the target
        let containing_modules: Vec<&Scope> = scopes
            .iter()
            .filter(|s| s.kind == ScopeKind::Module)
            .filter(|s| s.contains_line(target_scope.start.line))
            .filter(|s| {
                s.start.line < target_scope.start.line || s.end.line > target_scope.end.line
            })
            .collect();

        // Sort by scope size (largest first = outermost)
        let mut sorted_modules = containing_modules;
        sorted_modules.sort_by_key(|s| std::cmp::Reverse(s.end.line - s.start.line));

        for module in sorted_modules {
            if let Some(ref name) = module.name {
                modules.push(name.clone());
            }
        }

        modules
    }

    pub fn get_package_name_from_cargo_toml(cargo_toml_path: &Path) -> Result<String> {
        let contents = std::fs::read_to_string(cargo_toml_path)?;

        // Use cargo_toml crate for proper parsing
        let manifest = cargo_toml::Manifest::from_str(&contents)
            .map_err(|e| Error::ParseError(format!("Failed to parse Cargo.toml: {e}")))?;

        manifest
            .package
            .as_ref()
            .ok_or_else(|| {
                Error::ParseError("No [package] section found in Cargo.toml".to_string())
            })
            .map(|pkg| pkg.name.clone())
    }

    pub fn find_cargo_toml(start_path: &Path) -> Option<PathBuf> {
        // Convert to absolute path first if it's relative
        let abs_path = if start_path.is_relative() {
            std::env::current_dir().ok()?.join(start_path)
        } else {
            start_path.to_path_buf()
        };

        // Determine the search boundary
        let boundary = if let Ok(project_root) = std::env::var("PROJECT_ROOT") {
            PathBuf::from(project_root)
        } else if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home)
        } else if let Ok(user_profile) = std::env::var("USERPROFILE") {
            // Windows fallback
            PathBuf::from(user_profile)
        } else {
            // Last resort: try to detect home directory from current path
            // This is a heuristic - if we're in /home/username or /Users/username, stop there
            Self::detect_home_from_path(&abs_path)?
        };

        let mut current = if abs_path.is_file() {
            abs_path.parent()?
        } else {
            &abs_path
        };

        loop {
            let cargo_toml = current.join("Cargo.toml");
            if cargo_toml.exists() {
                return Some(cargo_toml);
            }

            // Stop if we've reached the boundary
            if current == boundary || !current.starts_with(&boundary) {
                return None;
            }

            current = current.parent()?;
        }
    }

    fn detect_home_from_path(path: &Path) -> Option<PathBuf> {
        let mut current = path;

        while let Some(parent) = current.parent() {
            if let Some(dir_name) = parent.file_name() {
                let dir_str = dir_name.to_str()?;
                // Check if we're at /home or /Users
                if dir_str == "home" || dir_str == "Users" {
                    // The home directory is likely current (e.g., /home/username)
                    return Some(current.to_path_buf());
                }
            }
            current = parent;
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Position;

    #[test]
    fn test_file_module_path() {
        let resolver = ModuleResolver::new();

        let path = Path::new("/project/src/models/user.rs");
        let modules = resolver.get_file_module_path(path).unwrap();
        assert_eq!(modules, vec!["models", "user"]);

        let path = Path::new("/project/src/lib.rs");
        let modules = resolver.get_file_module_path(path).unwrap();
        assert!(modules.is_empty());

        let path = Path::new("/project/tests/integration_test.rs");
        let modules = resolver.get_file_module_path(path).unwrap();
        assert_eq!(modules, vec!["tests", "integration_test"]);
    }

    #[test]
    fn test_inline_module_hierarchy() {
        let resolver = ModuleResolver::new();

        let scopes = vec![
            Scope {
                start: Position {
                    line: 1,
                    character: 0,
                },
                end: Position {
                    line: 100,
                    character: 0,
                },
                kind: ScopeKind::File(crate::FileScope::Unknown),
                name: None,
            },
            Scope {
                start: Position {
                    line: 10,
                    character: 0,
                },
                end: Position {
                    line: 50,
                    character: 0,
                },
                kind: ScopeKind::Module,
                name: Some("outer".to_string()),
            },
            Scope {
                start: Position {
                    line: 20,
                    character: 0,
                },
                end: Position {
                    line: 40,
                    character: 0,
                },
                kind: ScopeKind::Module,
                name: Some("inner".to_string()),
            },
            Scope {
                start: Position {
                    line: 25,
                    character: 0,
                },
                end: Position {
                    line: 30,
                    character: 0,
                },
                kind: ScopeKind::Function,
                name: Some("test_fn".to_string()),
            },
        ];

        let target = &scopes[3]; // test_fn
        let modules = resolver.get_inline_module_hierarchy(&scopes, target);
        assert_eq!(modules, vec!["outer", "inner"]);
    }

    #[test]
    fn test_full_module_path() {
        let resolver = ModuleResolver::with_package_name("my_crate".to_string());

        let scopes = vec![
            Scope {
                start: Position {
                    line: 1,
                    character: 0,
                },
                end: Position {
                    line: 100,
                    character: 0,
                },
                kind: ScopeKind::File(crate::FileScope::Unknown),
                name: None,
            },
            Scope {
                start: Position {
                    line: 10,
                    character: 0,
                },
                end: Position {
                    line: 50,
                    character: 0,
                },
                kind: ScopeKind::Module,
                name: Some("tests".to_string()),
            },
            Scope {
                start: Position {
                    line: 20,
                    character: 0,
                },
                end: Position {
                    line: 30,
                    character: 0,
                },
                kind: ScopeKind::Function,
                name: Some("test_user_creation".to_string()),
            },
        ];

        let file_path = Path::new("/project/src/models/user.rs");
        let target = &scopes[2]; // test_user_creation

        let module_path = resolver
            .resolve_module_path(file_path, &scopes, target)
            .unwrap();
        // Function names are not included in module paths - they're added by the command builder
        assert_eq!(module_path, "models::user::tests");
    }
    
    #[test]
    fn test_bin_file_module_path() {
        let resolver = ModuleResolver::with_package_name("my_crate".to_string());
        
        // Test that files in src/bin/ don't include 'bin' in the module path
        let scopes = vec![
            Scope {
                start: Position {
                    line: 1,
                    character: 0,
                },
                end: Position {
                    line: 100,
                    character: 0,
                },
                kind: ScopeKind::File(crate::FileScope::Unknown),
                name: None,
            },
            Scope {
                start: Position {
                    line: 10,
                    character: 0,
                },
                end: Position {
                    line: 50,
                    character: 0,
                },
                kind: ScopeKind::Module,
                name: Some("tests".to_string()),
            },
            Scope {
                start: Position {
                    line: 20,
                    character: 0,
                },
                end: Position {
                    line: 30,
                    character: 0,
                },
                kind: ScopeKind::Function,
                name: Some("test_proxy_binary".to_string()),
            },
        ];
        
        // Test src/bin/proxy.rs
        let file_path = Path::new("/project/src/bin/proxy.rs");
        let target = &scopes[2]; // test_proxy_binary
        
        let module_path = resolver
            .resolve_module_path(file_path, &scopes, target)
            .unwrap();
        // The module path should NOT include 'bin::'
        assert_eq!(module_path, "tests");
    }
    
    #[test]
    fn test_bin_subdir_module_path() {
        let resolver = ModuleResolver::with_package_name("my_crate".to_string());
        
        // Test files in subdirectories under src/bin/
        let scopes = vec![
            Scope {
                start: Position {
                    line: 1,
                    character: 0,
                },
                end: Position {
                    line: 100,
                    character: 0,
                },
                kind: ScopeKind::File(crate::FileScope::Unknown),
                name: None,
            },
            Scope {
                start: Position {
                    line: 10,
                    character: 0,
                },
                end: Position {
                    line: 50,
                    character: 0,
                },
                kind: ScopeKind::Module,
                name: Some("server".to_string()),
            },
            Scope {
                start: Position {
                    line: 15,
                    character: 0,
                },
                end: Position {
                    line: 40,
                    character: 0,
                },
                kind: ScopeKind::Module,
                name: Some("tests".to_string()),
            },
            Scope {
                start: Position {
                    line: 20,
                    character: 0,
                },
                end: Position {
                    line: 30,
                    character: 0,
                },
                kind: ScopeKind::Function,
                name: Some("test_server".to_string()),
            },
        ];
        
        // Test src/bin/myapp/server.rs
        let file_path = Path::new("/project/src/bin/myapp/server.rs");
        let target = &scopes[3]; // test_server function
        
        let module_path = resolver
            .resolve_module_path(file_path, &scopes, target)
            .unwrap();
        // For files under src/bin/, the bin/ prefix is excluded entirely
        // So the module path is just the inline modules
        assert_eq!(module_path, "server::tests");
    }
}
