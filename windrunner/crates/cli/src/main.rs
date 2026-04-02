use anyhow::Result;
use clap::Parser;
use std::env;

use cargo_runner::cli::{Cargo, CargoCommand, Commands, Runner};

fn main() -> Result<()> {
    // Initialize tracing based on RUST_LOG env var
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Parse commands based on how we're invoked
    let commands = parse_commands();

    // Execute the command
    commands.execute()
}

fn parse_commands() -> Commands {
    let args: Vec<String> = env::args().collect();

    // Check if invoked as "cargo runner" (cargo subcommand)
    // When cargo invokes a subcommand, it looks for cargo-<subcommand> binary
    // and passes the subcommand name as the first argument
    if args.get(1).map_or(false, |arg| arg == "runner") {
        let cargo = Cargo::parse();
        let CargoCommand::Runner(runner) = cargo.command;
        runner.command
    } else {
        // Direct invocation as "cargo-runner"
        Runner::parse().command
    }
}
