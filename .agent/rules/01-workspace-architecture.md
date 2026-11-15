# Workspace Architecture & Cross-Crate Boundaries

1. **Crate Layering (`Cargo.toml`)**:
   - `crates/shared`: Domain DTOs, `ErrorCode`, `ApiResponse<T>`, `Settings`, `routes.rs`, `Permission`, `utils/obfuscation.rs`. Zero DB or HTTP server state.
   - `crates/services`: SeaORM entities (`entities/*`), domain services, Moka `CacheService`, `Option<StorageService>`, `NotificationHub`, auth/RBAC/rate-limit middlewares.
   - `crates/api`: Axum `/api/v1/*` & SSE handlers + `gen-ts` AST codegen binary (`crates/api/src/bin/gen_ts.rs`). Keep handlers thin—delegate business logic and validation (`req.sanitize()`, `req.validate_input()?`) to `services`.
   - `crates/server`: Axum entrypoint serving `svelte-app/build` (`SVELTE_SITE_ROOT`), `public/` (`PUBLIC_ASSETS_ROOT`), and SSR-Lite `window.__APP_BOOTSTRAP__` injection.
   - `xtask`: Non-interactive `clap` CLI (`xtask/src/commands/*`).

2. **Route & Security Synchronization**:
   - Define every route once in `crates/shared/src/config/routes.rs` (`define_routes!`) and mirror it in `svelte-app/src/lib/config/routes.ts`.
   - Enforce workspace RBAC permissions (`PermResource × PermAction`) in `crates/services/src/permission.rs` and verify ownership/membership before any workspace mutation.

3. **Explicit Entity-to-DTO Mapping**:
   - Map SeaORM models using explicit helpers in `crates/services/src/entities/conversions.rs` so optional media keys (`avatar_key`, `logo_key`, `attachment_key`) and hydrated fields are always populated cleanly.
