# AGENTS.md

Authoritative engineering rules for AI coding agents working in **`axum-svelte-template`**.

---

## 1. Workspace Overview

- **Backend**: Rust (`stable`), Axum `0.8`, SeaORM `1.1.19`, PostgreSQL, Moka `0.12`, optional Cloudflare R2 (`aws-sdk-s3`).
- **Frontend**: SvelteKit (`adapter-static`), Svelte 5 Runes (`$state`, `$derived`, `$effect`, `$props`), Tailwind CSS v4 (`style/input.css`).
- **Crate Graph**: `shared` → `services` → `api` → `server`, plus standalone CLI `xtask`.
  - `crates/shared`: Isomorphic DTOs, `ErrorCode`, `ApiResponse<T>`, `Settings`, `routes.rs`, `Permission`, `obfuscation.rs`.
  - `crates/services`: SeaORM entities (`entities/*`), domain services, Moka `CacheService`, `StorageService`, `NotificationHub`, middlewares.
  - `crates/api`: REST/SSE router (`/api/v1/*`, `/api/notifications/sse`) and `gen-ts` AST TypeScript codegen (`crates/api/src/bin/gen_ts.rs`).
  - `crates/server`: Production Axum binary serving static `svelte-app/build`, `public/`, and SSR-Lite `window.__APP_BOOTSTRAP__`.
  - `xtask`: Non-interactive `clap` automation CLI.

---

## 2. Non-Negotiable Rules

### Strict Ban on Rust Formatters
- **NEVER** run `cargo fmt`, `rustfmt`, or `cargo xtask fmt` (not even with `--check`).
- Format edited Rust code cleanly by hand matching surrounding indentation and style.

### Rust Documentation Standard
- Follow `.agent/rules/02-rust-documentation.md`:
  - `// ===` module header block (`Purpose`, `Key Concepts`, `Architecture Notes`)
  - `// \`crate_name\` - ...` explanation comments above external crate imports
  - `// SECTION:` markers grouping logical sections
  - `///` doc comments on all `pub` items

### AST Codegen Single Source of Truth (`gen-ts`)
- **NEVER** hand-edit `svelte-app/src/lib/api/types.generated.ts` or `svelte-app/src/lib/api/endpoints.generated.ts`.
- Whenever you add, rename, or modify DTOs in `crates/shared/src/domain/` or routes/handlers in `crates/api/`, run:
  ```bash
  cargo xtask gen-ts
  ```
- Keep Rust DTOs idiomatic for `syn`-based AST parsing (`struct`, `enum`, `Vec<T>`, `Option<T>`, `Option<Option<T>>` with `deserialize_double_option`, `DateTime<Utc>`, `NaiveDate`).

### Error Handling & Bilingual i18n
- Domain errors MUST return typed `ErrorCode` variants (`crates/shared/src/domain/error_code.rs`) wrapped in `AppError` / `ApiResponse<T>` via `.to_response()` or `.to_response_created()`.
- Never leak raw database/SQL error strings to HTTP clients.
- Svelte UI translates every `ErrorCode` through `uiPrefs.err(code)` (`svelte-app/src/lib/i18n/{en,vi}.ts`). Keep `en.ts` and `vi.ts` keys strictly aligned with `types.ts`.

### Svelte 5 Runes & Tailwind v4 Rules
- Use Svelte 5 Runes exclusively (`$state`, `$derived`, `$effect`, `$props`); do not introduce Svelte 4 `export let` or legacy writable stores when class-based `.svelte.ts` Rune stores fit.
- Mirror backend route metadata in `svelte-app/src/lib/config/routes.ts` whenever modifying `crates/shared/src/config/routes.rs`.
- Shared Soft UI `@theme` tokens and `.dark` overrides live in `style/input.css` and are injected into `svelte-app/src/app.css` via `vite.config.ts` (`sharedStylesPlugin`).
- Static assets under `public/` resolve at `/logo/*`, `/icons/*`, `/guides/*`, `/js/*` (zero `/pkg/` prefix).

---

## 3. Non-Interactive Verification Commands

```bash
# Rust checks & Clippy (zero warnings allowed)
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --lib

# Regenerate TypeScript API contracts
cargo xtask gen-ts

# Frontend typecheck (from workspace root or svelte-app/)
cd svelte-app && npm run check

# Combined xtask check (Clippy + svelte-check + Rust unit tests)
cargo xtask check

# Database schema + trigger + seed setup (uses APP__DATABASE__URL from .env)
cargo xtask db schema

# Full-stack release build
cargo xtask build
```

---

## 4. Reference Docs & Skills

- Read the relevant guide in `docs/` before modifying a subsystem (`docs/README.md`).
- Workspace rules live in `.agent/rules/*.md`.
- Reusable task workflows live in `.agent/skills/*/SKILL.md` (`add-domain`, `sync-codegen`, `test-fullstack`).
