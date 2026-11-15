# Documentation Index

Start here before modifying any subsystem in **`axum-svelte-template`**.

| Guide | Topics Covered |
| :--- | :--- |
| [architecture.md](architecture.md) | 5-crate Cargo layout, request lifecycle, Moka caching, SSE `NotificationHub`, SPA bootstrap |
| [auth-and-rbac.md](auth-and-rbac.md) | Google OAuth 2.0, SHA-256 `auth-session`, Test Accounts, Hot Support, 12-permission Workspace RBAC |
| [database-and-migrations.md](database-and-migrations.md) | PostgreSQL schemas (`01..05_*.sql`), auto-provisioning signup trigger, SeaORM entities & `conversions.rs` |
| [codegen-and-api.md](codegen-and-api.md) | `ApiResponse<T>`, `ErrorCode`, `deserialize_double_option`, `gen-ts` AST scanner, typed TS client |
| [frontend-svelte5.md](frontend-svelte5.md) | Svelte 5 Runes, Tailwind v4 Soft UI (`style/input.css`), `vi`/`en` i18n, `/js/page-guide.js` |
| [storage-and-obfuscation.md](storage-and-obfuscation.md) | Optional Cloudflare R2, Canvas WebP compression, byte-for-byte XOR obfuscation, `ProtectedImage` |
| [deployment.md](deployment.md) | `APP__*` settings, `cargo xtask` CLI workflows, 6-stage `scripts/Dockerfile`, healthchecks |
