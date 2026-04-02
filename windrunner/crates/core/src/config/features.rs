use serde::{Deserialize, Serialize};

/// Features configuration that can be either "all" or a list of specific features
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Features {
    /// Use all features with --all-features flag
    All(String),
    /// Use specific features with --features=feature1,feature2
    Selected(Vec<String>),
}

impl Features {
    /// Convert to cargo command line arguments
    pub fn to_args(&self) -> Vec<String> {
        match self {
            Features::All(s) if s == "all" => vec!["--all-features".to_string()],
            Features::All(_) => vec![], // Invalid value, ignore
            Features::Selected(features) if !features.is_empty() => {
                vec![format!("--features={}", features.join(","))]
            }
            Features::Selected(_) => vec![], // Empty features, ignore
        }
    }

    /// Merge two Features, with the second one taking precedence if it's "all"
    pub fn merge(
        base: Option<&Features>,
        override_features: Option<&Features>,
    ) -> Option<Features> {
        match (base, override_features) {
            // Both are None
            (None, None) => None,
            // One is None
            (Some(base), None) => Some(base.clone()),
            (None, Some(override_f)) => Some(override_f.clone()),

            // Handle "all" cases
            (_, Some(Features::All(s))) if s == "all" => Some(Features::All("all".to_string())),
            (Some(Features::All(s)), _) if s == "all" => Some(Features::All("all".to_string())),

            // Both are All but not "all" - take override
            (Some(Features::All(_)), Some(Features::All(s))) => Some(Features::All(s.clone())),

            // Mixed All and Selected - prefer Selected
            (Some(Features::All(_)), Some(Features::Selected(features))) => {
                Some(Features::Selected(features.clone()))
            }
            (Some(Features::Selected(features)), Some(Features::All(_))) => {
                Some(Features::Selected(features.clone()))
            }

            // Both are Selected - merge them
            (
                Some(Features::Selected(base_features)),
                Some(Features::Selected(override_features)),
            ) => {
                let mut merged = base_features.clone();
                for feature in override_features {
                    if !merged.contains(feature) {
                        merged.push(feature.clone());
                    }
                }
                Some(Features::Selected(merged))
            }
        }
    }

    /// Check if this represents --all-features
    pub fn is_all(&self) -> bool {
        matches!(self, Features::All(s) if s == "all")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_features_to_args() {
        let all = Features::All("all".to_string());
        assert_eq!(all.to_args(), vec!["--all-features"]);

        let selected = Features::Selected(vec!["web".to_string(), "desktop".to_string()]);
        assert_eq!(selected.to_args(), vec!["--features=web,desktop"]);

        let empty = Features::Selected(vec![]);
        assert_eq!(empty.to_args(), Vec::<String>::new());

        let invalid = Features::All("invalid".to_string());
        assert_eq!(invalid.to_args(), Vec::<String>::new());
    }

    #[test]
    fn test_features_merge() {
        let base = Features::Selected(vec!["core".to_string(), "web".to_string()]);
        let override_f = Features::Selected(vec!["desktop".to_string()]);

        let merged = Features::merge(Some(&base), Some(&override_f)).unwrap();
        match merged {
            Features::Selected(features) => {
                assert_eq!(features, vec!["core", "web", "desktop"]);
            }
            _ => panic!("Expected Selected variant"),
        }

        // Test "all" overrides everything
        let all = Features::All("all".to_string());
        let merged = Features::merge(Some(&base), Some(&all)).unwrap();
        assert!(merged.is_all());

        // Test None cases
        let merged = Features::merge(None, Some(&base)).unwrap();
        match merged {
            Features::Selected(features) => {
                assert_eq!(features, vec!["core", "web"]);
            }
            _ => panic!("Expected Selected variant"),
        }
    }
}
