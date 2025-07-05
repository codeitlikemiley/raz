# raz-config

Configuration management for RAZ - handles loading, saving, validation, and inheritance.

## Features

- Hierarchical configuration (Global → Workspace → File)
- Configuration versioning and migration
- Built-in validation
- TOML-based configuration files
- Type-safe configuration schema
- Support for command overrides
- Template system for common setups

## Usage

```rust
use raz_config::{GlobalConfig, WorkspaceConfig, EffectiveConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load global configuration
    let global = GlobalConfig::load()?;
    
    // Load workspace configuration
    let workspace = WorkspaceConfig::load("/path/to/workspace")?;
    
    // Merge configurations to get effective config
    let effective = EffectiveConfig::new(global, workspace);
    
    // Access configuration values
    let providers = &effective.raz.providers;
    
    Ok(())
}
```

## Template System

RAZ provides a template system for initializing projects with common configurations and extending support for new frameworks.

## Current Status

⚠️ **Note**: Custom template files (`.raz/templates/`) are **not yet implemented** in the CLI. This guide documents the planned architecture and how to work with the current system.

## Current Options

### Built-in Templates
The configuration system includes built-in templates for common project types:
- `web` - Web development with Leptos and Dioxus
- `game` - Game development with Bevy
- `library` - Library development focus
- `desktop` - Desktop applications with Tauri and Egui

### Project-Specific Configuration
You can configure RAZ behavior using `.raz/config.toml`:

## Creating Custom Templates

### Template Structure

Custom templates are defined in `.raz/templates/` directory in your workspace:

```
.raz/
├── templates/
│   ├── my-framework.toml
│   ├── custom-test.toml
│   └── microservice.toml
└── overrides.toml
```

### Basic Template Format

```toml
# .raz/templates/my-framework.toml
[template]
name = "my-framework"
description = "Custom framework template"
version = "1.0.0"

[detection]
# Files that indicate this framework
indicators = [
    "my-framework.toml",
    "Cargo.toml:dependencies.my-framework",
    "src/main.rs:use my_framework::"
]

# Command patterns for different file types
[commands.binary]
command = "cargo"
args = ["run", "--bin", "{binary_name}"]
env = { MY_FRAMEWORK_ENV = "development" }

[commands.test]
command = "cargo" 
args = ["test", "--package", "{package_name}", "--", "{test_name}", "--exact"]
working_dir = "{workspace_root}"

[commands.example]
command = "my-framework"
args = ["run", "example", "{example_name}"]

[commands.serve]
command = "my-framework"
args = ["serve", "--port", "3000"]
background = true

# Framework-specific options
[validation.options]
allowed = [
    "--port",
    "--env",
    "--config",
    "--verbose"
]

[validation.env_vars]
allowed = [
    "MY_FRAMEWORK_ENV",
    "MY_FRAMEWORK_CONFIG",
    "PORT"
]
```

### Advanced Template Features

#### Conditional Commands

```toml
# Different commands based on file content or location
[commands.test]
command = "cargo"
args = ["test"]

[commands.test.conditions]
# Use different args for integration tests
"tests/" = ["test", "--test", "{test_name}"]
# Use different args for unit tests  
"src/" = ["test", "--package", "{package_name}", "--", "{test_name}", "--exact"]

[commands.test.file_patterns]
# Custom handling for specific file patterns
"*_integration.rs" = { args = ["test", "--test", "{file_stem}"] }
"*_benchmark.rs" = { command = "cargo", args = ["bench", "--bench", "{file_stem}"] }
```

#### Dynamic Variables

```toml
[variables]
# Define custom variables for use in commands
port = "3000"
config_file = "{workspace_root}/config.toml"
binary_name = "{package_name}-server"

[commands.serve]
command = "my-framework"
args = ["serve", "--port", "{port}", "--config", "{config_file}"]
```

#### Environment Detection

```toml
[environments]
# Different configurations for different environments
[environments.development]
env = { MY_FRAMEWORK_ENV = "dev", RUST_LOG = "debug" }
args_append = ["--reload"]

[environments.production]
env = { MY_FRAMEWORK_ENV = "prod" }
args_append = ["--optimize"]

[environments.testing]
env = { MY_FRAMEWORK_ENV = "test", RUST_BACKTRACE = "1" }
args_append = ["--no-capture"]
```

