use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::commands::{
    bazel_add_command, bazel_sync_command, build_sync_command, clean_command, context_command,
    init_command, override_command, run_command, runnables_command, unset_command, watch_command,
};

#[derive(Parser)]
#[command(bin_name = "cargo")]
#[command(version, propagate_version = true)]
pub struct Cargo {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
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
    /// List runnable items in a Rust file or entire workspace.
    #[command(name = "runnables", alias = "analyze", visible_alias = "a")]
    Runnables {
        /// Path to the Rust file with optional line number (e.g., src/main.rs:10).
        /// If omitted, scans the current Cargo workspace members from the cwd upward.
        filepath: Option<String>,

        /// Show only binaries.
        #[arg(long)]
        bin: bool,

        /// Show only tests.
        #[arg(long)]
        test: bool,

        /// Show only benchmarks.
        #[arg(long)]
        bench: bool,

        /// Show only doc tests.
        #[arg(long = "doc", alias = "docs")]
        doc: bool,

        /// Filter by name, module path, or label using a case-insensitive
        /// punctuation-insensitive substring match.
        #[arg(long, value_name = "QUERY")]
        name: Option<String>,

        /// Filter by symbol name, such as a struct, enum, union, or module
        /// doc-test symbol.
        #[arg(long, value_name = "SYMBOL")]
        symbol: Option<String>,

        /// Require an exact normalized name match instead of substring matching.
        #[arg(long)]
        exact: bool,

        /// Show verbose output with command details
        #[arg(short, long)]
        verbose: bool,

        /// Show current configuration
        #[arg(short, long)]
        config: bool,
    },

    /// Emit machine-readable context for a Rust file or the current project.
    Context {
        /// Path to the Rust file with optional line number (e.g., src/main.rs:10).
        filepath: Option<String>,

        /// Print JSON instead of human-readable output.
        #[arg(long)]
        json: bool,
    },

    /// Run Rust code at a specific location
    ///
    /// Intelligently dispatches to `cargo run`, `cargo test`, `cargo bench`,
    /// or the Bazel equivalent based on the file and project configuration.
    /// Defaults to the best entry point in the cwd, honoring Cargo
    /// `default-run` when present.
    #[command(visible_alias = "r")]
    Run {
        /// Path to the Rust file with optional line number (e.g., src/main.rs:10).
        /// Defaults to the cwd entry point, honoring Cargo `default-run`
        /// before falling back to src/main.rs, src/lib.rs, or first .rs found.
        filepath: Option<String>,

        /// Print the command without executing it
        #[arg(short, long)]
        dry_run: bool,
    },

    /// Initialize cargo-runner configuration
    ///
    /// Without flags: scans the project tree and generates .cargo-runner.json files.
    ///
    /// With --bazel:
    ///   If the project is already a Bazel workspace (MODULE.bazel exists),
    ///   updates .cargo-runner.json only.
    ///   If not yet a Bazel workspace, scaffolds the full workspace
    ///   (MODULE.bazel, .bazelversion, .bazelrc, BUILD.bazel, Cargo.lock)
    ///   and writes .cargo-runner.json, then runs `bazel sync`.
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

        /// Generate a Bazel-aware .cargo-runner.json.
        /// Also scaffolds the full Bazel workspace if one does not yet exist.
        #[arg(long)]
        bazel: bool,

        /// Bazel workspace name (defaults to directory name)
        #[arg(long, value_name = "NAME")]
        workspace_name: Option<String>,

