use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::commands::{
    analyze_command, bazel_add_command, bazel_clean_command, bazel_init_command,
    bazel_query_command, bazel_sync_command, bazel_test_command, build_sync_command,
    init_command, override_command, run_command, unset_command, watch_command,
};

#[derive(Parser)]
#[command(bin_name = "cargo")]
#[command(version, propagate_version = true)]
pub struct Cargo {
    #[command(subcommand)]
    pub command: CargoCommand,
}

#[derive(Subcommand, Debug)]
pub enum CargoCommand {
    #[command(name = "runner")]
    #[command(about = "Run Rust code at specific locations")]
    Runner(Runner),
}

#[derive(Parser, Debug)]
#[command(name = "cargo-runner")]
#[command(version, about, long_about = None)]
pub struct Runner {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Analyze a Rust file and list all runnable items
    #[command(visible_alias = "a")]
    Analyze {
        /// Path to the Rust file with optional line number (e.g., src/main.rs:10).
        /// Defaults to the cwd entry point (src/main.rs, src/lib.rs, or first .rs found).
        filepath: Option<String>,

        /// Show verbose output with command details
        #[arg(short, long)]
        verbose: bool,

        /// Show current configuration
        #[arg(short, long)]
        config: bool,
    },
    /// Run Rust code at a specific location
    #[command(visible_alias = "r")]
    Run {
        /// Path to the Rust file with optional line number (e.g., src/main.rs:10).
        /// Defaults to the cwd entry point (src/main.rs, src/lib.rs, or first .rs found).
        filepath: Option<String>,

        /// Print the command without executing it
        #[arg(short, long)]
        dry_run: bool,
    },
    /// Initialize cargo-runner configuration
    Init {
        /// Specify the current working directory
        #[arg(short, long)]
        cwd: Option<String>,

        /// Force overwrite existing configuration
        #[arg(short, long)]
        force: bool,

        /// Add rustc configuration for standalone Rust files
        #[arg(long)]
        rustc: bool,

        /// Add single-file script configuration
        #[arg(long)]
        single_file_script: bool,

        /// Generate a Bazel-aware .cargo-runner.json (for Bazel + Rust projects)
        #[arg(long)]
        bazel: bool,

        /// Bazel workspace name to embed in the generated config (default: directory name)
        #[arg(long, value_name = "NAME")]
        workspace_name: Option<String>,
    },
    /// Remove cargo-runner configuration
    Unset {
        /// Clean up all generated configuration files
        #[arg(short, long)]
        clean: bool,
    },
    /// Create override configuration for a specific file location
    #[command(visible_alias = "o")]
    Override {
        /// File path with optional line number (e.g., src/main.rs:10)
        filepath: String,

        /// Create override at project root level
        #[arg(short, long)]
        root: bool,

        /// Override arguments in the format: [-- <OVERRIDE_ARGS>...]
        ///
        /// Examples:
        ///   cargo runner override src/main.rs:10 -- --extra-args --release
        ///   cargo runner override src/main.rs:10 -- --remove-args
        ///   cargo runner override src/main.rs:10 -- --extra-env RUST_LOG=debug
        #[arg(last = true)]
        override_args: Vec<String>,
    },

    // ── Bazel transparent-proxy commands ─────────────────────────────────

    /// Sync Bazel crate-universe after `cargo add` (runs cargo update + bazel sync + gen IDE files)
    ///
    /// Run this after adding any external dependency with `cargo add` or
    /// after editing Cargo.toml directly. Bazel does not auto-detect Cargo
    /// lock changes; this command bridges the gap.
    ///
    /// Examples:
    ///   cargo runner sync              # sync all crates in the workspace
    ///   cargo runner sync --crate server    # sync only the `server` crate
    Sync {
        /// Limit sync to a specific crate (by name or directory basename)
        #[arg(long, value_name = "CRATE")]
        crate_name: Option<String>,

        /// Skip regenerating rust-project.json (faster, useful in CI)
        #[arg(long)]
        skip_ide: bool,
    },

