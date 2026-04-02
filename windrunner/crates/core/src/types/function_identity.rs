use super::FileType;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Represents the identity of a function in a Rust project
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionIdentity {
    pub package: Option<String>,
    pub module_path: Option<String>,
    pub file_path: Option<PathBuf>,
    pub function_name: Option<String>,
    pub file_type: Option<FileType>,
}

impl FunctionIdentity {
    /// Check if this identity matches another identity (partial match)
    /// Returns true if all non-None fields in `self` match the corresponding fields in `other`
    pub fn matches(&self, other: &FunctionIdentity) -> bool {
        // If self has a package requirement, it must match
        if let Some(ref my_package) = self.package {
            if let Some(ref other_package) = other.package {
                if my_package != other_package {
                    return false;
                }
            } else {
                // Self requires package but other doesn't have one
                return false;
            }
        }

        // If self has a module_path requirement, it must match
        if let Some(ref my_module) = self.module_path {
            if let Some(ref other_module) = other.module_path {
                if my_module != other_module {
                    return false;
                }
            } else {
                // Self requires module_path but other doesn't have one
                return false;
            }
        }

        // If self has a file_path requirement, it must match
        if let Some(ref my_file) = self.file_path {
            if let Some(ref other_file) = other.file_path {
                // Check if paths match, handling relative vs absolute paths
                if !paths_match(my_file, other_file) {
                    return false;
                }
            } else {
                // Self requires file_path but other doesn't have one
                return false;
            }
        }

        // If self has a function_name requirement, it must match
        if let Some(ref my_func) = self.function_name {
            if let Some(ref other_func) = other.function_name {
                if my_func != other_func {
                    return false;
                }
            } else {
                // Self requires function_name but other doesn't have one
                return false;
            }
        }

        // If self has a file_type requirement, it must match
        if let Some(my_type) = self.file_type {
            if let Some(other_type) = other.file_type {
                if my_type != other_type {
                    return false;
                }
            } else {
                // Self requires file_type but other doesn't have one
                return false;
            }
        }

        // All non-None fields in self match
        true
    }
}

/// Check if two paths match, handling relative vs absolute paths
fn paths_match(path1: &Path, path2: &Path) -> bool {
    // If both are the same, quick return
    if path1 == path2 {
        return true;
    }

    // If one is absolute and one is relative, check if the relative path
    // is a suffix of the absolute path
    if path1.is_absolute() && path2.is_relative() {
        path1.ends_with(path2)
    } else if path2.is_absolute() && path1.is_relative() {
        path2.ends_with(path1)
    } else {
        // Both absolute or both relative, and they're not equal
        false
    }
}
