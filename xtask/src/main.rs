// =============================================================================
// XTASK CLI — NON-INTERACTIVE WORKSPACE ORCHESTRATION
// =============================================================================
//
// Developer automation binary (`cargo xtask <subcommand>`) providing clean,
// zero-prompt workflows for TypeScript codegen, PostgreSQL schema setup,
// type/lint checks, full-stack builds, development orchestration, and testing.
//
// ## Key Concepts
//
// - **Non-Interactive by Design**: Uses `clap` strictly with zero blocking TTY
//   prompts so every command works reliably in CI pipelines and AI agent shells.
// - **Single Source of Truth**: Delegates TypeScript generation to `api::gen-ts`
//   and PostgreSQL schema application to `psql -v ON_ERROR_STOP=1`.
//
// ## Architecture Notes
//
// Registered in root `Cargo.toml` and executed via `cargo xtask <command>`.

// SECTION: SUBMODULES

mod commands;

// SECTION: IMPORTS

// `anyhow` - Ergonomic error propagation for CLI subcommands.
use anyhow::Result;

// `clap` - Declarative argument parser for subcommands and flags.
use clap::{Parser, Subcommand};

use commands::{
    build::{run_build, BuildArgs},
    codegen::run_gen_ts,
    db::{run_db, DbArgs},
    dev::{run_dev, DevArgs},
    test::{run_check, run_test, CheckArgs, TestArgs},
    workspace_root,
};

// SECTION: CLI DEFINITION

/// Top-level `cargo xtask` CLI entry structure.
#[derive(Debug, Parser)]
#[command(
    name = "xtask",
    about = "Non-interactive workspace automation for Axum + Svelte 5 Template",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Supported non-interactive workspace tasks.
#[derive(Debug, Subcommand)]
enum Commands {
    /// Regenerate TypeScript DTOs and endpoint bindings from Rust ASTs (`gen-ts`).
    GenTs,

    /// Apply PostgreSQL schemas, triggers, and starter seed SQL via `psql`.
    Db(DbArgs),

    /// Run `gen-ts`, build `svelte-app` static bundle, and compile release `server`.
    Build(BuildArgs),

    /// Run Clippy (`-D warnings`), `svelte-check`, and Rust unit tests.
    Check(CheckArgs),

    /// Print full-stack dev commands or start backend server / watch / preflight check.
    Dev(DevArgs),

    /// Run unit tests, PostgreSQL integration tests, and optional Playwright E2E.
    Test(TestArgs),
}

// SECTION: MAIN ENTRY POINT

fn main() -> Result<()> {
    // Load local `.env` when present so `APP__DATABASE__URL` and flags resolve seamlessly.
    let _ = dotenvy::dotenv();

    let cli = Cli::parse();
    let root = workspace_root()?;

    match cli.command {
        Commands::GenTs => run_gen_ts(&root),
        Commands::Db(args) => run_db(&root, args),
        Commands::Build(args) => run_build(&root, args),
        Commands::Check(args) => run_check(&root, args),
        Commands::Dev(args) => run_dev(&root, args),
        Commands::Test(args) => run_test(&root, args),
    }
}
