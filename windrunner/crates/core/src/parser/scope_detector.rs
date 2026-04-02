use super::utils::node_to_position;
use crate::{
    error::{Error, Result},
    types::{ExtendedScope, FileScope, Position, Scope, ScopeKind},
};
use cargo_toml::Manifest;
use std::path::Path;
use tree_sitter::{Node, Tree};

#[derive(Debug, Clone)]
struct ExtendedScopeDetails {
    #[allow(dead_code)]
    original_start: Position,
    #[allow(dead_code)]
    extended_start: Position,
    doc_comment_lines: u32,
    attribute_lines: u32,
    has_doc_tests: bool,
}

pub struct ScopeDetector;

impl Default for ScopeDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl ScopeDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn detect_scopes(
        &mut self,
        tree: &Tree,
        source: &str,
        file_path: &Path,
    ) -> Result<Vec<ExtendedScope>> {
        let mut scopes = Vec::new();
        let root_node = tree.root_node();

        let file_type = self.determine_file_type(file_path)?;
        let file_scope = Scope {
            start: node_to_position(&root_node, true),
            end: node_to_position(&root_node, false),
            kind: ScopeKind::File(file_type),
            name: None,
        };
        scopes.push(ExtendedScope::from(file_scope));

        self.visit_node(&root_node, source, &mut scopes)?;

        Ok(scopes)
    }

    fn determine_file_type(&self, file_path: &Path) -> Result<FileScope> {
        let path_str = file_path
            .to_str()
            .ok_or_else(|| Error::ParseError("Invalid file path".to_string()))?;
        let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Check well-known patterns first
        if path_str.contains("/src/lib.rs")
            || path_str == "src/lib.rs"
            || path_str.ends_with("/lib.rs")
        {
            return Ok(FileScope::Lib);
        }

        if path_str.contains("/src/main.rs")
            || path_str == "src/main.rs"
            || path_str.ends_with("/main.rs")
        {
            return Ok(FileScope::Bin {
                name: Some("main".to_string()),
            });
        }

        if path_str.contains("/src/bin/") || path_str.starts_with("src/bin/") {
            let name = file_path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string());
            return Ok(FileScope::Bin { name });
        }

        if (path_str.contains("/tests/") || path_str.starts_with("tests/"))
            && !path_str.contains("/src/")
        {
            let name = file_path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string());
            return Ok(FileScope::Test { name });
        }

        if path_str.contains("/benches/") || path_str.starts_with("benches/") {
            let name = file_path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string());
            return Ok(FileScope::Bench { name });
        }

        if path_str.contains("/examples/") || path_str.starts_with("examples/") {
            let name = file_path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string());
            return Ok(FileScope::Example { name });
        }

        if file_name == "build.rs" {
            return Ok(FileScope::Build);
        }

        // Any other .rs file under src/ is part of the library
        if path_str.contains("/src/")
            && file_path.extension().and_then(|s| s.to_str()) == Some("rs")
        {
            return Ok(FileScope::Lib);
        }

        // Check Cargo.toml for custom paths
        use crate::parser::module_resolver::ModuleResolver;
        let has_cargo_toml = ModuleResolver::find_cargo_toml(file_path).is_some();

        if has_cargo_toml {
            if let Some(file_type) = self.check_cargo_toml_for_file_type(file_path)? {
                return Ok(file_type);
            }
            // Has Cargo.toml but file doesn't match any patterns
            return Ok(FileScope::Unknown);
        }

        // No Cargo.toml found - this is a standalone Rust file
        if file_path.extension().and_then(|s| s.to_str()) == Some("rs") {
            let name = file_path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string());
            return Ok(FileScope::Standalone { name });
        }

        Ok(FileScope::Unknown)
    }

    fn check_cargo_toml_for_file_type(&self, file_path: &Path) -> Result<Option<FileScope>> {
        use crate::parser::module_resolver::ModuleResolver;

        // Find the Cargo.toml file
        let cargo_toml_path = if let Some(path) = ModuleResolver::find_cargo_toml(file_path) {
            path
        } else {
            return Ok(None);
        };

        // Get project root
        let project_root = cargo_toml_path
            .parent()
            .ok_or_else(|| Error::ParseError("Cannot determine project root".to_string()))?;

        // Parse Cargo.toml
        let manifest = Manifest::from_path(&cargo_toml_path)
            .map_err(|e| Error::ParseError(format!("Failed to parse Cargo.toml: {e}")))?;

        // Get relative path from project root
        let relative_path = file_path.strip_prefix(project_root).unwrap_or(file_path);
        let relative_str = relative_path
            .to_str()
            .ok_or_else(|| Error::ParseError("Invalid relative path".to_string()))?;

        // Check [[bin]] entries
        for bin in &manifest.bin {
            if let Some(path) = &bin.path {
                if path == relative_str {
                    let name = bin.name.clone().or_else(|| {
                        file_path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .map(|s| s.to_string())
                    });
                    return Ok(Some(FileScope::Bin { name }));
                }
            }
        }

        // Check [[test]] entries
        for test in &manifest.test {
            if let Some(path) = &test.path {
                if path == relative_str {
                    let name = test.name.clone().or_else(|| {
                        file_path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .map(|s| s.to_string())
                    });
                    return Ok(Some(FileScope::Test { name }));
                }
            }
        }

        // Check [[bench]] entries
        for bench in &manifest.bench {
            if let Some(path) = &bench.path {
                if path == relative_str {
                    let name = bench.name.clone().or_else(|| {
                        file_path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .map(|s| s.to_string())
                    });
                    return Ok(Some(FileScope::Bench { name }));
                }
            }
        }

        // Check [[example]] entries
        for example in &manifest.example {
            if let Some(path) = &example.path {
                if path == relative_str {
                    let name = example.name.clone().or_else(|| {
                        file_path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .map(|s| s.to_string())
                    });
                    return Ok(Some(FileScope::Example { name }));
                }
            }
        }

        // Check [lib] entry
        if let Some(lib) = &manifest.lib {
            if let Some(path) = &lib.path {
                if path == relative_str {
                    return Ok(Some(FileScope::Lib));
                }
            }
        }

        Ok(None)
    }

    fn visit_node(&self, node: &Node, source: &str, scopes: &mut Vec<ExtendedScope>) -> Result<()> {
        match node.kind() {
            "function_item" => {
                self.handle_function(node, source, scopes)?;
            }
            "mod_item" => {
                self.handle_module(node, source, scopes)?;
            }
            "struct_item" => {
                self.handle_struct(node, source, scopes)?;
            }
            "enum_item" => {
                self.handle_enum(node, source, scopes)?;
            }
            "union_item" => {
                self.handle_union(node, source, scopes)?;
            }
            "impl_item" => {
                self.handle_impl(node, source, scopes)?;
            }
            _ => {}
        }

        for child in node.children(&mut node.walk()) {
            self.visit_node(&child, source, scopes)?;
        }

        Ok(())
    }

    fn handle_function(
        &self,
        node: &Node,
        source: &str,
        scopes: &mut Vec<ExtendedScope>,
    ) -> Result<()> {
        let name_node = node
            .child_by_field_name("name")
            .ok_or_else(|| Error::ParseError("Function without name".to_string()))?;

        let name = name_node
            .utf8_text(source.as_bytes())
            .map_err(|e| Error::ParseError(format!("Invalid UTF-8 in function name: {e}")))?
            .to_string();

        // Get extended scope info
        let (extended_start, details) = self.find_extended_info(node, source);
        let end = node_to_position(node, false);

        let is_test = self.has_test_attribute(node, source);
        let is_bench = self.has_bench_attribute(node, source);

        let kind = if is_test {
            ScopeKind::Test
        } else if is_bench {
            ScopeKind::Benchmark
        } else {
            ScopeKind::Function
        };

        let scope = Scope {
            start: extended_start,
            end,
            kind,
            name: Some(name),
        };

        let mut extended_scope = ExtendedScope::new(scope)
            .with_extended_start(extended_start)
            .with_doc_comments(details.doc_comment_lines, details.has_doc_tests)
            .with_attributes(details.attribute_lines);

        // Set the correct original start position
        extended_scope.original_start = details.original_start;

        scopes.push(extended_scope);

        Ok(())
    }

    fn handle_module(
        &self,
        node: &Node,
        source: &str,
        scopes: &mut Vec<ExtendedScope>,
    ) -> Result<()> {
        let name_node = node
            .child_by_field_name("name")
            .ok_or_else(|| Error::ParseError("Module without name".to_string()))?;

        let name = name_node
            .utf8_text(source.as_bytes())
            .map_err(|e| Error::ParseError(format!("Invalid UTF-8 in module name: {e}")))?
            .to_string();

        // Get extended scope info for modules too (e.g., #[cfg(test)])
        let (extended_start, details) = self.find_extended_info(node, source);
        let end = node_to_position(node, false);

        let scope = Scope {
            start: extended_start,
            end,
            kind: ScopeKind::Module,
            name: Some(name),
        };

        let mut extended_scope = ExtendedScope::new(scope)
            .with_extended_start(extended_start)
            .with_doc_comments(details.doc_comment_lines, details.has_doc_tests)
            .with_attributes(details.attribute_lines);

        // Set the correct original start position
        extended_scope.original_start = details.original_start;

        scopes.push(extended_scope);

        Ok(())
    }

    fn handle_struct(
        &self,
        node: &Node,
        source: &str,
        scopes: &mut Vec<ExtendedScope>,
    ) -> Result<()> {
        let name_node = node
            .child_by_field_name("name")
            .ok_or_else(|| Error::ParseError("Struct without name".to_string()))?;

        let name = name_node
            .utf8_text(source.as_bytes())
            .map_err(|e| Error::ParseError(format!("Invalid UTF-8 in struct name: {e}")))?
            .to_string();

        // Get extended scope info
        let (extended_start, details) = self.find_extended_info(node, source);
        let end = node_to_position(node, false);

        let scope = Scope {
            start: extended_start,
            end,
            kind: ScopeKind::Struct,
            name: Some(name),
        };

        let mut extended_scope = ExtendedScope::new(scope)
            .with_extended_start(extended_start)
            .with_doc_comments(details.doc_comment_lines, details.has_doc_tests)
            .with_attributes(details.attribute_lines);

        // Set the correct original start position
        extended_scope.original_start = details.original_start;

        scopes.push(extended_scope);

        Ok(())
    }

    fn handle_enum(
        &self,
        node: &Node,
        source: &str,
        scopes: &mut Vec<ExtendedScope>,
    ) -> Result<()> {
        let name_node = node
            .child_by_field_name("name")
            .ok_or_else(|| Error::ParseError("Enum without name".to_string()))?;

        let name = name_node
            .utf8_text(source.as_bytes())
            .map_err(|e| Error::ParseError(format!("Invalid UTF-8 in enum name: {e}")))?
            .to_string();

        // Get extended scope info
        let (extended_start, details) = self.find_extended_info(node, source);
        let end = node_to_position(node, false);

        let scope = Scope {
            start: extended_start,
            end,
            kind: ScopeKind::Enum,
            name: Some(name),
        };

        let mut extended_scope = ExtendedScope::new(scope)
            .with_extended_start(extended_start)
            .with_doc_comments(details.doc_comment_lines, details.has_doc_tests)
            .with_attributes(details.attribute_lines);

        // Set the correct original start position
        extended_scope.original_start = details.original_start;

        scopes.push(extended_scope);

        Ok(())
    }

    fn handle_union(
        &self,
        node: &Node,
        source: &str,
        scopes: &mut Vec<ExtendedScope>,
    ) -> Result<()> {
        let name_node = node
            .child_by_field_name("name")
            .ok_or_else(|| Error::ParseError("Union without name".to_string()))?;

        let name = name_node
            .utf8_text(source.as_bytes())
            .map_err(|e| Error::ParseError(format!("Invalid UTF-8 in union name: {e}")))?
            .to_string();

        // Get extended scope info
        let (extended_start, details) = self.find_extended_info(node, source);
        let end = node_to_position(node, false);

        let scope = Scope {
            start: extended_start,
            end,
            kind: ScopeKind::Union,
            name: Some(name),
        };

        let mut extended_scope = ExtendedScope::new(scope)
            .with_extended_start(extended_start)
            .with_doc_comments(details.doc_comment_lines, details.has_doc_tests)
            .with_attributes(details.attribute_lines);

        // Set the correct original start position
        extended_scope.original_start = details.original_start;

        scopes.push(extended_scope);

        Ok(())
    }

    fn handle_impl(
        &self,
        node: &Node,
        source: &str,
        scopes: &mut Vec<ExtendedScope>,
    ) -> Result<()> {
        let mut name = String::from("impl");

        if let Some(type_node) = node.child_by_field_name("type") {
            if let Ok(type_name) = type_node.utf8_text(source.as_bytes()) {
                name = format!("impl {type_name}");
            }
        }

        if let Some(trait_node) = node.child_by_field_name("trait") {
            if let Ok(trait_name) = trait_node.utf8_text(source.as_bytes()) {
                name = format!("{trait_name} for");
                if let Some(type_node) = node.child_by_field_name("type") {
                    if let Ok(type_name) = type_node.utf8_text(source.as_bytes()) {
                        name = format!("{trait_name} for {type_name}");
                    }
                }
            }
        }

        // Get extended scope info
        let (extended_start, details) = self.find_extended_info(node, source);
        let end = node_to_position(node, false);

        let scope = Scope {
            start: extended_start,
            end,
            kind: ScopeKind::Impl,
            name: Some(name),
        };

        let mut extended_scope = ExtendedScope::new(scope)
            .with_extended_start(extended_start)
            .with_doc_comments(details.doc_comment_lines, details.has_doc_tests)
            .with_attributes(details.attribute_lines);

        // Set the correct original start position
        extended_scope.original_start = details.original_start;

        scopes.push(extended_scope);

        Ok(())
    }

    fn has_test_attribute(&self, node: &Node, source: &str) -> bool {
        // Check for attribute items before the function
        let mut sibling = node.prev_sibling();

        while let Some(s) = sibling {
            if s.kind() == "attribute_item" {
                if let Ok(text) = s.utf8_text(source.as_bytes()) {
                    if text.contains("#[test]") || text.contains("#[tokio::test]") {
                        return true;
                    }
                }
            } else if s.kind() != "line_comment" && s.kind() != "block_comment" {
                // Stop if we hit something that's not an attribute or comment
                break;
            }
            sibling = s.prev_sibling();
        }

        false
    }

    fn has_bench_attribute(&self, node: &Node, source: &str) -> bool {
        // Check for attribute items before the function
        let mut sibling = node.prev_sibling();

        while let Some(s) = sibling {
            if s.kind() == "attribute_item" {
                if let Ok(text) = s.utf8_text(source.as_bytes()) {
                    if text.contains("#[bench]") {
                        return true;
                    }
                }
            } else if s.kind() != "line_comment" && s.kind() != "block_comment" {
                // Stop if we hit something that's not an attribute or comment
                break;
            }
            sibling = s.prev_sibling();
        }

        false
    }

    fn find_extended_info(&self, node: &Node, source: &str) -> (Position, ExtendedScopeDetails) {
        let original_start = node_to_position(node, true);
        let mut current_sibling = node.prev_sibling();
        let mut extended_start = original_start;
        let mut doc_comment_lines = 0u32;
        let mut attribute_lines = 0u32;
        let mut has_doc_tests = false;
        let mut found_any = false;

        // Process siblings in reverse order to find the topmost doc comment or attribute
        let mut siblings_to_process = Vec::new();
        while let Some(sibling) = current_sibling {
            match sibling.kind() {
                "line_comment" => {
                    if let Ok(text) = sibling.utf8_text(source.as_bytes()) {
                        if text.starts_with("///") {
                            siblings_to_process.push((sibling, "doc_comment"));
                            if text.contains("```") {
                                has_doc_tests = true;
                            }
                        }
                        // Don't break on regular comments - just skip them
                        // This allows us to find doc comments that come before regular comments
                    }
                }
                "attribute_item" => {
                    siblings_to_process.push((sibling, "attribute"));
                }
                _ => {
                    // Stop if we hit anything else
                    break;
                }
            }
            current_sibling = sibling.prev_sibling();
        }

        // Process in forward order to get the correct counts
        for (sibling, kind) in siblings_to_process.iter().rev() {
            let pos = node_to_position(sibling, true);
            if !found_any {
                extended_start = pos;
                found_any = true;
            }

            match *kind {
                "doc_comment" => {
                    doc_comment_lines += 1;
                }
                "attribute" => {
                    let end_pos = node_to_position(sibling, false);
                    attribute_lines += end_pos.line - pos.line + 1;
                }
                _ => {}
            }
        }

        let details = ExtendedScopeDetails {
            original_start,
            extended_start,
            doc_comment_lines,
            attribute_lines,
            has_doc_tests,
        };

        (extended_start, details)
    }

    pub fn find_doc_tests(&self, node: &Node, source: &str) -> Vec<(Position, Position, String)> {
        find_doc_tests_recursive(node, source)
    }
}

