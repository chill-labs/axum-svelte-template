// =============================================================================
// ROUTES: ADMIN CONSOLE (USERS, HOT SUPPORT, TEST ACCOUNTS, GLOBAL NOTICES)
// =============================================================================

// `axum` - Path/Query/State/Json extractors and `Set-Cookie` emission.
use axum::extract::{Path, Query, State};
use axum::http::header::SET_COOKIE;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};

use services::domain::admin::service::HOT_SUPPORT_TTL_HOURS;
use services::middleware::auth::{resolve_session_user, HOT_SUPPORT_COOKIE_NAME};
use services::state::AppState;
use shared::domain::admin::{
    AdminUpdateUserRoleRequest, CreateGlobalNotificationRequest, CreateTestAccountRequest,
    GetAdminUsersQuery, HotSupportLoginRequest, HotSupportSessionInfo,
    UpdateGlobalNotificationRequest, UpdateTestAccountRequest,
};
use shared::domain::error::ToResponse;
use shared::domain::error_code::ErrorCode;
use shared::domain::response::ApiResponse;
use shared::domain::user::UserRole;

use crate::cookies::{build_session_cookie, clear_session_cookie, extract_cookie};
use crate::extractors::AdminUser;

/// Mounts `/api/v1/admin/*`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/admin/users", get(list_admin_users))
        .route("/api/v1/admin/users/{id}/role", put(update_user_role))
        .route("/api/v1/admin/hot-support/login", post(hot_support_login))
        .route("/api/v1/admin/hot-support/session", get(hot_support_session))
        .route("/api/v1/admin/hot-support/close", post(hot_support_close))
        .route(
            "/api/v1/admin/test-accounts",
            get(list_test_accounts).post(create_test_account),
        )
        .route(
            "/api/v1/admin/test-accounts/{id}",
            put(update_test_account).delete(delete_test_account),
        )
        .route(
            "/api/v1/admin/notifications",
            get(list_global_notifications).post(create_global_notification),
        )
        .route(
            "/api/v1/admin/notifications/{id}",
            put(update_global_notification).delete(delete_global_notification),
        )
}

/// Paginated system user directory for Tab 1 (`AdminUsersTab`).
pub async fn list_admin_users(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
    Query(query): Query<GetAdminUsersQuery>,
) -> Response {
    match state.admin_service.list_users(query).await {
        Ok((users, pagination)) => ApiResponse::success_paginated(users, pagination).into_response(),
        Err(err) => err.into_response(),
    }
}

/// Promotes or demotes a user's system role (`user` <-> `admin`).
pub async fn update_user_role(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    Path(id): Path<i32>,
    Json(req): Json<AdminUpdateUserRoleRequest>,
) -> Response {
    state
        .admin_service
        .update_user_role(admin.user.id, id, req)
        .await
        .to_response()
}

/// Starts an isolated `hot-support-session` cookie targeting `req.user_id`.
pub async fn hot_support_login(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    Json(req): Json<HotSupportLoginRequest>,
) -> Response {
    match state
        .admin_service
        .start_hot_support(admin.user.id, req)
        .await
    {
        Ok((info, raw_token)) => {
            let cookie_val = build_session_cookie(
                &state.settings,
                HOT_SUPPORT_COOKIE_NAME,
                &raw_token,
                HOT_SUPPORT_TTL_HOURS * 3_600,
            );
            let mut resp = (StatusCode::OK, Json(ApiResponse::success(info))).into_response();
            if let Ok(hv) = HeaderValue::from_str(&cookie_val) {
                resp.headers_mut().append(SET_COOKIE, hv);
            }
            resp
        }
        Err(err) => err.into_response(),
    }
}

/// Inspects whether the caller's browser currently holds an active `hot-support-session`.
pub async fn hot_support_session(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
    headers: HeaderMap,
) -> Response {
    let target = resolve_session_user(&state, &headers, true).await;
    let info = match target {
        Some(hs_user) => HotSupportSessionInfo {
            active: true,
            target_user_id: Some(hs_user.user.id),
            target_user_name: Some(hs_user.user.name),
            target_user_email: Some(hs_user.user.email),
            launch_url: Some("/todos?_hs=1".to_string()),
        },
        None => HotSupportSessionInfo {
            active: false,
            target_user_id: None,
            target_user_name: None,
            target_user_email: None,
            launch_url: None,
        },
    };
    Ok(info).to_response()
}

/// Closes an active `hot-support-session` while leaving the parent admin `auth-session` intact.
///
/// Validates `auth-session` directly so both `/admin` and the child `?_hs=1` banner
/// "Exit Support" button can invoke it cleanly when initiated by an admin browser.
pub async fn hot_support_close(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let Some(real_admin) = resolve_session_user(&state, &headers, false).await else {
        return ApiResponse::<()>::error(ErrorCode::Unauthorized).into_response();
    };
    if UserRole::from_db(&real_admin.user.role) != UserRole::Admin {
        return ApiResponse::<()>::error(ErrorCode::AdminRequired).into_response();
    }

    let raw_hs = extract_cookie(&headers, HOT_SUPPORT_COOKIE_NAME);
    if let Err(err) = state
        .admin_service
        .stop_hot_support(raw_hs.as_deref())
        .await
    {
        return err.into_response();
    }

    let clear_cookie = clear_session_cookie(&state.settings, HOT_SUPPORT_COOKIE_NAME);
    let mut resp = Ok(true).to_response();
    if let Ok(hv) = HeaderValue::from_str(&clear_cookie) {
        resp.headers_mut().append(SET_COOKIE, hv);
    }
    resp
}

/// Lists all Test Accounts managed in Tab 3 (`AdminTestAccountsTab`).
pub async fn list_test_accounts(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
) -> Response {
    state.admin_service.list_test_accounts().await.to_response()
}

/// Creates a new Argon2id password-enabled Test Account.
pub async fn create_test_account(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
    Json(req): Json<CreateTestAccountRequest>,
) -> Response {
    state
        .admin_service
        .create_test_account(req)
        .await
        .to_response_created()
}

/// Updates an existing Test Account (display name, role, language, or password).
pub async fn update_test_account(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
    Path(id): Path<i32>,
    Json(req): Json<UpdateTestAccountRequest>,
) -> Response {
    state
        .admin_service
        .update_test_account(id, req)
        .await
        .to_response()
}

/// Deletes a Test Account user row cleanly.
pub async fn delete_test_account(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
    Path(id): Path<i32>,
) -> Response {
    state
        .admin_service
        .delete_test_account(id)
        .await
        .map(|()| true)
        .to_response()
}

/// Lists all global HTML announcement banners in Tab 4 (`AdminNotificationsTab`).
pub async fn list_global_notifications(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
) -> Response {
    state
        .admin_service
        .list_global_notifications()
        .await
        .to_response()
}

/// Publishes a global HTML announcement and broadcasts via SSE when active.
pub async fn create_global_notification(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    Json(req): Json<CreateGlobalNotificationRequest>,
) -> Response {
    state
        .admin_service
        .create_global_notification(admin.user.id, req)
        .await
        .to_response_created()
}

/// Edits or toggles a global announcement.
pub async fn update_global_notification(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
    Path(id): Path<i32>,
    Json(req): Json<UpdateGlobalNotificationRequest>,
) -> Response {
    state
        .admin_service
        .update_global_notification(id, req)
        .await
        .to_response()
}

/// Deletes a global announcement.
pub async fn delete_global_notification(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
    Path(id): Path<i32>,
) -> Response {
    state
        .admin_service
        .delete_global_notification(id)
        .await
        .map(|()| true)
        .to_response()
}
