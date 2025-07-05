//! Example showing error handling with context

use raz_common::{
    env::EnvParser,
    error::{ErrorContext, OptionContext, Result},
    parse::parse_option,
};
use std::fs;

fn read_config(path: &str) -> Result<String> {
    // Add context to filesystem errors
    fs::read_to_string(path).context("Failed to read configuration file")
}

fn parse_config_value(config: &str, key: &str) -> Result<String> {
    // Find a line with the key
    config
        .lines()
        .find(|line| line.starts_with(key))
        .context(format!("Key '{key}' not found in config"))?
        .split_once('=')
        .map(|(_, value)| value.trim().to_string())
        .context(format!("Invalid format for key '{key}'"))
}

fn process_command_with_env(cmd: &str) -> Result<()> {
    // Parse environment variables and command options
    let (env_vars, args) =
        EnvParser::extract_env_vars(&parse_option(cmd).context("Failed to parse command")?);

    println!("Environment variables:");
    for (k, v) in &env_vars {
        println!("  {k} = {v}");
    }

    println!("Arguments:");
    for arg in &args {
        println!("  {arg}");
    }

    Ok(())
}

fn main() {
    // Example 1: File reading with context
    match read_config("config.toml") {
        Ok(content) => println!("Config loaded: {} bytes", content.len()),
        Err(e) => eprintln!("Error: {e}"),
    }

    // Example 2: Option handling with context
    let config = "key1=value1\nkey2=value2";
    match parse_config_value(config, "key3") {
        Ok(value) => println!("Found value: {value}"),
        Err(e) => eprintln!("Error: {e}"),
    }

    // Example 3: Command parsing with environment variables
    let command = "FOO=bar BAZ=qux cargo test --features serde";
    match process_command_with_env(command) {
        Ok(()) => println!("Command processed successfully"),
        Err(e) => eprintln!("Error: {e}"),
    }

    // Example 4: Error context with closure
    let result: Result<i32> = Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied))
        .with_context(|| {
            format!(
                "Permission denied while accessing resource at {}",
                "some/path"
            )
        });

    if let Err(e) = result {
        eprintln!("Error with context: {e}");
    }
}