fn find_doc_tests_recursive(node: &Node, source: &str) -> Vec<(Position, Position, String)> {
    let mut doc_tests = Vec::new();

    if node.kind() == "line_comment" {
        if let Ok(comment_text) = node.utf8_text(source.as_bytes()) {
            if comment_text.starts_with("///") && comment_text.contains("```") {
                let start = node_to_position(node, true);

                let mut current = Some(*node);
                let mut _end = node_to_position(node, false);
                let mut full_text = String::new();

                while let Some(n) = current {
                    if let Ok(text) = n.utf8_text(source.as_bytes()) {
                        if let Some(stripped) = text.strip_prefix("///") {
                            full_text.push_str(stripped.trim_start());
                            full_text.push('\n');
                            _end = node_to_position(&n, false);

                            if text.contains("```") && full_text.matches("```").count() >= 2 {
                                doc_tests.push((start, _end, full_text.clone()));
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                    current = n.next_sibling();
                }
            }
        }
    }

    for child in node.children(&mut node.walk()) {
        doc_tests.extend(find_doc_tests_recursive(&child, source));
    }

    doc_tests
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::RustParser;

    #[test]
    fn test_detect_function_scope() {
        let source = r#"
fn main() {
    println!("Hello, world!");
}

fn test_function() {
    assert_eq!(2 + 2, 4);
}
"#;

        let mut parser = RustParser::new().unwrap();
        let scopes = parser.get_scopes(source, Path::new("test.rs")).unwrap();

        assert!(scopes.iter().any(|s| s.name == Some("main".to_string())));
        assert!(
            scopes
                .iter()
                .any(|s| s.name == Some("test_function".to_string()))
        );
    }

    #[test]
    fn test_detect_test_scope() {
        let source = r#"
#[test]
fn test_addition() {
    assert_eq!(2 + 2, 4);
}
"#;

        let mut parser = RustParser::new().unwrap();
        let scopes = parser.get_scopes(source, Path::new("test.rs")).unwrap();

        let test_scope = scopes
            .iter()
            .find(|s| s.name == Some("test_addition".to_string()))
            .unwrap();

        assert_eq!(test_scope.kind, ScopeKind::Test);
    }
}
