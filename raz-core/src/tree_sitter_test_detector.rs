//! Tree-sitter based test detection for accurate AST parsing
//!
//! This module uses tree-sitter-rust to properly parse Rust code and detect:
//! - Test modules (#[cfg(test)] mod tests {})
//! - Test functions (#[test] fn test_name())
//! - Module boundaries and nesting
//! - Cursor position relative to AST nodes

use crate::{EntryPoint, EntryPointType, Position, RazError, RazResult};
use regex::Regex;

#[cfg(feature = "tree-sitter-support")]
use tree_sitter::{Node, Parser};

#[cfg(feature = "tree-sitter-support")]
pub struct TreeSitterTestDetector {
    parser: Parser,
}

#[cfg(feature = "tree-sitter-support")]
impl TreeSitterTestDetector {
    pub fn new() -> RazResult<Self> {
        let mut parser = Parser::new();
        let language = tree_sitter_rust::LANGUAGE;
        parser
            .set_language(&language.into())
            .map_err(|e| RazError::analysis(format!("Failed to set tree-sitter language: {e}")))?;

        Ok(Self { parser })
    }

    /// Detect all entry points in the file (tests, main, benchmarks, etc.)
    pub fn detect_entry_points(
        &mut self,
        source: &str,
        cursor: Option<Position>,
    ) -> RazResult<Vec<EntryPoint>> {
        let tree = self
            .parser
            .parse(source, None)
            .ok_or_else(|| RazError::analysis("Failed to parse source code".to_string()))?;

        let mut entry_points = Vec::new();
        let mut module_stack: Vec<(String, u32, u32)> = Vec::new(); // (name, start_line, end_line)

        // Walk the tree to find test modules and functions
        self.walk_tree(
            tree.root_node(),
            source,
            cursor,
            &mut entry_points,
            &mut module_stack,
        )?;

        // Also detect doctests
        self.detect_doctests(tree.root_node(), source, &mut entry_points)?;

        Ok(entry_points)
    }

    /// Detect doctest blocks in the source code using tree-sitter
    pub fn detect_doctests(
        &self,
        node: Node,
        source: &str,
        entry_points: &mut Vec<EntryPoint>,
    ) -> RazResult<()> {
        self.walk_tree_for_doctests(node, source, entry_points, "")?;
        Ok(())
    }

