// =============================================================================
// INTEGRATION TESTS: USER, OAUTH, PASSWORD, REAUTH, SEEDING & AUTH MIDDLEWARE
// =============================================================================
//
// Verifies PostgreSQL signup triggers, admin seeding, Google OAuth user upsert,
// Test Account Argon2id password login, Google `purpose=reauth` password reset,
// profile & onboarding updates, and `auth-session` vs `hot-support-session`
// middleware cookie isolation.

mod common;

// SECTION: IMPORTS

use axum::body::{to_bytes, Body};
use axum::extract::Extension;
use axum::http::header::{COOKIE, REFERER, SET_COOKIE};
use axum::http::{HeaderMap, HeaderValue, Request, StatusCode};
use axum::middleware::from_fn_with_state;
use axum::routing::get;
use axum::Router;
use common::TestApp;
use services::domain::user::service::GoogleUserInfo;
use services::middleware::auth::{
    auth_middleware, has_hot_support_signal, has_hot_support_signal_for_path, resolve_session_user,
    AuthenticatedUser, AUTH_COOKIE_NAME, HOT_SUPPORT_COOKIE_NAME,
};
use services::seed::SeedService;
use services::state::OAuthPendingState;
use services::test_api;
use shared::config::settings::GoogleConfig;
use shared::domain::admin::HotSupportLoginRequest;
use shared::domain::error::AppError;
use shared::domain::error_code::ErrorCode;
use shared::domain::todo::TodoListQuery;
use shared::domain::user::{
    CompleteOnboardingRequest, InitiatePasswordReauthRequest, LoginPasswordRequest,
    UpdateLanguageRequest, UpdateProfileRequest,
};
use tower::ServiceExt;

// SECTION: TRIGGER & SEEDING TESTS

#[tokio::test]
async fn signup_trigger_provisions_personal_workspace_and_starter_todos() {
    let app = TestApp::new().await;
    let user = app
        .create_user("alice@example.com", "Alice Example")
        .await;

    let active_ws_id = user
        .active_workspace_id
        .expect("trigger must populate active_workspace_id");

    let workspaces = app
        .state
        .workspace_service
        .list_workspaces(user.id)
        .await
        .expect("list provisioned workspaces");
    assert_eq!(workspaces.len(), 1);
    assert_eq!(workspaces[0].id, active_ws_id);
    assert_eq!(workspaces[0].name, "Personal Workspace");
    assert!(workspaces[0].is_owner);
    assert_eq!(workspaces[0].todo_count, 5);
    assert_eq!(workspaces[0].completed_todo_count, 1);

    let (todos_payload, pagination) = app
        .state
        .todo_service
        .list_todos(user.id, active_ws_id, TodoListQuery::default())
        .await
        .expect("list starter todos");
    assert_eq!(pagination.total_items, 5);
    assert_eq!(todos_payload.stats.total, 5);
    assert_eq!(todos_payload.stats.done_count, 1);
}

#[tokio::test]
async fn seed_default_admin_is_idempotent_and_promotes_existing_user() {
    let app = TestApp::new().await;

    let first = SeedService::seed_default_admin(&app.state.db, &app.state.settings)
        .await
        .expect("first admin seed");
    assert_eq!(first.role, "admin");
    assert!(first.onboarding_completed);
    assert!(first.active_workspace_id.is_some());

    let second = SeedService::seed_default_admin(&app.state.db, &app.state.settings)
        .await
        .expect("idempotent second admin seed");
    assert_eq!(first.id, second.id);
    assert_eq!(second.role, "admin");
}

// SECTION: OAUTH, TEST ACCOUNT LOGIN & PASSWORD REAUTH TESTS

