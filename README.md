# Axum + Svelte 5 Full-Stack Template

A production-oriented Rust + TypeScript starter pairing **Axum 0.8 + SeaORM 1.1 + PostgreSQL + Moka** with **SvelteKit (`adapter-static`) + Svelte 5 Runes + Tailwind CSS v4**, synchronized through an AST-based Rust-to-TypeScript contract generator (`gen-ts`).

---

## Tech Stack

| Layer | Technology |
| :--- | :--- |
| **Backend HTTP & SSE** | Rust (`stable`), Axum `0.8`, Tower HTTP (`.br`/`.gz` compression, CORS), `tokio::sync::broadcast` SSE |
| **ORM & Database** | PostgreSQL + SeaORM `1.1.19` + SQL schema/triggers/seeds applied via `cargo xtask db schema` |
| **In-Memory Caching** | Moka `0.12` weighted async caches (`CachedList<V>`, session & user hydration caches) |
| **Auth & Security** | Google OAuth 2.0, SHA-256 session tokens (`auth-session`), Argon2id Test Accounts, isolated Hot Support impersonation (`hot-support-session`) |
| **Multi-Tenancy & RBAC** | Workspaces + 12-permission `PermResource × PermAction` JSONB matrix (`Workspace`, `Members`, `Todos` × `View/Create/Edit/Delete`) |
| **Object Storage (Optional)** | Cloudflare R2 / S3 (`aws-sdk-s3`) with client-side Canvas WebP compression + XOR obfuscation (`ProtectedImage`) |
| **Frontend** | Svelte 5 (`$state`, `$derived`, `$effect`, `$props`), SvelteKit `adapter-static` (`200.html` SPA), Tailwind CSS v4 |
| **Type Safety** | `syn`-based Rust AST scanner (`cargo xtask gen-ts` / `cargo gen-ts`) emitting `types.generated.ts` & `endpoints.generated.ts` |
| **Automation & Deploy** | Non-interactive `cargo xtask` CLI + multi-stage `scripts/Dockerfile` (`node:22` + `rust` + `cargo-chef` + `debian:bookworm-slim`) |

---

## Architecture Overview

```text
Browser (Svelte 5 SPA + Soft UI Tailwind v4 + /js/page-guide.js + /js/upload.js)
   │
   ├─► GET /* ──────────────► Axum `server` (`crates/server`)
   │                             ├─ Serves `svelte-app/build/` + injects `window.__APP_BOOTSTRAP__`
   │                             ├─ Serves `public/` (`/logo`, `/icons`, `/guides`, `/js`)
   │                             └─ Streams `/api/notifications/sse` (`NotificationHub`)
   │
   └─► /api/v1/* ───────────► Axum JSON Handlers (`crates/api`)
                                 └─► Domain Services + Moka Cache (`crates/services`)
                                        ├─► PostgreSQL (`database/schema/01..05_*.sql` + triggers)
                                        └─► Optional Cloudflare R2 (`StorageService`)
```

---

## Repository Layout

