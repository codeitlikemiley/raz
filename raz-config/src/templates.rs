use crate::{
    FilterConfig, GlobalConfig, ProviderConfig, UiConfig,
    builder::{CommandConfigBuilder, ConfigBuilder},
};

pub struct ConfigTemplates;

impl ConfigTemplates {
    pub fn default_config() -> GlobalConfig {
        ConfigBuilder::new()
            .providers(vec!["cargo".to_string(), "rustc".to_string()])
            .cache_ttl(3600)
            .parallel_execution(true)
            .max_concurrent_jobs(4)
            .ui_config(UiConfig {
                color: Some(true),
                progress: Some(true),
                verbose: Some(false),
                format: Some("default".to_string()),
            })
            .filter_config(FilterConfig {
                priority_commands: vec!["test".to_string(), "build".to_string()],
                ignore_commands: vec![],
                auto_fix: Some(false),
                show_warnings: Some(true),
            })
            .build()
    }

    pub fn web_development() -> GlobalConfig {
        let mut builder = ConfigBuilder::new()
            .providers(vec![
                "cargo".to_string(),
                "rustc".to_string(),
                "leptos".to_string(),
                "dioxus".to_string(),
            ])
            .cache_ttl(1800)
            .parallel_execution(true)
            .max_concurrent_jobs(8);

        builder = builder
            .command(
                CommandConfigBuilder::new("serve", "cargo")
                    .args(vec!["leptos".to_string(), "serve".to_string()])
                    .description("Start Leptos development server")
                    .build(),
            )
            .command(
                CommandConfigBuilder::new("dx-serve", "dx")
                    .arg("serve")
                    .description("Start Dioxus development server")
                    .build(),
            )
            .command(
                CommandConfigBuilder::new("watch", "cargo")
                    .args(vec![
                        "watch".to_string(),
                        "-x".to_string(),
                        "run".to_string(),
                    ])
                    .description("Watch for changes and auto-rebuild")
                    .build(),
            );

        builder.build()
    }

    pub fn game_development() -> GlobalConfig {
        let mut builder = ConfigBuilder::new()
            .providers(vec![
                "cargo".to_string(),
                "rustc".to_string(),
                "bevy".to_string(),
            ])
            .cache_ttl(3600)
            .parallel_execution(true)
            .max_concurrent_jobs(4);

        let provider_config = ProviderConfig {
            bevy: Some(toml::Value::Table(toml::toml! {
                features = ["dynamic_linking", "bevy_dylib"]
                fast_compile = true
            })),
            ..Default::default()
        };

        builder = builder
            .provider_config(provider_config)
            .command(
                CommandConfigBuilder::new("run-fast", "cargo")
                    .arg("run")
                    .arg("--features")
                    .arg("bevy/dynamic_linking")
                    .env("CARGO_TARGET_DIR", "target-fast")
                    .description("Run with fast compile settings")
                    .build(),
            )
            .command(
                CommandConfigBuilder::new("check-wasm", "cargo")
                    .args(vec![
                        "check".to_string(),
                        "--target".to_string(),
                        "wasm32-unknown-unknown".to_string(),
                    ])
                    .description("Check WASM compatibility")
                    .build(),
            );

        builder.build()
    }

    pub fn library_development() -> GlobalConfig {
        let mut builder = ConfigBuilder::new()
            .providers(vec!["cargo".to_string(), "rustc".to_string()])
            .cache_ttl(7200)
            .parallel_execution(true)
            .max_concurrent_jobs(4);

        builder = builder
            .filter_config(FilterConfig {
                priority_commands: vec![
                    "test".to_string(),
                    "doc".to_string(),
                    "clippy".to_string(),
                ],
                ignore_commands: vec![],
                auto_fix: Some(true),
                show_warnings: Some(true),
            })
            .command(
                CommandConfigBuilder::new("test-all", "cargo")
                    .args(vec![
                        "test".to_string(),
                        "--all-features".to_string(),
                        "--".to_string(),
                        "--nocapture".to_string(),
                    ])
                    .description("Run all tests with all features")
                    .build(),
            )
            .command(
                CommandConfigBuilder::new("doc-open", "cargo")
                    .args(vec![
                        "doc".to_string(),
                        "--no-deps".to_string(),
                        "--open".to_string(),
                    ])
                    .description("Build and open documentation")
                    .build(),
            )
            .command(
                CommandConfigBuilder::new("publish-dry", "cargo")
                    .args(vec![
                        "publish".to_string(),
                        "--dry-run".to_string(),
                        "--allow-dirty".to_string(),
                    ])
                    .description("Test publishing without actually publishing")
                    .build(),
            );

        builder.build()
    }

    pub fn minimal() -> GlobalConfig {
        ConfigBuilder::new()
            .providers(vec!["cargo".to_string()])
            .build()
    }

    pub fn list_templates() -> Vec<(&'static str, &'static str)> {
        vec![
            (
                "default_config",
                "Default configuration with cargo and rustc",
            ),
            ("web", "Web development with Leptos and Dioxus"),
            ("game", "Game development with Bevy"),
            ("library", "Library development with testing focus"),
            ("minimal", "Minimal configuration with only cargo"),
        ]
    }

    pub fn get_template(name: &str) -> Option<GlobalConfig> {
        match name {
            "default" | "default_config" => Some(Self::default_config()),
            "web" => Some(Self::web_development()),
            "game" => Some(Self::game_development()),
            "library" => Some(Self::library_development()),
            "minimal" => Some(Self::minimal()),
            _ => None,
        }
    }
}
