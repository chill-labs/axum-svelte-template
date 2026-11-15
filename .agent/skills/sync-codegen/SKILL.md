---
name: sync-codegen
description: Regenerate and verify Rust-to-TypeScript DTO and endpoint contracts (`types.generated.ts` and `endpoints.generated.ts`) after modifying shared domain types, ErrorCode variants, or Axum API routes.
---

# Synchronize Rust-to-TypeScript Codegen (`gen-ts`)

Whenever you add or modify structs, enums, `ErrorCode` variants, or Axum routes in `crates/shared/` or `crates/api/`:

1. **Never Hand-Edit Generated Files**:
   - `svelte-app/src/lib/api/types.generated.ts`
   - `svelte-app/src/lib/api/endpoints.generated.ts`

2. **Regenerate Bindings**:
   ```bash
   cargo xtask gen-ts
   # Equivalent alias from .cargo/config.toml:
   cargo gen-ts
   ```

3. **Keep Rust Types AST-Friendly**:
   - Put public DTOs in `crates/shared/src/domain/*.rs`.
   - Use standard `serde` derive attributes (`#[serde(rename_all = "snake_case")]`, `#[serde(rename_all = "lowercase")]`, `deserialize_double_option`).
   - Avoid opaque macros inside DTO fields that `syn` cannot inspect directly (except `define_error_codes!` which `gen-ts` explicitly supports).

4. **Verify Frontend Type Compatibility**:
   ```bash
   cd svelte-app && npm run check
   ```
   Or run `cargo xtask check` to verify Clippy, `svelte-check`, and Rust unit tests together.
