// =============================================================================
// DEV-ONLY TEST API ENDPOINTS
// =============================================================================
//
// Provides deterministic Playwright E2E and QA helper endpoints mounted ONLY
// when `settings.env.is_dev()`:
// - `POST /api/test/sql` — Execute arbitrary SQL queries/mutations
// - `POST /api/test/hash-password` — Hash a plaintext password with Argon2id
// - `POST /api/test/session` — Upsert a user, create a session cookie, and
//   return `ApiResponse<UserResponse>`
//
// ## Security
//
// Protected by two independent guards:
// 1. `router(settings)` mounts routes only when `settings.env.is_dev()`.
// 2. Every handler explicitly verifies `state.settings.env.is_dev()` and
//    returns `ErrorCode::Forbidden` (`403`) otherwise.

// SECTION: IMPORTS

// `axum` - Router, extractors, HTTP headers, status codes, and JSON payloads.
use axum::extract::State;
use axum::http::header::SET_COOKIE;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};

// `chrono` - UTC timestamps for login & update audit columns.
use chrono::Utc;

// `sea_orm` - Raw SQL execution and user upsert ORM queries.
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseBackend, EntityTrait, FromQueryResult,
    JsonValue, QueryFilter, Set, Statement,
};

// `serde` - Request and response serialization for dev test endpoints.
use serde::{Deserialize, Serialize};

// `shared` - Configuration, standard response envelope, DTOs, and error codes.
use shared::config::settings::Settings;
use shared::domain::error::AppError;
use shared::domain::error_code::ErrorCode;
use shared::domain::response::ApiResponse;
use shared::domain::user::{UserResponse, UserRole};

// `crate` - Entities, auth cookie formatting, state, and password hashing.
use crate::domain::user::service::DEFAULT_SESSION_TTL_DAYS;
use crate::entities::users;
use crate::middleware::auth::{build_session_cookie_header, AUTH_COOKIE_NAME};
use crate::state::AppState;
use crate::utils::password::hash_password;

// SECTION: REQUEST & RESPONSE DTOs

/// Payload accepted by `POST /api/test/sql`.
#[derive(Debug, Deserialize)]
pub struct SqlRequest {
    /// Raw PostgreSQL query or DML statement.
    pub sql: String,
}

/// Response returned by `POST /api/test/sql`.
#[derive(Debug, Serialize, Deserialize)]
pub struct SqlResponse {
    /// `true` when SQL executed without errors.
    pub success: bool,
    /// Affected row count for DML statements (`INSERT`, `UPDATE`, `DELETE`).
    pub rows_affected: Option<u64>,
    /// JSON row objects for `SELECT`, `WITH`, `SHOW`, or `RETURNING` queries.
    pub rows: Option<Vec<JsonValue>>,
    /// Error message when execution failed.
    pub error: Option<String>,
}

/// Payload accepted by `POST /api/test/hash-password`.
#[derive(Debug, Deserialize)]
pub struct HashPasswordRequest {
    /// Plaintext password to hash via Argon2id.
    pub password: String,
}

/// Response returned by `POST /api/test/hash-password`.
#[derive(Debug, Serialize, Deserialize)]
pub struct HashPasswordResponse {
    /// PHC-formatted Argon2id hash string.
    pub hash: String,
}

/// Payload accepted by `POST /api/test/session` to seed a user & HttpOnly session cookie.
#[derive(Debug, Deserialize)]
pub struct TestSessionRequest {
    /// Target user email (normalized lowercase).
    pub email: String,
    /// Optional display name (defaults to `"Test User"` on insert).
    #[serde(default)]
    pub name: Option<String>,
    /// Optional role (`user` or `admin`).
    #[serde(default)]
    pub role: Option<UserRole>,
    /// Optional UI language (`vi` or `en`).
    #[serde(default)]
    pub language: Option<String>,
    /// Optional onboarding completion flag (defaults to `true`).
    #[serde(default)]
    pub onboarding_completed: Option<bool>,
    /// Optional test-account flag (defaults to `true`).
    #[serde(default)]
    pub is_test_account: Option<bool>,
    /// Optional plaintext password hashed into `users.password_hash`.
    #[serde(default)]
    pub password: Option<String>,
}

// SECTION: ROUTER REGISTRATION

/// Returns dev-only test routes mounted at `/api/test/*` when `settings.env.is_dev()`,
/// or an empty `Router` in staging/production.
pub fn router(settings: &Settings) -> Router<AppState> {
    if settings.env.is_dev() {
        test_api_routes()
    } else {
        Router::new()
    }
}

/// Builds the `/api/test/*` sub-router with defense-in-depth `ServerEnv::Dev` handler guards.
pub fn test_api_routes() -> Router<AppState> {
    Router::new()
        .route("/api/test/sql", post(handle_sql))
        .route("/api/test/hash-password", post(handle_hash_password))
        .route("/api/test/session", post(handle_create_session))
}

// SECTION: HANDLERS

