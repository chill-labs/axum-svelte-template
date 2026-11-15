// =============================================================================
// ROUTES: USER PROFILE & PREFERENCES
// =============================================================================

// `axum` - Extracts `AppState`, JSON payloads, and routes HTTP methods.
use axum::extract::State;
use axum::response::Response;
use axum::routing::put;
use axum::{Json, Router};

use services::state::AppState;
use shared::domain::error::ToResponse;
use shared::domain::user::{UpdateLanguageRequest, UpdateProfileRequest};

use crate::extractors::AuthUser;

/// Mounts `/api/v1/profile` and `/api/v1/profile/language`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/profile", put(update_profile))
        .route("/api/v1/profile/language", put(update_language))
}

/// Updates the authenticated user's profile fields.
pub async fn update_profile(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
    Json(req): Json<UpdateProfileRequest>,
) -> Response {
    state
        .user_service
        .update_profile(auth.user.id, req)
        .await
        .to_response()
}

/// Updates only the authenticated user's interface language preference (`vi` | `en`).
pub async fn update_language(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
    Json(req): Json<UpdateLanguageRequest>,
) -> Response {
    state
        .user_service
        .update_language(auth.user.id, req)
        .await
        .to_response()
}
