use raz_config::{
    CommandConfigBuilder, ConfigBuilder, ConfigValidator, ConfigVersion, EffectiveConfig,
    GlobalConfig, OverrideBuilder, OverrideMode, WorkspaceConfig, templates::ConfigTemplates,
};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_global_config_creation() {
    let config = ConfigBuilder::new()
        .enabled(true)
        .provider("cargo")
        .provider("rustc")
        .cache_ttl(3600)
        .build();

    assert!(config.raz.enabled);
    assert_eq!(config.raz.providers, vec!["cargo", "rustc"]);
    assert_eq!(config.raz.cache_ttl, Some(3600));
}

#[test]
fn test_config_save_and_load() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join(".config/raz/config.toml");

    unsafe {
        std::env::set_var("HOME", temp_dir.path());
    }

    let config = ConfigBuilder::new()
        .enabled(true)
        .providers(vec!["cargo".to_string(), "rustc".to_string()])
        .cache_ttl(7200)
        .build();

    config.save().unwrap();
    assert!(config_path.exists());

    let loaded = GlobalConfig::load().unwrap();
    assert_eq!(loaded.raz.enabled, config.raz.enabled);
    assert_eq!(loaded.raz.providers, config.raz.providers);
    assert_eq!(loaded.raz.cache_ttl, config.raz.cache_ttl);
}

#[test]
fn test_workspace_config() {
    let temp_dir = TempDir::new().unwrap();
    let workspace_path = temp_dir.path();
    let config_path = workspace_path.join(".raz/config.toml");

    fs::create_dir_all(config_path.parent().unwrap()).unwrap();

    let config_content = r#"
[raz]
version = 1
enabled = true
providers = ["cargo", "leptos"]
"#;

    fs::write(&config_path, config_content).unwrap();

    let loaded = WorkspaceConfig::load(workspace_path).unwrap().unwrap();
    assert_eq!(
        loaded.raz.as_ref().unwrap().providers,
        vec!["cargo", "leptos"]
    );
}

#[test]
fn test_effective_config_merge() {
    let global = ConfigBuilder::new()
        .providers(vec!["cargo".to_string()])
        .cache_ttl(3600)
        .build();

    let mut workspace = WorkspaceConfig::new(PathBuf::from("/test"));
    workspace.raz = Some(raz_config::RazConfig {
        version: ConfigVersion::CURRENT,
        enabled: true,
        providers: vec!["cargo".to_string(), "leptos".to_string()],
        cache_dir: None,
        cache_ttl: Some(7200),
        parallel_execution: None,
        max_concurrent_jobs: None,
    });

    let effective = EffectiveConfig::new(global, Some(workspace));

    assert_eq!(effective.raz.providers, vec!["cargo", "leptos"]);
    assert_eq!(effective.raz.cache_ttl, Some(7200));
}

#[test]
fn test_command_config_builder() {
    let command = CommandConfigBuilder::new("test-all", "cargo")
        .arg("test")
        .arg("--all-features")
        .env("RUST_BACKTRACE", "1")
        .description("Run all tests")
        .build();

    assert_eq!(command.name, "test-all");
    assert_eq!(command.command, "cargo");
    assert_eq!(command.args, vec!["test", "--all-features"]);
    assert_eq!(
        command.env.as_ref().unwrap().get("RUST_BACKTRACE"),
        Some(&"1".to_string())
    );
}

#[test]
fn test_override_builder() {
    let override_config = OverrideBuilder::new("test-override")
        .file(PathBuf::from("/src/main.rs"))
        .function("test_function")
        .env("RUST_LOG", "debug")
        .cargo_option("--release")
        .mode(OverrideMode::Append)
        .build();

    assert_eq!(override_config.key, "test-override");
    assert_eq!(
        override_config.file_path,
        Some(PathBuf::from("/src/main.rs"))
    );
    assert_eq!(
        override_config.function_name,
        Some("test_function".to_string())
    );
    assert_eq!(override_config.cargo_options, vec!["--release"]);
    assert_eq!(override_config.mode, OverrideMode::Append);
}

#[test]
fn test_override_matching() {
    let override1 = OverrideBuilder::new("override1")
        .file(PathBuf::from("/src/main.rs"))
        .function("test_fn")
        .build();

    let override2 = OverrideBuilder::new("override2")
        .file(PathBuf::from("/src/main.rs"))
        .build();

    let override3 = OverrideBuilder::new("override3").build();

    let file = PathBuf::from("/src/main.rs");
    assert!(override1.matches_context(Some(&file), Some("test_fn"), None));
    assert!(!override1.matches_context(Some(&file), Some("other_fn"), None));
    assert!(override2.matches_context(Some(&file), None, None));
    assert!(override3.matches_context(None, None, None));
}

#[test]
fn test_config_validation() {
    let validator = ConfigValidator::new(ConfigVersion::CURRENT);

    let valid_config = ConfigBuilder::new()
        .providers(vec!["cargo".to_string()])
        .cache_ttl(3600)
        .build();

    let report = validator.validate_global(&valid_config).unwrap();
    assert!(!report.has_errors());

    let invalid_config = ConfigBuilder::new().providers(vec![]).build();

    let result = validator.validate_global(&invalid_config);
    assert!(result.is_err());
}

#[test]
fn test_config_templates() {
    let web_config = ConfigTemplates::web_development();
    assert!(web_config.raz.providers.contains(&"leptos".to_string()));
    assert!(web_config.raz.providers.contains(&"dioxus".to_string()));

    let game_config = ConfigTemplates::game_development();
    assert!(game_config.raz.providers.contains(&"bevy".to_string()));

    let templates = ConfigTemplates::list_templates();
    assert!(templates.len() >= 5);

    let default = ConfigTemplates::get_template("default_config").unwrap();
    assert!(default.raz.providers.contains(&"cargo".to_string()));
}
