// =============================================================================
// ROUTES: AUTHENTICATION, GOOGLE OAUTH, TEST LOGIN & REAUTH
// =============================================================================

// `axum` - Query, State, HeaderMap, Json extractors and HTTP redirects/responses.
use axum::extract::{Query, State};
use axum::http::header::SET_COOKIE;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Json, Router};

// `serde` - Deserializes query parameters for Google OAuth initiate & callback routes.
use serde::Deserialize;

use services::domain::user::service::DEFAULT_SESSION_TTL_DAYS;
use services::middleware::auth::AUTH_COOKIE_NAME;
use services::state::{AppState, OAuthPendingState};
use shared::domain::error::ToResponse;
use shared::domain::response::ApiResponse;
use shared::domain::user::{
    CompleteOnboardingRequest, InitiatePasswordReauthRequest, LoginPasswordRequest,
    SessionBootstrapData,
};

use crate::cookies::{build_session_cookie, clear_session_cookie, extract_cookie};
use crate::extractors::{AuthUser, OptionalAuthUser};

/// Query parameters accepted when initiating Google OAuth login.
#[derive(Debug, Deserialize)]
pub struct GoogleLoginQuery {
    pub return_to: Option<String>,
}

/// Query parameters returned by Google OAuth authorization callback.
#[derive(Debug, Deserialize)]
pub struct GoogleCallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
}

/// Mounts `/api/v1/auth/*` endpoints plus `/auth/google/callback`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/auth/me", get(get_me))
        .route("/api/v1/auth/google/login", get(google_login))
        .route("/auth/google/callback", get(google_callback))
        .route("/api/v1/auth/login-password", post(login_password))
        .route("/api/v1/auth/logout", post(logout))
        .route("/api/v1/auth/onboarding", post(complete_onboarding))
        .route("/api/v1/auth/reauth-password", post(reauth_password))
}

/// Hydrates the active user session (`Option<SessionBootstrapData>`).
pub async fn get_me(
    State(state): State<AppState>,
    OptionalAuthUser(maybe_auth): OptionalAuthUser,
) -> Response {
    let Some(auth) = maybe_auth else {
        let empty: Option<SessionBootstrapData> = None;
        return Ok(empty).to_response();
    };

    state
        .user_service
        .get_session_bootstrap(auth.user.id, auth.is_hot_support, &state.permission_service)
        .await
        .map(Some)
        .to_response()
}

/// Redirects user agent to Google OAuth 2.0 consent page or returns error fallback redirect.
pub async fn google_login(
    State(state): State<AppState>,
    Query(query): Query<GoogleLoginQuery>,
) -> Response {
    let return_to = sanitize_return_to(query.return_to.as_deref());
    let state_token = state
        .user_service
        .create_oauth_state(OAuthPendingState {
            return_to,
            purpose: "login".to_string(),
            reauth_token: None,
            user_id: None,
        })
        .await;

    match state.user_service.build_google_auth_url(&state_token) {
        Ok(url) => Redirect::temporary(&url).into_response(),
        Err(err) => {
            let code = urlencoding::encode(err.code().as_str());
            Redirect::temporary(&format!("/auth/login?error={code}")).into_response()
        }
    }
}

