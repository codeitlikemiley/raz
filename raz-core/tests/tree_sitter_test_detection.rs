//! Tests for tree-sitter based test detection
//!
//! Verifies that the tree-sitter AST parser correctly identifies:
//! - Test modules with #[cfg(test)]
//! - Individual test functions
//! - Nested module structures
//! - Cursor position within test contexts

#[cfg(feature = "advanced-analysis")]
mod tests {
    use raz_core::tree_sitter_test_detector::TreeSitterTestDetector;
    use raz_core::{EntryPointType, Position};

    #[test]
    fn test_tree_sitter_detects_test_module() -> Result<(), Box<dyn std::error::Error>> {
        let source = r#"
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
    
    #[test]
    fn another_test() {
        assert!(true);
    }
}
"#;

        let mut detector = TreeSitterTestDetector::new()?;
        let entry_points = detector.detect_test_entry_points(source, None)?;

        // Debug: print what we found
        println!("Found {} entry points:", entry_points.len());
        for ep in &entry_points {
            println!("  - {} ({:?}) at line {}", ep.name, ep.entry_type, ep.line);
        }

        // Should find:
        // 1. The test module
        // 2. Two test functions
        assert_eq!(entry_points.len(), 3);

        // Check module
        let module = entry_points
            .iter()
            .find(|ep| ep.entry_type == EntryPointType::TestModule)
            .expect("Should find test module");
        assert_eq!(module.name, "tests");
        assert_eq!(module.full_path, Some("tests".to_string()));

        // Check test functions
        let test1 = entry_points
            .iter()
            .find(|ep| ep.name == "it_works")
            .expect("Should find it_works test");
        assert_eq!(test1.entry_type, EntryPointType::Test);
        assert_eq!(test1.full_path, Some("tests::it_works".to_string()));

        let test2 = entry_points
            .iter()
            .find(|ep| ep.name == "another_test")
            .expect("Should find another_test");
        assert_eq!(test2.entry_type, EntryPointType::Test);
        assert_eq!(test2.full_path, Some("tests::another_test".to_string()));

        Ok(())
    }

    #[test]
    fn test_nested_test_modules() -> Result<(), Box<dyn std::error::Error>> {
        let source = r#"
#[cfg(test)]
mod tests {
    mod unit {
        #[test]
        fn unit_test() {
            assert!(true);
        }
        
        mod deep {
            #[test]
            fn deep_test() {
                assert!(true);
            }
        }
    }
    
    mod integration {
        #[test]
        fn integration_test() {
            assert!(true);
        }
    }
}
"#;

        let mut detector = TreeSitterTestDetector::new()?;
        let entry_points = detector.detect_test_entry_points(source, None)?;

        // Verify nested paths
        let unit_test = entry_points
            .iter()
            .find(|ep| ep.name == "unit_test")
            .expect("Should find unit_test");
        assert_eq!(
            unit_test.full_path,
            Some("tests::unit::unit_test".to_string())
        );

        let deep_test = entry_points
            .iter()
            .find(|ep| ep.name == "deep_test")
            .expect("Should find deep_test");
        assert_eq!(
            deep_test.full_path,
            Some("tests::unit::deep::deep_test".to_string())
        );

        let integration_test = entry_points
            .iter()
            .find(|ep| ep.name == "integration_test")
            .expect("Should find integration_test");
        assert_eq!(
            integration_test.full_path,
            Some("tests::integration::integration_test".to_string())
        );

        Ok(())
    }

    #[test]
    fn test_cursor_in_test_module() -> Result<(), Box<dyn std::error::Error>> {
        let source = r#"
fn main() {
    println!("Hello");
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_something() {
        assert!(true);
    }
    
    // Cursor here, inside module but not on test
    
    #[test]
    fn test_another() {
        assert!(true);
    }
}
"#;

        let mut detector = TreeSitterTestDetector::new()?;

        // Test cursor on main function
        let cursor_on_main = Position { line: 1, column: 5 };
        let context = detector.find_test_context_at_cursor(source, cursor_on_main)?;
        assert!(
            context.is_none(),
            "Cursor on main should not be in test context"
        );

        // Test cursor inside test module
        let cursor_in_module = Position {
            line: 14,
            column: 4,
        }; // Empty line in module
        let context = detector.find_test_context_at_cursor(source, cursor_in_module)?;
        assert!(context.is_some(), "Cursor should be in test context");

        let ctx = context.unwrap();
        assert!(ctx.in_test_module.is_some());
        assert_eq!(ctx.in_test_module.unwrap().name, "tests");
        assert!(ctx.in_test_function.is_none()); // Not on a specific test

        // Test cursor on specific test
        let cursor_on_test = Position {
            line: 10,
            column: 8,
        };
        let context = detector.find_test_context_at_cursor(source, cursor_on_test)?;
        assert!(context.is_some());

        let ctx = context.unwrap();
        assert!(ctx.in_test_function.is_some());
        assert_eq!(ctx.in_test_function.unwrap().name, "test_something");

        Ok(())
    }

    #[test]
    fn test_various_test_attributes() -> Result<(), Box<dyn std::error::Error>> {
        let source = r#"
#[cfg(test)]
mod tests {
    #[test]
    fn regular_test() {
        assert!(true);
    }
    
    #[tokio::test]
    async fn async_test() {
        assert!(true);
    }
    
    #[async_std::test]
    async fn async_std_test() {
        assert!(true);
    }
    
    #[rstest::rstest]
    fn parameterized_test() {
        assert!(true);
    }
}
"#;

        let mut detector = TreeSitterTestDetector::new()?;
        let entry_points = detector.detect_test_entry_points(source, None)?;

        // Should detect all test variations
        assert!(entry_points.iter().any(|ep| ep.name == "regular_test"));
        assert!(entry_points.iter().any(|ep| ep.name == "async_test"));
        assert!(entry_points.iter().any(|ep| ep.name == "async_std_test"));
        // Note: rstest might not be detected without the #[test] attribute

        Ok(())
    }
}

#[cfg(not(feature = "advanced-analysis"))]
mod tests {
    #[test]
    fn test_tree_sitter_not_available() {
        eprintln!("Tree-sitter tests skipped: advanced-analysis feature not enabled");
    }
}
