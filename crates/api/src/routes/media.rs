// =============================================================================
// ROUTES: PROTECTED MEDIA & OPTIONAL R2 UPLOADS
// =============================================================================

// `axum` - Extracts path params, JSON payloads, and serves binary blobs safely.
use axum::extract::{Path, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};

use services::domain::media::service::DEFAULT_MEDIA_FIELD;
use services::state::AppState;
use shared::domain::error::ToResponse;
use shared::domain::media::{ConfirmUploadRequest, PresignUploadRequest};
use shared::domain::upload::UploadPurpose;

use crate::extractors::AuthUser;

/// Mounts `/api/v1/media/*`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/media/presign", post(presign_upload))
        .route("/api/v1/media/confirm", post(confirm_upload))
        .route("/api/v1/media/blob/{media_id}", get(get_media_blob))
        .route(
            "/api/v1/media/{entity_type}/{entity_id}",
            get(get_entity_media),
        )
}

/// Generates an R2 presigned PUT URL and random XOR obfuscation key (`503` if R2 unconfigured).
pub async fn presign_upload(
    State(state): State<AppState>,
    AuthUser(_auth): AuthUser,
    Json(req): Json<PresignUploadRequest>,
) -> Response {
    state.media_service.presign_upload(req).await.to_response()
}

/// Persists confirmed encrypted R2 upload metadata into `media_files`.
pub async fn confirm_upload(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
    Json(req): Json<ConfirmUploadRequest>,
) -> Response {
    let purpose = req.purpose;
    let entity_id = req.entity_id;

    match state.media_service.confirm_upload(auth.user.id, req).await {
        Ok(media) => {
            if purpose == UploadPurpose::UserAvatar && entity_id == auth.user.id {
                let _ = state
                    .user_service
                    .sync_uploaded_avatar(auth.user.id, media.public_url.clone())
                    .await;
            }
            Ok(media).to_response_created()
        }
        Err(err) => err.into_response(),
    }
}

/// Returns stored media metadata + XOR key for an entity (`ProtectedImage` descriptor).
pub async fn get_entity_media(
    State(state): State<AppState>,
    AuthUser(_auth): AuthUser,
    Path((entity_type, entity_id)): Path<(String, i32)>,
) -> Response {
    state
        .media_service
        .get_media(&entity_type, entity_id, DEFAULT_MEDIA_FIELD)
        .await
        .to_response()
}

/// Streams raw encrypted bytes when `R2_PUBLIC_URL` is omitted (proxied through server).
pub async fn get_media_blob(
    State(state): State<AppState>,
    AuthUser(_auth): AuthUser,
    Path(media_id): Path<i32>,
) -> Response {
    match state.media_service.fetch_blob(media_id).await {
        Ok((bytes, content_type)) => {
            let mut resp = (StatusCode::OK, bytes).into_response();
            if let Ok(ct) = HeaderValue::from_str(&content_type) {
                resp.headers_mut().insert(CONTENT_TYPE, ct);
            }
            resp.headers_mut().insert(
                CACHE_CONTROL,
                HeaderValue::from_static("private, max-age=3600"),
            );
            resp
        }
        Err(err) => err.into_response(),
    }
}
