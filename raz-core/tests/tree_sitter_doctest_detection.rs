use raz_core::Position;
use raz_core::file_detection::{EntryPointType, FileDetector};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_tree_sitter_doctest_detection() {
    let temp_dir = TempDir::new().unwrap();

    // Create cargo project
    let cargo_toml = r#"
[package]
name = "tree-sitter-doctest-test"
version = "0.1.0"
edition = "2021"
"#;
    fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml).unwrap();

    // Create lib.rs with doctests
    fs::create_dir(temp_dir.path().join("src")).unwrap();
    let lib_content = r#"/// Adds two numbers
/// ```
/// use tree_sitter_doctest_test::add;
/// assert_eq!(add(2, 3), 5);
/// ```
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

/// User struct
/// ```
/// use tree_sitter_doctest_test::User;
/// let u = User::new("Alice".to_string());
/// assert_eq!(u.name, "Alice");
/// ```
pub struct User {
    pub name: String,
}

impl User {
    /// Creates a new user
    /// ```
    /// use tree_sitter_doctest_test::User;
    /// let user = User::new("Bob".to_string());
    /// assert_eq!(user.name, "Bob");
    /// ```
    pub fn new(name: String) -> Self {
        User { name }
    }

    /// Get greeting
    /// ```
    /// use tree_sitter_doctest_test::User;
    /// let user = User::new("Charlie".to_string());
    /// assert_eq!(user.greet(), "Hello, Charlie!");
    /// ```
    pub fn greet(&self) -> String {
        format!("Hello, {}!", self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
"#;
    let lib_path = temp_dir.path().join("src/lib.rs");
    fs::write(&lib_path, lib_content).unwrap();

    // Test: Detect doctests using tree-sitter
    let context = FileDetector::detect_context(&lib_path, None).unwrap();

    // Count doctests
    let doctest_count = context
        .entry_points
        .iter()
        .filter(|ep| ep.entry_type == EntryPointType::DocTest)
        .count();

    println!("Found {doctest_count} doctests");
    for ep in &context.entry_points {
        if ep.entry_type == EntryPointType::DocTest {
            println!(
                "Doctest: {} at line {} (range: {:?})",
                ep.name, ep.line, ep.line_range
            );
        }
    }

    // We should have at least 4 doctests (add, User, User::new, User::greet)
    assert!(
        doctest_count >= 4,
        "Expected at least 4 doctests, found {doctest_count}"
    );

    // Test: Cursor in doc comment should detect doctest
    let cursor_in_add_doc = Position { line: 1, column: 0 }; // Line: /// ```
    let context_with_cursor =
        FileDetector::detect_context(&lib_path, Some(cursor_in_add_doc)).unwrap();

    // Should still detect all doctests
    let doctest_count_with_cursor = context_with_cursor
        .entry_points
        .iter()
        .filter(|ep| ep.entry_type == EntryPointType::DocTest)
        .count();

    assert_eq!(
        doctest_count_with_cursor, doctest_count,
        "Doctest count should be consistent with cursor position"
    );

    println!("Tree-sitter doctest detection test passed!");
}

#[test]
fn test_tree_sitter_vs_regex_doctest_detection() {
    let temp_dir = TempDir::new().unwrap();

    // Create cargo project
    let cargo_toml = r#"
[package]
name = "comparison-test"
version = "0.1.0"
edition = "2021"
"#;
    fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml).unwrap();

    // Create complex doctests that might confuse regex-based detection
    fs::create_dir(temp_dir.path().join("src")).unwrap();
    let lib_content = r#"/// Complex doctest example
/// This has multiple code blocks and comments
/// ```
/// use comparison_test::complex_function;
/// # This is a hidden line
/// # let hidden_var = 42;
/// let result = complex_function(1, 2);
/// assert_eq!(result, 3);
/// ```
/// And some more text...
/// ```
/// // Another code block in the same doctest
/// use comparison_test::complex_function;
/// assert_eq!(complex_function(10, 20), 30);
/// ```
pub fn complex_function(a: i32, b: i32) -> i32 {
    a + b
}
"#;
    let lib_path = temp_dir.path().join("src/lib.rs");
    fs::write(&lib_path, lib_content).unwrap();

    // Test: Detect doctests
    let context = FileDetector::detect_context(&lib_path, None).unwrap();

    let doctests: Vec<_> = context
        .entry_points
        .iter()
        .filter(|ep| ep.entry_type == EntryPointType::DocTest)
        .collect();

    println!("Found {} doctests:", doctests.len());
    for doctest in &doctests {
        println!(
            "  - {} at line {} (range: {:?})",
            doctest.name, doctest.line, doctest.line_range
        );
    }

    // Should find exactly one doctest for the complex_function
    assert_eq!(
        doctests.len(),
        1,
        "Expected exactly 1 doctest, found {}",
        doctests.len()
    );
    assert_eq!(doctests[0].name, "complex_function");

    println!("Complex doctest detection test passed!");
}
