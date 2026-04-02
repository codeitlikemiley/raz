use super::{Features, TestFramework};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Type alias for clarity - TestFramework is actually a generic framework config
pub type BinaryFramework = TestFramework;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CargoConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subcommand: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub features: Option<Features>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_env: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_test_binary_args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_framework: Option<TestFramework>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary_framework: Option<BinaryFramework>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linked_projects: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SingleFileScriptConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_env: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_test_binary_args: Option<Vec<String>>,
}
