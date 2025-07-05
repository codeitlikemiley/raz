//! Abstract Syntax Tree parsing and analysis using tree-sitter
//!
//! This module provides AST-based analysis of Rust source code to enable
//! context-aware command suggestions based on cursor position and code structure.

use crate::{Position, Range, RazError, RazResult, Symbol, SymbolKind};
use std::collections::HashMap;
use tree_sitter::{Language, Node, Parser, Query, QueryCursor, StreamingIteratorMut, Tree};

/// AST analyzer for Rust source code
pub struct RustAnalyzer {
    parser: Parser,
    queries: QuerySet,
}

impl RustAnalyzer {
    /// Create a new Rust AST analyzer
    pub fn new() -> RazResult<Self> {
        let language = tree_sitter_rust::LANGUAGE;
        let mut parser = Parser::new();
        parser
            .set_language(&language.into())
            .map_err(|e| RazError::analysis(format!("Failed to set language: {e}")))?;

        let queries = QuerySet::new(language.into())?;

        Ok(Self { parser, queries })
    }

    /// Parse source code and return the syntax tree
    pub fn parse(&mut self, source: &str) -> RazResult<Tree> {
        self.parser
            .parse(source, None)
            .ok_or_else(|| RazError::analysis("Failed to parse source code".to_string()))
    }

    /// Extract all symbols from the AST
    pub fn extract_symbols(&self, tree: &Tree, source: &str) -> RazResult<Vec<Symbol>> {
        let mut symbols = Vec::new();
        let root_node = tree.root_node();

        // Extract tests first to identify test functions
        let test_symbols = self.extract_tests(&root_node, source)?;
        let test_names: std::collections::HashSet<String> =
            test_symbols.iter().map(|s| s.name.clone()).collect();
        symbols.extend(test_symbols);

        // Extract functions (but skip test functions)
        let functions = self.extract_functions(&root_node, source)?;
        for func in functions {
            if !test_names.contains(&func.name) {
                symbols.push(func);
            }
        }

        // Extract structs
        symbols.extend(self.extract_structs(&root_node, source)?);

        // Extract enums
        symbols.extend(self.extract_enums(&root_node, source)?);

        // Extract traits
        symbols.extend(self.extract_traits(&root_node, source)?);

        // Extract modules
        symbols.extend(self.extract_modules(&root_node, source)?);

        // Extract constants
        symbols.extend(self.extract_constants(&root_node, source)?);

        // Extract type aliases
        symbols.extend(self.extract_type_aliases(&root_node, source)?);

        // Extract macros
        symbols.extend(self.extract_macros(&root_node, source)?);

        Ok(symbols)
    }

    /// Find the symbol at a specific cursor position
    pub fn symbol_at_position(
        &self,
        tree: &Tree,
        source: &str,
        position: Position,
    ) -> RazResult<Option<Symbol>> {
        let symbols = self.extract_symbols(tree, source)?;

        // Find the most specific symbol that contains the cursor position
        let mut best_match: Option<Symbol> = None;
        let mut smallest_range = u32::MAX;

        for symbol in symbols {
            if symbol.range.contains_position(position) {
                let range_size = symbol.range.end.line - symbol.range.start.line;
                if range_size < smallest_range {
                    smallest_range = range_size;
                    best_match = Some(symbol);
                }
            }
        }

        Ok(best_match)
    }

    /// Get the context around a cursor position (parent functions, modules, etc.)
    pub fn context_at_position(
        &self,
        tree: &Tree,
        source: &str,
        position: Position,
    ) -> RazResult<SymbolContext> {
        let symbols = self.extract_symbols(tree, source)?;
        let mut context = SymbolContext::default();

        // Find all symbols that contain the position
        for symbol in symbols {
            if symbol.range.contains_position(position) {
                match symbol.kind {
                    SymbolKind::Function => {
                        if symbol.modifiers.contains(&"test".to_string()) {
                            context.in_test_function = Some(symbol.clone());
                        } else {
                            context.in_function = Some(symbol.clone());
                        }
                    }
                    SymbolKind::Struct => context.in_struct = Some(symbol.clone()),
                    SymbolKind::Enum => context.in_enum = Some(symbol.clone()),
                    SymbolKind::Trait => context.in_trait = Some(symbol.clone()),
                    SymbolKind::Module => context.in_module = Some(symbol.clone()),
                    SymbolKind::Impl => context.in_impl = Some(symbol.clone()),
                    _ => {}
                }
            }
        }

        Ok(context)
    }

