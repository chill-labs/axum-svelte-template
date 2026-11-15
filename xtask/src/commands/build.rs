// =============================================================================
// FULL-STACK BUILD COMMAND (`cargo xtask build`)
// =============================================================================
//
// Orchestrates production build artifacts:
// 1. Regenerate TypeScript API contracts (`cargo run -p api --bin gen-ts`).
// 2. Build static Svelte 5 bundle (`svelte-app`: `npm run build`).
// 3. Compile optimized Axum HTTP server (`cargo build --release -p server --bin server`).
//
// ## Key Concepts
//
// - Supports selective skips (`--skip-gen-ts`, `--skip-frontend`, `--skip-backend`)
//   for rapid iteration in local or CI pipelines.
//
// ## Architecture Notes
//
// Docker multi-stage builds (`scripts/Dockerfile`) mirror these exact steps
// with `cargo-chef` layer caching.

// SECTION: IMPORTS

// `anyhow` - Propagates build errors cleanly.
use anyhow::Result;

// `clap` - Build command flags.
use clap::Args;

// `std` - Workspace path reference.
use std::path::Path;

use super::{codegen::run_gen_ts, print_step, run_cargo, run_svelte_npm};

// SECTION: CLI ARGUMENTS

/// Arguments for `cargo xtask build`.
#[derive(Debug, Args)]
pub struct BuildArgs {
    /// Skip regenerating TypeScript bindings via `gen-ts`.
    #[arg(long)]
    pub skip_gen_ts: bool,

    /// Skip building the `svelte-app` static bundle (`npm run build`).
    #[arg(long)]
    pub skip_frontend: bool,

    /// Skip compiling the release Rust `server` binary.
    #[arg(long)]
    pub skip_backend: bool,
}

// SECTION: EXECUTION

/// Executes the end-to-end production build sequence.
pub fn run_build(root: &Path, args: BuildArgs) -> Result<()> {
    if !args.skip_gen_ts {
        run_gen_ts(root)?;
    }

    if !args.skip_frontend {
        print_step("Building static Svelte 5 bundle (`svelte-app`: `npm run build`)");
        run_svelte_npm(root, &["run", "build"], true)?;
    }

    if !args.skip_backend {
        print_step("Compiling release Axum server (`cargo build --release -p server --bin server`)");
        run_cargo(root, &["build", "--release", "-p", "server", "--bin", "server"])?;
    }

    println!("\n[xtask] Build completed successfully.");
    Ok(())
}
