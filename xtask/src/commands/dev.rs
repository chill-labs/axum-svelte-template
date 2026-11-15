// =============================================================================
// DEVELOPMENT ORCHESTRATION COMMAND (`cargo xtask dev`)
// =============================================================================
//
// Displays full-stack startup guidance and optionally runs the backend server,
// `cargo-watch` hot reload, or fast workspace preflight checks without blocking
// non-interactive invocations by default.
//
// ## Key Concepts
//
// - Default mode (`cargo xtask dev`) prints exact commands and inspects `.env` /
//   `svelte-app/node_modules` readiness immediately and exits 0.
// - `--serve` launches `cargo run -p server --bin server`.
// - `--watch` launches `cargo watch -x "run -p server --bin server"` if installed.
// - `--check` runs fast `cargo check --workspace` + optional `svelte-app` typecheck.
//
// ## Architecture Notes
//
// In local development, Vite serves Svelte 5 on `http://localhost:5173` and
// proxies `/api/*`, `/js/*`, `/guides/*`, `/logo/*`, and `/icons/*` to Axum
// running on `http://127.0.0.1:3000`.

// SECTION: IMPORTS

// `anyhow` - Subprocess errors and actionable setup diagnostics.
use anyhow::{bail, Result};

// `clap` - CLI flags for `dev`.
use clap::Args;

// `std` - Path inspection and `cargo-watch` availability probing.
use std::path::Path;
use std::process::{Command, Stdio};

use super::{print_step, run_cargo, run_command, run_svelte_npm};

// SECTION: CLI ARGUMENTS

/// Arguments for `cargo xtask dev`.
#[derive(Debug, Args)]
pub struct DevArgs {
    /// Run `cargo run -p server --bin server` in the foreground.
    #[arg(long, conflicts_with_all = ["watch", "check"])]
    pub serve: bool,

    /// Run `cargo watch -x "run -p server --bin server"` (requires `cargo-watch`).
    #[arg(long, conflicts_with_all = ["serve", "check"])]
    pub watch: bool,

    /// Run quick `cargo check --workspace` and `svelte-app` `npm run check`.
    #[arg(long, conflicts_with_all = ["serve", "watch"])]
    pub check: bool,
}

// SECTION: EXECUTION

/// Prints dev environment status and optionally launches backend serve/watch/check.
pub fn run_dev(root: &Path, args: DevArgs) -> Result<()> {
    if args.serve {
        print_step("Starting Axum backend server (`cargo run -p server --bin server`)");
        return run_cargo(root, &["run", "-p", "server", "--bin", "server"]);
    }

    if args.watch {
        if !has_cargo_watch() {
            bail!(
                "`cargo-watch` is not installed. Install it via `cargo install cargo-watch` or run `cargo xtask dev --serve`."
            );
        }
        print_step("Starting Axum backend watch (`cargo watch -x \"run -p server --bin server\"`)");
        return run_command(
            "cargo",
            &["watch", "-x", "run -p server --bin server"],
            root,
        );
    }

    if args.check {
        print_step("Running fast workspace typecheck (`cargo check --workspace --all-targets`)");
        run_cargo(root, &["check", "--workspace", "--all-targets"])?;

        if root.join("svelte-app/package.json").exists() {
            print_step("Running Svelte typecheck (`svelte-app`: `npm run check`)");
            run_svelte_npm(root, &["run", "check"], true)?;
        }
        return Ok(());
    }

    print_dev_guide(root);
    Ok(())
}

/// Checks whether `cargo-watch` is installed on the host.
fn has_cargo_watch() -> bool {
    Command::new("cargo")
        .args(["watch", "--version"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// Displays current environment readiness and exact commands for full-stack development.
fn print_dev_guide(root: &Path) {
    let has_env = root.join(".env").exists();
    let has_node_modules = root.join("svelte-app/node_modules").exists();
    let has_svelte = root.join("svelte-app/package.json").exists();
    let has_watch = has_cargo_watch();

    println!("=== Axum + Svelte 5 Template — Development Startup Guide ===");
    println!();
    println!("Readiness:");
    println!(
        "  - .env file:              {}",
        if has_env { "OK" } else { "MISSING -> run: cp .env.example .env" }
    );
    println!(
        "  - svelte-app/:            {}",
        if has_svelte {
            if has_node_modules {
                "OK (node_modules installed)"
            } else {
                "MISSING deps -> run: cd svelte-app && npm ci"
            }
        } else {
            "Not initialized yet"
        }
    );
    println!(
        "  - cargo-watch:            {}",
        if has_watch {
            "Installed (`cargo xtask dev --watch` ready)"
        } else {
            "Optional (`cargo install cargo-watch`)"
        }
    );
    println!();
    println!("1. Apply PostgreSQL schema, triggers, and seed:");
    println!("   cargo xtask db schema");
    println!();
    println!("2. Sync Rust -> TypeScript DTO & endpoint bindings:");
    println!("   cargo xtask gen-ts");
    println!();
    println!("3. Terminal A — Backend API & Static Server (port 3000):");
    if has_watch {
        println!("   cargo xtask dev --watch   # or: cargo xtask dev --serve");
    } else {
        println!("   cargo xtask dev --serve   # or: cargo run -p server --bin server");
    }
    println!();
    println!("4. Terminal B — SvelteKit Vite Dev Server (HMR on port 5173, proxies /api/* -> :3000):");
    println!("   cd svelte-app && npm run dev");
    println!();
    println!("Open http://localhost:5173 in your browser during development (or http://localhost:3000 after `cargo xtask build`).");
}