## Custom Framework Providers

For more complex frameworks, create a custom provider:

### Provider Structure

```rust
// src/providers/my_framework.rs
use raz_core::{
    Provider, FileExecutionContext, ExecutableCommand, 
    FrameworkDetection, Position
};
use std::path::Path;

pub struct MyFrameworkProvider;

impl Provider for MyFrameworkProvider {
    fn name(&self) -> &str {
        "my-framework"
    }

    fn detect(&self, context: &FileExecutionContext) -> FrameworkDetection {
        // Check for framework indicators
        if context.has_dependency("my-framework") || 
           context.workspace_root.join("my-framework.toml").exists() {
            FrameworkDetection::Detected {
                confidence: 0.9,
                version: self.detect_version(context),
            }
        } else {
            FrameworkDetection::NotDetected
        }
    }

    fn generate_commands(
        &self,
        context: &FileExecutionContext,
        cursor: Option<Position>,
    ) -> Result<Vec<ExecutableCommand>, ProviderError> {
        let mut commands = Vec::new();

        match context.file_type {
            FileType::Binary => {
                commands.push(self.create_serve_command(context)?);
                commands.push(self.create_build_command(context)?);
            }
            FileType::Test => {
                commands.push(self.create_test_command(context, cursor)?);
            }
            FileType::Example => {
                commands.push(self.create_example_command(context)?);
            }
            _ => {}
        }

        Ok(commands)
    }
}

impl MyFrameworkProvider {
    fn create_serve_command(&self, context: &FileExecutionContext) -> Result<ExecutableCommand, ProviderError> {
        ExecutableCommand::builder()
            .command("my-framework")
            .args(vec!["serve".to_string()])
            .working_dir(context.workspace_root.clone())
            .env("MY_FRAMEWORK_ENV", "development")
            .label("Serve with My Framework")
            .background(true)
            .build()
    }

    fn create_test_command(
        &self, 
        context: &FileExecutionContext, 
        cursor: Option<Position>
    ) -> Result<ExecutableCommand, ProviderError> {
        let mut args = vec!["test".to_string()];
        
        if let Some(test_name) = context.get_test_at_cursor(cursor) {
            args.extend(vec!["--".to_string(), test_name, "--exact".to_string()]);
        }

        ExecutableCommand::builder()
            .command("cargo")
            .args(args)
            .working_dir(context.workspace_root.clone())
            .label("Run test with My Framework")
            .build()
    }

    fn detect_version(&self, context: &FileExecutionContext) -> Option<String> {
        // Parse Cargo.toml or framework config to detect version
        context.parse_dependency_version("my-framework")
    }
}
```

### Registering Custom Providers

```rust
// In your application or RAZ configuration
use raz_core::ProviderRegistry;
use my_framework::MyFrameworkProvider;

let mut registry = ProviderRegistry::new();
registry.register(Box::new(MyFrameworkProvider));

// Use the registry for command generation
let commands = registry.generate_commands(&context, cursor)?;
```

## Project-Specific Configuration

### Workspace Templates

Create workspace-specific templates in `.raz/config.toml`:

```toml
# .raz/config.toml
[workspace]
name = "my-project"
default_template = "microservice"

[templates.microservice]
inherit = "rust-binary"  # Inherit from built-in template

[templates.microservice.commands.test]
# Override test command for this project
command = "cargo"
args = ["test", "--features", "integration-tests"]
env = { DATABASE_URL = "sqlite::memory:" }

[templates.microservice.commands.serve]
command = "cargo"
args = ["run", "--bin", "server"]
env = { PORT = "8080", LOG_LEVEL = "info" }

# Custom commands specific to this project
[templates.microservice.commands.migrate]
command = "cargo"
args = ["run", "--bin", "migrate"]
label = "Run database migrations"

[templates.microservice.commands.docker-build]
command = "docker"
args = ["build", "-t", "my-project", "."]
label = "Build Docker image"
```

### File-Specific Overrides

