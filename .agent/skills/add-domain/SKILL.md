---
name: add-domain
description: End-to-end runbook for adding a new full-stack domain across PostgreSQL schema, SeaORM entities, shared DTOs/permissions/errors, Axum services/API handlers, gen-ts TypeScript bindings, Svelte 5 UI, and contextual help guides in axum-svelte-template.
---

# Add a Full-Stack Domain (`axum-svelte-template`)

Follow these ordered steps whenever adding a new entity/feature (e.g., `projects`, `tags`, `comments`):

1. **PostgreSQL DDL (`database/schema/`)**:
   - Add the next numbered SQL file (e.g., `database/schema/06_<domain>.sql`) using `CREATE TABLE IF NOT EXISTS` and explicit `CREATE INDEX IF NOT EXISTS`.
   - Add any triggers in `database/triggers/` if records auto-provision on user or workspace creation.
   - Apply locally via `cargo xtask db schema`.

2. **SeaORM Entity & Conversions (`crates/services/src/entities/`)**:
   - Add `<domain>.rs` entity module, register it in `crates/services/src/entities/mod.rs`, and add explicit DTO conversion helpers in `conversions.rs`.

3. **Shared Contracts (`crates/shared/src/domain/`)**:
   - Define response/request DTOs in `crates/shared/src/domain/<domain>.rs` and export from `mod.rs`.
   - Implement `Sanitize` and `validate_input()` on request types; use `deserialize_double_option` for nullable `PATCH` fields.
   - Register new `ErrorCode` variants in `error_code.rs` and new `PermResource` variants (if workspace-scoped) in `permission.rs`.
   - Add frontend routes in `crates/shared/src/config/routes.rs` (`define_routes!`).

4. **Service & Moka Caching (`crates/services/`)**:
   - Implement `<Domain>Service` enforcing workspace RBAC checks, `req.sanitize()` + `req.validate_input()?`, Moka cache invalidation, and `NotificationHub` events if real-time.
   - Register the service in `AppState`.

5. **Axum Handlers (`crates/api/`) & Codegen**:
   - Add thin handlers returning `ApiResponse<T>` via `.to_response()` / `.to_response_created()` and mount routes in `crates/api/src/lib.rs`.
   - Run `cargo xtask gen-ts` so `svelte-app/src/lib/api/{types,endpoints}.generated.ts` update automatically.

6. **Svelte 5 UI (`svelte-app/src/`) & Guides (`public/guides/`)**:
   - Mirror new routes in `svelte-app/src/lib/config/routes.ts`.
   - Add bilingual keys (`en.ts`, `vi.ts`, `types.ts`) including translations for any new `ErrorCode` variants.
   - Build Svelte 5 Runes components/routes consuming `$lib/api/endpoints.generated` and optional contextual guide entries in `public/guides/meta.json`.

7. **Verify**:
   - Run `cargo xtask check` and `cargo xtask test` (never run `cargo fmt` or `rustfmt`).