#[tokio::test]
async fn google_oauth_upsert_and_onboarding_flow() {
    let app = TestApp::new().await;

    let (created, is_new) = app
        .state
        .user_service
        .upsert_google_user(GoogleUserInfo {
            sub: "google-sub-1001".to_string(),
            email: "OAuthUser@Example.com".to_string(),
            name: "  OAuth   User  ".to_string(),
            picture: Some("https://lh3.example.com/avatar.png".to_string()),
            email_verified: true,
        })
        .await
        .expect("first oauth upsert");

    assert!(is_new);
    assert_eq!(created.email, "oauthuser@example.com");
    assert_eq!(created.name, "OAuth User");
    assert!(!created.onboarding_completed);
    assert!(created.active_workspace_id.is_some());

    let (returning, is_new_again) = app
        .state
        .user_service
        .upsert_google_user(GoogleUserInfo {
            sub: "google-sub-1001".to_string(),
            email: "oauthuser@example.com".to_string(),
            name: "OAuth User".to_string(),
            picture: None,
            email_verified: true,
        })
        .await
        .expect("second oauth upsert");
    assert!(!is_new_again);
    assert_eq!(returning.id, created.id);

    let onboarded = app
        .state
        .user_service
        .complete_onboarding(
            created.id,
            CompleteOnboardingRequest {
                name: "Updated Teacher".to_string(),
                language: "en".to_string(),
                first_workspace_name: Some("Engineering Team".to_string()),
            },
        )
        .await
        .expect("complete onboarding");

    assert!(onboarded.onboarding_completed);
    assert_eq!(onboarded.language, "en");
    assert_eq!(onboarded.name, "Updated Teacher");

    let bootstrap = app
        .state
        .user_service
        .get_session_bootstrap(created.id, false, &app.state.permission_service)
        .await
        .expect("session bootstrap");
    assert_eq!(
        bootstrap.active_workspace_name.as_deref(),
        Some("Engineering Team")
    );
    assert_eq!(bootstrap.active_workspace_role.as_deref(), Some("owner"));
}

#[tokio::test]
async fn test_account_password_login_and_google_reauth_password_update() {
    let app = TestApp::with_settings(|s| {
        s.google = GoogleConfig {
            client_id: "test-google-client-id".to_string(),
            client_secret: "test-google-client-secret".to_string(),
            redirect_uri: "http://localhost:3000/auth/google/callback".to_string(),
        };
    })
    .await;

    let account = app
        .create_test_account("qa@example.com", "QA User", "InitialPass!123")
        .await;

    let wrong_err = app
        .state
        .user_service
        .login_with_password(LoginPasswordRequest {
            email: "qa@example.com".to_string(),
            password: "WrongPassword!".to_string(),
        })
        .await
        .unwrap_err();
    assert_eq!(wrong_err.code(), ErrorCode::InvalidCredentials);

    let (logged_in, raw_token) = app
        .state
        .user_service
        .login_with_password(LoginPasswordRequest {
            email: " QA@EXAMPLE.COM ".to_string(),
            password: "InitialPass!123".to_string(),
        })
        .await
        .expect("login with valid test account credentials");
    assert_eq!(logged_in.id, account.id);
    assert!(!raw_token.is_empty());

    // Initiate password reauth -> consume OAuth state -> finalize new password.
    let (oauth_user, _) = app
        .state
        .user_service
        .upsert_google_user(GoogleUserInfo {
            sub: "google-sub-reauth-1".to_string(),
            email: "reauth@example.com".to_string(),
            name: "Reauth User".to_string(),
            picture: None,
            email_verified: true,
        })
        .await
        .expect("create oauth user");

    let reauth_redirect = app
        .state
        .user_service
        .initiate_password_reauth(
            oauth_user.id,
            InitiatePasswordReauthRequest {
                new_password: "NewStrongPassword!99".to_string(),
            },
        )
        .await
        .expect("initiate password reauth");
    assert!(reauth_redirect
        .redirect_url
        .starts_with("https://accounts.google.com/o/oauth2/v2/auth"));

    let state_param = reauth_redirect
        .redirect_url
        .split("state=")
        .nth(1)
        .expect("state query param present");
    let pending: OAuthPendingState = app
        .state
        .user_service
        .consume_oauth_state(state_param)
        .await
        .expect("oauth state stored in Moka");
    assert_eq!(pending.purpose, "reauth");

    // Re-authenticated by wrong Google identity -> forbidden.
    let mismatch_err = app
        .state
        .user_service
        .complete_password_reauth(
            &pending,
            &GoogleUserInfo {
                sub: "different-google-sub".to_string(),
                email: "intruder@example.com".to_string(),
                name: "Intruder".to_string(),
                picture: None,
                email_verified: true,
            },
        )
        .await;
    assert!(matches!(
        mismatch_err,
        Err(AppError::Code(ErrorCode::Forbidden))
    ));

    // Re-initiate and complete with matching Google identity.
    let reauth_redirect_2 = app
        .state
        .user_service
        .initiate_password_reauth(
            oauth_user.id,
            InitiatePasswordReauthRequest {
                new_password: "NewStrongPassword!99".to_string(),
            },
        )
        .await
        .expect("initiate second password reauth");
    let state_param_2 = reauth_redirect_2
        .redirect_url
        .split("state=")
        .nth(1)
        .expect("state param");
    let pending_2 = app
        .state
        .user_service
        .consume_oauth_state(state_param_2)
        .await
        .expect("consume valid state");

    app.state
        .user_service
        .complete_password_reauth(
            &pending_2,
            &GoogleUserInfo {
                sub: "google-sub-reauth-1".to_string(),
                email: "reauth@example.com".to_string(),
                name: "Reauth User".to_string(),
                picture: None,
                email_verified: true,
            },
        )
        .await
        .expect("complete password reauth");

    let (pw_login, _) = app
        .state
        .user_service
        .login_with_password(LoginPasswordRequest {
            email: "reauth@example.com".to_string(),
            password: "NewStrongPassword!99".to_string(),
        })
        .await
        .expect("login using newly verified password");
    assert_eq!(pw_login.id, oauth_user.id);
}