```toml
# .raz/config.toml
[file_patterns]
# Special handling for specific files
"src/bin/*.rs" = { template = "binary-with-args" }
"tests/integration/*.rs" = { template = "integration-test" }
"examples/*.rs" = { template = "example-with-features" }

[templates.binary-with-args]
[templates.binary-with-args.commands.run]
command = "cargo"
args = ["run", "--bin", "{file_stem}", "--"]
prompt_for_args = true  # Ask user for arguments

[templates.integration-test]
[templates.integration-test.commands.test]
command = "cargo"
args = ["test", "--test", "{file_stem}", "--features", "integration"]
env = { TEST_DATABASE = "test.db" }
```

## Examples

### Web Framework Template

```toml
# .raz/templates/axum-web.toml
[template]
name = "axum-web"
description = "Axum web framework template"

[detection]
indicators = [
    "Cargo.toml:dependencies.axum",
    "src/main.rs:use axum::"
]

[commands.serve]
command = "cargo"
args = ["run"]
env = { RUST_LOG = "info,my_app=debug" }
background = true
label = "Start Axum server"

[commands.test]
command = "cargo"
args = ["test", "--", "{test_name}", "--exact"]

[commands.watch]
command = "cargo"
args = ["watch", "-x", "run"]
background = true
label = "Watch and restart server"

[validation.options]
allowed = ["--port", "--host", "--env", "--features"]

[validation.env_vars]
allowed = ["PORT", "HOST", "DATABASE_URL", "RUST_LOG"]
```

### Game Development Template

```toml
# .raz/templates/bevy-game.toml
[template]
name = "bevy-game"
description = "Bevy game engine template"

[detection]
indicators = [
    "Cargo.toml:dependencies.bevy",
    "src/main.rs:use bevy::"
]

[commands.run]
command = "cargo"
args = ["run", "--features", "bevy/dynamic_linking"]
env = { RUST_LOG = "info" }
label = "Run game (fast compile)"

[commands.run-release]
command = "cargo"
args = ["run", "--release"]
label = "Run game (optimized)"

[commands.build-web]
command = "cargo"
args = ["build", "--target", "wasm32-unknown-unknown", "--release"]
label = "Build for web"

[commands.serve-web]
command = "basic-http-server"
args = ["--addr", "127.0.0.1:4000", "target/wasm32-unknown-unknown/release/"]
background = true
label = "Serve web build"
```

### CLI Tool Template

```toml
# .raz/templates/cli-tool.toml
[template]
name = "cli-tool"
description = "Command-line tool template"

[detection]
indicators = [
    "Cargo.toml:dependencies.clap",
    "src/main.rs:use clap::"
]

[commands.run]
command = "cargo"
args = ["run", "--"]
prompt_for_args = true
label = "Run CLI tool"

[commands.install]
command = "cargo"
args = ["install", "--path", "."]
label = "Install locally"

[commands.test-cli]
command = "cargo"
args = ["test", "--", "--test-threads", "1"]
label = "Run CLI tests"

[validation.options]
allowed = ["--verbose", "--quiet", "--help", "--version"]
```

## Best Practices

### Template Design

1. **Keep it simple**: Start with basic commands and add complexity gradually
2. **Use descriptive labels**: Make command purposes clear
3. **Provide sensible defaults**: Reduce configuration burden
4. **Document variables**: Comment template variables and their purposes

### Framework Detection

1. **Use multiple indicators**: Don't rely on a single file for detection
2. **Check dependencies**: Parse Cargo.toml for framework dependencies
3. **Validate context**: Ensure detected framework is actually being used
4. **Handle versions**: Account for different framework versions

### Command Generation

1. **Prioritize commands**: Put most common commands first
2. **Handle edge cases**: Account for different project structures
3. **Provide fallbacks**: Have backup commands when primary ones fail
4. **Support customization**: Allow users to override default behavior

### Template Usage

Templates can be accessed programmatically through the `ConfigTemplates` API:

```rust
use raz_config::templates::ConfigTemplates;

// Get a built-in template
let web_config = ConfigTemplates::web_development();
let game_config = ConfigTemplates::game_development();

// List available templates
let templates = ConfigTemplates::list_templates();
```

This system allows RAZ to support any framework or development pattern while maintaining its core philosophy of intelligent, context-aware command generation.