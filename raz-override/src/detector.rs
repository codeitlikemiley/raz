use crate::error::{OverrideError, Result};
use tree_sitter::{Parser, Query, QueryCursor, StreamingIteratorMut};

/// Information about a detected function
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionInfo {
    /// The function name
    pub name: String,
    /// Start line (0-indexed)
    pub start_line: usize,
    /// End line (0-indexed)
    pub end_line: usize,
    /// Start column
    pub start_column: usize,
    /// End column  
    pub end_column: usize,
    /// The full function signature
    pub signature: String,
    /// Whether this is a test function
    pub is_test: bool,
    /// Whether this is an async function
    pub is_async: bool,
}

/// Detects functions in Rust source code using tree-sitter
pub struct FunctionDetector {
    parser: Parser,
    function_query: Query,
}

impl FunctionDetector {
    /// Create a new function detector
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        let language = tree_sitter_rust::LANGUAGE;
        parser
            .set_language(&language.into())
            .map_err(|e| OverrideError::TreeSitterError(e.to_string()))?;

        // Query to find function items at any level
        // We'll filter out duplicates by checking parent nodes
        let query_source = r#"
(function_item
    name: (identifier) @function.name
) @function.definition
"#;

        let function_query = Query::new(&language.into(), query_source)
            .map_err(|e| OverrideError::TreeSitterError(e.to_string()))?;

        Ok(Self {
            parser,
            function_query,
        })
    }

    /// Find all functions in a source file
    pub fn find_functions(&mut self, source: &str) -> Result<Vec<FunctionInfo>> {
        let tree = self
            .parser
            .parse(source, None)
            .ok_or_else(|| OverrideError::ParseError("Failed to parse source".to_string()))?;

        let root_node = tree.root_node();
        let mut cursor = QueryCursor::new();

        let mut functions = Vec::new();
        let mut matches = cursor.matches(&self.function_query, root_node, source.as_bytes());

        while let Some(match_) = matches.next_mut() {
            let mut name = None;
            let mut node = None;

            for capture in match_.captures {
                let capture_name = &self.function_query.capture_names()[capture.index as usize];
                match capture_name as &str {
                    "function.name" => {
                        name = Some(
                            capture
                                .node
                                .utf8_text(source.as_bytes())
                                .map_err(|e| OverrideError::TreeSitterError(e.to_string()))?
                                .to_string(),
                        );
                    }
                    "function.definition" => {
                        node = Some(capture.node);
                    }
                    _ => {}
                }
            }

            if let (Some(name), Some(node)) = (name, node) {
                let start_pos = node.start_position();
                let end_pos = node.end_position();

                // Extract signature
                let signature = node
                    .utf8_text(source.as_bytes())
                    .map_err(|e| OverrideError::TreeSitterError(e.to_string()))?
                    .lines()
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string();

                // Check for test attribute
                let is_test = self.has_test_attribute(&node, source)?;

                // Check if async
                let is_async = signature.starts_with("async ");

                functions.push(FunctionInfo {
                    name,
                    start_line: start_pos.row,
                    end_line: end_pos.row,
                    start_column: start_pos.column,
                    end_column: end_pos.column,
                    signature,
                    is_test,
                    is_async,
                });
            }
        }

        Ok(functions)
    }

    /// Find the function at a specific line
    pub fn find_function_at_line(
        &mut self,
        source: &str,
        line: usize,
    ) -> Result<Option<FunctionInfo>> {
        let functions = self.find_functions(source)?;

        Ok(functions
            .into_iter()
            .find(|f| line >= f.start_line && line <= f.end_line))
    }

    /// Find the function at a specific position (line and column)
    pub fn find_function_at_position(
        &mut self,
        source: &str,
        line: usize,
        column: usize,
    ) -> Result<Option<FunctionInfo>> {
        let functions = self.find_functions(source)?;

        // Find the most specific function that contains the position
        Ok(functions
            .into_iter()
            .filter(|f| {
                line >= f.start_line
                    && line <= f.end_line
                    && (line > f.start_line || column >= f.start_column)
                    && (line < f.end_line || column <= f.end_column)
            })
            .min_by_key(|f| (f.end_line - f.start_line, f.end_column - f.start_column)))
    }

    /// Find functions by name (supports partial matching)
    pub fn find_functions_by_name(
        &mut self,
        source: &str,
        name: &str,
    ) -> Result<Vec<FunctionInfo>> {
        let functions = self.find_functions(source)?;

        Ok(functions
            .into_iter()
            .filter(|f| f.name.contains(name))
            .collect())
    }

    /// Check if a node has a test attribute
    fn has_test_attribute(&self, node: &tree_sitter::Node, source: &str) -> Result<bool> {
        // Check if the function name starts with test_
        if let Ok(text) = node.utf8_text(source.as_bytes()) {
            if text.contains("fn test_") {
                return Ok(true);
            }
        }

        // Check for test attribute
        if let Some(prev) = node.prev_sibling() {
            if prev.kind() == "attribute_item" {
                let text = prev
                    .utf8_text(source.as_bytes())
                    .map_err(|e| OverrideError::TreeSitterError(e.to_string()))?;
                return Ok(text.contains("#[test]") || text.contains("#[tokio::test]"));
            }
        }

        // Check parent for attributes (in case of impl blocks)
        let mut current = *node;
        while let Some(parent) = current.parent() {
            if parent.kind() == "impl_item" {
                break;
            }
            if let Some(prev) = parent.prev_sibling() {
                if prev.kind() == "attribute_item" {
                    let text = prev
                        .utf8_text(source.as_bytes())
                        .map_err(|e| OverrideError::TreeSitterError(e.to_string()))?;
                    if text.contains("#[test]") || text.contains("#[tokio::test]") {
                        return Ok(true);
                    }
                }
            }
            current = parent;
        }

        Ok(false)
    }
}

