// =============================================================================
// WORKSPACE & COLLABORATOR DTOs
// =============================================================================
//
// Multi-tenant workspace payloads, workspace switching requests, and
// collaborator invitation + RBAC permission updates.

use std::collections::HashMap;

// `chrono` - Timestamps for workspace creation and member join dates.
use chrono::{DateTime, Utc};

// `serde` - Serialization for API requests and responses.
use serde::{Deserialize, Serialize};

use crate::domain::error::AppError;
use crate::domain::error_code::ErrorCode;
use crate::domain::permission::{
    default_member_permissions, editor_permissions, viewer_permissions, Permission,
};
use crate::domain::request::{
    deserialize_double_option, normalize_whitespace, sanitize_double_opt, sanitize_opt, Sanitize,
};
use crate::domain::validator::{finish_validation, is_valid_email, is_valid_hex_color, is_valid_slug};

/// Predefined role templates mapped to default `Permission` sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum MemberRolePreset {
    Editor,
    #[default]
    Member,
    Viewer,
    Custom,
}

impl MemberRolePreset {
    /// Returns the role string persisted in `workspace_members.role`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Editor => "editor",
            Self::Member => "member",
            Self::Viewer => "viewer",
            Self::Custom => "custom",
        }
    }

    /// Resolves preset permissions (returns `None` for `Custom`).
    pub fn default_permissions(self) -> Option<Vec<Permission>> {
        match self {
            Self::Editor => Some(editor_permissions()),
            Self::Member => Some(default_member_permissions()),
            Self::Viewer => Some(viewer_permissions()),
            Self::Custom => None,
        }
    }
}

/// Workspace summary DTO rendered in switcher, sidebar, and `/workspaces`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceResponse {
    pub id: i32,
    pub owner_id: i32,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub color: String,
    pub logo_key: Option<String>,
    pub is_owner: bool,
    pub my_role: String,
    pub my_permissions: Vec<Permission>,
    pub member_count: i64,
    pub todo_count: i64,
    pub completed_todo_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request payload to create a new workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkspaceRequest {
    pub name: String,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub color: Option<String>,
}

impl Sanitize for CreateWorkspaceRequest {
    fn sanitize(&mut self) {
        self.name = normalize_whitespace(&self.name);
        if let Some(ref mut s) = self.slug {
            *s = s.trim().to_lowercase();
        }
        sanitize_opt(&mut self.slug);
        sanitize_opt(&mut self.description);
        if let Some(ref mut c) = self.color {
            *c = c.trim().to_string();
        }
        sanitize_opt(&mut self.color);
    }
}

impl CreateWorkspaceRequest {
    /// Validates name length, optional slug syntax, and optional hex color.
    pub fn validate_input(&self) -> Result<(), AppError> {
        let mut errors = HashMap::new();
        if self.name.is_empty() || self.name.len() > 120 {
            errors.insert("name".to_string(), ErrorCode::FieldRequired);
        }
        if let Some(ref slug) = self.slug {
            if !is_valid_slug(slug) {
                errors.insert("slug".to_string(), ErrorCode::InvalidSlug);
            }
        }
        if let Some(ref color) = self.color {
            if !is_valid_hex_color(color) {
                errors.insert("color".to_string(), ErrorCode::InvalidHexColor);
            }
        }
        finish_validation(errors)
    }
}

/// Partial update payload for a workspace supporting explicit `null` clear on `description`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWorkspaceRequest {
    pub name: Option<String>,
    pub slug: Option<String>,
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub description: Option<Option<String>>,
    pub color: Option<String>,
}

impl Sanitize for UpdateWorkspaceRequest {
    fn sanitize(&mut self) {
        if let Some(ref mut n) = self.name {
            *n = normalize_whitespace(n);
        }
        if let Some(ref mut s) = self.slug {
            *s = s.trim().to_lowercase();
        }
        sanitize_opt(&mut self.slug);
        sanitize_double_opt(&mut self.description);
        if let Some(ref mut c) = self.color {
            *c = c.trim().to_string();
        }
    }
}

impl UpdateWorkspaceRequest {
    /// Validates provided fields.
    pub fn validate_input(&self) -> Result<(), AppError> {
        let mut errors = HashMap::new();
        if let Some(ref name) = self.name {
            if name.is_empty() || name.len() > 120 {
                errors.insert("name".to_string(), ErrorCode::FieldRequired);
            }
        }
        if let Some(ref slug) = self.slug {
            if !is_valid_slug(slug) {
                errors.insert("slug".to_string(), ErrorCode::InvalidSlug);
            }
        }
        if let Some(ref color) = self.color {
            if !is_valid_hex_color(color) {
                errors.insert("color".to_string(), ErrorCode::InvalidHexColor);
            }
        }
        finish_validation(errors)
    }
}

/// Request to switch the user's active workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchWorkspaceRequest {
    pub workspace_id: i32,
}

/// Collaborator record displayed on `/workspaces/members`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceMemberResponse {
    pub id: i32,
    pub workspace_id: i32,
    pub user_id: i32,
    pub user_name: String,
    pub user_email: String,
    pub user_avatar_url: Option<String>,
    pub user_avatar_key: Option<String>,
    pub role: String,
    pub permissions: Vec<Permission>,
    pub is_owner: bool,
    pub joined_at: DateTime<Utc>,
}

/// Request to add a collaborator by existing user email.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddWorkspaceMemberRequest {
    pub email: String,
    pub role: MemberRolePreset,
    pub permissions: Option<Vec<Permission>>,
}

impl Sanitize for AddWorkspaceMemberRequest {
    fn sanitize(&mut self) {
        self.email = self.email.trim().to_lowercase();
    }
}

impl AddWorkspaceMemberRequest {
    /// Validates collaborator email syntax.
    pub fn validate_input(&self) -> Result<(), AppError> {
        let mut errors = HashMap::new();
        if !is_valid_email(&self.email) {
            errors.insert("email".to_string(), ErrorCode::InvalidEmail);
        }
        finish_validation(errors)
    }
}

/// Request to update a collaborator's role or custom permission matrix.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMemberPermissionsRequest {
    pub role: MemberRolePreset,
    pub permissions: Vec<Permission>,
}