#[tokio::test]
async fn profile_double_option_and_hot_support_session_isolation() {
    let app = TestApp::new().await;
    let admin = app.create_admin("admin@example.com", "Admin").await;
    let target = app.create_user("customer@example.com", "Customer").await;

    // Profile double-option clears avatar_url on explicit null while leaving untouched when None.
    let with_avatar = app
        .state
        .user_service
        .update_profile(
            target.id,
            UpdateProfileRequest {
                name: "Customer Updated".to_string(),
                language: Some("en".to_string()),
                avatar_url: Some(Some("https://cdn.example.com/pic.png".to_string())),
            },
        )
        .await
        .expect("set avatar_url");
    assert_eq!(
        with_avatar.avatar_url.as_deref(),
        Some("https://cdn.example.com/pic.png")
    );

    let cleared_avatar = app
        .state
        .user_service
        .update_profile(
            target.id,
            UpdateProfileRequest {
                name: "Customer Updated".to_string(),
                language: None,
                avatar_url: Some(None),
            },
        )
        .await
        .expect("clear avatar_url");
    assert_eq!(cleared_avatar.avatar_url, None);

    let lang_updated = app
        .state
        .user_service
        .update_language(
            target.id,
            UpdateLanguageRequest {
                language: "vi".to_string(),
            },
        )
        .await
        .expect("update language");
    assert_eq!(lang_updated.language, "vi");

    // Hot Support isolation: both `auth-session` (admin) and `hot-support-session` (target) coexist.
    let admin_tok = app
        .state
        .user_service
        .create_session(admin.id)
        .await
        .expect("admin session");
    let (hs_info, hs_tok) = app
        .state
        .admin_service
        .start_hot_support(admin.id, HotSupportLoginRequest { user_id: target.id })
        .await
        .expect("start hot support");
    assert!(hs_info.active);
    assert_eq!(hs_info.target_user_id, Some(target.id));

    let mut headers = HeaderMap::new();
    headers.insert(
        COOKIE,
        HeaderValue::from_str(&format!(
            "{AUTH_COOKIE_NAME}={admin_tok}; {HOT_SUPPORT_COOKIE_NAME}={hs_tok}"
        ))
        .unwrap(),
    );

    // Normal request -> resolves admin (`is_hot_support = false`).
    let normal = resolve_session_user(&app.state, &headers, false)
        .await
        .expect("resolve normal session");
    assert_eq!(normal.user.id, admin.id);
    assert!(!normal.is_hot_support);

    // Hot Support request -> resolves target user (`is_hot_support = true`).
    let hs = resolve_session_user(&app.state, &headers, true)
        .await
        .expect("resolve hot support session");
    assert_eq!(hs.user.id, target.id);
    assert!(hs.is_hot_support);

    // Signal checks: `X-Hot-Support: 1`, `?_hs=1`, and Referer strictly on `/api/notifications/sse`.
    let mut sse_headers = headers.clone();
    sse_headers.insert(
        REFERER,
        HeaderValue::from_static("http://localhost:3000/todos?_hs=1"),
    );
    assert!(!has_hot_support_signal(&sse_headers, None));
    assert!(!has_hot_support_signal_for_path(
        &sse_headers,
        "/api/v1/todos",
        None
    ));
    assert!(has_hot_support_signal_for_path(
        &sse_headers,
        "/api/notifications/sse",
        None
    ));

    // Stopping hot support invalidates `hot-support-session` without touching `auth-session`.
    app.state
        .admin_service
        .stop_hot_support(Some(&hs_tok))
        .await
        .expect("stop hot support");
    assert!(resolve_session_user(&app.state, &headers, true).await.is_none());
    assert_eq!(
        resolve_session_user(&app.state, &headers, false)
            .await
            .map(|a| a.user.id),
        Some(admin.id)
    );
}

