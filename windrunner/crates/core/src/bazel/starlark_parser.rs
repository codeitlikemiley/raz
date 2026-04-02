//! Starlark parser using tree-sitter-starlark

use crate::error::Result;
use tree_sitter::{Parser, Tree};

/// Parser for Starlark/BUILD files
pub struct StarlarkParser {
    parser: Parser,
}

impl StarlarkParser {
    /// Create a new Starlark parser
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        let language = tree_sitter_starlark::LANGUAGE;
        parser
            .set_language(&language.into())
            .map_err(|e| crate::error::Error::ParseError(format!("Failed to set Starlark language: {}", e)))?;
        
        Ok(Self { parser })
    }
    
    /// Parse BUILD file content into an AST
    pub fn parse_build_file(&mut self, content: &str) -> Result<StarlarkAst> {
        let tree = self.parser
            .parse(content, None)
            .ok_or_else(|| crate::error::Error::ParseError("Failed to parse BUILD file".to_string()))?;
            
        if tree.root_node().has_error() {
            return Err(crate::error::Error::ParseError("BUILD file contains syntax errors".to_string()));
        }
        
        Ok(StarlarkAst {
            tree,
            source: content.to_string(),
        })
    }
}

/// Parsed Starlark AST
pub struct StarlarkAst {
    pub tree: Tree,
    pub source: String,
}

impl StarlarkAst {
    /// Get the root node of the AST
    pub fn root(&self) -> tree_sitter::Node<'_> {
        self.tree.root_node()
    }
    
    /// Get a slice of the source code for a node
    pub fn node_text<'a>(&'a self, node: &tree_sitter::Node) -> &'a str {
        node.utf8_text(self.source.as_bytes()).unwrap_or("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_simple_build_file() {
        let content = r#"
load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")

rust_library(
    name = "mylib",
    srcs = ["src/lib.rs"],
)

rust_test(
    name = "mylib_test",
    crate = ":mylib",
)
"#;
        
        let mut parser = StarlarkParser::new().unwrap();
        let ast = parser.parse_build_file(content).unwrap();
        
        assert!(!ast.root().has_error());
        assert_eq!(ast.root().kind(), "module");
    }
    
    #[test]
    fn test_parse_invalid_starlark() {
        let content = r#"
rust_library(
    name = "mylib"  # Missing comma
    srcs = ["src/lib.rs"],
)
"#;
        
        let mut parser = StarlarkParser::new().unwrap();
        let result = parser.parse_build_file(content);
        
        assert!(result.is_err());
    }
}