use anyhow::{Context, Result};
use serde_json::{Map, Value, json};
use std::fs;
use std::path::Path;

pub struct OverrideManager;

impl OverrideManager {
    pub fn parse_override_args(args: &[String]) -> Map<String, Value> {
        let mut result = Map::new();
        let mut extra_args = Vec::new();
        let mut extra_env = Map::new();
        let mut extra_test_binary_args = Vec::new();
        let mut command = None;
        let mut subcommand = None;
        let mut channel = None;

        let mut remove_command = false;
        let mut remove_subcommand = false;
        let mut remove_channel = false;
        let mut remove_args = false;
        let mut remove_env = false;
        let mut remove_test_args = false;
        let mut env_to_remove = Vec::new();

        let mut i = 0;
        while i < args.len() {
            let arg = &args[i];

            if arg.starts_with('-') && !arg.starts_with("--") {
                match arg.as_str() {
                    "-command" | "-cmd" => remove_command = true,
                    "-subcommand" | "-sub" => remove_subcommand = true,
                    "-channel" | "-ch" => remove_channel = true,
                    "-arg" => remove_args = true,
                    "-env" => remove_env = true,
                    "-test" | "-/" => remove_test_args = true,
                    _ => {
                        let env_name = &arg[1..];
                        if env_name.chars().all(|c| c.is_uppercase() || c == '_')
                            && !env_name.is_empty()
                        {
                            env_to_remove.push(env_name.to_string());
                        }
                    }
                }
            } else if let Some(token) = arg.strip_prefix('@') {
                let parts: Vec<&str> = token.split('.').collect();
                if !parts.is_empty() {
                    let cmd = parts[0];
                    if cmd == "cargo" && parts.len() > 1 {
                        subcommand = Some(parts[1..].join(" "));
                    } else {
                        command = Some(cmd.to_string());
                        if parts.len() > 1 {
                            subcommand = Some(parts[1..].join(" "));
                        }
                    }
                }
            } else if arg.starts_with('+') && arg.len() > 1 {
                channel = Some(arg[1..].to_string());
            } else if arg == "/" || arg.starts_with('/') {
                if arg.len() > 1 {
                    extra_test_binary_args.push(arg[1..].to_string());
                }
                while i + 1 < args.len() {
                    i += 1;
                    extra_test_binary_args.push(args[i].clone());
                }
            } else if arg
                .chars()
                .take_while(|&c| c != '=')
                .all(|c| c.is_uppercase() || c == '_')
                && arg.contains('=')
            {
                let parts: Vec<&str> = arg.splitn(2, '=').collect();
                if parts.len() == 2 && !parts[0].is_empty() {
                    extra_env.insert(parts[0].to_string(), json!(parts[1]));
                }
            } else {
                extra_args.push(arg.clone());
            }
            i += 1;
        }

        if let Some(cmd) = command {
            if !remove_command {
                result.insert("command".to_string(), json!(cmd));
            }
        } else if remove_command {
            result.insert("remove_command".to_string(), json!(true));
        }

        if let Some(sub) = subcommand {
            if !remove_subcommand {
                result.insert("subcommand".to_string(), json!(sub));
            }
        } else if remove_subcommand {
            result.insert("remove_subcommand".to_string(), json!(true));
        }

        if let Some(ch) = channel {
            if !remove_channel {
                result.insert("channel".to_string(), json!(ch));
            }
        } else if remove_channel {
            result.insert("remove_channel".to_string(), json!(true));
        }

        if !extra_args.is_empty() && !remove_args {
            result.insert("extra_args".to_string(), json!(extra_args));
        } else if remove_args {
            result.insert("remove_args".to_string(), json!(true));
        }

        if !extra_env.is_empty() && !remove_env {
            result.insert("extra_env".to_string(), Value::Object(extra_env));
        } else if remove_env {
            result.insert("remove_env".to_string(), json!(true));
        }

        if !env_to_remove.is_empty() {
            result.insert("remove_env_keys".to_string(), json!(env_to_remove));
        }

        if !extra_test_binary_args.is_empty() && !remove_test_args {
            result.insert(
                "extra_test_binary_args".to_string(),
                json!(extra_test_binary_args),
            );
        } else if remove_test_args {
            result.insert("remove_test_args".to_string(), json!(true));
        }

        result
    }

    pub fn add_override_to_existing_config(
        config_path: &Path,
        override_entry: Map<String, Value>,
    ) -> Result<()> {
        let mut config: Map<String, Value> = if config_path.exists() {
            let content = fs::read_to_string(config_path)
                .with_context(|| format!("Failed to read config from {}", config_path.display()))?;
            serde_json::from_str(&content)
                .with_context(|| format!("Failed to parse config from {}", config_path.display()))?
        } else {
            let mut new_config = Map::new();
            new_config.insert(
                "cargo".to_string(),
                json!({
                    "extra_args": [],
                    "extra_env": {},
                    "extra_test_binary_args": []
                }),
            );
            new_config.insert("overrides".to_string(), json!([]));
            new_config
        };

        let overrides = config
            .entry("overrides".to_string())
            .or_insert(json!([]))
            .as_array_mut()
            .ok_or_else(|| anyhow::anyhow!("overrides is not an array"))?;

        overrides.push(Value::Object(override_entry));

        let json_string = serde_json::to_string_pretty(&config)?;
        fs::write(config_path, json_string)
            .with_context(|| format!("Failed to write config to {}", config_path.display()))?;

        Ok(())
    }
}
