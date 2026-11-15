// =============================================================================
// CHECK & TEST COMMANDS (`cargo xtask check` & `cargo xtask test`)
// =============================================================================
//
// Runs strict static verification (Clippy `-D warnings`, Svelte `svelte-check`,
// Rust unit tests) and multi-tier test execution (unit tests, PostgreSQL
// integration tests, and optional Playwright E2E suites).
//
// ## Key Concepts
//
// - **Never runs formatters**: Strictly omits `cargo fmt` / `rustfmt`.
// - Detects whether `crates/services/tests` contains integration test files so
//   `cargo xtask test` degrades cleanly when integration tests are skipped or empty.
//
// ## Architecture Notes
//
// Designed for both local verification and headless CI checks.

// SECTION: IMPORTS

// `anyhow` - Propagates lint or test failures.
use anyhow::Result;

// `clap` - CLI flags for `check` and `test`.
use clap::Args;

// `std` - Directory inspection for integration test presence.
use std::path::Path;

// `walkdir` - Scans `crates/` for integration test files.
use walkdir::WalkDir;

use super::{print_step, run_cargo, run_svelte_npm};

// SECTION: CLI ARGUMENTS

/// Arguments for `cargo xtask check`.
#[derive(Debug, Args)]
pub struct CheckArgs {
    /// Skip `svelte-app` `npm run check`.
    #[arg(long)]
    pub skip_frontend: bool,

    /// Skip running workspace Rust unit tests.
    #[arg(long)]
    pub skip_tests: bool,
}

/// Arguments for `cargo xtask test`.
#[derive(Debug, Args)]
pub struct TestArgs {
    /// Run unit tests only (`cargo test --workspace --lib`).
    #[arg(long)]
    pub unit_only: bool,

    /// Include PostgreSQL integration tests in `crates/services/tests` (enabled automatically if present unless `--unit-only`).
    #[arg(long)]
    pub integration: bool,

    /// Include Playwright E2E tests (`cd svelte-app && npm run e2e`).
    #[arg(long)]
    pub e2e: bool,

    /// Run all test tiers (unit + integration + Playwright E2E).
    #[arg(long)]
    pub all: bool,
}

// SECTION: EXECUTION

/// Executes Clippy (`-D warnings`), Svelte typecheck, and Rust unit tests.
pub fn run_check(root: &Path, args: CheckArgs) -> Result<()> {
    print_step("Running Clippy (`cargo clippy --workspace --all-targets -- -D warnings`)");
    run_cargo(
        root,
        &["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"],
    )?;

    if !args.skip_frontend && root.join("svelte-app/package.json").exists() {
        print_step("Running Svelte type & diagnostics check (`svelte-app`: `npm run check`)");
        run_svelte_npm(root, &["run", "check"], true)?;
    }

    if !args.skip_tests {
        print_step("Running Rust unit tests (`cargo test --workspace --lib`)");
        run_cargo(root, &["test", "--workspace", "--lib"])?;
    }

    println!("\n[xtask] All checks passed.");
    Ok(())
}

/// Executes Rust unit tests, PostgreSQL integration tests, and optional Playwright E2E.
pub fn run_test(root: &Path, args: TestArgs) -> Result<()> {
    print_step("Running Rust unit tests (`cargo test --workspace --lib`)");
    run_cargo(root, &["test", "--workspace", "--lib"])?;

    if !args.unit_only {
        let services_tests_dir = root.join("crates/services/tests");
        let has_integration_files = has_rust_files(&services_tests_dir);

        if has_integration_files {
            print_step("Running PostgreSQL integration tests (`crates/services/tests`)");
            run_cargo(
                root,
                &["test", "-p", "services", "--tests", "--", "--test-threads=4"],
            )?;
        } else if args.integration || args.all {
            print_step("Running `services` test targets (`cargo test -p services`)");
            run_cargo(root, &["test", "-p", "services", "--", "--test-threads=4"])?;
        } else {
            println!(
                "\n[xtask] No files in `crates/services/tests/` yet; skipped integration suite."
            );
        }
    }

    if (args.e2e || args.all) && !args.unit_only {
        print_step("Running Playwright E2E tests (`svelte-app`: `npm run e2e`)");
        run_svelte_npm(root, &["run", "e2e"], true)?;
    }

    println!("\n[xtask] Test run finished.");
    Ok(())
}

/// Returns `true` when `dir` exists and contains at least one `.rs` test file.
fn has_rust_files(dir: &Path) -> bool {
    if !dir.exists() {
        return false;
    }

    WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok)
        .any(|entry| {
            entry.file_type().is_file()
                && entry.path().extension().and_then(|e| e.to_str()) == Some("rs")
        })
}
