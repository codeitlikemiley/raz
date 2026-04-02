//! Extract rule calls from Starlark AST

use crate::error::Result;
use std::collections::HashMap;
use tree_sitter::{Node, TreeCursor};

use super::starlark_parser::StarlarkAst;

/// A rule call in a BUILD file
#[derive(Debug, Clone)]
pub struct RuleCall {
    pub rule_type: String,
    pub name: String,
    pub attributes: HashMap<String, AttributeValue>,
    pub location: SourceLocation,
}

/// Attribute values in rule calls
#[derive(Debug, Clone, PartialEq)]
pub enum AttributeValue {
    String(String),
    List(Vec<String>),
    Label(String),
    Glob(GlobPattern),
    Boolean(bool),
    Dict(HashMap<String, String>),
}

/// Glob pattern (e.g., glob(["*.rs"]))
#[derive(Debug, Clone, PartialEq)]
pub struct GlobPattern {
    pub patterns: Vec<String>,
    pub exclude: Vec<String>,
}

/// Source location in the BUILD file
#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
}

/// Extracts rule calls from Starlark AST
pub struct RuleExtractor;

impl RuleExtractor {
    /// Extract all rule calls from the AST
    pub fn extract_rules(ast: &StarlarkAst) -> Result<Vec<RuleCall>> {
        let mut rules = Vec::new();
        let mut cursor = ast.tree.walk();
        
        Self::visit_node(&mut cursor, ast, &mut rules)?;
        
        Ok(rules)
    }
    