        /// Skip `bazel sync` after scaffolding the Bazel workspace
        #[arg(long)]
        skip_sync: bool,
    },

    /// Remove cargo-runner configuration
    Unset {
        /// Clean up all generated configuration files
        #[arg(short, long)]
        clean: bool,
    },

    /// Create override configuration for a specific file location
    ///
    /// Override tokens (passed after `--`):
    /// - `@cmd.sub` — Set command and subcommand (e.g., `@dx.run`)
    /// - `+channel` — Set Rust toolchain channel (e.g., `+nightly`)
    /// - `KEY=value` — Set environment variable (e.g., `RUST_LOG=debug`)
    /// - `/args...` — Test binary args (like `--` in cargo test)
    /// - `-command` — Remove command override
    /// - `-` — Remove the entire override
    ///
    /// Named flags: `--command`, `--subcommand`, `--channel`
    #[command(visible_alias = "o")]
    Override {
        /// File path with optional line number (e.g., src/main.rs:10)
        filepath: String,

        /// Create override at project root level
        #[arg(short, long)]
        root: bool,

        /// Set the command (e.g., dx, cargo, bazel)
        #[arg(long)]
        command: Option<String>,

        /// Set the subcommand (e.g., serve, run, build, watch)
        #[arg(long)]
        subcommand: Option<String>,

        /// Set the Rust toolchain channel (e.g., nightly, stable)
        #[arg(long)]
        channel: Option<String>,

        /// Override tokens and extra arguments (see help above)
        #[arg(last = true)]
        override_args: Vec<String>,
    },

    // ── Bazel transparent-proxy commands ──────────────────────────────────────
    // Low-level Bazel pipeline steps; most users only need init + run.
    /// Sync Bazel crate-universe after `cargo add`
    ///
    /// Run this after adding any external dependency with `cargo add` or after
    /// editing Cargo.toml directly.
    ///
    /// Examples:
    ///   cargo runner sync                       # sync all crates
    ///   cargo runner sync --crate-name server   # sync only the `server` crate
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
    /// Examples:
    ///   cargo runner add tokio --features full
    ///   cargo runner add serde --features derive --dev
    Add {
        /// Name of the crate to add (e.g. `tokio`)
        crate_name: String,

        /// Comma-separated list of features to enable
        #[arg(long, value_name = "FEATURES")]
        features: Option<String>,

        /// Add as a dev-dependency
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
    /// Examples:
    ///   cargo runner build-sync              # sync the current crate
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

    // ── Unified utility commands ───────────────────────────────────────────────
    /// Clean build outputs (auto-detects Cargo or Bazel)
    ///
    /// For Cargo projects runs `cargo clean`.
    /// For Bazel projects runs `bazel clean`, with optional cache clearing.
    ///
    /// Examples:
    ///   cargo runner clean            # clean (auto-detect)
    ///   cargo runner clean --cache    # Bazel: also clear shared disk + repo caches
    ///   cargo runner clean --expunge  # Bazel: bazel clean --expunge
    Clean {
        /// Bazel: run `bazel clean --expunge` (removes all Bazel state for this workspace)
        #[arg(long)]
        expunge: bool,

        /// Bazel: clear shared caches (~/.cache/bazel-disk and ~/.cache/bazel-repo)
        #[arg(long)]
        cache: bool,
    },

    /// Watch src/ for file changes and auto-trigger build/run/test (auto-detects Cargo or Bazel)
    ///
    /// Uses `cargo watch` if installed (Cargo projects), otherwise falls back to
    /// a built-in notify-based watcher. Bazel projects always use the notify watcher.
    ///
    /// Examples:
    ///   cargo runner watch               # watch + build on change
    ///   cargo runner watch --run         # watch + run on change
    ///   cargo runner watch --test        # watch + test on change
    ///   cargo runner watch src/main.rs   # watch the file's directory
    ///   cargo runner watch --debounce 500
    Watch {
        /// Rust file to scope the watch directory to (defaults to src/ or cwd)
        filepath: Option<String>,

        /// Run the target on change instead of just building
        #[arg(long, short = 'r')]
        run: bool,

        /// Test the target on change instead of just building
        #[arg(long, short = 't')]
        test: bool,

        /// Debounce delay in milliseconds before re-triggering (default: 300)
        #[arg(long, default_value = "300", value_name = "MS")]
        debounce: u64,
    },
}

impl Commands {
    /// Execute the command
    pub fn execute(self) -> Result<()> {
        match self {
            Commands::Runnables {
                filepath,
                bin,
                test,
                bench,
                doc,
                name,
                symbol,
                exact,
                verbose,
                config,
            } => {
                let filters = crate::commands::analyze::RunnableFilters {
                    bin,
                    test,
                    bench,
                    doc,
                    name,
                    symbol,
                    exact,
                };
                if let Some(fp) = filepath {
                    runnables_command(Some(&fp), filters, verbose, config)
                } else {
                    runnables_command(None, filters, verbose, config)
                }
            }
            Commands::Context { filepath, json } => context_command(filepath.as_deref(), json),
            Commands::Run { filepath, dry_run } => {
                let fp = crate::utils::path::resolve_filepath_arg(filepath)?;
                run_command(&fp, dry_run)
            }
            Commands::Init {
                cwd,
                force,
                rustc,
                single_file_script,
                bazel,
                workspace_name,
                skip_sync,
            } => init_command(
                cwd.as_deref(),
                force,
                rustc,
                single_file_script,
                bazel,
                workspace_name.as_deref(),
                skip_sync,
            ),
            Commands::Unset { clean } => unset_command(clean),
            Commands::Override {
                filepath,
                root,
                command,
                subcommand,
                channel,
                override_args,
            } => override_command(&filepath, root, command, subcommand, channel, override_args),

            // Bazel transparent-proxy commands
            Commands::Sync {
                crate_name,
                skip_ide,
            } => bazel_sync_command(crate_name.as_deref(), skip_ide),
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
            Commands::BuildSync {
                crate_name,
                dry_run,
            } => build_sync_command(crate_name.as_deref(), dry_run),

            // Unified utilities
            Commands::Clean { expunge, cache } => clean_command(expunge, cache),
            Commands::Watch {
                filepath,
                run,
                test,
                debounce,
            } => watch_command(filepath.as_deref(), run, test, debounce),
        }
    }
}