    /// Recursively walk the tree to find doc comments with code blocks
    /// Since tree-sitter doesn't include comments in the AST, we need to use source positions
    fn walk_tree_for_doctests(
        &self,
        node: Node,
        source: &str,
        entry_points: &mut Vec<EntryPoint>,
        _parent_name: &str,
    ) -> RazResult<()> {
        match node.kind() {
            "function_item" | "struct_item" | "enum_item" | "impl_item" => {
                // Get the name of this item
                let item_name = self.extract_item_name(&node, source).unwrap_or_default();

                // Since tree-sitter doesn't include comments, we need to search the source text
                // Look for doc comments preceding this item's position
                let item_start_line = node.start_position().row;

                if let Some((doc_start, doc_end)) =
                    self.find_doc_comment_before_line(source, item_start_line)
                {
                    // Check if the doc comment contains code blocks
                    let lines: Vec<&str> = source.lines().collect();
                    let doc_lines = &lines[doc_start..=doc_end];
                    let doc_text = doc_lines.join("\n");

                    if self.contains_rust_code_blocks(&doc_text) {
                        let start_line = doc_start as u32 + 1; // Convert to 1-based
                        let end_line = doc_end as u32 + 1;

                        let doctest_name = if item_name.is_empty() {
                            format!("doctest_{start_line}_{end_line}")
                        } else {
                            item_name.clone()
                        };

                        entry_points.push(EntryPoint {
                            name: doctest_name,
                            entry_type: EntryPointType::DocTest,
                            line: start_line,
                            column: 0,
                            line_range: (start_line, end_line),
                            full_path: Some(item_name.clone()),
                        });
                    }
                }

                // Recursively process children with current item name as parent
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    self.walk_tree_for_doctests(child, source, entry_points, &item_name)?;
                }
            }
            _ => {
                // For other node types, just recurse
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    self.walk_tree_for_doctests(child, source, entry_points, _parent_name)?;
                }
            }
        }

        Ok(())
    }

    /// Find doc comment lines that precede the given line
    /// Returns (start_line, end_line) in 0-based indexing
    fn find_doc_comment_before_line(
        &self,
        source: &str,
        item_line: usize,
    ) -> Option<(usize, usize)> {
        let lines: Vec<&str> = source.lines().collect();

        if item_line == 0 || item_line >= lines.len() {
            return None;
        }

        // Walk backwards from the item line to find doc comments
        let mut end_line = None;
        let mut start_line = None;

        for i in (0..item_line).rev() {
            let line = lines[i].trim();

            // Check if this is a doc comment
            if line.starts_with("///") || line.starts_with("//!") {
                if end_line.is_none() {
                    end_line = Some(i);
                }
                start_line = Some(i);
            } else if line.is_empty() && start_line.is_some() {
                // Empty line within doc comment block - continue
                continue;
            } else if start_line.is_some() {
                // Hit a non-doc-comment line - stop
                break;
            } else {
                // Haven't found any doc comments yet and hit non-doc line - continue searching
                continue;
            }
        }

        if let (Some(start), Some(end)) = (start_line, end_line) {
            Some((start, end))
        } else {
            None
        }
    }

    /// Check if the doc comment text contains Rust code blocks
    fn contains_rust_code_blocks(&self, doc_text: &str) -> bool {
        // Look for ``` blocks (with or without rust specifier)
        let has_code_blocks = doc_text.contains("```");

        if !has_code_blocks {
            return false;
        }

        // Simple heuristic: if it contains ``` and common Rust patterns, it's likely a doctest
        let rust_patterns = [
            "assert_eq!",
            "assert!",
            "panic!",
            "println!",
            "fn ",
            "let ",
            "use ",
            "::new",
            "#[",
            "impl ",
            "struct ",
            "enum ",
            "trait ",
            "mod ",
            "pub ",
        ];

        rust_patterns
            .iter()
            .any(|pattern| doc_text.contains(pattern))
    }

    /// Extract the name of an item (function, struct, etc.)
    fn extract_item_name(&self, node: &Node, source: &str) -> Option<String> {
        match node.kind() {
            "function_item" => self.get_function_name(node, source).ok(),
            "struct_item" => self.get_struct_name(node, source).ok(),
            "enum_item" => self.get_enum_name(node, source).ok(),
            "impl_item" => self.get_impl_target_name(node, source).ok(),
            _ => None,
        }
    }

    /// Get struct name from struct_item node
    fn get_struct_name(&self, node: &Node, source: &str) -> RazResult<String> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "type_identifier" {
                if let Ok(text) = child.utf8_text(source.as_bytes()) {
                    return Ok(text.to_string());
                }
            }
        }
        Err(RazError::analysis("Struct name not found".to_string()))
    }

    /// Get enum name from enum_item node
    fn get_enum_name(&self, node: &Node, source: &str) -> RazResult<String> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "type_identifier" {
                if let Ok(text) = child.utf8_text(source.as_bytes()) {
                    return Ok(text.to_string());
                }
            }
        }
        Err(RazError::analysis("Enum name not found".to_string()))
    }

    /// Get impl target name from impl_item node
    fn get_impl_target_name(&self, node: &Node, source: &str) -> RazResult<String> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "type_identifier" {
                if let Ok(text) = child.utf8_text(source.as_bytes()) {
                    return Ok(text.to_string());
                }
            }
        }
        Err(RazError::analysis("Impl target name not found".to_string()))
    }

    /// Recursively walk the AST to find test-related nodes
    fn walk_tree(
        &self,
        node: Node,
        source: &str,
        _cursor: Option<Position>,
        entry_points: &mut Vec<EntryPoint>,
        module_stack: &mut Vec<(String, u32, u32)>,
    ) -> RazResult<()> {
        // Debug output
        // #[cfg(debug_assertions)]
        // {
        //     let indent = "  ".repeat(module_stack.len());
        //     eprintln!("{}Walking node: {} at line {}", indent, node.kind(), node.start_position().row + 1);
        // }

        match node.kind() {
            "mod_item" => {
                // Get module name first
                let module_name = self.get_module_name(&node, source)?;
                let start_line = node.start_position().row as u32 + 1;
                let end_line = node.end_position().row as u32 + 1;

                // Check if this is a test module or if we're inside a test context
                let is_test_mod = self.is_test_module(&node, source);
                let in_test_context = module_stack.iter().any(|(name, _, _)| name == "tests");

                if is_test_mod || in_test_context {
                    // Build full module path
                    let full_path = if module_stack.is_empty() {
                        module_name.clone()
                    } else {
                        format!(
                            "{}::{}",
                            module_stack
                                .iter()
                                .map(|(name, _, _)| name.as_str())
                                .collect::<Vec<_>>()
                                .join("::"),
                            module_name
                        )
                    };

                    // Add module entry point
                    entry_points.push(EntryPoint {
                        name: module_name.clone(),
                        entry_type: EntryPointType::TestModule,
                        line: start_line,
                        column: 0,
                        line_range: (start_line, end_line),
                        full_path: Some(full_path.clone()),
                    });
                }

                // Push to stack for nested tracking (even if not a test module)
                module_stack.push((module_name, start_line, end_line));

                // Look specifically for declaration_list which contains the module body
                let mut mod_cursor = node.walk();
                for child in node.children(&mut mod_cursor) {
                    if child.kind() == "declaration_list" {
                        // Process all items in the module body
                        let mut body_cursor = child.walk();
                        for body_item in child.children(&mut body_cursor) {
                            self.walk_tree(body_item, source, _cursor, entry_points, module_stack)?;
                        }
                    }
                }

                // Pop module from stack
                module_stack.pop();

                // Return early to avoid double-processing
                return Ok(());
            }
            "function_item" => {
                let fn_name = self.get_function_name(&node, source)?;
                let start_line = node.start_position().row as u32 + 1;
                let end_line = node.end_position().row as u32 + 1;

                // Check if this is a test function
                if self.is_test_function(&node, source) {
                    // Build full path including module hierarchy
                    let full_path = if module_stack.is_empty() {
                        fn_name.clone()
                    } else {
                        format!(
                            "{}::{}",
                            module_stack
                                .iter()
                                .map(|(name, _, _)| name.as_str())
                                .collect::<Vec<_>>()
                                .join("::"),
                            fn_name
                        )
                    };

                    entry_points.push(EntryPoint {
                        name: fn_name,
                        entry_type: EntryPointType::Test,
                        line: start_line,
                        column: node.start_position().column as u32,
                        line_range: (start_line, end_line),
                        full_path: Some(full_path),
                    });
                } else if fn_name == "main" {
                    // Detect main function with proper line range
                    entry_points.push(EntryPoint {
                        name: "main".to_string(),
                        entry_type: EntryPointType::Main,
                        line: start_line,
                        column: node.start_position().column as u32,
                        line_range: (start_line, end_line),
                        full_path: None,
                    });
                } else if self.is_bench_function(&node, source) {
                    // Detect benchmark function with proper line range
                    entry_points.push(EntryPoint {
                        name: fn_name,
                        entry_type: EntryPointType::Benchmark,
                        line: start_line,
                        column: node.start_position().column as u32,
                        line_range: (start_line, end_line),
                        full_path: None,
                    });
                }
            }
            _ => {}
        }

        // Store the current module stack size before processing children
        let initial_stack_size = module_stack.len();

        // Recursively process children
        let mut child_cursor = node.walk();
        for child in node.children(&mut child_cursor) {
            self.walk_tree(child, source, _cursor, entry_points, module_stack)?;
        }

        // Pop any modules that were added for this node
        while module_stack.len() > initial_stack_size {
            module_stack.pop();
        }

        Ok(())
    }

    /// Check if a module node is a test module (#[cfg(test)])
    fn is_test_module(&self, node: &Node, source: &str) -> bool {
        // Look for #[cfg(test)] attribute
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "attribute_item" {
                let attr_text = child.utf8_text(source.as_bytes()).unwrap_or("");
                if attr_text.contains("cfg(test)") {
                    return true;
                }
            }
        }

        // Also check if module name contains "test"
        if let Ok(name) = self.get_module_name(node, source) {
            return name.contains("test");
        }

        false
    }

    /// Check if a function node is a test function (#[test])
    fn is_test_function(&self, node: &Node, source: &str) -> bool {
        // Check previous sibling for test attributes
        if let Some(prev_sibling) = node.prev_sibling() {
            if prev_sibling.kind() == "attribute_item" {
                let attr_text = prev_sibling.utf8_text(source.as_bytes());
                if let Ok(text) = attr_text {
                    // Match various test attributes
                    if text.contains("test") && !text.contains("bench") {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Check if a function node is a benchmark function (#[bench])
    fn is_bench_function(&self, node: &Node, source: &str) -> bool {
        // Check previous sibling for bench attributes
        if let Some(prev_sibling) = node.prev_sibling() {
            if prev_sibling.kind() == "attribute_item" {
                let attr_text = prev_sibling.utf8_text(source.as_bytes());
                if let Ok(text) = attr_text {
                    if text.contains("bench") {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Extract module name from a mod_item node
    fn get_module_name(&self, node: &Node, source: &str) -> RazResult<String> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" {
                if let Ok(text) = child.utf8_text(source.as_bytes()) {
                    return Ok(text.to_string());
                }
            }
        }

        Err(RazError::analysis("Module name not found".to_string()))
    }

    /// Extract function name from a function_item node
    fn get_function_name(&self, node: &Node, source: &str) -> RazResult<String> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" {
                if let Ok(text) = child.utf8_text(source.as_bytes()) {
                    return Ok(text.to_string());
                }
            }
        }

        Err(RazError::analysis("Function name not found".to_string()))
    }

    /// Find what test context the cursor is in
    pub fn find_test_context_at_cursor(
        &mut self,
        source: &str,
        cursor: Position,
    ) -> RazResult<Option<TestContext>> {
        let tree = self
            .parser
            .parse(source, None)
            .ok_or_else(|| RazError::analysis("Failed to parse source code".to_string()))?;

        let cursor_byte = self.position_to_byte_offset(source, cursor);
        let mut context = TestContext::default();

        self.find_context_recursive(
            tree.root_node(),
            source,
            cursor,
            cursor_byte,
            &mut context,
            &mut Vec::new(),
        )?;

        if context.in_test_function.is_some() || context.in_test_module.is_some() {
            Ok(Some(context))
        } else {
            Ok(None)
        }
    }

    /// Find the doctest associated with the cursor position
    /// This uses tree-sitter AST to properly associate functions with their doctests
    pub fn find_doctest_at_cursor(
        &mut self,
        source: &str,
        cursor: Position,
    ) -> RazResult<Option<EntryPoint>> {
        let tree = self
            .parser
            .parse(source, None)
            .ok_or_else(|| RazError::analysis("Failed to parse source code".to_string()))?;

        // First check if cursor is in a test context - if so, don't associate with doctests
        if let Some(test_context) = self.find_test_context_at_cursor(source, cursor)? {
            if test_context.in_test_function.is_some() || test_context.in_test_module.is_some() {
                return Ok(None);
            }
        }

        let cursor_byte = self.position_to_byte_offset(source, cursor);

        // Walk the tree to find what item the cursor is in or near
        if let Some(associated_item) =
            self.find_item_at_cursor(tree.root_node(), source, cursor_byte)?
        {
            // Now find the doctest for this item
            let mut entry_points = Vec::new();
            self.detect_doctests(tree.root_node(), source, &mut entry_points)?;

            // Find the doctest that matches this item
            for ep in entry_points {
                if ep.entry_type == EntryPointType::DocTest {
                    if let Some(full_path) = &ep.full_path {
                        if full_path == &associated_item {
                            return Ok(Some(ep));
                        }
                    }
                }
            }
        }

        // Fallback: check if cursor is directly in a doc comment
        self.find_doctest_containing_cursor(tree.root_node(), source, cursor_byte)
    }

    /// Find the item (function, struct, etc.) that the cursor is in or near
    fn find_item_at_cursor(
        &self,
        node: Node,
        source: &str,
        cursor_byte: usize,
    ) -> RazResult<Option<String>> {
        // Check if cursor is within this node
        if cursor_byte >= node.start_byte() && cursor_byte <= node.end_byte() {
            match node.kind() {
                "function_item" => {
                    if let Ok(name) = self.get_function_name(&node, source) {
                        return Ok(Some(name));
                    }
                }
                "struct_item" => {
                    if let Ok(name) = self.get_struct_name(&node, source) {
                        return Ok(Some(name));
                    }
                }
                "enum_item" => {
                    if let Ok(name) = self.get_enum_name(&node, source) {
                        return Ok(Some(name));
                    }
                }
                "impl_item" => {
                    if let Ok(name) = self.get_impl_target_name(&node, source) {
                        return Ok(Some(name));
                    }
                }
                _ => {}
            }

            // Check children
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if let Some(item_name) = self.find_item_at_cursor(child, source, cursor_byte)? {
                    return Ok(Some(item_name));
                }
            }
        }

        Ok(None)
    }

    /// Find doctest that contains the cursor position
    /// Since comments aren't in the AST, we check the source directly
    fn find_doctest_containing_cursor(
        &self,
        _node: Node,
        source: &str,
        cursor_byte: usize,
    ) -> RazResult<Option<EntryPoint>> {
        // Convert byte offset to line/column
        let cursor_line = self.byte_offset_to_line(source, cursor_byte);

        // Check if the cursor line is within a doc comment that has code blocks
        let lines: Vec<&str> = source.lines().collect();

        if cursor_line >= lines.len() {
            return Ok(None);
        }

        let current_line = lines[cursor_line].trim();

        // Check if cursor is on a doc comment line
        if current_line.starts_with("///") || current_line.starts_with("//!") {
            // Find the full doc comment block containing this line
            if let Some((doc_start, doc_end)) =
                self.find_doc_comment_block_containing_line(source, cursor_line)
            {
                let doc_lines = &lines[doc_start..=doc_end];
                let doc_text = doc_lines.join("\n");

                if self.contains_rust_code_blocks(&doc_text) {
                    // Find the associated item that follows this doc comment
                    let associated_item = self
                        .find_item_name_after_line(source, doc_end)
                        .unwrap_or_else(|| format!("doctest_{}_{}", doc_start + 1, doc_end + 1));

                    return Ok(Some(EntryPoint {
                        name: associated_item.clone(),
                        entry_type: EntryPointType::DocTest,
                        line: doc_start as u32 + 1,
                        column: 0,
                        line_range: (doc_start as u32 + 1, doc_end as u32 + 1),
                        full_path: Some(associated_item),
                    }));
                }
            }
        }

        Ok(None)
    }

    /// Convert byte offset to line number (0-based)
    fn byte_offset_to_line(&self, source: &str, byte_offset: usize) -> usize {
        let mut current_byte = 0;

        for (line_num, line) in source.lines().enumerate() {
            let line_end = current_byte + line.len() + 1; // +1 for newline

            if byte_offset <= line_end {
                return line_num;
            }

            current_byte = line_end;
        }

        // If we reach here, cursor is at end of file
        source.lines().count().saturating_sub(1)
    }

    /// Find the complete doc comment block that contains the given line
    fn find_doc_comment_block_containing_line(
        &self,
        source: &str,
        line_num: usize,
    ) -> Option<(usize, usize)> {
        let lines: Vec<&str> = source.lines().collect();

        if line_num >= lines.len() {
            return None;
        }

        // Find the start of the doc comment block
        let mut start_line = line_num;
        for i in (0..=line_num).rev() {
            let line = lines[i].trim();
            if line.starts_with("///") || line.starts_with("//!") {
                start_line = i;
            } else if line.is_empty() {
                // Empty lines are allowed within doc comments
                continue;
            } else {
                // Hit a non-doc-comment line
                break;
            }
        }

        // Find the end of the doc comment block
        let mut end_line = line_num;
        for (i, line) in lines.iter().enumerate().skip(line_num) {
            let line = line.trim();
            if line.starts_with("///") || line.starts_with("//!") {
                end_line = i;
            } else if line.is_empty() {
                // Empty lines are allowed within doc comments
                continue;
            } else {
                // Hit a non-doc-comment line
                break;
            }
        }

        // Verify this is actually a doc comment
        if lines[start_line].trim().starts_with("///")
            || lines[start_line].trim().starts_with("//!")
        {
            Some((start_line, end_line))
        } else {
            None
        }
    }

    /// Find the name of the item that follows the given line
    fn find_item_name_after_line(&self, source: &str, after_line: usize) -> Option<String> {
        let lines: Vec<&str> = source.lines().collect();

        // Look for function/struct/enum/impl declarations after the doc comment
        for line in lines.iter().skip(after_line + 1) {
            let line = line.trim();

            if line.is_empty() || line.starts_with("///") || line.starts_with("//!") {
                continue;
            }

            // Try to extract item name from various patterns
            if let Some(name) = self.extract_name_from_declaration(line) {
                return Some(name);
            }

            // If we hit a non-empty, non-comment line without extracting a name, stop
            break;
        }

        None
    }

    /// Extract item name from a declaration line
    fn extract_name_from_declaration(&self, line: &str) -> Option<String> {
        // Use lazy_static for regex patterns to avoid recompilation
        use once_cell::sync::Lazy;

        static FN_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"fn\s+(\w+)").unwrap());
        static STRUCT_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"struct\s+(\w+)").unwrap());
        static ENUM_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"enum\s+(\w+)").unwrap());
        static IMPL_REGEX: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"impl\s+(?:\w+\s+for\s+)?(\w+)").unwrap());

        // Function: pub fn name(...) or fn name(...)
        if let Some(captures) = FN_REGEX.captures(line) {
            return captures.get(1).map(|m| m.as_str().to_string());
        }

        // Struct: pub struct Name or struct Name
        if let Some(captures) = STRUCT_REGEX.captures(line) {
            return captures.get(1).map(|m| m.as_str().to_string());
        }

        // Enum: pub enum Name or enum Name
        if let Some(captures) = ENUM_REGEX.captures(line) {
            return captures.get(1).map(|m| m.as_str().to_string());
        }

        // Impl: impl Name or impl Trait for Name
        if let Some(captures) = IMPL_REGEX.captures(line) {
            return captures.get(1).map(|m| m.as_str().to_string());
        }

        None
    }

    /// Recursively find the test context at cursor position
    fn find_context_recursive(
        &self,
        node: Node,
        source: &str,
        _cursor: Position,
        cursor_byte: usize,
        context: &mut TestContext,
        module_stack: &mut Vec<String>,
    ) -> RazResult<()> {
        let node_start = node.start_byte();
        let node_end = node.end_byte();

        // Check if cursor is within this node
        if cursor_byte >= node_start && cursor_byte <= node_end {
            let initial_stack_size = module_stack.len();

            match node.kind() {
                "mod_item" => {
                    let module_name = self.get_module_name(&node, source)?;
                    let is_test_mod = self.is_test_module(&node, source);

                    // Always push to stack for path tracking, regardless of test status
                    module_stack.push(module_name.clone());

                    // Only set context if it's a test module
                    if is_test_mod {
                        context.in_test_module = Some(TestModule {
                            name: module_name,
                            full_path: module_stack.join("::"),
                            line_range: (
                                node.start_position().row as u32 + 1,
                                node.end_position().row as u32 + 1,
                            ),
                        });
                    }
                }
                "function_item" => {
                    if self.is_test_function(&node, source) {
                        let fn_name = self.get_function_name(&node, source)?;
                        let full_path = if module_stack.is_empty() {
                            fn_name.clone()
                        } else {
                            format!("{}::{}", module_stack.join("::"), fn_name)
                        };

                        context.in_test_function = Some(TestFunction {
                            name: fn_name,
                            full_path,
                            line: node.start_position().row as u32 + 1,
                        });
                    }
                }
                _ => {}
            }

            // Check children
            let mut tree_cursor = node.walk();
            for child in node.children(&mut tree_cursor) {
                self.find_context_recursive(
                    child,
                    source,
                    _cursor,
                    cursor_byte,
                    context,
                    module_stack,
                )?;
            }

            // Restore module stack to original size
            module_stack.truncate(initial_stack_size);
        }

        Ok(())
    }

    /// Convert Position to byte offset in source
    fn position_to_byte_offset(&self, source: &str, position: Position) -> usize {
        let mut current_line = 0;
        let mut line_start_byte = 0;

        for (byte_pos, ch) in source.char_indices() {
            if current_line == position.line {
                // We're on the target line, add column offset
                for (current_column, (col_byte_pos, _)) in
                    source[line_start_byte..].char_indices().enumerate()
                {
                    if current_column >= position.column as usize {
                        return line_start_byte + col_byte_pos;
                    }
                }
                // If we reach end of line, return end of line
                return byte_pos;
            }

            if ch == '\n' {
                current_line += 1;
                line_start_byte = byte_pos + 1;
                if current_line > position.line {
                    return line_start_byte;
                }
            }
        }

        source.len() // Return end of file if position is beyond
    }
}

