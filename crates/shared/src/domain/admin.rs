// =============================================================================
// ADMIN CONSOLE DTOs
// =============================================================================
//
// Models powering the 4 Admin Console tabs:
// 1. Users (`AdminUserEntry`, `AdminUpdateUserRoleRequest`)
// 2. Hot Support (`HotSupportLoginRequest`, `HotSupportSessionInfo`)
// 3. Test Accounts (`AdminTestAccountEntry`, CRUD requests)
// 4. Global Notifications (`AdminGlobalNotificationEntry`, CRUD + broadcast)

use std::collections::HashMap;

// `chrono` - Admin listing timestamps.
use chrono::{DateTime, Utc};

// `serde` - Serialization across admin JSON handlers.
use serde::{Deserialize, Serialize};

use crate::domain::error::AppError;
use crate::domain::error_code::ErrorCode;
use crate::domain::notification::GlobalNotificationAudience;
use crate::domain::request::{normalize_whitespace, sanitize_opt, Sanitize};
use crate::domain::user::UserRole;
use crate::domain::validator::{finish_validation, is_valid_email, is_valid_password};

/// Query params for listing users in `/admin`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GetAdminUsersQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub search: Option<String>,
    pub role: Option<UserRole>,
}

/// User row shown in `AdminUsersTab`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminUserResponse {
    pub id: i32,
    pub email: String,
    pub name: String,
    pub role: UserRole,
    pub is_test_account: bool,
    pub onboarding_completed: bool,
    pub workspace_count: i64,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Request updating a user's system role (`user` <-> `admin`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminUpdateUserRoleRequest {
    pub role: UserRole,
}

/// Request initiating an isolated Hot Support impersonation session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotSupportLoginRequest {
    pub user_id: i32,
}

/// Active Hot Support session metadata returned by `/api/v1/admin/hot-support/*`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotSupportSessionInfo {
    pub active: bool,
    pub target_user_id: Option<i32>,
    pub target_user_name: Option<String>,
    pub target_user_email: Option<String>,
    pub launch_url: Option<String>,
}

/// Test account entry shown in `AdminTestAccountsTab`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminTestAccountResponse {
    pub id: i32,
    pub email: String,
    pub name: String,
    pub role: UserRole,
    pub language: String,
    pub active_workspace_id: Option<i32>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Request creating a local password-enabled Test Account in `/admin`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTestAccountRequest {
    pub email: String,
    pub name: String,
    pub password: String,
    pub role: Option<UserRole>,
    pub language: Option<String>,
}

impl Sanitize for CreateTestAccountRequest {
    fn sanitize(&mut self) {
        self.email = self.email.trim().to_lowercase();
        self.name = normalize_whitespace(&self.name);
        sanitize_opt(&mut self.language);
    }
}

impl CreateTestAccountRequest {
    /// Validates test account inputs.
    pub fn validate_input(&self) -> Result<(), AppError> {
        let mut errors = HashMap::new();
        if !is_valid_email(&self.email) {
            errors.insert("email".to_string(), ErrorCode::InvalidEmail);
        }
        if self.name.is_empty() || self.name.len() > 100 {
            errors.insert("name".to_string(), ErrorCode::FieldRequired);
        }
        if !is_valid_password(&self.password) {
            errors.insert("password".to_string(), ErrorCode::PasswordTooWeak);
        }
        finish_validation(errors)
    }
}

/// Request updating an existing Test Account (optional password rotation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTestAccountRequest {
    pub name: Option<String>,
    pub password: Option<String>,
    pub role: Option<UserRole>,
    pub language: Option<String>,
}

impl Sanitize for UpdateTestAccountRequest {
    fn sanitize(&mut self) {
        if let Some(ref mut name) = self.name {
            *name = normalize_whitespace(name);
        }
        sanitize_opt(&mut self.password);
        sanitize_opt(&mut self.language);
    }
}

impl UpdateTestAccountRequest {
    /// Validates provided optional updates.
    pub fn validate_input(&self) -> Result<(), AppError> {
        let mut errors = HashMap::new();
        if let Some(ref name) = self.name {
            if name.is_empty() || name.len() > 100 {
                errors.insert("name".to_string(), ErrorCode::FieldRequired);
            }
        }
        if let Some(ref password) = self.password {
            if !is_valid_password(password) {
                errors.insert("password".to_string(), ErrorCode::PasswordTooWeak);
            }
        }
        finish_validation(errors)
    }
}

/// Admin management record for a global HTML notification banner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminGlobalNotificationResponse {
    pub id: i32,
    pub title: String,
    pub html_content: String,
    pub target_audience: GlobalNotificationAudience,
    pub is_active: bool,
    pub published_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Request publishing a new global HTML announcement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateGlobalNotificationRequest {
    pub title: String,
    pub html_content: String,
    pub target_audience: GlobalNotificationAudience,
    pub is_active: Option<bool>,
}

impl Sanitize for CreateGlobalNotificationRequest {
    fn sanitize(&mut self) {
        self.title = normalize_whitespace(&self.title);
        self.html_content = self.html_content.trim().to_string();
    }
}

impl CreateGlobalNotificationRequest {
    /// Validates required announcement title and HTML body.
    pub fn validate_input(&self) -> Result<(), AppError> {
        let mut errors = HashMap::new();
        if self.title.is_empty() || self.title.len() > 200 {
            errors.insert("title".to_string(), ErrorCode::FieldRequired);
        }
        if self.html_content.is_empty() {
            errors.insert("html_content".to_string(), ErrorCode::FieldRequired);
        }
        finish_validation(errors)
    }
}

/// Request toggling or editing an existing global HTML announcement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateGlobalNotificationRequest {
    pub title: Option<String>,
    pub html_content: Option<String>,
    pub target_audience: Option<GlobalNotificationAudience>,
    pub is_active: Option<bool>,
}