    /// Add an external crate to a Bazel + Rust project (cargo add + bazel sync + IDE refresh)
    ///
    /// Wraps `cargo add` and automatically runs the full Bazel sync pipeline
    /// so the new dependency is immediately available both in code and in the IDE.
    ///
    /// Examples:
    ///   cargo runner add tokio --features full
    ///   cargo runner add serde --features derive --dev
    ///   cargo runner add tokio --crate-dir server --features full
    Add {
        /// Name of the crate to add (e.g. `tokio`)
        crate_name: String,

        /// Comma-separated list of features to enable (e.g. `full,rt-multi-thread`)
        #[arg(long, value_name = "FEATURES")]
        features: Option<String>,

        /// Add as a dev-dependency (`[dev-dependencies]`)
        #[arg(long)]
        dev: bool,

        /// Target crate directory (defaults to nearest ancestor with Cargo.toml + BUILD.bazel)
        #[arg(long, value_name = "DIR")]
        crate_dir: Option<String>,

        /// Skip regenerating rust-project.json
        #[arg(long)]
        skip_ide: bool,
    },

    /// Scaffold or update BUILD.bazel targets based on the crate's src/ layout
    ///
    /// Scans the crate for new files (src/bin/*.rs, tests/*.rs, examples/*.rs,
    /// benches/*.rs) and generates the corresponding Bazel targets. Only touches
    /// lines inside the `# BEGIN raz-managed` / `# END raz-managed` block,
    /// leaving hand-authored stanzas untouched.
    ///
    /// Examples:
    ///   cargo runner build-sync              # sync the current crate
    ///   cargo runner build-sync --crate server    # sync a specific crate
    ///   cargo runner build-sync --dry-run    # preview without writing
    #[command(name = "build-sync")]
    BuildSync {
        /// Limit to a specific crate (by name or directory basename)
        #[arg(long, value_name = "CRATE")]
        crate_name: Option<String>,

        /// Print what would change without writing any files
        #[arg(short, long)]
        dry_run: bool,
    },

    /// Convert a plain `cargo new` project into a Bazel + Rust workspace
    ///
    /// Generates MODULE.bazel, .bazelversion, .bazelrc, BUILD.bazel (root + crate),
    /// Cargo.lock, and .cargo-runner.json, then runs `bazel sync` to pull deps.
    ///
    /// Examples:
    ///   cargo runner bazel-init              # convert current directory
    ///   cargo runner bazel-init --skip-sync  # generate files, skip bazel sync
    ///   cargo runner bazel-init --force      # overwrite existing files
    #[command(name = "bazel-init")]
    BazelInit {
        /// Specify the project directory (defaults to cwd)
        #[arg(short, long)]
        cwd: Option<String>,

        /// Overwrite existing Bazel files
        #[arg(short, long)]
        force: bool,

        /// Skip running `bazel sync` after scaffolding
        #[arg(long)]
        skip_sync: bool,

        /// Override the Bazel workspace name (defaults to directory name)
        #[arg(long, value_name = "NAME")]
        workspace_name: Option<String>,
    },

    /// List Bazel targets in the workspace
    ///
    /// Wraps `bazel query` with a friendlier interface.
    ///
    /// Examples:
    ///   cargo runner bazel-query               # list all targets
    ///   cargo runner bazel-query --tests        # list only rust_test targets
    ///   cargo runner bazel-query --bins         # list only rust_binary targets
    ///   cargo runner bazel-query 'deps(//:foo)' # raw bazel query expression
    #[command(name = "bazel-query")]
    BazelQuery {
        /// Bazel query expression (defaults to `//...`)
        #[arg(value_name = "EXPR")]
        expr: Option<String>,

        /// Output format passed to `--output` (default: label)
        #[arg(long, default_value = "label")]
        output: String,

        /// Show only rust_test targets
        #[arg(long)]
        tests: bool,

        /// Show only rust_binary targets
        #[arg(long)]
        bins: bool,
    },