#[tokio::test]
async fn auth_middleware_and_dev_test_api_end_to_end() {
    let app = TestApp::new().await;

    async fn whoami(Extension(auth): Extension<Option<AuthenticatedUser>>) -> String {
        match auth {
            Some(a) => format!("{}:{}", a.user.email, a.is_hot_support),
            None => "guest".to_string(),
        }
    }

    let router = Router::new()
        .route("/whoami", get(whoami))
        .merge(test_api::router(&app.state.settings))
        .layer(from_fn_with_state(app.state.clone(), auth_middleware))
        .with_state(app.state.clone());

    // 1. Unauthenticated request inserts `None` cleanly.
    let guest_res = router
        .clone()
        .oneshot(Request::builder().uri("/whoami").body(Body::empty()).unwrap())
        .await
        .expect("guest request");
    assert_eq!(guest_res.status(), StatusCode::OK);
    let guest_body = to_bytes(guest_res.into_body(), 1024).await.unwrap();
    assert_eq!(&guest_body[..], b"guest");

    // 2. `/api/test/session` creates user + cookie + returns UserResponse.
    let sess_res = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/test/session")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"email":"e2e@example.com","name":"E2E Tester","language":"en"}"#,
                ))
                .unwrap(),
        )
        .await
        .expect("test session request");
    assert_eq!(sess_res.status(), StatusCode::OK);
    let set_cookie = sess_res
        .headers()
        .get(SET_COOKIE)
        .expect("set-cookie header")
        .to_str()
        .unwrap()
        .to_string();
    let cookie_pair = set_cookie.split(';').next().unwrap().to_string();

    // 3. Authenticated request with cookie resolves user via `auth_middleware`.
    let auth_res = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/whoami")
                .header(COOKIE, &cookie_pair)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("authed request");
    let auth_body = to_bytes(auth_res.into_body(), 1024).await.unwrap();
    assert_eq!(&auth_body[..], b"e2e@example.com:false");

    // 4. `/api/test/sql` executes SELECT cleanly.
    let sql_res = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/test/sql")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"sql":"SELECT email FROM users WHERE email = 'e2e@example.com'"}"#,
                ))
                .unwrap(),
        )
        .await
        .expect("sql request");
    assert_eq!(sql_res.status(), StatusCode::OK);
    let sql_body = to_bytes(sql_res.into_body(), 4096).await.unwrap();
    let sql_json: test_api::SqlResponse = serde_json::from_slice(&sql_body).unwrap();
    assert!(sql_json.success);
    assert_eq!(sql_json.rows.unwrap().len(), 1);
}
