use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandOverride {
    pub key: String,
    pub file_path: Option<PathBuf>,
    pub function_name: Option<String>,
    pub line_range: Option<(usize, usize)>,
    pub env: IndexMap<String, String>,
    pub cargo_options: Vec<String>,
    pub rustc_options: Vec<String>,
    pub args: Vec<String>,
    pub mode: OverrideMode,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OverrideMode {
    Replace,
    Append,
    Prepend,
    Remove,
}

impl Default for OverrideMode {
    fn default() -> Self {
        Self::Replace
    }
}

impl CommandOverride {
    pub fn new(key: String) -> Self {
        Self {
            key,
            file_path: None,
            function_name: None,
            line_range: None,
            env: IndexMap::new(),
            cargo_options: Vec::new(),
            rustc_options: Vec::new(),
            args: Vec::new(),
            mode: OverrideMode::default(),
            created_at: chrono::Utc::now(),
            description: None,
        }
    }

    pub fn with_file(mut self, path: PathBuf) -> Self {
        self.file_path = Some(path);
        self
    }

    pub fn with_function(mut self, name: String) -> Self {
        self.function_name = Some(name);
        self
    }

    pub fn with_line_range(mut self, start: usize, end: usize) -> Self {
        self.line_range = Some((start, end));
        self
    }

    pub fn with_env(mut self, key: String, value: String) -> Self {
        self.env.insert(key, value);
        self
    }

    pub fn with_cargo_option(mut self, option: String) -> Self {
        self.cargo_options.push(option);
        self
    }

    pub fn with_rustc_option(mut self, option: String) -> Self {
        self.rustc_options.push(option);
        self
    }

    pub fn with_arg(mut self, arg: String) -> Self {
        self.args.push(arg);
        self
    }

    pub fn with_mode(mut self, mode: OverrideMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_description(mut self, desc: String) -> Self {
        self.description = Some(desc);
        self
    }

    pub fn generate_key(&self) -> String {
        let mut parts = Vec::new();

        if let Some(path) = &self.file_path {
            parts.push(path.to_string_lossy().to_string());
        }

        if let Some(func) = &self.function_name {
            parts.push(func.clone());
        }

        if let Some((start, end)) = self.line_range {
            parts.push(format!("L{start}-{end}"));
        }

        if parts.is_empty() {
            self.key.clone()
        } else {
            parts.join(":")
        }
    }

    pub fn matches_context(
        &self,
        file: Option<&PathBuf>,
        function: Option<&str>,
        line: Option<usize>,
    ) -> bool {
        if let Some(f) = &self.file_path {
            if let Some(current_file) = file {
                if f != current_file {
                    return false;
                }
            } else {
                return false;
            }
        }

        if let Some(func) = &self.function_name {
            if let Some(current_func) = function {
                if func != current_func {
                    return false;
                }
            } else {
                return false;
            }
        }

        if let Some((start, end)) = self.line_range {
            if let Some(current_line) = line {
                if current_line < start || current_line > end {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverrideCollection {
    pub overrides: IndexMap<String, CommandOverride>,
}

impl OverrideCollection {
    pub fn new() -> Self {
        Self {
            overrides: IndexMap::new(),
        }
    }

    pub fn add(&mut self, override_config: CommandOverride) {
        let key = override_config.generate_key();
        self.overrides.insert(key, override_config);
    }

    pub fn remove(&mut self, key: &str) -> Option<CommandOverride> {
        self.overrides.shift_remove(key)
    }

    pub fn find_best_match(
        &self,
        file: Option<&PathBuf>,
        function: Option<&str>,
        line: Option<usize>,
    ) -> Option<&CommandOverride> {
        self.overrides
            .values()
            .filter(|o| o.matches_context(file, function, line))
            .max_by_key(|o| {
                let mut score = 0;
                if o.file_path.is_some() {
                    score += 100;
                }
                if o.function_name.is_some() {
                    score += 50;
                }
                if o.line_range.is_some() {
                    score += 25;
                }
                score
            })
    }

    pub fn get_all_for_file(&self, file: &PathBuf) -> Vec<&CommandOverride> {
        self.overrides
            .values()
            .filter(|o| o.file_path.as_ref() == Some(file))
            .collect()
    }
}

impl Default for OverrideCollection {
    fn default() -> Self {
        Self::new()
    }
}

/// Override system settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverrideSettings {
    /// Enable automatic validation before save
    #[serde(default = "default_validate_before_save")]
    pub validate_before_save: bool,

    /// Number of backups to keep
    #[serde(default = "default_max_backups")]
    pub max_backups: usize,

    /// Prompt for rollback on command failure
    #[serde(default = "default_prompt_rollback_on_failure")]
    pub prompt_rollback_on_failure: bool,

    /// Auto-rollback after N consecutive failures
    #[serde(default = "default_auto_rollback_threshold")]
    pub auto_rollback_threshold: u32,
}

impl Default for OverrideSettings {
    fn default() -> Self {
        Self {
            validate_before_save: true,
            max_backups: 5,
            prompt_rollback_on_failure: true,
            auto_rollback_threshold: 3,
        }
    }
}

fn default_validate_before_save() -> bool {
    true
}

fn default_max_backups() -> usize {
    5
}

fn default_prompt_rollback_on_failure() -> bool {
    true
}

fn default_auto_rollback_threshold() -> u32 {
    3
}
