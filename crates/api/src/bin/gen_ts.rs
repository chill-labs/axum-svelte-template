// =============================================================================
// BINARY: gen-ts — Regenerate SvelteKit TypeScript API Contracts
// =============================================================================

fn main() {
    let root = api::codegen::workspace_root();
    match api::codegen::write_generated_typescript(&root) {
        Ok(res) => {
            println!(
                "Generated TypeScript contracts in {} ({} bytes types, {} bytes endpoints).",
                root.join("svelte-app/src/lib/api").display(),
                res.types_ts.len(),
                res.endpoints_ts.len()
            );
        }
        Err(err) => {
            eprintln!("Failed generating TypeScript contracts: {err}");
            std::process::exit(1);
        }
    }
}
