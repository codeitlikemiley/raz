use crate::types::{Position, Runnable, RunnableKind};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRange {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetRef {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub plugin_id: String,
    pub file_path: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<SourceRange>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selector: Option<String>,
    pub priority: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runnable: Option<Runnable>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, Value>,
}

impl TargetRef {
    pub fn from_runnable(plugin_id: impl Into<String>, runnable: Runnable) -> Self {
        let plugin_id = plugin_id.into();
        let label = runnable.label.clone();
        let kind = format!("{:?}", runnable.kind);
        let selector = runnable.get_function_name();
        let range = Some(match &runnable.kind {
            RunnableKind::DocTest { .. } => {
                if let Some(ref extended) = runnable.extended_scope {
                    SourceRange {
                        start: extended.scope.start,
                        end: extended.scope.end,
                    }
                } else {
                    SourceRange {
                        start: runnable.scope.start,
                        end: runnable.scope.end,
                    }
                }
            }
            _ => SourceRange {
                start: runnable.scope.start,
                end: runnable.scope.end,
            },
        });

        Self {
            id: format!("{plugin_id}:{label}"),
            label,
            kind,
            plugin_id,
            file_path: runnable.file_path.clone(),
            range,
            selector,
            priority: 0,
            runnable: Some(runnable),
            metadata: BTreeMap::new(),
        }
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    pub fn line_contains(&self, line: u32) -> bool {
        if let Some(runnable) = &self.runnable {
            if matches!(runnable.kind, RunnableKind::DocTest { .. }) {
                if let Some(ref extended) = runnable.extended_scope {
                    return extended.scope.contains_line(line);
                }
            }
            return runnable.scope.contains_line(line);
        }

        if let Some(range) = &self.range {
            return line >= range.start.line && line <= range.end.line;
        }

        false
    }

    pub fn into_runnable(self) -> Option<Runnable> {
        self.runnable
    }
}