    /// Recursively visit nodes looking for function calls
    fn visit_node(cursor: &mut TreeCursor, ast: &StarlarkAst, rules: &mut Vec<RuleCall>) -> Result<()> {
        let node = cursor.node();
        
        // Check if this is a function call
        if node.kind() == "call" {
            if let Some(rule) = Self::extract_rule_call(&node, ast)? {
                rules.push(rule);
            }
        }
        
        // Visit children
        if cursor.goto_first_child() {
            loop {
                Self::visit_node(cursor, ast, rules)?;
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
        
        Ok(())
    }
    
    /// Extract a rule call from a call node
    fn extract_rule_call(node: &Node, ast: &StarlarkAst) -> Result<Option<RuleCall>> {
        // Get the function name
        let function_node = node.child_by_field_name("function");
        if function_node.is_none() {
            return Ok(None);
        }
        
        let function_node = function_node.unwrap();
        let rule_type = ast.node_text(&function_node);
        
        // Only process known rule types (this list can be expanded)
        let known_rules = [
            "rust_binary", "rust_library", "rust_test", "rust_test_suite",
            "rust_doc_test", "rust_benchmark", "cargo_build_script",
            // Also handle aliases that might be created
            "rust_bench", "rust_proc_macro", "rust_shared_library", "rust_static_library"
        ];
        
        if !known_rules.contains(&rule_type) {
            return Ok(None);
        }
        
        // Extract arguments
        let mut attributes = HashMap::new();
        let arguments_node = node.child_by_field_name("arguments");
        
        if let Some(args_node) = arguments_node {
            Self::extract_arguments(&args_node, ast, &mut attributes)?;
        }
        
        // Extract the name attribute (required for all rules)
        let name = match attributes.get("name") {
            Some(AttributeValue::String(name)) => name.clone(),
            _ => return Ok(None), // Skip rules without names
        };
        
        let location = SourceLocation {
            line: node.start_position().row + 1,
            column: node.start_position().column,
        };
        
        Ok(Some(RuleCall {
            rule_type: rule_type.to_string(),
            name,
            attributes,
            location,
        }))
    }
    
    /// Extract arguments from an arguments node
    fn extract_arguments(node: &Node, ast: &StarlarkAst, attributes: &mut HashMap<String, AttributeValue>) -> Result<()> {
        let mut cursor = node.walk();
        
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                
                if child.kind() == "keyword_argument" {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        if let Some(value_node) = child.child_by_field_name("value") {
                            let name = ast.node_text(&name_node);
                            if let Some(value) = Self::extract_value(&value_node, ast)? {
                                attributes.insert(name.to_string(), value);
                            }
                        }
                    }
                }
                
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
        
        Ok(())
    }
    
    /// Extract a value from a node
    fn extract_value(node: &Node, ast: &StarlarkAst) -> Result<Option<AttributeValue>> {
        match node.kind() {
            "string" => {
                let text = ast.node_text(node);
                // Remove quotes
                let value = text.trim_matches('"').trim_matches('\'').to_string();
                Ok(Some(AttributeValue::String(value)))
            }
            
            "list" => {
                let mut items = Vec::new();
                let mut cursor = node.walk();
                
                if cursor.goto_first_child() {
                    loop {
                        let child = cursor.node();
                        if child.kind() == "string" {
                            let text = ast.node_text(&child);
                            let value = text.trim_matches('"').trim_matches('\'').to_string();
                            items.push(value);
                        }
                        
                        if !cursor.goto_next_sibling() {
                            break;
                        }
                    }
                }
                
                Ok(Some(AttributeValue::List(items)))
            }
            
            "call" => {
                // Check if it's a glob() call
                if let Some(func_node) = node.child_by_field_name("function") {
                    let func_name = ast.node_text(&func_node);
                    if func_name == "glob" {
                        return Self::extract_glob_pattern(node, ast);
                    }
                    // For other function calls (like all_crate_deps), just skip them
                    // This prevents errors when parsing complex BUILD files
                }
                Ok(None)
            }
            
            "true" => Ok(Some(AttributeValue::Boolean(true))),
            "false" => Ok(Some(AttributeValue::Boolean(false))),
            
            "unary_expression" => {
                // Handle label references like :mylib
                let text = ast.node_text(node);
                if text.starts_with(':') {
                    Ok(Some(AttributeValue::Label(text.to_string())))
                } else {
                    Ok(None)
                }
            }
            
            "binary_expression" => {
                // Handle expressions like all_crate_deps() + ["dep"]
                // For now, just extract the list part if it exists
                let mut cursor = node.walk();
                if cursor.goto_first_child() {
                    loop {
                        let child = cursor.node();
                        if child.kind() == "list" {
                            return Self::extract_value(&child, ast);
                        }
                        if !cursor.goto_next_sibling() {
                            break;
                        }
                    }
                }
                Ok(None)
            }
            
            _ => {
                // For other types, try to get the text representation
                let text = ast.node_text(node);
                if text.starts_with(':') || text.starts_with("//") || text.starts_with("@") {
                    Ok(Some(AttributeValue::Label(text.to_string())))
                } else {
                    Ok(None)
                }
            }
        }
    }
    
    /// Extract a glob pattern
    fn extract_glob_pattern(node: &Node, ast: &StarlarkAst) -> Result<Option<AttributeValue>> {
        let mut patterns = Vec::new();
        let exclude = Vec::new(); // TODO: Handle exclude patterns
        
        if let Some(args_node) = node.child_by_field_name("arguments") {
            let mut cursor = args_node.walk();
            
            if cursor.goto_first_child() {
                loop {
                    let child = cursor.node();
                    
                    if child.kind() == "list" {
                        // Extract patterns from the list
                        if let Some(AttributeValue::List(items)) = Self::extract_value(&child, ast)? {
                            patterns = items;
                        }
                    }
                    
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
            }
        }
        
        Ok(Some(AttributeValue::Glob(GlobPattern { patterns, exclude })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bazel::StarlarkParser;
    
    #[test]
    fn test_extract_simple_rules() {
        let content = r#"
rust_library(
    name = "mylib",
    srcs = ["src/lib.rs"],
    deps = [":dep1", "//other:dep2"],
)

rust_test(
    name = "mylib_test",
    crate = ":mylib",
)
"#;
        
        let mut parser = StarlarkParser::new().unwrap();
        let ast = parser.parse_build_file(content).unwrap();
        let rules = RuleExtractor::extract_rules(&ast).unwrap();
        
        assert_eq!(rules.len(), 2);
        
        assert_eq!(rules[0].rule_type, "rust_library");
        assert_eq!(rules[0].name, "mylib");
        
        assert_eq!(rules[1].rule_type, "rust_test");
        assert_eq!(rules[1].name, "mylib_test");
    }
    
    #[test]
    fn test_extract_glob_pattern() {
        let content = r#"
rust_test_suite(
    name = "integration_tests",
    srcs = glob(["tests/*.rs"]),
)
"#;
        
        let mut parser = StarlarkParser::new().unwrap();
        let ast = parser.parse_build_file(content).unwrap();
        let rules = RuleExtractor::extract_rules(&ast).unwrap();
        
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].rule_type, "rust_test_suite");
        
        if let Some(AttributeValue::Glob(glob)) = rules[0].attributes.get("srcs") {
            assert_eq!(glob.patterns, vec!["tests/*.rs"]);
        } else {
            panic!("Expected glob pattern for srcs");
        }
    }
}