# System Architecture

## 1. Crate Dependency Graph

```text
crates/shared   (Isomorphic DTOs, Settings, ErrorCode, Routes, RBAC, XOR utils)
      ▲
crates/services (SeaORM entities, Domain Services, Moka Caches, R2 Storage, SSE Hub, Middlewares)
      ▲
crates/api      (Axum /api/v1/* handlers, /api/notifications/sse, `gen-ts` AST generator)
      ▲
crates/server   (Axum binary: serves SPA shell, `svelte-app/build`, `public/`, and API)
```

- **`crates/shared`** has zero database or HTTP server implementation details—it defines the contracts parsed by `gen-ts`.
- **`crates/services`** owns `AppState` (`DatabaseConnection`, `CacheService`, `Option<StorageService>`, `NotificationHub`, domain services).
- **`crates/api`** contains thin Axum handlers that extract auth/RBAC context, invoke `services`, and serialize `ApiResponse<T>` via `.to_response()` / `.to_response_created()`.
- **`crates/server`** serves the static SvelteKit `adapter-static` output (`SVELTE_SITE_ROOT=svelte-app/build`), injects `window.__APP_BOOTSTRAP__` into `200.html` / `index.html`, mounts `public/` (`PUBLIC_ASSETS_ROOT=public`), and nests `/api/v1`.

---

## 2. Request & Hydration Lifecycle

1. **Initial Page Load (`GET /todos`, `/workspaces`, etc.)**:
   - `crates/server` inspects cookies (`auth-session`, `hot-support-session`, `lang`, `theme`).
   - Resolves `SessionBootstrapData` (if signed in) and injects JSON safely into `<!--APP_BOOTSTRAP_SLOT-->` inside the SPA HTML shell alongside server-resolved `<html lang="..." class="dark?">`.
   - Browser boots Svelte 5 immediately with zero flash of unstyled theme, wrong language, or redundant `/api/v1/auth/me` spinner.
2. **SPA Client Navigation & API Calls**:
   - SvelteKit client router handles transitions and guards routes via `svelte-app/src/lib/config/routes.ts` (mirroring `crates/shared/src/config/routes.rs`).
   - API mutations hit `/api/v1/*` using generated endpoint helpers from `endpoints.generated.ts`.
3. **Real-Time Updates (`/api/notifications/sse`)**:
   - Authenticated browser tabs open an `EventSource` connected to `NotificationHub` (`tokio::sync::broadcast`).
   - `RealtimeEvent` frames push `UserNotification`, `GlobalAnnouncement`, `TodoChanged { workspace_id, todo_id }`, and `KeepAlive`.

---

## 3. Caching & Invalidation (`crates/services/src/cache.rs`)

- **Weighted `CachedList<V>`**: Bounded Moka caches for per-user workspace lists and per-workspace Todo lists.
- **Auth Caches**: Session token digest (`SHA-256`) -> `user_id` and `user_id` -> `users::Model`, honoring `APP__CACHE__SESSION_TTL_SECS`, `APP__CACHE__USER_TTL_SECS`, and `APP__CACHE__MAX_CAPACITY`.
- **Write-Through Invalidation**: Service mutations invalidate affected keys immediately after DB commit.
