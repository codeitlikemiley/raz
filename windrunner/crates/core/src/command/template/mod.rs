//! Template engine for command generation.
//!
//! Provides a DSL for expressing commands with optional and conditional parts.
//!
//! # Syntax
//!
//! - `{name}` — required placeholder, resolves to empty string if missing
//! - `{name?default}` — optional placeholder with fallback default value
//! - `{?condition:content}` — conditional block; `content` only rendered when `condition` resolves
//! - `{?condition:content|else_content}` — conditional with else branch
//!
//! # Example
//!
//! ```rust
//! # use cargo_runner_core::command::template::CommandTemplate;
//! let template = CommandTemplate::parse(
//!     "{cmd?cargo} test {?package:--package {package}} -- {test_name} --exact"
//! ).unwrap();
//!
//! let cmd = template.render(|ph| match ph {
//!     "package" => Some("myapp".to_string()),
//!     "test_name" => Some("test_foo".to_string()),
//!     _ => None,
//! }).unwrap();
//!
//! assert_eq!(cmd, "cargo test --package myapp -- test_foo --exact");
//! ```

mod parser;
mod renderer;

pub use parser::TemplateParser;
pub use renderer::TemplateRenderer;

/// A parsed command template containing literal text and variable parts.
#[derive(Debug, Clone)]
pub struct CommandTemplate {
    pub(crate) parts: Vec<TemplatePart>,
}

/// A single part of a parsed template.
#[derive(Debug, Clone)]
pub enum TemplatePart {
    /// Verbatim text.
    Literal(String),

    /// A placeholder that resolves via the provided resolver.
    /// `default` is used when the resolver returns `None`.
    Placeholder {
        name: String,
        default: Option<String>,
    },

    /// Renders `template` when `condition` resolves to `Some(_)`,
    /// optionally renders `else_template` otherwise.
    Conditional {
        condition: String,
        template: CommandTemplate,
        else_template: Option<CommandTemplate>,
    },

    /// Renders `template` only when `placeholder` resolves to `Some(_)`.
    /// The resolved value is available as `{placeholder}` within `template`.
    OptionalWrapper {
        placeholder: String,
        template: CommandTemplate,
    },
}

impl CommandTemplate {
    /// Parse a template string into a `CommandTemplate`.
    pub fn parse(template: &str) -> crate::error::Result<Self> {
        TemplateParser::parse(template)
    }

    /// Render this template using `resolver` to fill placeholders.
    ///
    /// The resolver is called with each placeholder name and should return
    /// `Some(value)` to fill it or `None` to use the default / suppress
    /// optional blocks.
    pub fn render<F>(&self, resolver: F) -> crate::error::Result<String>
    where
        F: Fn(&str) -> Option<String>,
    {
        TemplateRenderer::render(self, resolver)
    }
}

/// Pre-built templates for common Cargo and Bazel command patterns.
pub struct Templates;

impl Templates {
    // ── Cargo ─────────────────────────────────────────────────────────────

    pub fn cargo_test() -> CommandTemplate {
        CommandTemplate::parse(
            "{cmd?cargo} {?channel:+{channel}} test \
             {?package:--package {package}} \
             {?target} \
             {?test_name:{test_name} --exact} \
             {?args:{args}}",
        )
        .expect("static template must parse")
    }

    pub fn cargo_run() -> CommandTemplate {
        CommandTemplate::parse(
            "{cmd?cargo} {?channel:+{channel}} run \
             {?package:--package {package}} \
             {?bin:--bin {bin}} \
             {?example:--example {example}} \
             {?release:--release} \
             {?args:{args}} \
             {?run_args:-- {run_args}}",
        )
        .expect("static template must parse")
    }

    pub fn cargo_bench() -> CommandTemplate {
        CommandTemplate::parse(
            "{cmd?cargo} {?channel:+{channel}} bench \
             {?package:--package {package}} \
             {?bench_name:{bench_name}} \
             {?args:{args}} \
             {?bench_args:-- {bench_args}}",
        )
        .expect("static template must parse")
    }

    // ── Bazel ─────────────────────────────────────────────────────────────

    pub fn bazel_test() -> CommandTemplate {
        CommandTemplate::parse(
            "{cmd?bazel} test {target} \
             {?test_output:--test_output={test_output}} \
             {?nocapture:--test_arg=--nocapture} \
             {?test_filter:--test_arg=--exact --test_arg={test_filter}} \
             {?extra_args:{extra_args}}",
        )
        .expect("static template must parse")
    }

