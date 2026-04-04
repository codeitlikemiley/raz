//! Template renderer: walks a `CommandTemplate` and calls the resolver
//! closure to produce a final command string.

use super::{CommandTemplate, TemplatePart};
use crate::error::Result;

pub struct TemplateRenderer;

impl TemplateRenderer {
    /// Render `template` using `resolver` to look up placeholder values.
    ///
    /// - Returns `Some(value)` from resolver → value is inserted.
    /// - Returns `None` → placeholder default is used; if no default, block is omitted.
    pub fn render<F>(template: &CommandTemplate, resolver: F) -> Result<String>
    where
        F: Fn(&str) -> Option<String>,
    {
        let rendered = Self::render_parts(&template.parts, &resolver)?;
        Ok(Self::normalize_whitespace(&rendered))
    }

    fn render_parts(
        parts: &[TemplatePart],
        resolver: &dyn Fn(&str) -> Option<String>,
    ) -> Result<String> {
        let mut segments: Vec<String> = Vec::new();

        for part in parts {
            if let Some(s) = Self::render_part(part, resolver)? {
                if !s.is_empty() {
                    segments.push(s);
                }
            }
        }

        Ok(segments.join(""))
    }

    fn render_part(
        part: &TemplatePart,
        resolver: &dyn Fn(&str) -> Option<String>,
    ) -> Result<Option<String>> {
        match part {
            TemplatePart::Literal(text) => Ok(Some(text.clone())),

            TemplatePart::Placeholder { name, default } => {
                let value = resolver(name).or_else(|| default.clone());
                Ok(value)
            }

            TemplatePart::OptionalWrapper {
                placeholder,
                template,
            } => {
                // Only render the inner template if the placeholder resolves
                if resolver(placeholder).is_some() {
                    let rendered = Self::render_parts(&template.parts, resolver)?;
                    Ok(Some(rendered))
                } else {
                    Ok(None)
                }
            }

            TemplatePart::Conditional {
                condition,
                template,
                else_template,
            } => {
                if resolver(condition).is_some() {
                    let rendered = Self::render_parts(&template.parts, resolver)?;
                    Ok(Some(rendered))
                } else if let Some(else_tmpl) = else_template {
                    let rendered = Self::render_parts(&else_tmpl.parts, resolver)?;
                    Ok(Some(rendered))
                } else {
                    Ok(None)
                }
            }
        }
    }

    /// Collapse runs of whitespace (spaces, tabs) into a single space and trim.
    fn normalize_whitespace(s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        let mut last_was_space = false;

        for ch in s.chars() {
            if ch == ' ' || ch == '\t' {
                if !last_was_space {
                    result.push(' ');
                }
                last_was_space = true;
            } else {
                result.push(ch);
                last_was_space = false;
            }
        }

        result.trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::template::CommandTemplate;

    fn empty(_: &str) -> Option<String> {
        None
    }

    #[test]
    fn test_renders_literal() {
        let t = CommandTemplate::parse("cargo test").unwrap();
        assert_eq!(TemplateRenderer::render(&t, empty).unwrap(), "cargo test");
    }

    #[test]
    fn test_renders_placeholder_default() {
        let t = CommandTemplate::parse("{cmd?bazel} test").unwrap();
        assert_eq!(TemplateRenderer::render(&t, empty).unwrap(), "bazel test");
    }

    #[test]
    fn test_renders_placeholder_resolved() {
        let t = CommandTemplate::parse("{cmd?bazel} test").unwrap();
        let result = TemplateRenderer::render(&t, |ph| {
            if ph == "cmd" {
                Some("bazelisk".to_string())
            } else {
                None
            }
        })
        .unwrap();
        assert_eq!(result, "bazelisk test");
    }

    #[test]
    fn test_optional_wrapper_present() {
        let t = CommandTemplate::parse("cargo test {?pkg:--package {pkg}}").unwrap();
        let result = TemplateRenderer::render(&t, |ph| {
            if ph == "pkg" {
                Some("myapp".to_string())
            } else {
                None
            }
        })
        .unwrap();
        assert_eq!(result, "cargo test --package myapp");
    }

    #[test]
    fn test_optional_wrapper_absent() {
        let t = CommandTemplate::parse("cargo test {?pkg:--package {pkg}}").unwrap();
        let result = TemplateRenderer::render(&t, empty).unwrap();
        assert_eq!(result, "cargo test");
    }

    #[test]
    fn test_conditional_with_else_true_branch() {
        let t = CommandTemplate::parse("{?release:--release|--debug}").unwrap();
        let result = TemplateRenderer::render(&t, |ph| {
            if ph == "release" {
                Some("1".to_string())
            } else {
                None
            }
        })
        .unwrap();
        assert_eq!(result, "--release");
    }

    #[test]
    fn test_conditional_with_else_false_branch() {
        let t = CommandTemplate::parse("{?release:--release|--debug}").unwrap();
        let result = TemplateRenderer::render(&t, empty).unwrap();
        assert_eq!(result, "--debug");
    }

    #[test]
    fn test_whitespace_normalization() {
        // Multiple absent optional blocks should not leave double-spaces
        let t = CommandTemplate::parse("cargo {?a:--flag-a} {?b:--flag-b} test").unwrap();
        let result = TemplateRenderer::render(&t, empty).unwrap();
        assert!(!result.contains("  "), "double space found in: {result:?}");
        assert_eq!(result, "cargo test");
    }
}
