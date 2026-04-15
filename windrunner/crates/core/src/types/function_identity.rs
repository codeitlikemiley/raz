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

macro_rules! match_opt_field {
    ($self:ident, $other:ident, $field:ident) => {
        if let Some(ref my_val) = $self.$field {
            if let Some(ref other_val) = $other.$field {
                if my_val != other_val {
                    return false;
                }
            } else {
                return false;
            }
        }
    };
}

impl FunctionIdentity {
    /// Check if this identity matches another identity (partial match)
    /// Returns true if all non-None fields in `self` match the corresponding fields in `other`
    pub fn matches(&self, other: &FunctionIdentity) -> bool {
        match_opt_field!(self, other, package);
        match_opt_field!(self, other, module_path);
        match_opt_field!(self, other, function_name);
        match_opt_field!(self, other, file_type);

        if let Some(ref my_file) = self.file_path {
            if let Some(ref other_file) = other.file_path {
                if !paths_match(my_file, other_file) {
                    return false;
                }
            } else {
                return false;
            }
        }

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
