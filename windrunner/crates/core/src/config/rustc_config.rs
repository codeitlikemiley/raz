use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RustcConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_framework: Option<RustcFramework>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary_framework: Option<RustcFramework>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub benchmark_framework: Option<RustcFramework>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RustcFramework {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build: Option<RustcPhaseConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exec: Option<RustcPhaseConfig>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RustcPhaseConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_test_binary_args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pipe: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suppress_stderr: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_env: Option<HashMap<String, String>>,
}
