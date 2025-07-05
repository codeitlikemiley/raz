//! Example showing basic usage of raz-common utilities

use raz_common::{
    error::Result,
    output::OutputFormatter,
    parse::{get_flag_value, has_flag, parse_option},
    shell::ShellCommand,
    time::{Elapsed, TimeUtils},
};
use std::time::Instant;

fn main() -> Result<()> {
    // Output formatting examples
    println!("{}", OutputFormatter::header("Raz Common Examples"));
    println!("{}", OutputFormatter::info("Starting examples..."));

    // Shell command parsing
    let cmd = ShellCommand::parse("cargo test --features 'foo bar' --release")?;
    println!("{} {}", OutputFormatter::label("Command"), cmd);
    println!("{} {:?}", OutputFormatter::label("Parts"), cmd.parts());

    // Parse options with proper quote handling
    let options = parse_option("--message \"Hello, World!\" --verbose")?;
    println!("{} {:?}", OutputFormatter::label("Parsed options"), options);

    // Command line flag utilities
    let args = vec![
        "--features".to_string(),
        "serde".to_string(),
        "--release".to_string(),
        "--target".to_string(),
        "wasm32".to_string(),
    ];

    println!("\n{}", OutputFormatter::header("Flag Utilities"));
    println!("Has --release flag: {}", has_flag(&args, "--release"));
    println!(
        "--features value: {:?}",
        get_flag_value(&args, "--features")
    );
    println!("--target value: {:?}", get_flag_value(&args, "--target"));

    // Time utilities
    let start = Instant::now();
    std::thread::sleep(std::time::Duration::from_millis(1500));

    println!("\n{}", OutputFormatter::header("Time Formatting"));
    println!("Elapsed: {}", start.elapsed_formatted());
    println!(
        "Current time: {}",
        TimeUtils::format_timestamp(TimeUtils::now_local())
    );

    println!("\n{}", OutputFormatter::success("All examples completed!"));

    Ok(())
}
