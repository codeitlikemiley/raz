use serde::{Deserialize, Serialize};

use super::position::Position;
use super::scope_kind::ScopeKind;

/// Represents a scope in the source code with start/end positions and metadata
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scope {
    pub start: Position,
    pub end: Position,
    pub kind: ScopeKind,
    pub name: Option<String>,
}

impl Scope {
    /// Check if a position is within this scope
    pub fn contains(&self, position: Position) -> bool {
        position >= self.start && position <= self.end
    }

    /// Check if a line number is within this scope
    pub fn contains_line(&self, line: u32) -> bool {
        line >= self.start.line && line <= self.end.line
    }
}

/// Extended scope information including doc comments and attributes
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtendedScope {
    pub scope: Scope,
    /// The original start position without doc comments/attributes
    pub original_start: Position,
    /// Number of doc comment lines
    pub doc_comment_lines: u32,
    /// Number of attribute lines
    pub attribute_lines: u32,
    /// Whether this scope has doc tests
    pub has_doc_tests: bool,
}

impl ExtendedScope {
    pub fn new(scope: Scope) -> Self {
        let original_start = scope.start;
        Self {
            scope,
            original_start,
            doc_comment_lines: 0,
            attribute_lines: 0,
            has_doc_tests: false,
        }
    }

    pub fn with_doc_comments(mut self, lines: u32, has_tests: bool) -> Self {
        self.doc_comment_lines = lines;
        self.has_doc_tests = has_tests;
        self
    }

    pub fn with_attributes(mut self, lines: u32) -> Self {
        self.attribute_lines = lines;
        self
    }

    pub fn with_extended_start(mut self, start: Position) -> Self {
        self.scope.start = start;
        self
    }

    pub fn to_scope(self) -> Scope {
        self.scope
    }
}

impl From<Scope> for ExtendedScope {
    fn from(scope: Scope) -> Self {
        ExtendedScope::new(scope)
    }
}

impl From<ExtendedScope> for Scope {
    fn from(extended: ExtendedScope) -> Self {
        extended.scope
    }
}
