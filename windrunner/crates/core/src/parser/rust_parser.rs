use crate::{
    error::{Error, Result},
    parser::scope_detector::ScopeDetector,
    types::{ExtendedScope, Position, Scope},
};
use std::path::Path;
use tree_sitter::Parser;

pub struct RustParser {
    parser: Parser,
}

impl RustParser {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .map_err(|e| Error::TreeSitterError(format!("Failed to set language: {e}")))?;
        Ok(Self { parser })
    }

    pub fn parse(&mut self, source: &str) -> Result<tree_sitter::Tree> {
        self.parser
            .parse(source, None)
            .ok_or_else(|| Error::ParseError("Failed to parse source code".to_string()))
    }

    pub fn get_scopes(&mut self, source: &str, file_path: &Path) -> Result<Vec<Scope>> {
        let tree = self.parse(source)?;
        let mut detector = ScopeDetector::new();
        let extended_scopes = detector.detect_scopes(&tree, source, file_path)?;

        // For regular scopes, we want the original positions without doc comments/attributes
        Ok(extended_scopes
            .into_iter()
            .map(|es| {
                let mut scope = es.scope.clone();
                // Use the original start position (without extended range)
                scope.start = es.original_start;
                scope
            })
            .collect())
    }

    pub fn get_extended_scopes(
        &mut self,
        source: &str,
        file_path: &Path,
    ) -> Result<Vec<ExtendedScope>> {
        let tree = self.parse(source)?;
        let mut detector = ScopeDetector::new();
        detector.detect_scopes(&tree, source, file_path)
    }

    pub fn find_doc_tests(&mut self, source: &str) -> Result<Vec<(Position, Position, String)>> {
        let tree = self.parse(source)?;
        let detector = ScopeDetector::new();
        Ok(detector.find_doc_tests(&tree.root_node(), source))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ScopeKind;
    use std::path::PathBuf;

    #[test]
    fn test_parser_creation() {
        let parser = RustParser::new();
        assert!(parser.is_ok());
    }

    #[test]
    fn test_basic_parsing() {
        let mut parser = RustParser::new().unwrap();
        let source = r#"
fn main() {
    println!("Hello, world!");
}
"#;
        let tree = parser.parse(source);
        assert!(tree.is_ok());
    }

    #[test]
    fn test_parse_empty_source() {
        let mut parser = RustParser::new().unwrap();
        let source = "";
        let tree = parser.parse(source);
        assert!(tree.is_ok());
    }

    #[test]
    fn test_parse_invalid_syntax() {
        let mut parser = RustParser::new().unwrap();
        let source = "fn main() { let x = ; }"; // Invalid syntax
        let tree = parser.parse(source);
        // Tree-sitter still parses invalid syntax, creating error nodes
        assert!(tree.is_ok());
    }

    #[test]
    fn test_get_scopes_simple_function() {
        let mut parser = RustParser::new().unwrap();
        let source = r#"
fn hello() {
    println!("Hello");
}
"#;
        let path = PathBuf::from("test.rs");
        let scopes = parser.get_scopes(source, &path).unwrap();

        println!("\n=== test_get_scopes_simple_function ===");
        println!("Source code:\n{}", source);
        println!("Found {} scopes:", scopes.len());
        for (i, scope) in scopes.iter().enumerate() {
            println!(
                "  [{}] {:?} '{}' at lines {}-{}",
                i,
                scope.kind,
                scope.name.as_deref().unwrap_or("<unnamed>"),
                scope.start.line + 1,
                scope.end.line + 1
            );
        }

        // Should have at least 2 scopes: file and function
        assert!(scopes.len() >= 2);

        // Find the function scope
        let func_scope = scopes
            .iter()
            .find(|s| matches!(s.kind, ScopeKind::Function));
        assert!(func_scope.is_some());
        assert_eq!(func_scope.unwrap().name.as_deref(), Some("hello"));
    }

    #[test]
    fn test_get_scopes_multiple_functions() {
        let mut parser = RustParser::new().unwrap();
        let source = r#"
fn first() {}
fn second() {}
fn third() {}
"#;
        let path = PathBuf::from("test.rs");
        let scopes = parser.get_scopes(source, &path).unwrap();

        let function_scopes: Vec<_> = scopes
            .iter()
            .filter(|s| matches!(s.kind, ScopeKind::Function))
            .collect();

        assert_eq!(function_scopes.len(), 3);
    }

    #[test]
    fn test_get_scopes_with_test_function() {
        let mut parser = RustParser::new().unwrap();
        let source = r#"
#[test]
fn test_something() {
    assert_eq!(1, 1);
}
"#;
        let path = PathBuf::from("test.rs");
        let scopes = parser.get_scopes(source, &path).unwrap();

        println!("\n=== test_get_scopes_with_test_function ===");
        println!("Source code:\n{}", source);
        println!("Found {} scopes:", scopes.len());
        for (i, scope) in scopes.iter().enumerate() {
            println!(
                "  [{}] {:?} '{}' at lines {}-{}",
                i,
                scope.kind,
                scope.name.as_deref().unwrap_or("<unnamed>"),
                scope.start.line + 1,
                scope.end.line + 1
            );
        }

        let test_scope = scopes.iter().find(|s| matches!(s.kind, ScopeKind::Test));
        assert!(test_scope.is_some());
        assert_eq!(test_scope.unwrap().name.as_deref(), Some("test_something"));
    }

    #[test]
    fn test_get_scopes_with_modules() {
        let mut parser = RustParser::new().unwrap();
        let source = r#"
mod my_module {
    fn inner_function() {}
}

mod another_module {
    fn another_function() {}
}
"#;
        let path = PathBuf::from("test.rs");
        let scopes = parser.get_scopes(source, &path).unwrap();

        let module_scopes: Vec<_> = scopes
            .iter()
            .filter(|s| matches!(s.kind, ScopeKind::Module))
            .collect();

        assert_eq!(module_scopes.len(), 2);
    }

    #[test]
    fn test_get_scopes_with_structs() {
        let mut parser = RustParser::new().unwrap();
        let source = r#"
struct Point {
    x: f64,
    y: f64,
}

struct Rectangle {
    top_left: Point,
    bottom_right: Point,
}
"#;
        let path = PathBuf::from("test.rs");
        let scopes = parser.get_scopes(source, &path).unwrap();

        let struct_scopes: Vec<_> = scopes
            .iter()
            .filter(|s| matches!(s.kind, ScopeKind::Struct))
            .collect();

        assert_eq!(struct_scopes.len(), 2);
    }

    #[test]
    fn test_get_scopes_with_enums() {
        let mut parser = RustParser::new().unwrap();
        let source = r#"
enum Color {
    Red,
    Green,
    Blue,
}

enum Option<T> {
    Some(T),
    None,
}
"#;
        let path = PathBuf::from("test.rs");
        let scopes = parser.get_scopes(source, &path).unwrap();

        let enum_scopes: Vec<_> = scopes
            .iter()
            .filter(|s| matches!(s.kind, ScopeKind::Enum))
            .collect();

        assert_eq!(enum_scopes.len(), 2);
    }

    #[test]
    fn test_get_scopes_with_impl_blocks() {
        let mut parser = RustParser::new().unwrap();
        let source = r#"
struct MyStruct;

impl MyStruct {
    fn new() -> Self {
        MyStruct
    }
    
    fn method(&self) {}
}

impl Display for MyStruct {
    fn fmt(&self, f: &mut Formatter) -> Result {
        Ok(())
    }
}
"#;
        let path = PathBuf::from("test.rs");
        let scopes = parser.get_scopes(source, &path).unwrap();

        let impl_scopes: Vec<_> = scopes
            .iter()
            .filter(|s| matches!(s.kind, ScopeKind::Impl))
            .collect();

        assert_eq!(impl_scopes.len(), 2);
    }

    #[test]
    fn test_get_scopes_nested_structures() {
        let mut parser = RustParser::new().unwrap();
        let source = r#"
mod outer {
    struct OuterStruct;
    
    mod inner {
        fn inner_function() {
            let closure = || {
                println!("Inside closure");
            };
        }
    }
    
    impl OuterStruct {
        fn method() {}
    }
}
"#;
        let path = PathBuf::from("test.rs");
        let scopes = parser.get_scopes(source, &path).unwrap();

        // Should have multiple nested scopes
        assert!(scopes.len() > 5);

        // Check for specific scope types
        assert!(scopes.iter().any(|s| matches!(s.kind, ScopeKind::Module)));
        assert!(scopes.iter().any(|s| matches!(s.kind, ScopeKind::Struct)));
        assert!(scopes.iter().any(|s| matches!(s.kind, ScopeKind::Impl)));
        assert!(scopes.iter().any(|s| matches!(s.kind, ScopeKind::Function)));
    }

    #[test]
    fn test_get_extended_scopes() {
        let mut parser = RustParser::new().unwrap();
        let source = r#"
/// This is a doc comment
/// with multiple lines
#[derive(Debug)]
fn documented_function() {}

#[test]
#[ignore]
fn test_with_attributes() {}
"#;
        let path = PathBuf::from("test.rs");
        let extended_scopes = parser.get_extended_scopes(source, &path).unwrap();

        println!("\n=== test_get_extended_scopes ===");
        println!("Source code:\n{}", source);
        println!("Found {} extended scopes:", extended_scopes.len());
        for (i, es) in extended_scopes.iter().enumerate() {
            println!(
                "  [{}] {:?} '{}' at lines {}-{}",
                i,
                es.scope.kind,
                es.scope.name.as_deref().unwrap_or("<unnamed>"),
                es.scope.start.line + 1,
                es.scope.end.line + 1
            );
            println!("      - Doc comment lines: {}", es.doc_comment_lines);
            println!("      - Attribute lines: {}", es.attribute_lines);
            println!("      - Has doc tests: {}", es.has_doc_tests);
            println!(
                "      - Original start: line {}",
                es.original_start.line + 1
            );
        }

        // Extended scopes should include doc comments and attributes
        assert!(extended_scopes.len() >= 3); // file + 2 functions

        // Find documented function
        let doc_func = extended_scopes
            .iter()
            .find(|s| s.scope.name.as_deref() == Some("documented_function"));
        assert!(doc_func.is_some());

        // Check if it has doc comment lines
        let doc_func = doc_func.unwrap();
        assert!(doc_func.doc_comment_lines > 0);
    }

    #[test]
    fn test_extended_scope_includes_attributes() {
        let mut parser = RustParser::new().unwrap();
        let source = r#"
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct MyStruct {
    field: i32,
}

/// A documented function
/// 
/// With multiple lines of documentation
#[inline]
#[must_use]
pub fn important_function() -> i32 {
    42
}
"#;
        let path = PathBuf::from("test.rs");
        let runnables = parser.get_extended_scopes(source, &path).unwrap();

        // Find the struct
        let struct_scope = runnables
            .iter()
            .find(|s| s.scope.name.as_deref() == Some("MyStruct"))
            .expect("Should find MyStruct");

        // The extended scope should start before the #[derive] attribute
        assert!(
            struct_scope.attribute_lines > 0,
            "Struct should have attribute lines"
        );

        // Find the function
        let func_scope = runnables
            .iter()
            .find(|s| s.scope.name.as_deref() == Some("important_function"))
            .expect("Should find important_function");

        assert!(
            func_scope.doc_comment_lines > 0,
            "Function should have doc comment lines"
        );
        assert!(
            func_scope.attribute_lines > 0,
            "Function should have attribute lines"
        );
    }

    #[test]
    fn test_extended_scope_ranges() {
        let mut parser = RustParser::new().unwrap();
        let source = r#"/// Doc comment for test
#[test]
fn my_test() {
    assert!(true);
}"#;
        let path = PathBuf::from("test.rs");

        let extended_scopes = parser.get_extended_scopes(source, &path).unwrap();
        let regular_scopes = parser.get_scopes(source, &path).unwrap();

        println!("\n=== test_extended_scope_ranges ===");
        println!("Source code:\n{}", source);
        println!("\nExtended scopes:");
        for (i, es) in extended_scopes.iter().enumerate() {
            println!(
                "  [{}] {:?} '{}' at lines {}-{} (doc: {}, attr: {})",
                i,
                es.scope.kind,
                es.scope.name.as_deref().unwrap_or("<unnamed>"),
                es.scope.start.line + 1,
                es.scope.end.line + 1,
                es.doc_comment_lines,
                es.attribute_lines
            );
        }
        println!("\nRegular scopes:");
        for (i, s) in regular_scopes.iter().enumerate() {
            println!(
                "  [{}] {:?} '{}' at lines {}-{}",
                i,
                s.kind,
                s.name.as_deref().unwrap_or("<unnamed>"),
                s.start.line + 1,
                s.end.line + 1
            );
        }

        // Find test function in both
        let extended_test = extended_scopes
            .iter()
            .find(|s| s.scope.name.as_deref() == Some("my_test"))
            .expect("Should find extended test scope");

        let regular_test = regular_scopes
            .iter()
            .find(|s| s.name.as_deref() == Some("my_test"))
            .expect("Should find regular test scope");

        // Extended scope should start earlier (includes doc comment and attribute)
        assert!(
            extended_test.scope.start.line < regular_test.start.line,
            "Extended scope should start before regular scope. Extended: line {}, Regular: line {}",
            extended_test.scope.start.line,
            regular_test.start.line
        );

        // Extended scope should have doc comment lines
        assert_eq!(
            extended_test.doc_comment_lines, 1,
            "Should have 1 doc comment line"
        );
        assert_eq!(
            extended_test.attribute_lines, 1,
            "Should have 1 attribute line"
        );
    }

    #[test]
    fn test_cfg_test_module_extended_scope() {
        let mut parser = RustParser::new().unwrap();
        let source = r#"
fn regular_function() {}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_something() {
        assert!(true);
    }
}
"#;
        let path = PathBuf::from("test.rs");
        let extended_scopes = parser.get_extended_scopes(source, &path).unwrap();

        // Find the tests module
        let test_module = extended_scopes
            .iter()
            .find(|s| s.scope.name.as_deref() == Some("tests"))
            .expect("Should find tests module");

        // Module with #[cfg(test)] should have attribute lines
        assert!(
            test_module.attribute_lines > 0,
            "Test module should include #[cfg(test)] attribute"
        );

        // The extended scope should start at the #[cfg(test)] line
        let regular_scopes = parser.get_scopes(source, &path).unwrap();
        let regular_module = regular_scopes
            .iter()
            .find(|s| s.name.as_deref() == Some("tests"))
            .expect("Should find regular tests module");

        assert!(
            test_module.scope.start.line < regular_module.start.line,
            "Extended module scope should include the #[cfg(test)] attribute"
        );
    }

    #[test]
    fn test_get_scopes_benchmark_function() {
        let mut parser = RustParser::new().unwrap();
        let source = r#"
#[bench]
fn bench_something(b: &mut Bencher) {
    b.iter(|| {
        // benchmark code
    });
}
"#;
        let path = PathBuf::from("test.rs");
        let scopes = parser.get_scopes(source, &path).unwrap();

        let bench_scope = scopes
            .iter()
            .find(|s| matches!(s.kind, ScopeKind::Benchmark));
        assert!(bench_scope.is_some());
        assert_eq!(
            bench_scope.unwrap().name.as_deref(),
            Some("bench_something")
        );
    }

    #[test]
    fn test_get_scopes_with_unions() {
        let mut parser = RustParser::new().unwrap();
        let source = r#"
union MyUnion {
    i: i32,
    f: f32,
}
"#;
        let path = PathBuf::from("test.rs");
        let scopes = parser.get_scopes(source, &path).unwrap();

        let union_scope = scopes.iter().find(|s| matches!(s.kind, ScopeKind::Union));
        assert!(union_scope.is_some());
        assert_eq!(union_scope.unwrap().name.as_deref(), Some("MyUnion"));
    }

    #[test]
    fn test_scope_positions() {
        let mut parser = RustParser::new().unwrap();
        let source = r#"fn test() {
    println!("test");
}"#;
        let path = PathBuf::from("test.rs");
        let scopes = parser.get_scopes(source, &path).unwrap();

        let func_scope = scopes
            .iter()
            .find(|s| matches!(s.kind, ScopeKind::Function))
            .unwrap();

        // Check that positions are reasonable
        assert_eq!(func_scope.start.line, 0);
        assert_eq!(func_scope.end.line, 2);
        assert!(
            func_scope.start.character < func_scope.end.character
                || func_scope.start.line < func_scope.end.line
        );
    }

    #[test]
    fn test_get_scopes_async_functions() {
        let mut parser = RustParser::new().unwrap();
        let source = r#"
async fn async_function() {
    tokio::time::sleep(Duration::from_secs(1)).await;
}

pub async fn public_async() {}
"#;
        let path = PathBuf::from("test.rs");
        let scopes = parser.get_scopes(source, &path).unwrap();

        let async_funcs: Vec<_> = scopes
            .iter()
            .filter(|s| matches!(s.kind, ScopeKind::Function))
            .collect();

        assert_eq!(async_funcs.len(), 2);
    }

    #[test]
    fn test_get_scopes_const_and_static() {
        let mut parser = RustParser::new().unwrap();
        let source = r#"
const MAX_SIZE: usize = 100;
static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn regular_function() {}
"#;
        let path = PathBuf::from("test.rs");
        let scopes = parser.get_scopes(source, &path).unwrap();

        // Should still parse functions correctly even with const/static items
        let func_scope = scopes
            .iter()
            .find(|s| matches!(s.kind, ScopeKind::Function));
        assert!(func_scope.is_some());
    }

    #[test]
    fn test_get_scopes_generic_functions() {
        let mut parser = RustParser::new().unwrap();
        let source = r#"
fn generic_function<T: Display>(item: T) {
    println!("{}", item);
}

fn multiple_generics<T, U, V>(t: T, u: U, v: V) 
where
    T: Clone,
    U: Debug,
    V: Default,
{
}
"#;
        let path = PathBuf::from("test.rs");
        let scopes = parser.get_scopes(source, &path).unwrap();

        let funcs: Vec<_> = scopes
            .iter()
            .filter(|s| matches!(s.kind, ScopeKind::Function))
            .collect();

        assert_eq!(funcs.len(), 2);
        assert!(
            funcs
                .iter()
                .any(|f| f.name.as_deref() == Some("generic_function"))
        );
        assert!(
            funcs
                .iter()
                .any(|f| f.name.as_deref() == Some("multiple_generics"))
        );
    }

    #[test]
    fn test_get_scopes_macro_definitions() {
        let mut parser = RustParser::new().unwrap();
        let source = r#"
macro_rules! my_macro {
    () => {
        println!("Hello from macro!");
    };
}

fn normal_function() {
    my_macro!();
}
"#;
        let path = PathBuf::from("test.rs");
        let scopes = parser.get_scopes(source, &path).unwrap();

        // Should still find the normal function
        let func_scope = scopes
            .iter()
            .find(|s| s.name.as_deref() == Some("normal_function"));
        assert!(func_scope.is_some());
    }

    #[test]
    fn test_unicode_in_source() {
        let mut parser = RustParser::new().unwrap();
        let source = r#"
fn 你好() {
    let emoji = "😀";
    println!("Unicode: {}", emoji);
}

fn café() {
    // French café
}
"#;
        let path = PathBuf::from("test.rs");
        let scopes = parser.get_scopes(source, &path).unwrap();

        let funcs: Vec<_> = scopes
            .iter()
            .filter(|s| matches!(s.kind, ScopeKind::Function))
            .collect();

        assert_eq!(funcs.len(), 2);
    }

    #[test]
    fn test_complex_real_world_code() {
        let mut parser = RustParser::new().unwrap();
        let source = r#"
use std::collections::HashMap;

/// A cache implementation
pub struct Cache<K, V> {
    map: HashMap<K, V>,
    capacity: usize,
}

impl<K: Eq + std::hash::Hash, V> Cache<K, V> {
    pub fn new(capacity: usize) -> Self {
        Self {
            map: HashMap::with_capacity(capacity),
            capacity,
        }
    }
    
    pub fn get(&self, key: &K) -> Option<&V> {
        self.map.get(key)
    }
    
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        if self.map.len() >= self.capacity {
            // Remove oldest entry (simplified)
            if let Some(first_key) = self.map.keys().next().cloned() {
                self.map.remove(&first_key);
            }
        }
        self.map.insert(key, value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cache_new() {
        let cache: Cache<String, i32> = Cache::new(10);
        assert_eq!(cache.capacity, 10);
    }
    
    #[test]
    fn test_cache_insert_and_get() {
        let mut cache = Cache::new(5);
        cache.insert("key1", 100);
        assert_eq!(cache.get(&"key1"), Some(&100));
    }
}
"#;
        let path = PathBuf::from("cache.rs");
        let scopes = parser.get_scopes(source, &path).unwrap();

        println!("\n=== test_complex_real_world_code ===");
        println!("Source has {} lines", source.lines().count());
        println!("Found {} scopes:", scopes.len());
        println!("\nScopes hierarchy:");
        for (i, scope) in scopes.iter().enumerate() {
            let indent = match scope.kind {
                ScopeKind::File(_) => "",
                ScopeKind::Module => "  ",
                ScopeKind::Struct | ScopeKind::Impl => "  ",
                ScopeKind::Function | ScopeKind::Test => "    ",
                _ => "  ",
            };
            println!(
                "{}{:2}. {:?} '{}' at lines {}-{}",
                indent,
                i,
                scope.kind,
                scope.name.as_deref().unwrap_or("<unnamed>"),
                scope.start.line + 1,
                scope.end.line + 1
            );
        }

        // Verify we found all the important scopes
        assert!(scopes.iter().any(|s| s.name.as_deref() == Some("Cache")));
        assert!(scopes.iter().any(|s| matches!(s.kind, ScopeKind::Impl)));
        assert!(scopes.iter().any(|s| s.name.as_deref() == Some("tests")));
        assert!(
            scopes
                .iter()
                .any(|s| s.name.as_deref() == Some("test_cache_new"))
        );
        assert!(
            scopes
                .iter()
                .any(|s| s.name.as_deref() == Some("test_cache_insert_and_get"))
        );
    }
}
