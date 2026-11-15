// =============================================================================
// USER & AUTHENTICATION DOMAIN SERVICE
// =============================================================================
//
// Implements user account lifecycle, SHA-256 cookie sessions, Google OAuth 2.0
// exchange, Test Account Argon2id password login, Google `purpose=reauth`
// password updates, onboarding, and `SessionBootstrapData` hydration.
//
// ## Key Concepts
//
// - **Trigger Awareness**: Inserting a new `users` row fires the PostgreSQL
//   `trg_create_default_workspace` trigger (`AFTER INSERT`). Services always
//   re-fetch the newly inserted user row so `active_workspace_id` reflects the
//   trigger-provisioned `Personal Workspace`.
// - **Write-Through Cache**: Mutations update or evict `CacheService.users` and
//   `CacheService.sessions` immediately to prevent stale auth decisions.
//
// ## Architecture Notes
//
// Exposed in `AppState::user_service`.

// SECTION: IMPORTS

// `chrono` - UTC timestamps for session expiration and login audit trails.
use chrono::{Duration, Utc};

// `moka` - Async TTL caches for OAuth state and pending password reauth tokens.
use moka::future::Cache;

// `reqwest` - Outbound HTTPS client for Google OAuth token & userinfo exchange.
use reqwest::Client;

// `sea_orm` - Asynchronous PostgreSQL CRUD for `users`, `sessions`, and `workspaces`.
// - `ActiveModelTrait` - Insert and update operations
// - `ColumnTrait` - Column filter predicates (`Email`, `GoogleId`, `OwnerId`, etc.)
// - `DatabaseConnection` - PostgreSQL pool handle
// - `EntityTrait` - Query entry points
// - `QueryFilter`, `QueryOrder` - Filtering and sorting
// - `Set` - ActiveModel field assignments
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};

// `serde` - Deserializes Google OAuth token & OpenID Connect userinfo JSON payloads.
use serde::{Deserialize, Serialize};

// `shared` - Shared DTOs, validators, settings, and typed error definitions.
use shared::config::settings::Settings;
use shared::domain::error::AppError;
use shared::domain::error_code::ErrorCode;
use shared::domain::request::{normalize_whitespace, Sanitize};
use shared::domain::user::{
    CompleteOnboardingRequest, InitiatePasswordReauthRequest, LoginPasswordRequest,
    ReauthRedirectResponse, SessionBootstrapData, UpdateLanguageRequest, UpdateProfileRequest,
    UserResponse, UserRole,
};
use shared::domain::validator::slugify;

// `crate` - Caching, media resolution, notifications, RBAC, and crypto helpers.
use crate::cache::CacheService;
use crate::domain::media::service::{MediaService, DEFAULT_MEDIA_FIELD};
use crate::domain::notification::service::NotificationService;
use crate::entities::conversions::map_user_response;
use crate::entities::{sessions, users, workspace_members, workspaces};
use crate::permission::PermissionService;
use crate::state::{OAuthPendingState, PendingPassword};
use crate::utils::password::{hash_password, verify_password};
use crate::utils::token::{generate_raw_token, hash_token};
use shared::domain::upload::UploadPurpose;

// SECTION: CONSTANTS & TYPES

/// Default persistent browser session lifetime (`30` days).
pub const DEFAULT_SESSION_TTL_DAYS: i64 = 30;

/// Profile fields extracted from Google's OpenID Connect userinfo endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleUserInfo {
    /// Stable Google subject identifier (`sub`).
    pub sub: String,
    /// Primary email address from Google account.
    pub email: String,
    /// Display name returned by Google.
    #[serde(default)]
    pub name: String,
    /// Google profile photo URL.
    #[serde(default)]
    pub picture: Option<String>,
    /// Whether Google verified ownership of `email`.
    #[serde(default)]
    pub email_verified: bool,
}

#[derive(Debug, Deserialize)]
struct GoogleTokenResponse {
    access_token: String,
}

// SECTION: USER SERVICE

