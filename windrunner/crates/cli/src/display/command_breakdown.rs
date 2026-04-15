use cargo_runner_core::Command;

pub fn print_command_breakdown(command: &Command) {
    use cargo_runner_core::CommandStrategy;

    println!("   🔧 Command breakdown:");

    match command.strategy {
        CommandStrategy::Rustc => {
            println!("      • command: rustc");

            // Parse rustc-specific arguments
            let mut has_test = false;
            let mut has_crate_type = false;
            let mut crate_name = None;
            let mut output_name = None;
            let mut source_file = None;
            let mut extra_args = Vec::new();

            let mut i = 0;
            while i < command.args.len() {
                let arg = &command.args[i];

                if arg == "--test" {
                    has_test = true;
                } else if arg == "--crate-type" && i + 1 < command.args.len() {
                    has_crate_type = true;
                    i += 1; // Skip the value
                } else if arg == "--crate-name" && i + 1 < command.args.len() {
                    crate_name = Some(command.args[i + 1].clone());
                    i += 1;
                } else if arg == "-o" && i + 1 < command.args.len() {
                    output_name = Some(command.args[i + 1].clone());
                    i += 1;
                } else if !arg.starts_with('-') && source_file.is_none() {
                    source_file = Some(arg.clone());
                } else if arg.starts_with('-') {
                    extra_args.push(arg.clone());
                }

                i += 1;
            }

            if has_test {
                println!("      • mode: test");
            } else if has_crate_type {
                println!("      • mode: binary");
            }

            if let Some(name) = crate_name {
                println!("      • crate-name: {name}");
            }

            if let Some(name) = output_name {
                println!("      • output: {name}");
            }

            if let Some(file) = source_file {
                println!("      • source: {file}");
            }

            if !extra_args.is_empty() {
                println!("      • extraArgs: {extra_args:?}");
            }

            if let Some(test_filter) = &command.test_filter {
                println!("      • testFilter: {test_filter}");
            }

            // Check for test binary args in env
            let has_test_extra_args = command
                .env
                .iter()
                .find(|(k, _)| k.as_str() == "_RUSTC_TEST_EXTRA_ARGS");
            if let Some((_, extra_args)) = has_test_extra_args {
                let args: Vec<&str> = extra_args.split_whitespace().collect();
                if !args.is_empty() {
                    println!("      • extraTestBinaryArgs: {args:?}");
                }
            }
        }
        CommandStrategy::Bazel => {
            println!("      • command: bazel");

            // Parse Bazel-specific arguments
            if !command.args.is_empty() {
                let subcommand = &command.args[0];
                println!("      • subcommand: {subcommand}");

                // Show target if present
                if command.args.len() > 1 {
                    println!("      • target: {}", command.args[1]);
                }

                // Show other args
                let extra_args: Vec<_> = command.args.iter().skip(2).collect();
                if !extra_args.is_empty() {
                    println!("      • extraArgs: {extra_args:?}");
                }

                // Check for doc test limitation note
                if let Some((_, msg)) = command
                    .env
                    .iter()
                    .find(|(k, _)| k.as_str() == "_BAZEL_DOC_TEST_LIMITATION")
                {
                    println!("      • ⚠️  Note: {msg}");
                }
            }
        }
        _ => {
            // Original cargo command parsing
            let args = &command.args;
            let (subcommand, package, extra_args, test_binary_args) = parse_cargo_command(args);

            println!("      • command: cargo");

            if let Some(subcmd) = subcommand {
                println!("      • subcommand: {subcmd}");
            }

            if let Some(pkg) = package {
                println!("      • package: {pkg}");
            }

            if !extra_args.is_empty() {
                println!("      • extraArgs: {extra_args:?}");
            }

            if !test_binary_args.is_empty() {
                println!("      • extraTestBinaryArgs: {test_binary_args:?}");
            }
        }
    }

    // Show environment variables (excluding internal ones)
    if !command.env.is_empty() {
        let visible_env: Vec<_> = command
            .env
            .iter()
            .filter(|(k, _)| !k.starts_with('_'))
            .collect();

        if !visible_env.is_empty() {
            println!("      • extraEnv:");
            for (key, value) in visible_env {
                println!("         - {key}={value}");
            }
        }
    }

    println!("   🚀 Final command: {}", command.to_shell_command());
}

pub fn parse_cargo_command(
    args: &[String],
) -> (Option<String>, Option<String>, Vec<String>, Vec<String>) {
    let mut subcommand = None;
    let mut package = None;
    let mut extra_args = Vec::new();
    let mut test_binary_args = Vec::new();

    let mut i = 0;
    let mut after_separator = false;

    while i < args.len() {
        let arg = &args[i];

        if arg == "--" {
            after_separator = true;
            i += 1;
            continue;
        }

        if after_separator {
            test_binary_args.push(arg.clone());
        } else if subcommand.is_none() && !arg.starts_with('-') && !arg.starts_with('+') {
            // Handle commands like "test", "run", etc.
            subcommand = Some(arg.clone());
        } else if arg.starts_with('+') && subcommand.is_none() {
            // Handle toolchain overrides like "+nightly"
            // This is part of cargo invocation, not a subcommand
            extra_args.push(arg.clone());
        } else if arg == "--package" || arg == "-p" {
            if i + 1 < args.len() {
                package = Some(args[i + 1].clone());
                i += 1;
            }
        } else if arg.starts_with("--package=") {
            package = Some(
                arg.strip_prefix("--package=")
                    .expect("starts_with checked")
                    .to_string(),
            );
        } else if arg.starts_with('-') {
            // Skip the value if this is a known flag that takes a value
            if matches!(
                arg.as_str(),
                "--bin" | "--example" | "--test" | "--bench" | "--features"
            ) {
                extra_args.push(arg.clone());
                if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                    i += 1;
                    extra_args.push(args[i].clone());
                }
            } else {
                extra_args.push(arg.clone());
            }
        }

        i += 1;
    }

    (subcommand, package, extra_args, test_binary_args)
}