#[derive(Debug, Default)]
pub struct TestContext {
    pub in_test_module: Option<TestModule>,
    pub in_test_function: Option<TestFunction>,
}

#[derive(Debug)]
pub struct TestModule {
    pub name: String,
    pub full_path: String,
    pub line_range: (u32, u32),
}

#[derive(Debug)]
pub struct TestFunction {
    pub name: String,
    pub full_path: String,
    pub line: u32,
}

// Fallback implementation when tree-sitter is not available
#[cfg(not(feature = "tree-sitter-support"))]
pub struct TreeSitterTestDetector;

#[cfg(not(feature = "tree-sitter-support"))]
impl TreeSitterTestDetector {
    pub fn new() -> RazResult<Self> {
        Ok(Self)
    }

    pub fn detect_test_entry_points(
        &mut self,
        _source: &str,
        _cursor: Option<Position>,
    ) -> RazResult<Vec<EntryPoint>> {
        Err(RazError::analysis(
            "Tree-sitter support not enabled. Enable the 'advanced-analysis' feature.".to_string(),
        ))
    }

    pub fn find_test_context_at_cursor(
        &mut self,
        _source: &str,
        _cursor: Position,
    ) -> RazResult<Option<TestContext>> {
        Err(RazError::analysis(
            "Tree-sitter support not enabled. Enable the 'advanced-analysis' feature.".to_string(),
        ))
    }
}

#[cfg(not(feature = "tree-sitter-support"))]
#[derive(Debug, Default)]
pub struct TestContext;
