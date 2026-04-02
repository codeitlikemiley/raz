#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::{
        types::{Runnable, RunnableKind, Scope, ScopeKind, FileType, Position},
        config::{Config, BazelConfig, BazelFramework},
        command::CommandType,
    };
    use std::path::PathBuf;
    
    fn create_test_runnable(file_path: &str, kind: RunnableKind, module_path: &str) -> Runnable {
        Runnable {
            label: "test_label".to_string(),
            kind,
            module_path: module_path.to_string(),
            file_path: PathBuf::from(file_path),
            scope: Scope {
                start: Position { line: 1, character: 0 },
                end: Position { line: 10, character: 0 },
                kind: ScopeKind::Function,
                name: Some("test_function".to_string()),
            },
            extended_scope: None,
        }
    }
    
    #[test]
    fn test_benchmark_file_runs_binary() {
        let runnable = create_test_runnable(
            "benches/fibonacci_benchmark.rs",
            RunnableKind::ModuleTests { module_name: "benches::fibonacci_benchmark".to_string() },
            "",
        );
        
        let config = Config::default();
        
        let command = <BazelCommandBuilder as CommandBuilderImpl>::build(&runnable, None, &config, FileType::CargoProject).unwrap();
        
        // Should run the binary, not tests
        assert_eq!(command.command_type, CommandType::Bazel);
        assert_eq!(command.args[0], "run");
        // Should have optimization flag
        assert!(command.args.contains(&"-c".to_string()));
        assert!(command.args.contains(&"opt".to_string()));
    }
    
    #[test]
    fn test_test_command_includes_nocapture() {
        let runnable = create_test_runnable(
            "src/lib.rs",
            RunnableKind::Test {
                test_name: "test_something".to_string(),
                is_async: false,
            },
            "tests",
        );
        
        let config = Config::default();
        
        let command = <BazelCommandBuilder as CommandBuilderImpl>::build(&runnable, None, &config, FileType::CargoProject).unwrap();
        
        // Should include --nocapture in test args
        assert_eq!(command.command_type, CommandType::Bazel);
        assert_eq!(command.args[0], "test");
        
        // Find --test_arg --nocapture sequence
        let mut found_nocapture = false;
        for i in 0..command.args.len() - 1 {
            if command.args[i] == "--test_arg" && command.args[i + 1] == "--nocapture" {
                found_nocapture = true;
                break;
            }
        }
        assert!(found_nocapture, "Should include --test_arg --nocapture");
    }
    
    #[test]
    fn test_module_tests_with_module_name() {
        let runnable = create_test_runnable(
            "src/bin/proxy.rs",
            RunnableKind::ModuleTests { module_name: "tests".to_string() },
            "", // Empty module path to test the module name fallback
        );
        
        let config = Config::default();
        
        let command = <BazelCommandBuilder as CommandBuilderImpl>::build(&runnable, None, &config, FileType::CargoProject).unwrap();
        
        // Should include the module name as filter
        assert_eq!(command.command_type, CommandType::Bazel);
        assert_eq!(command.args[0], "test");
        
        // Find --test_arg tests sequence
        let mut found_module_filter = false;
        for i in 0..command.args.len() - 1 {
            if command.args[i] == "--test_arg" && command.args[i + 1] == "tests" {
                found_module_filter = true;
                break;
            }
        }
        assert!(found_module_filter, "Should include module name 'tests' as filter");
    }
    
    #[test]
    fn test_working_directory_set() {
        // Create a temporary directory structure with MODULE.bazel
        let temp_dir = tempfile::TempDir::new().unwrap();
        let module_file = temp_dir.path().join("MODULE.bazel");
        std::fs::write(&module_file, "module(name = \"test\")").unwrap();
        
        let src_dir = temp_dir.path().join("src");
        std::fs::create_dir(&src_dir).unwrap();
        let file_path = src_dir.join("lib.rs");
        
        let runnable = create_test_runnable(
            file_path.to_str().unwrap(),
            RunnableKind::Test {
                test_name: "test_something".to_string(),
                is_async: false,
            },
            "tests",
        );
        
        let config = Config::default();
        
        let command = <BazelCommandBuilder as CommandBuilderImpl>::build(&runnable, None, &config, FileType::CargoProject).unwrap();
        
        // Should set working directory to workspace root
        assert!(command.working_dir.is_some());
        let working_dir = PathBuf::from(command.working_dir.unwrap());
        assert_eq!(working_dir, temp_dir.path());
    }
    
    #[test]
    fn test_build_script_uses_build_command() {
        let runnable = create_test_runnable(
            "build.rs",
            RunnableKind::Binary { bin_name: None },
            "",
        );
        
        let config = Config::default();
        
        let command = <BazelCommandBuilder as CommandBuilderImpl>::build(&runnable, None, &config, FileType::CargoProject).unwrap();
        
        // Should use 'bazel build' for build.rs files
        assert_eq!(command.command_type, CommandType::Bazel);
        assert_eq!(command.args[0], "build");
    }
    
    #[test]
    fn test_custom_framework_config() {
        let runnable = create_test_runnable(
            "src/lib.rs",
            RunnableKind::Test {
                test_name: "test_custom".to_string(),
                is_async: false,
            },
            "tests",
        );
        
        let mut config = Config::default();
        config.bazel = Some(BazelConfig {
            test_framework: Some(BazelFramework {
                command: Some("bazelisk".to_string()),
                subcommand: Some("test".to_string()),
                args: Some(vec!["--test_output".to_string(), "all".to_string()]),
                test_args: Some(vec!["--verbose".to_string(), "{test_filter}".to_string()]),
                ..Default::default()
            }),
            ..Default::default()
        });
        
        let command = <BazelCommandBuilder as CommandBuilderImpl>::build(&runnable, None, &config, FileType::CargoProject).unwrap();
        
        // Should use custom command
        assert_eq!(command.command_type, CommandType::Shell);
        assert_eq!(command.args[0], "bazelisk");
        assert!(command.args.contains(&"--test_output".to_string()));
        assert!(command.args.contains(&"all".to_string()));
        
        // Should have custom test args
        let mut found_verbose = false;
        for i in 0..command.args.len() - 1 {
            if command.args[i] == "--test_arg" && command.args[i + 1] == "--verbose" {
                found_verbose = true;
                break;
            }
        }
        assert!(found_verbose, "Should include custom --verbose flag");
    }
}