/// Handles Google OAuth callback for both standard login and `purpose=reauth`.
pub async fn google_callback(
    State(state): State<AppState>,
    Query(query): Query<GoogleCallbackQuery>,
) -> Response {
    if query.error.is_some() {
        return Redirect::temporary("/auth/login?error=OAUTH_EXCHANGE_FAILED").into_response();
    }

    let (Some(code), Some(state_token)) = (query.code, query.state) else {
        return Redirect::temporary("/auth/login?error=OAUTH_EXCHANGE_FAILED").into_response();
    };

    let Some(pending) = state.user_service.consume_oauth_state(&state_token).await else {
        return Redirect::temporary("/auth/login?error=OAUTH_EXCHANGE_FAILED").into_response();
    };

    let google_info = match state.user_service.exchange_google_code(&code).await {
        Ok(info) => info,
        Err(err) => {
            let err_code = urlencoding::encode(err.code().as_str());
            return Redirect::temporary(&format!("/auth/login?error={err_code}")).into_response();
        }
    };

    if pending.purpose == "reauth" {
        return match state
            .user_service
            .complete_password_reauth(&pending, &google_info)
            .await
        {
            Ok(_) => Redirect::temporary("/auth/reset-success").into_response(),
            Err(err) => {
                let err_code = urlencoding::encode(err.code().as_str());
                Redirect::temporary(&format!("/auth/reset-password?error={err_code}"))
                    .into_response()
            }
        };
    }

    let (user, _is_new) = match state.user_service.upsert_google_user(google_info).await {
        Ok(res) => res,
        Err(err) => {
            let err_code = urlencoding::encode(err.code().as_str());
            return Redirect::temporary(&format!("/auth/login?error={err_code}")).into_response();
        }
    };

    let raw_token = match state.user_service.create_session(user.id).await {
        Ok(token) => token,
        Err(_) => {
            return Redirect::temporary("/auth/login?error=INTERNAL_ERROR").into_response();
        }
    };

    let dest = if !user.onboarding_completed {
        "/auth/onboarding".to_string()
    } else {
        sanitize_return_to(Some(&pending.return_to))
    };

    let cookie_val = build_session_cookie(
        &state.settings,
        AUTH_COOKIE_NAME,
        &raw_token,
        DEFAULT_SESSION_TTL_DAYS * 86_400,
    );

    let mut resp = Redirect::temporary(&dest).into_response();
    if let Ok(hv) = HeaderValue::from_str(&cookie_val) {
        resp.headers_mut().append(SET_COOKIE, hv);
    }
    resp
}

/// Logs in with email & password (intended for admin-created Test Accounts).
pub async fn login_password(
    State(state): State<AppState>,
    Json(req): Json<LoginPasswordRequest>,
) -> Response {
    match state.user_service.login_with_password(req).await {
        Ok((user_res, raw_token)) => {
            let bootstrap = state
                .user_service
                .get_session_bootstrap(user_res.id, false, &state.permission_service)
                .await;

            match bootstrap {
                Ok(data) => {
                    let cookie_val = build_session_cookie(
                        &state.settings,
                        AUTH_COOKIE_NAME,
                        &raw_token,
                        DEFAULT_SESSION_TTL_DAYS * 86_400,
                    );
                    let mut resp = (StatusCode::OK, Json(ApiResponse::success(data))).into_response();
                    if let Ok(hv) = HeaderValue::from_str(&cookie_val) {
                        resp.headers_mut().append(SET_COOKIE, hv);
                    }
                    resp
                }
                Err(err) => err.into_response(),
            }
        }
        Err(err) => err.into_response(),
    }
}

/// Revokes the active session cookie in DB & cache and instructs client to clear `auth-session`.
pub async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Some(raw_token) = extract_cookie(&headers, AUTH_COOKIE_NAME) {
        let _ = state.user_service.revoke_session(&raw_token).await;
    }

    let clear_cookie = clear_session_cookie(&state.settings, AUTH_COOKIE_NAME);
    let mut resp = Ok(true).to_response();
    if let Ok(hv) = HeaderValue::from_str(&clear_cookie) {
        resp.headers_mut().append(SET_COOKIE, hv);
    }
    resp
}

/// Finalizes user onboarding (preferred display name, language, starter workspace name).
pub async fn complete_onboarding(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
    Json(req): Json<CompleteOnboardingRequest>,
) -> Response {
    match state.user_service.complete_onboarding(auth.user.id, req).await {
        Ok(_) => state
            .user_service
            .get_session_bootstrap(
                auth.user.id,
                auth.is_hot_support,
                &state.permission_service,
            )
            .await
            .to_response(),
        Err(err) => err.into_response(),
    }
}

/// Validates and stores an encrypted pending password, returning a Google `purpose=reauth` URL.
pub async fn reauth_password(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
    Json(req): Json<InitiatePasswordReauthRequest>,
) -> Response {
    state
        .user_service
        .initiate_password_reauth(auth.user.id, req)
        .await
        .to_response()
}

fn sanitize_return_to(raw: Option<&str>) -> String {
    match raw.map(str::trim) {
        Some(path) if path.starts_with('/') && !path.starts_with("//") => path.to_string(),
        _ => "/todos".to_string(),
    }
}