impl Default for FunctionDetector {
    fn default() -> Self {
        Self::new().expect("Failed to create FunctionDetector")
    }
}

/// Find function in a file at a specific position
pub fn find_function_at_position(
    file_path: &std::path::Path,
    line: usize,
    column: Option<usize>,
) -> Result<Option<FunctionInfo>> {
    let source = std::fs::read_to_string(file_path)?;
    let mut detector = FunctionDetector::new()?;

    if let Some(col) = column {
        detector.find_function_at_position(&source, line, col)
    } else {
        detector.find_function_at_line(&source, line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_simple_function() {
        let source = r#"
fn main() {
    println!("Hello, world!");
}

fn helper() -> i32 {
    42
}
"#;

        let mut detector = FunctionDetector::new().unwrap();
        let functions = detector.find_functions(source).unwrap();

        assert_eq!(functions.len(), 2);
        assert_eq!(functions[0].name, "main");
        assert_eq!(functions[1].name, "helper");
    }

    #[test]
    fn test_find_impl_methods() {
        let source = r#"
struct MyStruct;

impl MyStruct {
    fn new() -> Self {
        Self
    }
    
    fn method(&self) {
        // method body
    }
}
"#;

        let mut detector = FunctionDetector::new().unwrap();
        let functions = detector.find_functions(source).unwrap();

        assert_eq!(functions.len(), 2);
        assert_eq!(functions[0].name, "new");
        assert_eq!(functions[1].name, "method");
    }

    #[test]
    fn test_find_test_functions() {
        let source = r#"
#[test]
fn test_something() {
    assert_eq!(1 + 1, 2);
}

#[tokio::test]
async fn test_async() {
    // async test
}

fn test_by_name() {
    // This should also be detected as a test
}
"#;

        let mut detector = FunctionDetector::new().unwrap();
        let functions = detector.find_functions(source).unwrap();

        assert_eq!(functions.len(), 3);
        assert!(functions[0].is_test);
        assert!(functions[1].is_test);
        assert!(functions[1].is_async);
        assert!(functions[2].is_test); // Detected by name
    }

    #[test]
    fn test_find_function_at_line() {
        let source = r#"
fn first() {
    // line 2
    // line 3
}

fn second() {
    // line 7
}
"#;

        let mut detector = FunctionDetector::new().unwrap();

        let func = detector.find_function_at_line(source, 2).unwrap();
        assert_eq!(func.unwrap().name, "first");

        let func = detector.find_function_at_line(source, 7).unwrap();
        assert_eq!(func.unwrap().name, "second");

        let func = detector.find_function_at_line(source, 5).unwrap();
        assert!(func.is_none());
    }

    #[test]
    fn test_find_function_at_position() {
        let source = r#"
fn outer() {
    fn inner() {
        // line 3, various columns
    }
}
"#;

        let mut detector = FunctionDetector::new().unwrap();

        // Position inside inner function
        let func = detector.find_function_at_position(source, 3, 8).unwrap();
        assert_eq!(func.unwrap().name, "inner");

        // Position at the edge of outer function
        let func = detector.find_function_at_position(source, 1, 0).unwrap();
        assert_eq!(func.unwrap().name, "outer");
    }
}