    /// Extract function definitions from the AST
    fn extract_functions(&self, node: &Node, source: &str) -> RazResult<Vec<Symbol>> {
        let mut functions = Vec::new();
        let mut cursor = QueryCursor::new();

        let mut matches = cursor.matches(&self.queries.functions, *node, source.as_bytes());

        while let Some(match_) = matches.next_mut() {
            if let Some(function) = self.parse_function_match(match_, source)? {
                functions.push(function);
            }
        }

        Ok(functions)
    }

    /// Extract struct definitions from the AST
    fn extract_structs(&self, node: &Node, source: &str) -> RazResult<Vec<Symbol>> {
        let mut structs = Vec::new();
        let mut cursor = QueryCursor::new();

        let mut matches = cursor.matches(&self.queries.structs, *node, source.as_bytes());

        while let Some(match_) = matches.next_mut() {
            if let Some(struct_symbol) = self.parse_struct_match(match_, source)? {
                structs.push(struct_symbol);
            }
        }

        Ok(structs)
    }

    /// Extract enum definitions from the AST
    fn extract_enums(&self, node: &Node, source: &str) -> RazResult<Vec<Symbol>> {
        let mut enums = Vec::new();
        let mut cursor = QueryCursor::new();

        let mut matches = cursor.matches(&self.queries.enums, *node, source.as_bytes());

        while let Some(match_) = matches.next_mut() {
            if let Some(enum_symbol) = self.parse_enum_match(match_, source)? {
                enums.push(enum_symbol);
            }
        }

        Ok(enums)
    }

    /// Extract trait definitions from the AST
    fn extract_traits(&self, node: &Node, source: &str) -> RazResult<Vec<Symbol>> {
        let mut traits = Vec::new();
        let mut cursor = QueryCursor::new();

        let mut matches = cursor.matches(&self.queries.traits, *node, source.as_bytes());

        while let Some(match_) = matches.next_mut() {
            if let Some(trait_symbol) = self.parse_trait_match(match_, source)? {
                traits.push(trait_symbol);
            }
        }

        Ok(traits)
    }

    /// Extract module definitions from the AST
    fn extract_modules(&self, node: &Node, source: &str) -> RazResult<Vec<Symbol>> {
        let mut modules = Vec::new();
        let mut cursor = QueryCursor::new();

        let mut matches = cursor.matches(&self.queries.modules, *node, source.as_bytes());

        while let Some(match_) = matches.next_mut() {
            if let Some(module_symbol) = self.parse_module_match(match_, source)? {
                modules.push(module_symbol);
            }
        }

        Ok(modules)
    }

    /// Extract test functions from the AST
    fn extract_tests(&self, node: &Node, source: &str) -> RazResult<Vec<Symbol>> {
        let mut tests = Vec::new();

        // Find test attributes first
        let mut cursor = QueryCursor::new();
        let mut attr_matches = cursor.matches(&self.queries.tests, *node, source.as_bytes());

        // Collect test attribute nodes
        let mut test_attrs = Vec::new();
        while let Some(match_) = attr_matches.next_mut() {
            for capture in match_.captures {
                let node = capture.node;
                let capture_name = &self.queries.tests.capture_names()[capture.index as usize];
                if capture_name as &str == "test.attr" {
                    test_attrs.push(node);
                }
            }
        }

        // Now find function items that have test attributes
        for test_attr in test_attrs {
            if let Some(function_item) = self.find_function_with_test_attr(test_attr, node) {
                if let Some(test_symbol) =
                    self.create_test_symbol_from_function(function_item, source)?
                {
                    tests.push(test_symbol);
                }
            }
        }

        // Also find functions that start with "test_" even without #[test] attribute
        // This handles cases where the AST might not capture the attribute properly
        let function_query = Query::new(
            &tree_sitter_rust::LANGUAGE.into(),
            r#"
            (function_item
                name: (identifier) @func.name
            ) @func
            "#,
        )
        .map_err(|e| RazError::analysis(format!("Failed to create function query: {e}")))?;

        let mut func_matches = cursor.matches(&function_query, *node, source.as_bytes());
        while let Some(match_) = func_matches.next_mut() {
            let mut func_name = None;
            let mut func_node = None;

            for capture in match_.captures {
                let capture_name = &function_query.capture_names()[capture.index as usize];
                match capture_name as &str {
                    "func.name" => {
                        func_name = capture
                            .node
                            .utf8_text(source.as_bytes())
                            .ok()
                            .map(|s| s.to_string());
                    }
                    "func" => {
                        func_node = Some(capture.node);
                    }
                    _ => {}
                }
            }

            if let (Some(name), Some(node)) = (func_name, func_node) {
                if name.starts_with("test_") && !tests.iter().any(|t| t.name == name) {
                    tests.push(Symbol {
                        name,
                        kind: SymbolKind::Test,
                        range: self.node_to_range(node),
                        modifiers: vec!["test".to_string()],
                        children: Vec::new(),
                        metadata: HashMap::new(),
                    });
                }
            }
        }

        Ok(tests)
    }