```text
axum-svelte-template/
├── Cargo.toml                  # 5-crate Rust workspace (`shared`, `services`, `api`, `server`, `xtask`)
├── .env.example                # Hierarchical `APP__*` configuration template
├── AGENTS.md                   # Authoritative AI coding agent instructions
├── style/input.css             # Shared Tailwind CSS v4 Soft UI `@theme` tokens & `.dark` overrides
├── scripts/Dockerfile          # 6-stage Node + cargo-chef + Debian runtime Dockerfile
├── database/
│   ├── schema/01..05_*.sql     # PostgreSQL DDL (`users`, `sessions`, `workspaces`, `todos`, `notifications`, `media_files`)
│   ├── triggers/*.sql          # Auto-provision `Personal Workspace` + 5 starter todos on user insert
│   └── seed/*.sql              # Idempotent SQL seed scripts
├── public/
│   ├── logo/                   # SVG brand logo, favicon, Apple touch icon, webmanifest
│   ├── icons/                  # Generic SVG icons (`google.svg`, `todo.svg`, `workspace.svg`, `upload.svg`)
│   ├── guides/                 # Contextual help (`meta.json` + `*-guide.html`)
│   └── js/                     # `page-guide.js` & `upload.js` (Canvas WebP + XOR obfuscation)
├── crates/
│   ├── shared/                 # DTOs, `ErrorCode`, `ApiResponse<T>`, `Settings`, `routes.rs`, `Permission`, `obfuscation.rs`
│   ├── services/               # SeaORM entities, domain services, Moka `CacheService`, `StorageService`, `NotificationHub`, RBAC
│   ├── api/                    # Axum REST/SSE router + `gen-ts` AST TypeScript codegen binary
│   ├── server/                 # Axum HTTP binary + SSR-Lite SPA shell + `window.__APP_BOOTSTRAP__`
│   └── xtask/                  # Non-interactive developer CLI (`gen-ts`, `db`, `build`, `check`, `dev`, `test`)
├── svelte-app/                 # SvelteKit `adapter-static` Svelte 5 frontend + Playwright E2E
└── docs/                       # Detailed subsystem & architecture guides
```

---

## Quickstart

### Prerequisites
- **Rust** (`stable` toolchain via `rust-toolchain.toml` + `clippy`)
- **Node.js** `22+` and `npm`
- **PostgreSQL** `15+` with `psql` CLI on `PATH`

### 1. Configure Environment
```bash
cp .env.example .env
# Edit APP__DATABASE__URL (and optional APP__GOOGLE__* / APP__R2__* variables)
```

### 2. Apply Database Schema, Triggers & Seed
```bash
cargo xtask db schema
```
Every newly registered user automatically receives a `Personal Workspace` (`owner` membership) and 5 starter Todo tasks via `database/triggers/1_example_workspace_triggers.sql`. On server boot, `APP__ADMIN__EMAIL` is seeded as `role = 'admin'`.

### 3. Generate TypeScript Contracts & Install Frontend Dependencies
```bash
cargo xtask gen-ts
cd svelte-app && npm ci && cd ..
```

### 4. Run in Development (Two Terminals)
```bash
# Terminal A: Backend API & static assets on http://127.0.0.1:3000
cargo xtask dev --serve      # or: cargo xtask dev --watch (requires cargo-watch)

# Terminal B: Svelte 5 + Vite HMR on http://localhost:5173 (proxies /api/* -> :3000)
cd svelte-app && npm run dev
```
Open **`http://localhost:5173`**. Run `cargo xtask dev` with no flags anytime to inspect readiness and print startup commands.

---

## Auth & RBAC Model

- **Google OAuth 2.0**: Primary user sign-in (`/auth/login` -> `/auth/google/callback`) and sensitive-action re-authentication (`purpose="reauth"`).
- **Sessions**: Cryptographically random cookie token stored as a SHA-256 hex digest in PostgreSQL `sessions` and cached in Moka.
- **Test Accounts (`/auth/login-password`)**: Admin-provisioned accounts (`is_test_account = true`, Argon2id `password_hash`) for automated E2E and QA verification without needing live Google OAuth credentials.
- **Hot Support Impersonation**: Admins can inspect a user's workspace via an isolated `hot-support-session` cookie without clobbering their primary admin session.
- **Workspace RBAC**: `PermResource` (`Workspace`, `Members`, `Todos`) × `PermAction` (`View`, `Create`, `Edit`, `Delete`), with presets (`owner`, `editor`, `member`, `viewer`, `custom`).

---

## Verification & Docker Production Build

```bash
# Non-interactive lint, Svelte check, and unit tests
cargo xtask check

# Run unit tests + Postgres integration tests (+ optional --e2e)
cargo xtask test
cargo xtask test --e2e

# Production bundle: gen-ts -> svelte-app/build -> target/release/server
cargo xtask build

# Multi-stage Docker build
docker build -f scripts/Dockerfile -t axum-svelte-template:latest .
```

See [docs/README.md](docs/README.md) for deep-dive guides on every subsystem.