/// Executes arbitrary SQL in local dev environments for E2E fixture setup/cleanup.
async fn handle_sql(
    State(state): State<AppState>,
    Json(payload): Json<SqlRequest>,
) -> impl IntoResponse {
    if !state.settings.env.is_dev() {
        return (
            StatusCode::FORBIDDEN,
            Json(SqlResponse {
                success: false,
                rows_affected: None,
                rows: None,
                error: Some("Test API is only available in dev environment".to_string()),
            }),
        );
    }

    let sql = payload.sql.trim();
    let upper = sql.to_ascii_uppercase();
    let returns_rows = upper.starts_with("SELECT")
        || upper.starts_with("WITH")
        || upper.starts_with("SHOW")
        || upper.contains(" RETURNING ");

    let stmt = Statement::from_string(DatabaseBackend::Postgres, sql.to_string());

    if returns_rows {
        match state.db.query_all(stmt).await {
            Ok(results) => {
                let rows: Vec<JsonValue> = results
                    .iter()
                    .filter_map(|row| JsonValue::from_query_result(row, "").ok())
                    .collect();
                (
                    StatusCode::OK,
                    Json(SqlResponse {
                        success: true,
                        rows_affected: None,
                        rows: Some(rows),
                        error: None,
                    }),
                )
            }
            Err(err) => (
                StatusCode::OK,
                Json(SqlResponse {
                    success: false,
                    rows_affected: None,
                    rows: None,
                    error: Some(err.to_string()),
                }),
            ),
        }
    } else {
        match state.db.execute(stmt).await {
            Ok(res) => (
                StatusCode::OK,
                Json(SqlResponse {
                    success: true,
                    rows_affected: Some(res.rows_affected()),
                    rows: None,
                    error: None,
                }),
            ),
            Err(err) => (
                StatusCode::OK,
                Json(SqlResponse {
                    success: false,
                    rows_affected: None,
                    rows: None,
                    error: Some(err.to_string()),
                }),
            ),
        }
    }
}

/// Hashes a plaintext password using the application's Argon2id settings.
async fn handle_hash_password(
    State(state): State<AppState>,
    Json(payload): Json<HashPasswordRequest>,
) -> Result<Json<HashPasswordResponse>, AppError> {
    if !state.settings.env.is_dev() {
        return Err(AppError::Code(ErrorCode::Forbidden));
    }

    let hash = hash_password(&payload.password)?;
    Ok(Json(HashPasswordResponse { hash }))
}

/// Upserts a user by email, mints an `auth-session` HttpOnly cookie, and returns `UserResponse`.
async fn handle_create_session(
    State(state): State<AppState>,
    Json(payload): Json<TestSessionRequest>,
) -> Result<(HeaderMap, Json<ApiResponse<UserResponse>>), AppError> {
    if !state.settings.env.is_dev() {
        return Err(AppError::Code(ErrorCode::Forbidden));
    }

    let email = payload.email.trim().to_lowercase();
    if email.is_empty() {
        return Err(AppError::Code(ErrorCode::InvalidEmail));
    }

    let existing = users::Entity::find()
        .filter(users::Column::Email.eq(&email))
        .one(&state.db)
        .await
        .map_err(|err| AppError::Internal(format!("Failed querying test session user: {err}")))?;

    let password_hash = match payload.password {
        Some(ref pw) if !pw.is_empty() => Some(hash_password(pw)?),
        _ => None,
    };

    let now = Utc::now();

    let user = match existing {
        Some(row) => {
            let mut active: users::ActiveModel = row.into();
            if let Some(name) = payload.name {
                active.name = Set(name);
            }
            if let Some(role) = payload.role {
                active.role = Set(role.as_str().to_string());
            }
            if let Some(lang) = payload.language {
                active.language = Set(lang);
            }
            if let Some(onboarded) = payload.onboarding_completed {
                active.onboarding_completed = Set(onboarded);
            }
            if let Some(is_test) = payload.is_test_account {
                active.is_test_account = Set(is_test);
            }
            if let Some(hash) = password_hash {
                active.password_hash = Set(Some(hash));
            }
            active.last_login_at = Set(Some(now.into()));
            active.updated_at = Set(now.into());
            active.update(&state.db).await.map_err(|err| {
                AppError::Internal(format!("Failed updating test session user: {err}"))
            })?
        }
        None => {
            let inserted = users::ActiveModel {
                email: Set(email),
                password_hash: Set(password_hash),
                name: Set(payload.name.unwrap_or_else(|| "Test User".to_string())),
                role: Set(payload.role.unwrap_or(UserRole::User).as_str().to_string()),
                email_verified: Set(true),
                language: Set(payload.language.unwrap_or_else(|| "vi".to_string())),
                is_test_account: Set(payload.is_test_account.unwrap_or(true)),
                onboarding_completed: Set(payload.onboarding_completed.unwrap_or(true)),
                last_login_at: Set(Some(now.into())),
                ..Default::default()
            }
            .insert(&state.db)
            .await
            .map_err(|err| {
                AppError::Internal(format!("Failed inserting test session user: {err}"))
            })?;

            // Re-fetch so `active_workspace_id` populated by `trg_create_default_workspace` is hydrated.
            state.user_service.get_user_by_id(inserted.id).await?
        }
    };

    state.cache.users.insert(user.id, user.clone()).await;
    let raw_token = state.user_service.create_session(user.id).await?;
    let user_response = state.user_service.to_user_response(&user).await?;

    let cookie_val = build_session_cookie_header(
        AUTH_COOKIE_NAME,
        &raw_token,
        DEFAULT_SESSION_TTL_DAYS * 86_400,
        state.settings.env.use_secure_cookies(),
    );

    let mut headers = HeaderMap::new();
    let header_value = HeaderValue::from_str(&cookie_val)
        .map_err(|err| AppError::Internal(format!("Invalid cookie header value: {err}")))?;
    headers.insert(SET_COOKIE, header_value);

    Ok((headers, Json(ApiResponse::success(user_response))))
}
