//! Template parser: converts a template string into a `CommandTemplate`.

use super::{CommandTemplate, TemplatePart};
use crate::error::{Error, Result};

pub struct TemplateParser;

impl TemplateParser {
    /// Parse a template string into a [`CommandTemplate`].
    pub fn parse(template: &str) -> Result<CommandTemplate> {
        let mut parts = Vec::new();
        let mut chars = template.chars().peekable();
        let mut current = String::new();

        while let Some(ch) = chars.next() {
            if ch == '{' {
                // Flush any accumulated literal text
                if !current.is_empty() {
                    parts.push(TemplatePart::Literal(current.clone()));
                    current.clear();
                }
                // Parse placeholder (everything until matching `}`)
                let placeholder = Self::parse_placeholder(&mut chars)?;
                parts.push(placeholder);
            } else {
                current.push(ch);
            }
        }

        // Flush any trailing literal
        if !current.is_empty() {
            parts.push(TemplatePart::Literal(current));
        }

        Ok(CommandTemplate { parts })
    }

    /// Parse a single placeholder, starting *after* the opening `{`.
    fn parse_placeholder(chars: &mut std::iter::Peekable<std::str::Chars>) -> Result<TemplatePart> {
        let mut content = String::new();
        let mut depth = 1usize;

        for ch in chars.by_ref() {
            match ch {
                '{' => {
                    depth += 1;
                    content.push(ch);
                }
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    content.push(ch);
                }
                _ => content.push(ch),
            }
        }

        if depth != 0 {
            return Err(Error::TemplateError("Unclosed placeholder in template"));
        }

        Self::parse_placeholder_content(&content)
    }

    /// Interpret the string inside `{...}`.
    fn parse_placeholder_content(content: &str) -> Result<TemplatePart> {
        let content = content.trim();

        if let Some(rest) = content.strip_prefix('?') {
            // `{?condition:template}` or `{?condition:template|else_template}`
            if let Some(colon_pos) = rest.find(':') {
                let condition = rest[..colon_pos].trim().to_string();
                let template_str = rest[colon_pos + 1..].trim();

                // Split on `|` for the else branch (top-level `|` only, not inside nested `{}`)
                let (then_str, else_str) = Self::split_on_pipe(template_str);

                let inner_template = Self::parse(then_str)?;

                // If the condition name appears as the sole placeholder in the inner
                // template, use `OptionalWrapper` for cleaner semantics.
                let is_simple_wrapper = inner_template.parts.iter().any(
                    |p| matches!(p, TemplatePart::Placeholder { name, .. } if name == &condition),
                );

                if else_str.is_none() && is_simple_wrapper {
                    Ok(TemplatePart::OptionalWrapper {
                        placeholder: condition,
                        template: inner_template,
                    })
                } else {
                    let else_template = else_str.map(Self::parse).transpose()?;
                    Ok(TemplatePart::Conditional {
                        condition,
                        template: inner_template,
                        else_template,
                    })
                }
            } else {
                // `{?name}` — simple optional placeholder (renders value or nothing)
                Ok(TemplatePart::OptionalWrapper {
                    placeholder: rest.trim().to_string(),
                    template: CommandTemplate {
                        parts: vec![TemplatePart::Placeholder {
                            name: rest.trim().to_string(),
                            default: None,
                        }],
                    },
                })
            }
        } else if let Some(default_pos) = content.find('?') {
            // `{name?default}` — placeholder with fallback default
            let name = content[..default_pos].trim().to_string();
            let default = content[default_pos + 1..].trim().to_string();
            Ok(TemplatePart::Placeholder {
                name,
                default: if default.is_empty() {
                    None
                } else {
                    Some(default)
                },
            })
        } else {
            // `{name}` — plain required placeholder
            Ok(TemplatePart::Placeholder {
                name: content.to_string(),
                default: None,
            })
        }
    }

    /// Split `s` on the first top-level `|` character (respecting `{}`  nesting).
    fn split_on_pipe(s: &str) -> (&str, Option<&str>) {
        let mut depth = 0usize;
        for (i, ch) in s.char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => depth = depth.saturating_sub(1),
                '|' if depth == 0 => return (&s[..i], Some(&s[i + 1..])),
                _ => {}
            }
        }
        (s, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::template::TemplatePart;

    #[test]
    fn test_parse_literal_only() {
        let t = TemplateParser::parse("cargo test").unwrap();
        assert_eq!(t.parts.len(), 1);
        assert!(matches!(&t.parts[0], TemplatePart::Literal(s) if s == "cargo test"));
    }

    #[test]
    fn test_parse_simple_placeholder() {
        let t = TemplateParser::parse("{name}").unwrap();
        assert_eq!(t.parts.len(), 1);
        assert!(
            matches!(&t.parts[0], TemplatePart::Placeholder { name, default } if name == "name" && default.is_none())
        );
    }

    #[test]
    fn test_parse_placeholder_with_default() {
        let t = TemplateParser::parse("{cmd?cargo}").unwrap();
        assert_eq!(t.parts.len(), 1);
        assert!(matches!(
            &t.parts[0],
            TemplatePart::Placeholder { name, default }
            if name == "cmd" && default.as_deref() == Some("cargo")
        ));
    }

    #[test]
    fn test_parse_optional_wrapper() {
        let t = TemplateParser::parse("{?pkg:--package {pkg}}").unwrap();
        assert_eq!(t.parts.len(), 1);
        assert!(matches!(
            &t.parts[0],
            TemplatePart::OptionalWrapper { placeholder, .. }
            if placeholder == "pkg"
        ));
    }

    #[test]
    fn test_parse_conditional_with_else() {
        let t = TemplateParser::parse("{?release:--release|--debug}").unwrap();
        assert_eq!(t.parts.len(), 1);
        assert!(matches!(
            &t.parts[0],
            TemplatePart::Conditional { condition, else_template: Some(_), .. }
            if condition == "release"
        ));
    }

    #[test]
    fn test_parse_unclosed_brace_errors() {
        let result = TemplateParser::parse("{unclosed");
        assert!(result.is_err());
    }
}
