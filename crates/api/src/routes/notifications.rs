// =============================================================================
// ROUTES: PERSONAL NOTIFICATIONS & GLOBAL ANNOUNCEMENTS
// =============================================================================

// `axum` - Path/State extractors and route handlers.
use axum::extract::{Path, State};
use axum::response::Response;
use axum::routing::{get, post};
use axum::Router;

use services::state::AppState;
use shared::domain::error::ToResponse;
use shared::domain::user::UserRole;

use crate::extractors::{AuthUser, OptionalAuthUser};

/// Mounts `/api/v1/notifications*`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/notifications", get(get_notifications))
        .route("/api/v1/notifications/read-all", post(mark_all_read))
        .route("/api/v1/notifications/{id}/read", post(mark_one_read))
        .route(
            "/api/v1/notifications/global/{id}/read",
            post(mark_global_read),
        )
}

/// Returns personal notifications (when signed in) plus eligible active global HTML notices.
pub async fn get_notifications(
    State(state): State<AppState>,
    OptionalAuthUser(maybe_auth): OptionalAuthUser,
) -> Response {
    let (user_id, role) = match maybe_auth {
        Some(auth) => (Some(auth.user.id), Some(UserRole::from_db(&auth.user.role))),
        None => (None, None),
    };

    state
        .notification_service
        .get_feed(user_id, role)
        .await
        .to_response()
}

/// Marks a single personal notification as read.
pub async fn mark_one_read(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
    Path(id): Path<i32>,
) -> Response {
    state
        .notification_service
        .mark_read(auth.user.id, id)
        .await
        .map(|()| true)
        .to_response()
}

/// Marks all unread personal & global notices as read for the signed-in user.
pub async fn mark_all_read(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
) -> Response {
    let role = UserRole::from_db(&auth.user.role);
    state
        .notification_service
        .mark_all_read(auth.user.id, role)
        .await
        .map(|()| true)
        .to_response()
}

/// Records an idempotent read receipt on a global broadcast notice.
pub async fn mark_global_read(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
    Path(id): Path<i32>,
) -> Response {
    state
        .notification_service
        .mark_global_read(auth.user.id, id)
        .await
        .map(|()| true)
        .to_response()
}
