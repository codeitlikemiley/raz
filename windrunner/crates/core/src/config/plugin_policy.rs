use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct PluginPolicy {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<Value>,
}

impl PluginPolicy {
    pub fn merge(&mut self, other: PluginPolicy) {
        if other.enabled.is_some() {
            self.enabled = other.enabled;
        }

        if other.priority.is_some() {
            self.priority = other.priority;
        }

        if let Some(other_settings) = other.settings {
            match (&mut self.settings, other_settings) {
                (Some(base), Value::Object(other_map)) if base.is_object() => {
                    merge_json_value(base, Value::Object(other_map));
                }
                (slot, value) => {
                    *slot = Some(value);
                }
            }
        }
    }
}

fn merge_json_value(base: &mut Value, override_value: Value) {
    match (base, override_value) {
        (Value::Object(base_map), Value::Object(override_map)) => {
            for (key, value) in override_map {
                match base_map.get_mut(&key) {
                    Some(existing) => merge_json_value(existing, value),
                    None => {
                        base_map.insert(key, value);
                    }
                }
            }
        }
        (base_slot, value) => {
            *base_slot = value;
        }
    }
}
