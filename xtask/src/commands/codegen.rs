// =============================================================================
// CODEGEN COMMAND (`cargo xtask gen-ts`)
// =============================================================================
//
// Invokes the `api` crate's AST TypeScript code generator (`gen-ts`) so
// `svelte-app/src/lib/api/{types.generated.ts, endpoints.generated.ts}` stay
// synchronized with Rust DTOs and Axum router definitions.
//
// ## Key Concepts
//
// - Runs `cargo run -p api --bin gen-ts` cleanly from the workspace root.
//
// ## Architecture Notes
//
// Reused automatically at the start of `cargo xtask build`.

// SECTION: IMPORTS

// `anyhow` - Propagates compilation or codegen errors.
use anyhow::Result;

// `std` - Workspace root path reference.
use std::path::Path;

use super::{print_step, run_cargo};

// SECTION: EXECUTION

/// Regenerates TypeScript DTOs and typed API endpoint clients via `api::gen-ts`.
pub fn run_gen_ts(root: &Path) -> Result<()> {
    print_step("Regenerating TypeScript DTO & endpoint contracts (`api::gen-ts`)");
    run_cargo(root, &["run", "-p", "api", "--bin", "gen-ts"])
}