/// Service handling user authentication, OAuth, sessions, and profile updates.
#[derive(Clone)]
pub struct UserService {
    /// PostgreSQL connection pool.
    db: DatabaseConnection,
    /// Application settings (Google OAuth credentials, server env, etc.).
    settings: Settings,
    /// Moka session/user/workspace cache manager.
    cache: CacheService,
    /// Media service resolving encrypted user avatar keys.
    media_service: MediaService,
    /// 10-minute TTL cache of pending Google OAuth `state` entries.
    oauth_states: Cache<String, OAuthPendingState>,
    /// 10-minute TTL cache of pending Argon2 password hashes awaiting Google `reauth`.
    pending_passwords: Cache<String, PendingPassword>,
    /// Notification service used for unread badge counts during session bootstrap.
    notification_service: NotificationService,
    /// Reusable HTTP client for Google OAuth code exchange.
    http: Client,
}

impl UserService {
    /// Constructs a new `UserService`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        db: DatabaseConnection,
        settings: Settings,
        cache: CacheService,
        media_service: MediaService,
        oauth_states: Cache<String, OAuthPendingState>,
        pending_passwords: Cache<String, PendingPassword>,
        notification_service: NotificationService,
    ) -> Self {
        Self {
            db,
            settings,
            cache,
            media_service,
            oauth_states,
            pending_passwords,
            notification_service,
            http: Client::new(),
        }
    }

    // SECTION: SESSION MANAGEMENT

    /// Creates a new database session for `user_id` with default 30-day TTL and returns the raw cookie token.
    pub async fn create_session(&self, user_id: i32) -> Result<String, AppError> {
        self.create_session_with_ttl(user_id, Duration::days(DEFAULT_SESSION_TTL_DAYS))
            .await
    }

    /// Creates a database session for `user_id` with a custom lifetime and caches its hash.
    pub async fn create_session_with_ttl(
        &self,
        user_id: i32,
        ttl: Duration,
    ) -> Result<String, AppError> {
        let raw_token = generate_raw_token();
        let hashed = hash_token(&raw_token);
        let expires_at = Utc::now() + ttl;

        sessions::ActiveModel {
            id: Set(hashed.clone()),
            user_id: Set(user_id),
            expires_at: Set(expires_at.into()),
            ..Default::default()
        }
        .insert(&self.db)
        .await
        .map_err(|err| AppError::Internal(format!("Failed creating session: {err}")))?;

        self.cache.sessions.insert(hashed, user_id).await;
        Ok(raw_token)
    }

    /// Revokes a single session by its plaintext cookie token (`logout`).
    pub async fn revoke_session(&self, raw_token: &str) -> Result<(), AppError> {
        let trimmed = raw_token.trim();
        if trimmed.is_empty() {
            return Ok(());
        }
        let hashed = hash_token(trimmed);
        sessions::Entity::delete_by_id(&hashed)
            .exec(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed revoking session: {err}")))?;
        self.cache.invalidate_session(&hashed).await;
        Ok(())
    }

    /// Deletes all persisted sessions belonging to `user_id`.
    pub async fn revoke_all_user_sessions(&self, user_id: i32) -> Result<(), AppError> {
        sessions::Entity::delete_many()
            .filter(sessions::Column::UserId.eq(user_id))
            .exec(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed revoking user sessions: {err}")))?;
        self.cache.sessions.invalidate_all();
        Ok(())
    }

    // SECTION: PASSWORD AUTHENTICATION (TEST ACCOUNTS & REAUTH PASSWORDS)

    /// Authenticates a password-enabled Test Account (or local-password user) and issues a session.
    ///
    /// # Returns
    ///
    /// `(UserResponse, raw_session_token)` for setting the `auth-session` cookie.
    pub async fn login_with_password(
        &self,
        mut req: LoginPasswordRequest,
    ) -> Result<(UserResponse, String), AppError> {
        req.sanitize();
        req.validate_input()?;

        let user = users::Entity::find()
            .filter(users::Column::Email.eq(&req.email))
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed querying user: {err}")))?
            .ok_or(AppError::Code(ErrorCode::InvalidCredentials))?;

        let Some(ref hash) = user.password_hash else {
            return Err(AppError::Code(ErrorCode::InvalidCredentials));
        };

        if !verify_password(&req.password, hash)? {
            return Err(AppError::Code(ErrorCode::InvalidCredentials));
        }

        let now = Utc::now();
        let mut active: users::ActiveModel = user.into();
        active.last_login_at = Set(Some(now.into()));
        active.updated_at = Set(now.into());
        let updated = active
            .update(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed updating login timestamp: {err}")))?;

        self.cache.users.insert(updated.id, updated.clone()).await;
        let raw_token = self.create_session(updated.id).await?;
        let response = self.to_user_response(&updated).await?;

        Ok((response, raw_token))
    }

    // SECTION: GOOGLE OAUTH 2.0 & REAUTH FLOWS

    /// Generates and stores a short-lived `OAuthPendingState` token (10 min TTL).
    pub async fn create_oauth_state(&self, pending: OAuthPendingState) -> String {
        let state_token = generate_raw_token();
        self.oauth_states.insert(state_token.clone(), pending).await;
        state_token
    }

    /// Consumes a single-use `OAuthPendingState` token from Moka cache.
    pub async fn consume_oauth_state(&self, state_token: &str) -> Option<OAuthPendingState> {
        let found = self.oauth_states.get(state_token).await;
        if found.is_some() {
            self.oauth_states.invalidate(state_token).await;
        }
        found
    }

    /// Constructs the Google OAuth 2.0 authorization URL embedding `state_token`.
    pub fn build_google_auth_url(&self, state_token: &str) -> Result<String, AppError> {
        if !self.settings.google.is_configured() {
            return Err(AppError::Code(ErrorCode::OAuthNotConfigured));
        }

        let client_id = urlencoding::encode(self.settings.google.client_id.trim());
        let redirect_uri = urlencoding::encode(self.settings.google.redirect_uri.trim());
        let scope = urlencoding::encode("openid email profile");
        let state = urlencoding::encode(state_token);

        Ok(format!(
            "https://accounts.google.com/o/oauth2/v2/auth?client_id={client_id}&redirect_uri={redirect_uri}&response_type=code&scope={scope}&access_type=online&prompt=select_account&state={state}"
        ))
    }

    /// Exchanges an authorization `code` with Google OAuth and fetches the user's OIDC profile.
    pub async fn exchange_google_code(&self, code: &str) -> Result<GoogleUserInfo, AppError> {
        if !self.settings.google.is_configured() {
            return Err(AppError::Code(ErrorCode::OAuthNotConfigured));
        }

        let token_res = self
            .http
            .post("https://oauth2.googleapis.com/token")
            .form(&[
                ("code", code),
                ("client_id", self.settings.google.client_id.trim()),
                ("client_secret", self.settings.google.client_secret.trim()),
                ("redirect_uri", self.settings.google.redirect_uri.trim()),
                ("grant_type", "authorization_code"),
            ])
            .send()
            .await
            .map_err(|err| AppError::WithDetail {
                code: ErrorCode::OAuthExchangeFailed,
                detail: format!("Google token request failed: {err}"),
            })?;

        if !token_res.status().is_success() {
            return Err(AppError::WithDetail {
                code: ErrorCode::OAuthExchangeFailed,
                detail: format!("Google token endpoint returned {}", token_res.status()),
            });
        }

        let token_body: GoogleTokenResponse =
            token_res.json().await.map_err(|err| AppError::WithDetail {
                code: ErrorCode::OAuthExchangeFailed,
                detail: format!("Invalid Google token JSON: {err}"),
            })?;

        let userinfo_res = self
            .http
            .get("https://openidconnect.googleapis.com/v1/userinfo")
            .bearer_auth(&token_body.access_token)
            .send()
            .await
            .map_err(|err| AppError::WithDetail {
                code: ErrorCode::OAuthExchangeFailed,
                detail: format!("Google userinfo request failed: {err}"),
            })?;

        if !userinfo_res.status().is_success() {
            return Err(AppError::WithDetail {
                code: ErrorCode::OAuthExchangeFailed,
                detail: format!("Google userinfo endpoint returned {}", userinfo_res.status()),
            });
        }

        userinfo_res
            .json::<GoogleUserInfo>()
            .await
            .map_err(|err| AppError::WithDetail {
                code: ErrorCode::OAuthExchangeFailed,
                detail: format!("Invalid Google userinfo JSON: {err}"),
            })
    }

    /// Finds an existing user by `google_id` or `email`, or inserts a new user
    /// (automatically triggering `trg_create_default_workspace` on signup).
    ///
    /// # Returns
    ///
    /// `(user_model, is_new_user)`.
    pub async fn upsert_google_user(
        &self,
        info: GoogleUserInfo,
    ) -> Result<(users::Model, bool), AppError> {
        let normalized_email = info.email.trim().to_lowercase();
        if normalized_email.is_empty() {
            return Err(AppError::Code(ErrorCode::InvalidEmail));
        }

        let existing = users::Entity::find()
            .filter(users::Column::GoogleId.eq(&info.sub))
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed querying google_id: {err}")))?;

        let existing = match existing {
            Some(found) => Some(found),
            None => users::Entity::find()
                .filter(users::Column::Email.eq(&normalized_email))
                .one(&self.db)
                .await
                .map_err(|err| AppError::Internal(format!("Failed querying email: {err}")))?,
        };

        let now = Utc::now();

        if let Some(user) = existing {
            let mut active: users::ActiveModel = user.clone().into();
            if user.google_id.is_none() {
                active.google_id = Set(Some(info.sub));
            }
            if info.email_verified && !user.email_verified {
                active.email_verified = Set(true);
            }
            if user.avatar_url.is_none() && info.picture.is_some() {
                active.avatar_url = Set(info.picture);
            }
            active.last_login_at = Set(Some(now.into()));
            active.updated_at = Set(now.into());

            let updated = active
                .update(&self.db)
                .await
                .map_err(|err| AppError::Internal(format!("Failed updating OAuth user: {err}")))?;
            self.cache.users.insert(updated.id, updated.clone()).await;
            return Ok((updated, false));
        }

        let clean_name = {
            let n = normalize_whitespace(&info.name);
            if n.is_empty() {
                normalized_email
                    .split('@')
                    .next()
                    .unwrap_or("User")
                    .to_string()
            } else {
                n
            }
        };

        let inserted = users::ActiveModel {
            email: Set(normalized_email),
            name: Set(clean_name),
            avatar_url: Set(info.picture),
            role: Set(UserRole::User.as_str().to_string()),
            email_verified: Set(info.email_verified),
            google_id: Set(Some(info.sub)),
            language: Set("vi".to_string()),
            is_test_account: Set(false),
            onboarding_completed: Set(false),
            last_login_at: Set(Some(now.into())),
            ..Default::default()
        }
        .insert(&self.db)
        .await
        .map_err(|err| AppError::Internal(format!("Failed inserting OAuth user: {err}")))?;

        // Re-fetch so `active_workspace_id` populated by `trg_create_default_workspace` is hydrated.
        let hydrated = self.get_user_by_id(inserted.id).await?;
        self.cache.users.insert(hydrated.id, hydrated.clone()).await;
        Ok((hydrated, true))
    }

    /// Validates a new password, caches its Argon2 hash under a pending reauth token,
    /// and returns the Google OAuth `purpose=reauth` redirect URL.
    pub async fn initiate_password_reauth(
        &self,
        user_id: i32,
        req: InitiatePasswordReauthRequest,
    ) -> Result<ReauthRedirectResponse, AppError> {
        req.validate_input()?;

        if !self.settings.google.is_configured() {
            return Err(AppError::Code(ErrorCode::OAuthNotConfigured));
        }

        let _user = self.get_user_by_id(user_id).await?;
        let password_hash = hash_password(&req.new_password)?;
        let reauth_token = generate_raw_token();

        self.pending_passwords
            .insert(
                reauth_token.clone(),
                PendingPassword {
                    user_id,
                    password_hash,
                },
            )
            .await;

        let state_token = self
            .create_oauth_state(OAuthPendingState {
                return_to: "/auth/reset-success".to_string(),
                purpose: "reauth".to_string(),
                reauth_token: Some(reauth_token),
                user_id: Some(user_id),
            })
            .await;

        let redirect_url = self.build_google_auth_url(&state_token)?;
        Ok(ReauthRedirectResponse { redirect_url })
    }

    /// Finalizes a pending password change after Google OAuth `purpose=reauth` callback verification.
    pub async fn complete_password_reauth(
        &self,
        pending_state: &OAuthPendingState,
        verified_google: &GoogleUserInfo,
    ) -> Result<users::Model, AppError> {
        if pending_state.purpose != "reauth" {
            return Err(AppError::Code(ErrorCode::Forbidden));
        }

        let expected_user_id = pending_state
            .user_id
            .ok_or(AppError::Code(ErrorCode::Forbidden))?;
        let reauth_token = pending_state
            .reauth_token
            .as_deref()
            .ok_or(AppError::Code(ErrorCode::Forbidden))?;

        let pending_pw = self
            .pending_passwords
            .get(reauth_token)
            .await
            .ok_or(AppError::Code(ErrorCode::Unauthorized))?;
        self.pending_passwords.invalidate(reauth_token).await;

        if pending_pw.user_id != expected_user_id {
            return Err(AppError::Code(ErrorCode::Forbidden));
        }

        let user = self.get_user_by_id(expected_user_id).await?;
        let google_matches = user
            .google_id
            .as_deref()
            .is_some_and(|gid| gid == verified_google.sub)
            || user.email.eq_ignore_ascii_case(verified_google.email.trim());

        if !google_matches {
            return Err(AppError::Code(ErrorCode::Forbidden));
        }

        let mut active: users::ActiveModel = user.into();
        active.password_hash = Set(Some(pending_pw.password_hash));
        active.updated_at = Set(Utc::now().into());

        let updated = active
            .update(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed updating password hash: {err}")))?;

        self.cache.users.insert(updated.id, updated.clone()).await;
        Ok(updated)
    }

    // SECTION: PROFILE, ONBOARDING & SESSION BOOTSTRAP

    /// Fetches a raw `users::Model` by primary key ID (`ErrorCode::UserNotFound` if missing).
    pub async fn get_user_by_id(&self, user_id: i32) -> Result<users::Model, AppError> {
        users::Entity::find_by_id(user_id)
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed querying user {user_id}: {err}")))?
            .ok_or(AppError::Code(ErrorCode::UserNotFound))
    }

    /// Maps a `users::Model` into `UserResponse` with resolved `avatar_key`.
    pub async fn to_user_response(&self, model: &users::Model) -> Result<UserResponse, AppError> {
        let avatar = self
            .media_service
            .get_media(
                UploadPurpose::UserAvatar.entity_type(),
                model.id,
                DEFAULT_MEDIA_FIELD,
            )
            .await?;

        Ok(map_user_response(model, avatar.map(|m| m.xor_key)))
    }

    /// Loads a user's hydrated `UserResponse`.
    pub async fn get_profile(&self, user_id: i32) -> Result<UserResponse, AppError> {
        let user = self.get_user_by_id(user_id).await?;
        self.to_user_response(&user).await
    }

    /// Updates user display name, preferred language, and optional three-state `avatar_url`.
    pub async fn update_profile(
        &self,
        user_id: i32,
        mut req: UpdateProfileRequest,
    ) -> Result<UserResponse, AppError> {
        req.sanitize();
        req.validate_input()?;

        let user = self.get_user_by_id(user_id).await?;
        let mut active: users::ActiveModel = user.into();
        active.name = Set(req.name);
        if let Some(lang) = req.language {
            active.language = Set(lang);
        }

        let mut cleared_avatar = false;
        if let Some(avatar_opt) = req.avatar_url {
            if avatar_opt.is_none() {
                cleared_avatar = true;
            }
            active.avatar_url = Set(avatar_opt);
        }
        active.updated_at = Set(Utc::now().into());

        let updated = active
            .update(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed updating profile: {err}")))?;

        if cleared_avatar {
            self.media_service
                .delete_media(
                    UploadPurpose::UserAvatar.entity_type(),
                    user_id,
                    DEFAULT_MEDIA_FIELD,
                )
                .await?;
        }

        self.cache.users.insert(updated.id, updated.clone()).await;
        self.to_user_response(&updated).await
    }

    /// Updates the user's preferred UI locale (`vi` or `en`).
    pub async fn update_language(
        &self,
        user_id: i32,
        mut req: UpdateLanguageRequest,
    ) -> Result<UserResponse, AppError> {
        req.sanitize();
        req.validate_input()?;

        let user = self.get_user_by_id(user_id).await?;
        let mut active: users::ActiveModel = user.into();
        active.language = Set(req.language);
        active.updated_at = Set(Utc::now().into());

        let updated = active
            .update(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed updating language: {err}")))?;

        self.cache.users.insert(updated.id, updated.clone()).await;
        self.to_user_response(&updated).await
    }

    /// Completes first-time onboarding and optionally renames the user's initial workspace.
    pub async fn complete_onboarding(
        &self,
        user_id: i32,
        mut req: CompleteOnboardingRequest,
    ) -> Result<UserResponse, AppError> {
        req.sanitize();
        req.validate_input()?;

        let user = self.get_user_by_id(user_id).await?;
        let active_ws_id = user.active_workspace_id;

        let mut active: users::ActiveModel = user.into();
        active.name = Set(req.name);
        active.language = Set(req.language);
        active.onboarding_completed = Set(true);
        active.updated_at = Set(Utc::now().into());

        if let Some(ws_name) = req.first_workspace_name {
            if let Some(ws_id) = active_ws_id {
                if let Some(ws) = workspaces::Entity::find_by_id(ws_id)
                    .one(&self.db)
                    .await
                    .map_err(|err| {
                        AppError::Internal(format!("Failed loading onboarding workspace: {err}"))
                    })?
                {
                    if ws.owner_id == user_id {
                        let mut ws_active: workspaces::ActiveModel = ws.into();
                        ws_active.name = Set(ws_name);
                        ws_active.updated_at = Set(Utc::now().into());
                        ws_active.update(&self.db).await.map_err(|err| {
                            AppError::Internal(format!("Failed renaming onboarding workspace: {err}"))
                        })?;
                        self.cache.invalidate_workspace_list(user_id).await;
                    }
                }
            } else {
                let unique_slug = format!("{}-{}", slugify(&ws_name), user_id);
                let created_ws = workspaces::ActiveModel {
                    owner_id: Set(user_id),
                    name: Set(ws_name),
                    slug: Set(unique_slug),
                    color: Set("#3B82F6".to_string()),
                    ..Default::default()
                }
                .insert(&self.db)
                .await
                .map_err(|err| {
                    AppError::Internal(format!("Failed creating onboarding workspace: {err}"))
                })?;

                workspace_members::ActiveModel {
                    workspace_id: Set(created_ws.id),
                    user_id: Set(user_id),
                    role: Set("owner".to_string()),
                    permissions: Set(serde_json::json!([])),
                    ..Default::default()
                }
                .insert(&self.db)
                .await
                .map_err(|err| {
                    AppError::Internal(format!("Failed inserting owner membership: {err}"))
                })?;

                active.active_workspace_id = Set(Some(created_ws.id));
                self.cache.invalidate_workspace_list(user_id).await;
            }
        }

        let updated = active
            .update(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed completing onboarding: {err}")))?;

        self.cache.users.insert(updated.id, updated.clone()).await;
        self.to_user_response(&updated).await
    }

    /// Updates `users.avatar_url` after an encrypted R2 avatar upload is confirmed.
    pub async fn sync_uploaded_avatar(
        &self,
        user_id: i32,
        public_url: String,
    ) -> Result<(), AppError> {
        let user = self.get_user_by_id(user_id).await?;
        let mut active: users::ActiveModel = user.into();
        active.avatar_url = Set(Some(public_url));
        active.updated_at = Set(Utc::now().into());
        let updated = active
            .update(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed syncing avatar URL: {err}")))?;
        self.cache.users.insert(updated.id, updated).await;
        Ok(())
    }

    /// Hydrates full `SessionBootstrapData` for SSR shell injection and `/api/v1/auth/me`.
    pub async fn get_session_bootstrap(
        &self,
        user_id: i32,
        is_hot_support: bool,
        permission_service: &PermissionService,
    ) -> Result<SessionBootstrapData, AppError> {
        let mut user = self.get_user_by_id(user_id).await?;

        let mut active_ws_id = user.active_workspace_id;
        let mut active_access = match active_ws_id {
            Some(ws_id) => {
                permission_service
                    .resolve_user_workspace_access(user_id, ws_id)
                    .await?
            }
            None => None,
        };

        if active_access.is_none() {
            let fallback_ws_id = self.find_first_accessible_workspace_id(user_id).await?;
            if fallback_ws_id != user.active_workspace_id {
                let mut active: users::ActiveModel = user.clone().into();
                active.active_workspace_id = Set(fallback_ws_id);
                active.updated_at = Set(Utc::now().into());
                user = active.update(&self.db).await.map_err(|err| {
                    AppError::Internal(format!("Failed updating fallback workspace: {err}"))
                })?;
                self.cache.users.insert(user.id, user.clone()).await;
            }
            active_ws_id = fallback_ws_id;
            if let Some(ws_id) = active_ws_id {
                active_access = permission_service
                    .resolve_user_workspace_access(user_id, ws_id)
                    .await?;
            }
        }

        let active_workspace_name = match active_ws_id {
            Some(ws_id) => workspaces::Entity::find_by_id(ws_id)
                .one(&self.db)
                .await
                .map_err(|err| AppError::Internal(format!("Failed loading workspace name: {err}")))?
                .map(|ws| ws.name),
            None => None,
        };

        let (active_workspace_role, permissions) = match active_access {
            Some((role, perms, _)) => (Some(role), perms),
            None => (None, Vec::new()),
        };

        let user_response = self.to_user_response(&user).await?;
        let unread_notifications = self
            .notification_service
            .count_unread(user.id, UserRole::from_db(&user.role))
            .await?;

        Ok(SessionBootstrapData {
            user: user_response,
            active_workspace_id: active_ws_id,
            active_workspace_name,
            active_workspace_role,
            permissions,
            unread_notifications,
            is_hot_support,
        })
    }

    /// Finds the oldest workspace owned by or shared with `user_id`.
    async fn find_first_accessible_workspace_id(&self, user_id: i32) -> Result<Option<i32>, AppError> {
        if let Some(owned) = workspaces::Entity::find()
            .filter(workspaces::Column::OwnerId.eq(user_id))
            .order_by_asc(workspaces::Column::Id)
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed finding owned workspace: {err}")))?
        {
            return Ok(Some(owned.id));
        }

        let membership = workspace_members::Entity::find()
            .filter(workspace_members::Column::UserId.eq(user_id))
            .order_by_asc(workspace_members::Column::WorkspaceId)
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed finding member workspace: {err}")))?;

        Ok(membership.map(|m| m.workspace_id))
    }
}