    /// Clean Bazel build outputs and optionally the shared caches
    ///
    /// Examples:
    ///   cargo runner bazel-clean               # clean build outputs
    ///   cargo runner bazel-clean --expunge     # remove all Bazel state
    ///   cargo runner bazel-clean --disk-cache  # clear shared build cache
    ///   cargo runner bazel-clean --all-caches  # clear disk + repo caches
    #[command(name = "bazel-clean")]
    BazelClean {
        /// Run `bazel clean --expunge` (removes all Bazel state for this workspace)
        #[arg(long)]
        expunge: bool,

        /// Clear the shared disk cache (~/.cache/bazel-disk)
        #[arg(long)]
        disk_cache: bool,

        /// Clear the shared repository cache (~/.cache/bazel-repo)
        #[arg(long)]
        repo_cache: bool,

        /// Clear both disk and repository caches
        #[arg(long)]
        all_caches: bool,
    },

    /// Run `bazel test` for a crate or the whole workspace
    ///
    /// Examples:
    ///   cargo runner test                           # test everything (//...)
    ///   cargo runner test //:my_crate_test          # specific target
    ///   cargo runner test --filter my_fn            # run tests matching a name
    ///   cargo runner test --crate server            # test a named crate
    ///   cargo runner test --streamed                # show all output live
    #[command(name = "test")]
    BazelTest {
        /// Bazel target to test (defaults to //...)
        #[arg(value_name = "TARGET")]
        target: Option<String>,

        /// Filter to a specific test name (passed as --test_arg=--exact <name>)
        #[arg(long, short = 'f', value_name = "NAME")]
        filter: Option<String>,

        /// Limit to a specific crate by name or directory basename
        #[arg(long, value_name = "CRATE")]
        crate_name: Option<String>,

        /// Stream all test output (--test_output=streamed)
        #[arg(long)]
        streamed: bool,
    },

    /// Watch src/ for changes and trigger a Bazel build, test, or run
    ///
    /// Examples:
    ///   cargo runner watch               # watch + bazel build on change
    ///   cargo runner watch --test        # watch + bazel test on change
    ///   cargo runner watch --run         # watch + bazel run on change
    ///   cargo runner watch --target //:my_bin --run
    ///   cargo runner watch --debounce 500
    #[command(name = "watch")]
    Watch {
        /// Bazel target to build/test/run (auto-detected from cwd if omitted)
        #[arg(long, value_name = "TARGET")]
        target: Option<String>,

        /// Run the target instead of building it
        #[arg(long, short = 'r')]
        run: bool,

        /// Test the target instead of building it
        #[arg(long, short = 't')]
        test: bool,

        /// Debounce delay in milliseconds (default: 300)
        #[arg(long, default_value = "300", value_name = "MS")]
        debounce: u64,
    },
}

impl Commands {
    /// Execute the command
    pub fn execute(self) -> Result<()> {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/cargo-runner-execute.log")
        {
            writeln!(f, "DEBUG Commands::execute called with: {:?}", self).ok();
        }

        match self {
            Commands::Analyze {
                filepath,
                verbose,
                config,
            } => {
                let fp = resolve_filepath_arg(filepath)?;
                analyze_command(&fp, verbose, config)
            }
            Commands::Run { filepath, dry_run } => {
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .append(true)
                    .open("/tmp/cargo-runner-execute.log")
                {
                    writeln!(
                        f,
                        "DEBUG calling run_command with filepath={:?}, dry_run={}",
                        filepath, dry_run
                    )
                    .ok();
                }
                let fp = resolve_filepath_arg(filepath)?;
                run_command(&fp, dry_run)
            }
            Commands::Init {
                cwd,
                force,
                rustc,
                single_file_script,
                bazel,
                workspace_name,
            } => init_command(
                cwd.as_deref(),
                force,
                rustc,
                single_file_script,
                bazel,
                workspace_name.as_deref(),
            ),
            Commands::Unset { clean } => unset_command(clean),
            Commands::Override {
                filepath,
                root,
                override_args,
            } => override_command(&filepath, root, override_args),

            // Bazel transparent-proxy commands
            Commands::Sync { crate_name, skip_ide } => {
                bazel_sync_command(crate_name.as_deref(), skip_ide)
            }
            Commands::Add {
                crate_name,
                features,
                dev,
                crate_dir,
                skip_ide,
            } => bazel_add_command(
                &crate_name,
                features.as_deref(),
                dev,
                crate_dir.as_deref(),
                skip_ide,
            ),
            Commands::BuildSync { crate_name, dry_run } => {
                build_sync_command(crate_name.as_deref(), dry_run)
            }
            Commands::BazelInit {
                cwd,
                force,
                skip_sync,
                workspace_name,
            } => bazel_init_command(
                cwd.as_deref(),
                force,
                skip_sync,
                workspace_name.as_deref(),
            ),
            Commands::BazelQuery { expr, output, tests, bins } => {
                bazel_query_command(expr.as_deref(), &output, tests, bins)
            }
            Commands::BazelClean {
                expunge,
                disk_cache,
                repo_cache,
                all_caches,
            } => bazel_clean_command(expunge, disk_cache, repo_cache, all_caches),
            Commands::BazelTest {
                target,
                filter,
                crate_name,
                streamed,
            } => bazel_test_command(
                target.as_deref(),
                filter.as_deref(),
                crate_name.as_deref(),
                streamed,
            ),
            Commands::Watch { target, run, test, debounce } => {
                watch_command(target.as_deref(), run, test, debounce)
            }
        }
    }
}