    pub fn bazel_run() -> CommandTemplate {
        CommandTemplate::parse(
            "{cmd?bazel} run {target} \
             {?config:--config={config}} \
             {?extra_args:{extra_args}} \
             {?run_args:-- {run_args}}",
        )
        .expect("static template must parse")
    }

    pub fn bazel_build() -> CommandTemplate {
        CommandTemplate::parse(
            "{cmd?bazel} build {target} \
             {?config:--config={config}} \
             {?extra_args:{extra_args}}",
        )
        .expect("static template must parse")
    }

    pub fn bazel_doc_test() -> CommandTemplate {
        CommandTemplate::parse(
            "{cmd?bazel} test {target} \
             {?test_output:--test_output={test_output}} \
             {?extra_args:{extra_args}}",
        )
        .expect("static template must parse")
    }

    // ── Dioxus ────────────────────────────────────────────────────────────

    pub fn dioxus_serve() -> CommandTemplate {
        CommandTemplate::parse(
            "{cmd?dx} serve \
             {?port:--port {port}} \
             {?hot_reload:--hot-reload} \
             {?release:--release} \
             {?platform:--platform {platform}} \
             {?args:{args}}",
        )
        .expect("static template must parse")
    }

    // ── Leptos ────────────────────────────────────────────────────────────

    pub fn leptos_watch() -> CommandTemplate {
        CommandTemplate::parse(
            "{cmd?cargo} leptos {leptos_cmd?watch} \
             {?release:--release} \
             {?port:--port {port}} \
             {?args:{args}}",
        )
        .expect("static template must parse")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolve_empty(_: &str) -> Option<String> {
        None
    }

    #[test]
    fn test_simple_placeholder_with_default() {
        let t = CommandTemplate::parse("{cmd?cargo} test").unwrap();
        let result = t.render(resolve_empty).unwrap();
        assert_eq!(result, "cargo test");
    }

    #[test]
    fn test_placeholder_resolved() {
        let t = CommandTemplate::parse("{cmd?cargo} test").unwrap();
        let result = t
            .render(|ph| {
                if ph == "cmd" {
                    Some("bazel".to_string())
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(result, "bazel test");
    }

    #[test]
    fn test_conditional_block_present() {
        let t = CommandTemplate::parse("cargo test {?package:--package {package}}").unwrap();
        let result = t
            .render(|ph| {
                if ph == "package" {
                    Some("myapp".to_string())
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(result, "cargo test --package myapp");
    }

    #[test]
    fn test_conditional_block_absent() {
        let t = CommandTemplate::parse("cargo test {?package:--package {package}}").unwrap();
        let result = t.render(resolve_empty).unwrap();
        assert_eq!(result, "cargo test");
    }

    #[test]
    fn test_whitespace_normalization() {
        // Extra spaces between absent optional parts are collapsed
        let t = CommandTemplate::parse(
            "cargo test {?package:--package {package}} {?filter:{filter}} --exact",
        )
        .unwrap();
        let result = t
            .render(|ph| {
                if ph == "filter" {
                    Some("foo".to_string())
                } else {
                    None
                }
            })
            .unwrap();
        // No double-space where package block was absent
        assert!(!result.contains("  "));
        assert!(result.contains("foo --exact"));
    }

    #[test]
    fn test_full_cargo_test_template() {
        let t = Templates::cargo_test();
        let result = t
            .render(|ph| match ph {
                "package" => Some("myapp".to_string()),
                "test_name" => Some("test_login".to_string()),
                _ => None,
            })
            .unwrap();
        assert!(result.contains("cargo"));
        assert!(result.contains("--package myapp"));
        assert!(result.contains("test_login --exact"));
    }

    #[test]
    fn test_full_bazel_test_no_filter() {
        let t = Templates::bazel_test();
        let result = t
            .render(|ph| match ph {
                "target" => Some("//server:server_test".to_string()),
                "test_output" => Some("streamed".to_string()),
                _ => None,
            })
            .unwrap();
        assert!(result.starts_with("bazel test //server:server_test"));
        assert!(result.contains("--test_output=streamed"));
        // No filter → no --test_arg=--exact
        assert!(!result.contains("--exact"));
    }
}
