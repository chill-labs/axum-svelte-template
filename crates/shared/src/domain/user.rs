// =============================================================================
// USER, AUTHENTICATION & BOOTSTRAP DTOs
// =============================================================================
//
// Defines user profile responses, `window.__APP_BOOTSTRAP__` session payloads,
// Test Account password login, profile updates, and Google reauth actions.

use std::collections::HashMap;

// `chrono` - Timestamp serialization for audit and login fields.
use chrono::{DateTime, Utc};

// `serde` - JSON serialization across SSR bootstrap and REST handlers.
use serde::{Deserialize, Serialize};

use crate::domain::error::AppError;
use crate::domain::error_code::ErrorCode;
use crate::domain::permission::Permission;
use crate::domain::request::{
    deserialize_double_option, normalize_whitespace, sanitize_double_opt, sanitize_opt, Sanitize,
};
use crate::domain::validator::{finish_validation, is_valid_email, is_valid_password};

/// User account role (`user` or `admin`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    #[default]
    User,
    Admin,
}

impl UserRole {
    /// Parses database string into `UserRole`.
    pub fn from_db(val: &str) -> Self {
        if val.eq_ignore_ascii_case("admin") {
            Self::Admin
        } else {
            Self::User
        }
    }

    /// Canonical lowercase DB value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Admin => "admin",
        }
    }
}

/// Public/authenticated user profile DTO returned by `/api/v1/auth/me` and `/api/v1/profile`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: i32,
    pub email: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub avatar_key: Option<String>,
    pub role: UserRole,
    pub email_verified: bool,
    pub has_google: bool,
    pub language: String,
    pub is_test_account: bool,
    pub active_workspace_id: Option<i32>,
    pub onboarding_completed: bool,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Complete session context embedded into `window.__APP_BOOTSTRAP__` and `/api/v1/auth/me`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionBootstrapData {
    pub user: UserResponse,
    pub active_workspace_id: Option<i32>,
    pub active_workspace_name: Option<String>,
    pub active_workspace_role: Option<String>,
    pub permissions: Vec<Permission>,
    pub unread_notifications: u64,
    pub is_hot_support: bool,
}

/// Login request exclusively for admin-provisioned Test Accounts (`/auth/login-password`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginPasswordRequest {
    pub email: String,
    pub password: String,
}

impl Sanitize for LoginPasswordRequest {
    fn sanitize(&mut self) {
        self.email = self.email.trim().to_lowercase();
    }
}

impl LoginPasswordRequest {
    /// Validates email and non-empty password.
    pub fn validate_input(&self) -> Result<(), AppError> {
        let mut errors = HashMap::new();
        if !is_valid_email(&self.email) {
            errors.insert("email".to_string(), ErrorCode::InvalidEmail);
        }
        if self.password.is_empty() {
            errors.insert("password".to_string(), ErrorCode::FieldRequired);
        }
        finish_validation(errors)
    }
}

/// Profile update request supporting three-state avatar URL updates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProfileRequest {
    pub name: String,
    pub language: Option<String>,
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub avatar_url: Option<Option<String>>,
}

impl Sanitize for UpdateProfileRequest {
    fn sanitize(&mut self) {
        self.name = normalize_whitespace(&self.name);
        sanitize_opt(&mut self.language);
        sanitize_double_opt(&mut self.avatar_url);
    }
}

impl UpdateProfileRequest {
    /// Validates display name length and optional language code (`vi` or `en`).
    pub fn validate_input(&self) -> Result<(), AppError> {
        let mut errors = HashMap::new();
        if self.name.is_empty() || self.name.len() > 100 {
            errors.insert("name".to_string(), ErrorCode::ValidationFailed);
        }
        if let Some(ref lang) = self.language {
            if lang != "vi" && lang != "en" {
                errors.insert("language".to_string(), ErrorCode::ValidationFailed);
            }
        }
        finish_validation(errors)
    }
}

/// Lightweight preference update for language selector (`vi` / `en`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateLanguageRequest {
    pub language: String,
}

impl Sanitize for UpdateLanguageRequest {
    fn sanitize(&mut self) {
        self.language = self.language.trim().to_lowercase();
    }
}

impl UpdateLanguageRequest {
    /// Accepts `vi` or `en`.
    pub fn validate_input(&self) -> Result<(), AppError> {
        if self.language != "vi" && self.language != "en" {
            let mut errors = HashMap::new();
            errors.insert("language".to_string(), ErrorCode::ValidationFailed);
            return Err(AppError::FieldErrors(errors));
        }
        Ok(())
    }
}

/// Onboarding completion payload capturing preferred name and language.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteOnboardingRequest {
    pub name: String,
    pub language: String,
    pub first_workspace_name: Option<String>,
}

impl Sanitize for CompleteOnboardingRequest {
    fn sanitize(&mut self) {
        self.name = normalize_whitespace(&self.name);
        self.language = self.language.trim().to_lowercase();
        if let Some(ref mut ws) = self.first_workspace_name {
            *ws = normalize_whitespace(ws);
        }
        sanitize_opt(&mut self.first_workspace_name);
    }
}

impl CompleteOnboardingRequest {
    /// Validates onboarding fields.
    pub fn validate_input(&self) -> Result<(), AppError> {
        let mut errors = HashMap::new();
        if self.name.is_empty() || self.name.len() > 100 {
            errors.insert("name".to_string(), ErrorCode::FieldRequired);
        }
        if self.language != "vi" && self.language != "en" {
            errors.insert("language".to_string(), ErrorCode::ValidationFailed);
        }
        finish_validation(errors)
    }
}

/// Initiates Google `purpose=reauth` verification before storing a local password.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitiatePasswordReauthRequest {
    pub new_password: String,
}

impl InitiatePasswordReauthRequest {
    /// Ensures strong password requirements before queueing pending reauth token.
    pub fn validate_input(&self) -> Result<(), AppError> {
        if !is_valid_password(&self.new_password) {
            let mut errors = HashMap::new();
            errors.insert("new_password".to_string(), ErrorCode::PasswordTooWeak);
            return Err(AppError::FieldErrors(errors));
        }
        Ok(())
    }
}

/// Redirect URL returned after storing a pending reauth credential hash.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReauthRedirectResponse {
    pub redirect_url: String,
}