/// Resolve an optional filepath argument to a concrete path string.
///
/// If `arg` is `None`, auto-detect the best Rust entry point from cwd:
///   1. `src/main.rs`          — standard binary crate
///   2. `src/bin/<first>.rs`   — named binary (no main.rs)
///   3. `src/lib.rs`           — library-only crate (maps to `cargo test`)
///   4. First `*.rs` in `src/` — fallback for non-standard layouts
///   5. First `*.rs` in cwd    — last resort
fn resolve_filepath_arg(arg: Option<String>) -> anyhow::Result<String> {
    if let Some(path) = arg {
        return Ok(path);
    }

    let cwd = std::env::current_dir()?;

    // 1. src/main.rs
    let main_rs = cwd.join("src/main.rs");
    if main_rs.exists() {
        return Ok(main_rs.to_string_lossy().into_owned());
    }

    // 2. src/bin/*.rs — named binaries (sorted for determinism)
    let bin_dir = cwd.join("src/bin");
    if let Ok(entries) = std::fs::read_dir(&bin_dir) {
        let mut bins: Vec<_> = entries
            .flatten()
            .filter(|e| e.path().extension().map_or(false, |x| x == "rs"))
            .collect();
        bins.sort_by_key(|e| e.file_name());
        if let Some(first) = bins.first() {
            return Ok(first.path().to_string_lossy().into_owned());
        }
    }

    // 3. src/lib.rs — library crate; cargo runner maps this to `cargo test`
    let lib_rs = cwd.join("src/lib.rs");
    if lib_rs.exists() {
        return Ok(lib_rs.to_string_lossy().into_owned());
    }

    // 4. Any other .rs in src/
    if let Ok(entries) = std::fs::read_dir(cwd.join("src")) {
        let mut rs_files: Vec<_> = entries
            .flatten()
            .filter(|e| e.path().extension().map_or(false, |x| x == "rs"))
            .collect();
        rs_files.sort_by_key(|e| e.file_name());
        if let Some(first) = rs_files.first() {
            return Ok(first.path().to_string_lossy().into_owned());
        }
    }

    // 5. Any .rs in cwd itself
    if let Ok(entries) = std::fs::read_dir(&cwd) {
        let mut rs_files: Vec<_> = entries
            .flatten()
            .filter(|e| e.path().extension().map_or(false, |x| x == "rs"))
            .collect();
        rs_files.sort_by_key(|e| e.file_name());
        if let Some(first) = rs_files.first() {
            return Ok(first.path().to_string_lossy().into_owned());
        }
    }

    anyhow::bail!(
        "No Rust entry point found in {}.\n\
         Hint: pass a file explicitly, e.g.  cargo runner run src/main.rs",
        cwd.display()
    )
}
