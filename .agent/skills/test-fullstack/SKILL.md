---
name: test-fullstack
description: Run non-interactive Rust unit checks, PostgreSQL integration suites, Svelte typechecks, and Playwright E2E verification across axum-svelte-template.
---

# Full-Stack Verification Workflow (`axum-svelte-template`)

Execute non-interactive checks at the appropriate scope without ever invoking Rust formatters (`cargo fmt`, `rustfmt`):

## 1. Fast Static & Unit Check
```bash
cargo xtask check
```
This runs:
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cd svelte-app && npm run check` (when `svelte-app/node_modules` is present)
- `cargo test --workspace --lib`

## 2. Database Setup & Integration Tests
Ensure `.env` defines a reachable `APP__DATABASE__URL`, then apply schemas, triggers, and seeds:
```bash
cargo xtask db schema
cargo xtask test
```
When `crates/services/tests/` contains integration test files, `cargo xtask test` runs `cargo test -p services --tests -- --test-threads=4`. Pass `--unit-only` to run unit tests exclusively.

## 3. End-to-End Playwright Verification
Ensure PostgreSQL is migrated, `svelte-app` dependencies are installed (`cd svelte-app && npm ci`), and run:
```bash
cargo xtask test --e2e
```
Playwright configuration lives in `svelte-app/e2e/playwright.config.ts` and defaults to `BASE_URL=http://127.0.0.1:3100`.

## 4. Production Release & Docker Smoke Check
```bash
cargo xtask build
docker build -f scripts/Dockerfile -t axum-svelte-template:latest .
```
