// =============================================================================
// REST API ROUTER AGGREGATION
// =============================================================================

// `axum` - Composable HTTP sub-router merger.
use axum::Router;

use services::state::AppState;

/// Admin Console management endpoints (`/api/v1/admin/*`).
pub mod admin;
/// Authentication, Google OAuth, password test login, onboarding, and reauth.
pub mod auth;
/// Database & storage healthcheck telemetry (`/api/v1/health`).
pub mod health;
/// Polymorphic encrypted media presign/confirm/blob endpoints (`/api/v1/media/*`).
pub mod media;
/// Personal notification feed and global HTML announcement read tracking.
pub mod notifications;
/// User profile and UI language preference endpoints.
pub mod profile;
/// Workspace-scoped Todo CRUD, filtering, stats, and reordering.
pub mod todos;
/// Multi-tenant workspace and collaborator RBAC management endpoints.
pub mod workspaces;

/// Combines all `/api/v1/*` domain routers onto a shared `Router<AppState>`.
pub fn api_v1_router() -> Router<AppState> {
    Router::new()
        .merge(health::router())
        .merge(auth::router())
        .merge(profile::router())
        .merge(workspaces::router())
        .merge(todos::router())
        .merge(notifications::router())
        .merge(media::router())
        .merge(admin::router())
}
