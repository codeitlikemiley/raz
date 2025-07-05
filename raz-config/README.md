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