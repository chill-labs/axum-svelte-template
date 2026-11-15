# Deployment, Configuration & `cargo xtask` CLI

## 1. Environment Configuration (`crates/shared/src/config/settings.rs`)

Settings load from `.env` (if present) layered with `APP__*` environment variables (`__` separator):

| Variable | Default / Example | Notes |
| :--- | :--- | :--- |
| `APP__ENV` | `dev` (`dev` \| `staging` \| `prod`) | Enables `Secure` cookies and requires `APP__JWT__SECRET >= 32` chars outside `dev` |
| `APP__SERVER__HOST` / `PORT` | `127.0.0.1` / `3000` | Use `0.0.0.0` inside Docker containers |
| `APP__SERVER__DOMAIN` | `http://localhost:3000` | Canonical origin for OAuth redirects and OpenGraph tags |
| `SVELTE_SITE_ROOT` | `svelte-app/build` | Path to built SvelteKit static bundle |
| `PUBLIC_ASSETS_ROOT` | `public` | Path to `/logo`, `/icons`, `/guides`, `/js`, `robots.txt`, `sitemap.xml` |
| `APP__DATABASE__URL` | `postgres://...` | PostgreSQL connection string (`MAX_CONNECTIONS=20`) |
| `APP__ADMIN__EMAIL` / `NAME` | `admin@example.com` | Auto-seeded bootstrap admin account on startup |
| `APP__GOOGLE__*` | optional in dev | Required for live Google OAuth 2.0 login & re-auth |
| `APP__R2__*` | optional | Enables `StorageService` for obfuscated media uploads |

---

## 2. Non-Interactive `cargo xtask` CLI

Every `xtask` command is 100% non-interactive (no TTY prompts):

```bash
cargo xtask gen-ts                        # Regenerate TS DTOs & API endpoints (`api::gen-ts`)
cargo xtask db schema [--database-url ..] # Apply schema/*.sql, triggers/*.sql, seed/*.sql via psql
cargo xtask db seed [--database-url ..]   # Apply seed/*.sql only
cargo xtask dev                           # Print readiness & dev commands (`--serve`, `--watch`, `--check`)
cargo xtask check                         # Clippy (-D warnings) + svelte-check + Rust unit tests
cargo xtask test [--unit-only] [--e2e]    # Unit + Postgres integration + optional Playwright E2E
cargo xtask build                         # gen-ts -> npm run build -> release `server` binary
```

---

## 3. Multi-Stage Docker Build (`scripts/Dockerfile`)

`scripts/Dockerfile` compiles both tiers cleanly without copying host `node_modules/` or `target/`:
1. **`web-builder`** (`node:22-bookworm-slim`): Installs `svelte-app` deps, injects `style/input.css`, builds static bundle (`svelte-app/build`).
2. **`rust-base` / `planner` / `cacher` / `rust-builder`** (`rust:1-bookworm` + `cargo-chef`): Pre-caches third-party crates, builds release `server` binary.
3. **`runtime`** (`debian:bookworm-slim`): Copies `/app/server`, `/app/svelte-app/build`, and `/app/public`, sets `SVELTE_SITE_ROOT=svelte-app/build`, `PUBLIC_ASSETS_ROOT=public`, `APP__ENV=prod`, exposes port `3000`, and monitors `/api/v1/health`.

```bash
docker build -f scripts/Dockerfile -t axum-svelte-template:latest .
docker run --env-file .env -p 3000:3000 axum-svelte-template:latest
```
