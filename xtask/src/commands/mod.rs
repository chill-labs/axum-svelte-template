// =============================================================================
// XTASK COMMAND MODULES & SHARED PROCESS HELPERS
// =============================================================================
//
// Organizes `xtask` subcommand modules and shared subprocess execution wrappers
// with clear step banners, working-directory isolation, and non-zero exit checks.
//
// ## Key Concepts
//
// - Resolves workspace root reliably from `CARGO_MANIFEST_DIR` so commands work
//   from any subdirectory.
// - Never mutates process-wide env vars (safe in Rust 2024 edition).
//
// ## Architecture Notes
//
// Imported by `xtask/src/main.rs` and all sibling `commands/*.rs` modules.

// SECTION: SUBMODULES

pub mod build;
pub mod codegen;
pub mod db;
pub mod dev;
pub mod test;

// SECTION: IMPORTS

// `anyhow` - Context-rich process and path errors.
use anyhow::{bail, Context, Result};

// `std` - Subprocess execution, paths, and filesystem inspection.
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

// SECTION: WORKSPACE PATHS & SUBPROCESS HELPERS

/// Resolves the repository root directory containing workspace `Cargo.toml`.
pub fn workspace_root() -> Result<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .map(Path::to_path_buf)
        .context("Failed to locate workspace root from xtask CARGO_MANIFEST_DIR")
}

/// Prints a consistent section banner for long-running task steps.
pub fn print_step(label: &str) {
    println!("\n==> [xtask] {label}");
}

/// Runs a subprocess inheriting stdout/stderr and fails on non-zero exit status.
pub fn run_command(program: &str, args: &[&str], cwd: &Path) -> Result<()> {
    println!("$ (cd {} && {} {})", cwd.display(), program, args.join(" "));

    let status = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("Failed to spawn `{program}`"))?;

    if !status.success() {
        bail!(
            "Command `{} {}` exited with status {}",
            program,
            args.join(" "),
            status
        );
    }

    Ok(())
}

/// Executes a Cargo command in the workspace root using `CARGO` env var fallback.
pub fn run_cargo(root: &Path, args: &[&str]) -> Result<()> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    run_command(&cargo, args, root)
}

/// Executes an npm script inside `svelte-app/` if the frontend project exists.
pub fn run_svelte_npm(root: &Path, npm_args: &[&str], require_node_modules: bool) -> Result<()> {
    let svelte_dir = root.join("svelte-app");
    if !svelte_dir.join("package.json").exists() {
        bail!(
            "Frontend directory `svelte-app/package.json` not found at {}",
            svelte_dir.display()
        );
    }

    if require_node_modules && !svelte_dir.join("node_modules").exists() {
        bail!(
            "`svelte-app/node_modules` is missing. Install frontend dependencies first:\n  cd svelte-app && npm ci"
        );
    }

    let npm_bin = if cfg!(windows) { "npm.cmd" } else { "npm" };
    run_command(npm_bin, npm_args, &svelte_dir)
}

/// Resolves the PostgreSQL connection string from CLI flags or environment variables.
pub fn resolve_database_url(cli_override: Option<String>) -> Result<String> {
    if let Some(url) = cli_override {
        let trimmed = url.trim().to_string();
        if !trimmed.is_empty() {
            return Ok(trimmed);
        }
    }

    for key in ["APP__DATABASE__URL", "DATABASE_URL"] {
        if let Ok(val) = std::env::var(key) {
            let trimmed = val.trim().to_string();
            if !trimmed.is_empty() {
                return Ok(trimmed);
            }
        }
    }

    bail!(
        "Database URL is not configured. Set `APP__DATABASE__URL` in `.env` (or pass `--database-url <URL>`)."
    )
}