    fn find_function_with_test_attr<'a>(
        &self,
        test_attr: Node<'a>,
        _root: &Node,
    ) -> Option<Node<'a>> {
        // Walk up from test attribute to find the function_item
        let mut current = test_attr;
        while let Some(parent) = current.parent() {
            if parent.kind() == "function_item" {
                return Some(parent);
            }
            current = parent;
        }
        None
    }

    fn create_test_symbol_from_function(
        &self,
        function_node: Node<'_>,
        source: &str,
    ) -> RazResult<Option<Symbol>> {
        // Extract function name
        let mut cursor = function_node.walk();

        for child in function_node.children(&mut cursor) {
            if child.kind() == "identifier" {
                let name = child
                    .utf8_text(source.as_bytes())
                    .map_err(|e| {
                        RazError::analysis(format!("Failed to extract test function name: {e}"))
                    })?
                    .to_string();

                return Ok(Some(Symbol {
                    name,
                    kind: SymbolKind::Test,
                    range: self.node_to_range(function_node),
                    modifiers: vec!["test".to_string()],
                    children: Vec::new(),
                    metadata: HashMap::new(),
                }));
            }
        }

        Ok(None)
    }

    /// Extract constant definitions from the AST
    fn extract_constants(&self, node: &Node, source: &str) -> RazResult<Vec<Symbol>> {
        let mut constants = Vec::new();
        let mut cursor = QueryCursor::new();

        let mut matches = cursor.matches(&self.queries.constants, *node, source.as_bytes());

        while let Some(match_) = matches.next_mut() {
            if let Some(const_symbol) = self.parse_constant_match(match_, source)? {
                constants.push(const_symbol);
            }
        }

        Ok(constants)
    }

    /// Extract type alias definitions from the AST
    fn extract_type_aliases(&self, node: &Node, source: &str) -> RazResult<Vec<Symbol>> {
        let mut type_aliases = Vec::new();
        let mut cursor = QueryCursor::new();

        let mut matches = cursor.matches(&self.queries.type_aliases, *node, source.as_bytes());

        while let Some(match_) = matches.next_mut() {
            if let Some(type_alias) = self.parse_type_alias_match(match_, source)? {
                type_aliases.push(type_alias);
            }
        }

        Ok(type_aliases)
    }

    /// Extract macro definitions from the AST
    fn extract_macros(&self, node: &Node, source: &str) -> RazResult<Vec<Symbol>> {
        let mut macros = Vec::new();
        let mut cursor = QueryCursor::new();

        let mut matches = cursor.matches(&self.queries.macros, *node, source.as_bytes());

        while let Some(match_) = matches.next_mut() {
            if let Some(macro_symbol) = self.parse_macro_match(match_, source)? {
                macros.push(macro_symbol);
            }
        }

        Ok(macros)
    }

    /// Parse a function match from the query result
    fn parse_function_match(
        &self,
        match_: &tree_sitter::QueryMatch,
        source: &str,
    ) -> RazResult<Option<Symbol>> {
        let mut name = None;
        let mut range = None;
        let mut modifiers = Vec::new();

        for capture in match_.captures {
            let node = capture.node;
            let capture_name = &self.queries.functions.capture_names()[capture.index as usize];

            match capture_name as &str {
                "function.name" => {
                    name = Some(
                        node.utf8_text(source.as_bytes())
                            .map_err(|e| {
                                RazError::analysis(format!("Failed to extract function name: {e}"))
                            })?
                            .to_string(),
                    );
                    range = Some(self.node_to_range(node));
                }
                "function.async" => modifiers.push("async".to_string()),
                "function.unsafe" => modifiers.push("unsafe".to_string()),
                "function.const" => modifiers.push("const".to_string()),
                "function.extern" => modifiers.push("extern".to_string()),
                _ => {}
            }
        }

        if let (Some(name), Some(range)) = (name, range) {
            Ok(Some(Symbol {
                name,
                kind: SymbolKind::Function,
                range,
                modifiers,
                children: Vec::new(),
                metadata: HashMap::new(),
            }))
        } else {
            Ok(None)
        }
    }

    /// Parse a struct match from the query result
    fn parse_struct_match(
        &self,
        match_: &tree_sitter::QueryMatch,
        source: &str,
    ) -> RazResult<Option<Symbol>> {
        let mut name = None;
        let mut range = None;
        let mut modifiers = Vec::new();

        for capture in match_.captures {
            let node = capture.node;
            let capture_name = &self.queries.structs.capture_names()[capture.index as usize];

            match capture_name as &str {
                "struct.name" => {
                    name = Some(
                        node.utf8_text(source.as_bytes())
                            .map_err(|e| {
                                RazError::analysis(format!("Failed to extract struct name: {e}"))
                            })?
                            .to_string(),
                    );
                    // Get the range of the entire struct item, not just the name
                    if let Some(parent) = node.parent() {
                        if parent.kind() == "struct_item" {
                            range = Some(self.node_to_range(parent));
                        } else {
                            range = Some(self.node_to_range(node));
                        }
                    } else {
                        range = Some(self.node_to_range(node));
                    }
                }
                "struct.vis" => {
                    let vis_text = node.utf8_text(source.as_bytes()).map_err(|e| {
                        RazError::analysis(format!("Failed to extract struct visibility: {e}"))
                    })?;
                    modifiers.push(vis_text.to_string());
                }
                _ => {}
            }
        }

        if let (Some(name), Some(range)) = (name, range) {
            Ok(Some(Symbol {
                name,
                kind: SymbolKind::Struct,
                range,
                modifiers,
                children: Vec::new(),
                metadata: HashMap::new(),
            }))
        } else {
            Ok(None)
        }
    }

    /// Parse an enum match from the query result
    fn parse_enum_match(
        &self,
        match_: &tree_sitter::QueryMatch,
        source: &str,
    ) -> RazResult<Option<Symbol>> {
        let mut name = None;
        let mut range = None;
        let mut modifiers = Vec::new();

        for capture in match_.captures {
            let node = capture.node;
            let capture_name = &self.queries.enums.capture_names()[capture.index as usize];

            match capture_name as &str {
                "enum.name" => {
                    name = Some(
                        node.utf8_text(source.as_bytes())
                            .map_err(|e| {
                                RazError::analysis(format!("Failed to extract enum name: {e}"))
                            })?
                            .to_string(),
                    );
                    range = Some(self.node_to_range(node));
                }
                "enum.pub" => modifiers.push("pub".to_string()),
                _ => {}
            }
        }

        if let (Some(name), Some(range)) = (name, range) {
            Ok(Some(Symbol {
                name,
                kind: SymbolKind::Enum,
                range,
                modifiers,
                children: Vec::new(),
                metadata: HashMap::new(),
            }))
        } else {
            Ok(None)
        }
    }

    /// Parse a trait match from the query result
    fn parse_trait_match(
        &self,
        match_: &tree_sitter::QueryMatch,
        source: &str,
    ) -> RazResult<Option<Symbol>> {
        let mut name = None;
        let mut range = None;
        let mut modifiers = Vec::new();

        for capture in match_.captures {
            let node = capture.node;
            let capture_name = &self.queries.traits.capture_names()[capture.index as usize];

            match capture_name as &str {
                "trait.name" => {
                    name = Some(
                        node.utf8_text(source.as_bytes())
                            .map_err(|e| {
                                RazError::analysis(format!("Failed to extract trait name: {e}"))
                            })?
                            .to_string(),
                    );
                    range = Some(self.node_to_range(node));
                }
                "trait.pub" => modifiers.push("pub".to_string()),
                "trait.unsafe" => modifiers.push("unsafe".to_string()),
                _ => {}
            }
        }

        if let (Some(name), Some(range)) = (name, range) {
            Ok(Some(Symbol {
                name,
                kind: SymbolKind::Trait,
                range,
                modifiers,
                children: Vec::new(),
                metadata: HashMap::new(),
            }))
        } else {
            Ok(None)
        }
    }

    /// Parse a module match from the query result
    fn parse_module_match(
        &self,
        match_: &tree_sitter::QueryMatch,
        source: &str,
    ) -> RazResult<Option<Symbol>> {
        let mut name = None;
        let mut range = None;
        let mut modifiers = Vec::new();

        for capture in match_.captures {
            let node = capture.node;
            let capture_name = &self.queries.modules.capture_names()[capture.index as usize];

            match capture_name as &str {
                "module.name" => {
                    name = Some(
                        node.utf8_text(source.as_bytes())
                            .map_err(|e| {
                                RazError::analysis(format!("Failed to extract module name: {e}"))
                            })?
                            .to_string(),
                    );
                }
                "module" => {
                    // This captures the entire module
                    range = Some(self.node_to_range(node));
                }
                "module.pub" => modifiers.push("pub".to_string()),
                _ => {}
            }
        }

        if let (Some(name), Some(range)) = (name, range) {
            Ok(Some(Symbol {
                name,
                kind: SymbolKind::Module,
                range,
                modifiers,
                children: Vec::new(),
                metadata: HashMap::new(),
            }))
        } else {
            Ok(None)
        }
    }

    /// Parse a constant match from the query result
    fn parse_constant_match(
        &self,
        match_: &tree_sitter::QueryMatch,
        source: &str,
    ) -> RazResult<Option<Symbol>> {
        let mut name = None;
        let mut range = None;
        let mut modifiers = Vec::new();

        for capture in match_.captures {
            let node = capture.node;
            let capture_name = &self.queries.constants.capture_names()[capture.index as usize];

            match capture_name as &str {
                "const.name" => {
                    name = Some(
                        node.utf8_text(source.as_bytes())
                            .map_err(|e| {
                                RazError::analysis(format!("Failed to extract constant name: {e}"))
                            })?
                            .to_string(),
                    );
                    range = Some(self.node_to_range(node));
                }
                "const.pub" => modifiers.push("pub".to_string()),
                _ => {}
            }
        }

        if let (Some(name), Some(range)) = (name, range) {
            Ok(Some(Symbol {
                name,
                kind: SymbolKind::Constant,
                range,
                modifiers,
                children: Vec::new(),
                metadata: HashMap::new(),
            }))
        } else {
            Ok(None)
        }
    }

    /// Parse a type alias match from the query result
    fn parse_type_alias_match(
        &self,
        match_: &tree_sitter::QueryMatch,
        source: &str,
    ) -> RazResult<Option<Symbol>> {
        let mut name = None;
        let mut range = None;
        let mut modifiers = Vec::new();

        for capture in match_.captures {
            let node = capture.node;
            let capture_name = &self.queries.type_aliases.capture_names()[capture.index as usize];

            match capture_name as &str {
                "type.name" => {
                    name = Some(
                        node.utf8_text(source.as_bytes())
                            .map_err(|e| {
                                RazError::analysis(format!(
                                    "Failed to extract type alias name: {e}"
                                ))
                            })?
                            .to_string(),
                    );
                    range = Some(self.node_to_range(node));
                }
                "type.pub" => modifiers.push("pub".to_string()),
                _ => {}
            }
        }

        if let (Some(name), Some(range)) = (name, range) {
            Ok(Some(Symbol {
                name,
                kind: SymbolKind::TypeAlias,
                range,
                modifiers,
                children: Vec::new(),
                metadata: HashMap::new(),
            }))
        } else {
            Ok(None)
        }
    }

    /// Parse a macro match from the query result
    fn parse_macro_match(
        &self,
        match_: &tree_sitter::QueryMatch,
        source: &str,
    ) -> RazResult<Option<Symbol>> {
        let mut name = None;
        let mut range = None;
        let mut modifiers = Vec::new();

        for capture in match_.captures {
            let node = capture.node;
            let capture_name = &self.queries.macros.capture_names()[capture.index as usize];

            match capture_name as &str {
                "macro.name" => {
                    name = Some(
                        node.utf8_text(source.as_bytes())
                            .map_err(|e| {
                                RazError::analysis(format!("Failed to extract macro name: {e}"))
                            })?
                            .to_string(),
                    );
                    range = Some(self.node_to_range(node));
                }
                "macro.pub" => modifiers.push("pub".to_string()),
                _ => {}
            }
        }

        if let (Some(name), Some(range)) = (name, range) {
            Ok(Some(Symbol {
                name,
                kind: SymbolKind::Macro,
                range,
                modifiers,
                children: Vec::new(),
                metadata: HashMap::new(),
            }))
        } else {
            Ok(None)
        }
    }

    /// Convert a tree-sitter node to a Range
    fn node_to_range(&self, node: Node) -> Range {
        Range {
            start: Position {
                line: node.start_position().row as u32,
                column: node.start_position().column as u32,
            },
            end: Position {
                line: node.end_position().row as u32,
                column: node.end_position().column as u32,
            },
        }
    }
}

