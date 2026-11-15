// =============================================================================
// ROUTES: HEALTHCHECK (`/api/v1/health`)
// =============================================================================

// `axum` - Extracts `AppState` and renders JSON responses.
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};

// `chrono` - Generates UTC timestamp for readiness output.
use chrono::Utc;

use services::state::AppState;
use shared::config::settings::ServerEnv;
use shared::domain::healthcheck::HealthcheckResponse;
use shared::domain::response::ApiResponse;

/// Mounts `/api/v1/health`.
pub fn router() -> Router<AppState> {
    Router::new().route("/api/v1/health", get(check_health))
}

/// Checks PostgreSQL connectivity and reports environment & optional storage readiness.
pub async fn check_health(State(state): State<AppState>) -> Response {
    let db_ok = state.db.ping().await.is_ok();

    let environment = match state.settings.env {
        ServerEnv::Dev => "dev",
        ServerEnv::Staging => "staging",
        ServerEnv::Prod => "prod",
    };

    let payload = HealthcheckResponse {
        status: if db_ok { "ok" } else { "degraded" }.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        database: db_ok,
        storage_enabled: state.storage.is_some(),
        environment: environment.to_string(),
        timestamp: Utc::now().to_rfc3339(),
    };

    let status = if db_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (status, Json(ApiResponse::success(payload))).into_response()
}
