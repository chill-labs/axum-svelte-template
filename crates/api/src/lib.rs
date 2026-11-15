// =============================================================================
// API CRATE ROOT
// =============================================================================
//
// Exposes Axum HTTP REST handlers, auth/admin extractors, cookie helpers,
// and the read-only AST TypeScript generator (`gen-ts`).

/// AST TypeScript codegen (`types.generated.ts` and `endpoints.generated.ts`).
pub mod codegen;
/// HTTP Cookie extraction and hardened `Set-Cookie` builders.
pub mod cookies;
/// Request extractors (`AuthUser`, `OptionalAuthUser`, `AdminUser`).
pub mod extractors;
/// Domain REST API routers (`/api/v1/*` and `/auth/google/callback`).
pub mod routes;

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::codegen::{generate_typescript, workspace_root};

    #[test]
    fn generated_ts_is_up_to_date() {
        let root = workspace_root();
        let expected = generate_typescript(&root).expect("TypeScript codegen succeeds");

        let api_dir = root.join("svelte-app/src/lib/api");
        let actual_types = fs::read_to_string(api_dir.join("types.generated.ts"))
            .expect("types.generated.ts must exist; run `cargo run -p api --bin gen-ts`");
        let actual_endpoints = fs::read_to_string(api_dir.join("endpoints.generated.ts"))
            .expect("endpoints.generated.ts must exist; run `cargo run -p api --bin gen-ts`");

        assert_eq!(
            actual_types, expected.types_ts,
            "types.generated.ts is out of date — run `cargo run -p api --bin gen-ts`"
        );
        assert_eq!(
            actual_endpoints, expected.endpoints_ts,
            "endpoints.generated.ts is out of date — run `cargo run -p api --bin gen-ts`"
        );
    }
}
