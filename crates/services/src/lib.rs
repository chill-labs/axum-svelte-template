// =============================================================================
// SERVICES CRATE ROOT
// =============================================================================
//
// Aggregates PostgreSQL SeaORM entities, business-logic domain services,
// Moka caching, optional Cloudflare R2 object storage, workspace RBAC
// enforcement, real-time SSE broadcasting, auth middleware, startup seeding,
// and dev-only test helpers.
//
// ## Architecture Notes
//
// `crates/api` and `crates/server` consume `AppState` and `auth_middleware`
// directly from this crate without touching SeaORM models inside handlers.

// SECTION: PUBLIC MODULES

/// Bounded Moka in-memory caching layer (`CacheService` and `CachedList<V>`).
pub mod cache;

/// Domain business-logic services (`user`, `workspace`, `todo`, `notification`, `media`, `admin`).
pub mod domain;

/// SeaORM 1.1 entities matching `database/schema/01..05_*.sql` and DTO mappers.
pub mod entities;

/// Axum HTTP middleware for `auth-session` and Hot Support cookie isolation.
pub mod middleware;

/// Workspace RBAC `PermResource × PermAction` permission service with Moka cache.
pub mod permission;

/// Idempotent startup seeding (`SeedService::seed_default_admin`).
pub mod seed;

/// Real-time `tokio::sync::broadcast` SSE hub (`NotificationHub`).
pub mod sse;

/// Shared `AppState` container wiring DB, caches, storage, and domain services.
pub mod state;

/// Optional Cloudflare R2 / S3 object storage wrapper (`Option<StorageService>`).
pub mod storage;

/// Dev-only `/api/test/*` SQL, password-hashing, and session seeding endpoints.
pub mod test_api;

/// Server-only Argon2id password and SHA-256 session token cryptographic helpers.
pub mod utils;