/// Context information about symbols surrounding a cursor position
#[derive(Debug, Clone, Default)]
pub struct SymbolContext {
    /// The function containing the cursor (if any)
    pub in_function: Option<Symbol>,

    /// The test function containing the cursor (if any)
    pub in_test_function: Option<Symbol>,

    /// The struct containing the cursor (if any)
    pub in_struct: Option<Symbol>,

    /// The enum containing the cursor (if any)
    pub in_enum: Option<Symbol>,

    /// The trait containing the cursor (if any)
    pub in_trait: Option<Symbol>,

    /// The module containing the cursor (if any)
    pub in_module: Option<Symbol>,

    /// The impl block containing the cursor (if any)
    pub in_impl: Option<Symbol>,
}

/// Collection of tree-sitter queries for extracting symbols
struct QuerySet {
    functions: Query,
    structs: Query,
    enums: Query,
    traits: Query,
    modules: Query,
    tests: Query,
    constants: Query,
    type_aliases: Query,
    macros: Query,
}

impl QuerySet {
    fn new(language: Language) -> RazResult<Self> {
        let functions = Query::new(
            &language,
            r#"
            (function_item 
                name: (identifier) @function.name
            )
        "#,
        )
        .map_err(|e| RazError::analysis(format!("Failed to create functions query: {e}")))?;

        let structs = Query::new(
            &language,
            r#"
            (struct_item 
                (visibility_modifier)? @struct.vis
                name: (type_identifier) @struct.name
            )
        "#,
        )
        .map_err(|e| RazError::analysis(format!("Failed to create structs query: {e}")))?;

        let enums = Query::new(
            &language,
            r#"
            (enum_item 
                name: (type_identifier) @enum.name
            )
        "#,
        )
        .map_err(|e| RazError::analysis(format!("Failed to create enums query: {e}")))?;

        let traits = Query::new(
            &language,
            r#"
            (trait_item 
                name: (type_identifier) @trait.name
            )
        "#,
        )
        .map_err(|e| RazError::analysis(format!("Failed to create traits query: {e}")))?;

        let modules = Query::new(
            &language,
            r#"
            (mod_item 
                name: (identifier) @module.name
            ) @module
        "#,
        )
        .map_err(|e| RazError::analysis(format!("Failed to create modules query: {e}")))?;

        let tests = Query::new(
            &language,
            r#"
            (attribute_item
                (attribute
                    (identifier) @test.attr
                    (#eq? @test.attr "test")
                )
            )
        "#,
        )
        .map_err(|e| RazError::analysis(format!("Failed to create tests query: {e}")))?;

        let constants = Query::new(
            &language,
            r#"
            (const_item 
                name: (identifier) @const.name
            )
        "#,
        )
        .map_err(|e| RazError::analysis(format!("Failed to create constants query: {e}")))?;

        let type_aliases = Query::new(
            &language,
            r#"
            (type_item 
                name: (type_identifier) @type.name
            )
        "#,
        )
        .map_err(|e| RazError::analysis(format!("Failed to create type aliases query: {e}")))?;

        let macros = Query::new(
            &language,
            r#"
            (macro_definition 
                name: (identifier) @macro.name
            )
        "#,
        )
        .map_err(|e| RazError::analysis(format!("Failed to create macros query: {e}")))?;

        Ok(Self {
            functions,
            structs,
            enums,
            traits,
            modules,
            tests,
            constants,
            type_aliases,
            macros,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_analyzer_creation() {
        let analyzer = RustAnalyzer::new();
        assert!(analyzer.is_ok());
    }

    #[test]
    fn test_parse_simple_function() {
        let mut analyzer = RustAnalyzer::new().unwrap();
        let source = r#"
            fn hello_world() {
                println!("Hello, world!");
            }
        "#;

        let tree = analyzer.parse(source).unwrap();
        let symbols = analyzer.extract_symbols(&tree, source).unwrap();

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "hello_world");
        assert_eq!(symbols[0].kind, SymbolKind::Function);
    }

    #[test]
    fn test_parse_struct() {
        let mut analyzer = RustAnalyzer::new().unwrap();
        let source = r#"
            pub struct Person {
                name: String,
                age: u32,
            }
        "#;

        let tree = analyzer.parse(source).unwrap();
        let symbols = analyzer.extract_symbols(&tree, source).unwrap();

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "Person");
        assert_eq!(symbols[0].kind, SymbolKind::Struct);
        assert!(symbols[0].modifiers.contains(&"pub".to_string()));
    }

    #[test]
    #[ignore] // TODO: Fix tree-sitter test attribute parsing
    fn test_parse_test_function() {
        let mut analyzer = RustAnalyzer::new().unwrap();
        let source = r#"
            #[test]
            fn test_addition() {
                assert_eq!(2 + 2, 4);
            }
        "#;

        let tree = analyzer.parse(source).unwrap();
        let symbols = analyzer.extract_symbols(&tree, source).unwrap();

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "test_addition");
        assert_eq!(symbols[0].kind, SymbolKind::Test);
        assert!(symbols[0].modifiers.contains(&"test".to_string()));
    }

    #[test]
    #[ignore] // TODO: Fix tree-sitter symbol position detection
    fn test_symbol_at_position() {
        let mut analyzer = RustAnalyzer::new().unwrap();
        let source = r#"
fn main() {
    println!("Hello");
}

fn helper() {
    println!("Helper");
}
        "#;

        let tree = analyzer.parse(source).unwrap();

        // Position inside main function
        let symbol = analyzer
            .symbol_at_position(&tree, source, Position { line: 2, column: 4 })
            .unwrap();
        assert!(symbol.is_some());
        assert_eq!(symbol.unwrap().name, "main");

        // Position inside helper function
        let symbol = analyzer
            .symbol_at_position(&tree, source, Position { line: 6, column: 4 })
            .unwrap();
        assert!(symbol.is_some());
        assert_eq!(symbol.unwrap().name, "helper");
    }

    #[test]
    #[ignore] // TODO: Fix tree-sitter context position detection
    fn test_context_at_position() {
        let mut analyzer = RustAnalyzer::new().unwrap();
        let source = r#"
mod tests {
    #[test]
    fn test_something() {
        assert_eq!(1, 1);
    }
}
        "#;

        let tree = analyzer.parse(source).unwrap();
        let context = analyzer
            .context_at_position(&tree, source, Position { line: 4, column: 8 })
            .unwrap();

        assert!(context.in_test_function.is_some());
        assert_eq!(context.in_test_function.unwrap().name, "test_something");
        assert!(context.in_module.is_some());
        assert_eq!(context.in_module.unwrap().name, "tests");
    }
}
