use raz_core::Position;
use raz_core::file_detection::FileDetector;
use raz_core::universal_command_generator::UniversalCommandGenerator;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_doctest_function_association() {
    let temp_dir = TempDir::new().unwrap();

    // Create cargo project
    let cargo_toml = r#"
[package]
name = "function-association-test"
version = "0.1.0"
edition = "2021"
"#;
    fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml).unwrap();

    // Create lib.rs with doctests that should be associated with functions
    fs::create_dir(temp_dir.path().join("src")).unwrap();
    let lib_content = r#"/// Adds two numbers
/// ```
/// use function_association_test::add;
/// assert_eq!(add(2, 3), 5);
/// ```
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

/// User struct
/// ```
/// use function_association_test::User;
/// let u = User::new("Alice".to_string());
/// assert_eq!(u.name, "Alice");
/// ```
pub struct User {
    pub name: String,
}

impl User {
    /// Creates a new user
    /// ```
    /// use function_association_test::User;
    /// let user = User::new("Bob".to_string());
    /// assert_eq!(user.name, "Bob");
    /// ```
    pub fn new(name: String) -> Self {
        User { name }
    }

    /// Get greeting
    /// ```
    /// use function_association_test::User;
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
    fn test_add() {
        assert_eq!(add(1, 1), 2);
    }
}
"#;
    let lib_path = temp_dir.path().join("src/lib.rs");
    fs::write(&lib_path, lib_content).unwrap();

    // Test Case 1: Cursor on function declaration should find associated doctest
    let cursor_on_add_fn = Position { line: 5, column: 0 }; // pub fn add line
    let context1 = FileDetector::detect_context(&lib_path, Some(cursor_on_add_fn)).unwrap();
    let commands1 =
        UniversalCommandGenerator::generate_commands(&context1, Some(cursor_on_add_fn)).unwrap();

    println!("\n=== Test Case 1: Cursor on function declaration ===");
    let test_at_cursor1 =
        UniversalCommandGenerator::find_test_at_cursor(&context1.entry_points, cursor_on_add_fn);
    println!(
        "find_test_at_cursor result: {:?}",
        test_at_cursor1.map(|e| &e.name)
    );

    let has_specific_doctest = commands1.iter().any(|c| {
        c.args.contains(&"--doc".to_string())
            && c.args.contains(&"add".to_string())
            && c.args.contains(&"--show-output".to_string())
    });
    println!("Has specific doctest command: {has_specific_doctest}");

    for cmd in &commands1 {
        if cmd.args.contains(&"--doc".to_string()) {
            println!("Doc command: {} - args: {:?}", cmd.label, cmd.args);
        }
    }

    assert!(
        has_specific_doctest,
        "Should find specific doctest for 'add' when cursor is on function declaration"
    );

    // Test Case 2: Cursor inside function body should find associated doctest
    let cursor_in_add_body = Position { line: 6, column: 4 }; // inside function body
    let context2 = FileDetector::detect_context(&lib_path, Some(cursor_in_add_body)).unwrap();
    let commands2 =
        UniversalCommandGenerator::generate_commands(&context2, Some(cursor_in_add_body)).unwrap();

    println!("\n=== Test Case 2: Cursor inside function body ===");
    let test_at_cursor2 =
        UniversalCommandGenerator::find_test_at_cursor(&context2.entry_points, cursor_in_add_body);
    println!(
        "find_test_at_cursor result: {:?}",
        test_at_cursor2.map(|e| &e.name)
    );

    let has_specific_doctest2 = commands2.iter().any(|c| {
        c.args.contains(&"--doc".to_string())
            && c.args.contains(&"add".to_string())
            && c.args.contains(&"--show-output".to_string())
    });
    println!("Has specific doctest command: {has_specific_doctest2}");

    assert!(
        has_specific_doctest2,
        "Should find specific doctest for 'add' when cursor is inside function body"
    );

    // Test Case 3: Cursor on method should find associated doctest
    let cursor_on_greet = Position {
        line: 35,
        column: 0,
    }; // pub fn greet line
    let context3 = FileDetector::detect_context(&lib_path, Some(cursor_on_greet)).unwrap();
    let commands3 =
        UniversalCommandGenerator::generate_commands(&context3, Some(cursor_on_greet)).unwrap();

    println!("\n=== Test Case 3: Cursor on method declaration ===");
    let test_at_cursor3 =
        UniversalCommandGenerator::find_test_at_cursor(&context3.entry_points, cursor_on_greet);
    println!(
        "find_test_at_cursor result: {:?}",
        test_at_cursor3.map(|e| &e.name)
    );

    let has_greet_doctest = commands3.iter().any(|c| {
        c.args.contains(&"--doc".to_string())
            && (c.args.contains(&"greet".to_string())
                || c.args.contains(&"User::greet".to_string()))
            && c.args.contains(&"--show-output".to_string())
    });
    println!("Has greet doctest command: {has_greet_doctest}");

    for cmd in &commands3 {
        if cmd.args.contains(&"--doc".to_string())
            && cmd.args.contains(&"--show-output".to_string())
        {
            println!("Specific doc command: {} - args: {:?}", cmd.label, cmd.args);
        }
    }

    assert!(
        has_greet_doctest,
        "Should find specific doctest for 'greet' when cursor is on method declaration"
    );

    println!("All function association tests passed!");
}
