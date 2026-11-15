// =============================================================================
// ENTITY -> DOMAIN DTO CONVERSIONS
// =============================================================================
//
// Explicit mapping helpers translating SeaORM rows plus resolved media keys and
// permissions into API response DTOs (avoiding blind `From` conversions that omit
// polymorphic media keys).

// `chrono` - Normalizes DB `FixedOffset` timestamps into UTC.
use chrono::Utc;

use shared::domain::admin::{AdminGlobalNotificationResponse, AdminTestAccountResponse};
use shared::domain::media::MediaFileResponse;
use shared::domain::notification::{
    GlobalAnnouncementResponse, GlobalNotificationAudience, NotificationResponse,
};
use shared::domain::permission::Permission;
use shared::domain::todo::{TodoPriority, TodoResponse, TodoStatus};
use shared::domain::user::{UserResponse, UserRole};

use super::{global_notifications, media_files, notifications, todos, users};

/// Maps a `users::Model` and optional encrypted media key into `UserResponse`.
pub fn map_user_response(model: &users::Model, avatar_key: Option<String>) -> UserResponse {
    UserResponse {
        id: model.id,
        email: model.email.clone(),
        name: model.name.clone(),
        avatar_url: model.avatar_url.clone(),
        avatar_key,
        role: UserRole::from_db(&model.role),
        email_verified: model.email_verified,
        has_google: model.google_id.is_some(),
        language: model.language.clone(),
        is_test_account: model.is_test_account,
        active_workspace_id: model.active_workspace_id,
        onboarding_completed: model.onboarding_completed,
        last_login_at: model.last_login_at.map(|ts| ts.with_timezone(&Utc)),
        created_at: model.created_at.with_timezone(&Utc),
    }
}

/// Maps a `users::Model` row into `AdminTestAccountResponse`.
pub fn map_test_account_response(model: &users::Model) -> AdminTestAccountResponse {
    AdminTestAccountResponse {
        id: model.id,
        email: model.email.clone(),
        name: model.name.clone(),
        role: UserRole::from_db(&model.role),
        language: model.language.clone(),
        active_workspace_id: model.active_workspace_id,
        last_login_at: model.last_login_at.map(|ts| ts.with_timezone(&Utc)),
        created_at: model.created_at.with_timezone(&Utc),
    }
}

/// Maps a `todos::Model` + creator/assignee names + optional attachment key into `TodoResponse`.
pub fn map_todo_response(
    model: &todos::Model,
    created_by_name: String,
    assigned_to_name: Option<String>,
    attachment_key: Option<String>,
) -> TodoResponse {
    TodoResponse {
        id: model.id,
        workspace_id: model.workspace_id,
        created_by: model.created_by,
        created_by_name,
        assigned_to: model.assigned_to,
        assigned_to_name,
        title: model.title.clone(),
        description: model.description.clone(),
        status: TodoStatus::from_db(&model.status),
        priority: TodoPriority::from_db(&model.priority),
        due_date: model.due_date,
        is_completed: model.is_completed,
        completed_at: model.completed_at.map(|ts| ts.with_timezone(&Utc)),
        position: model.position,
        attachment_key,
        created_at: model.created_at.with_timezone(&Utc),
        updated_at: model.updated_at.with_timezone(&Utc),
    }
}

/// Maps a `notifications::Model` into `NotificationResponse`.
pub fn map_notification_response(model: &notifications::Model) -> NotificationResponse {
    NotificationResponse {
        id: model.id,
        user_id: model.user_id,
        workspace_id: model.workspace_id,
        kind: model.kind.clone(),
        title: model.title.clone(),
        body: model.body.clone(),
        link_url: model.link_url.clone(),
        is_read: model.is_read,
        created_at: model.created_at.with_timezone(&Utc),
    }
}

/// Maps a `global_notifications::Model` into `AdminGlobalNotificationResponse`.
pub fn map_admin_global_notification(
    model: &global_notifications::Model,
) -> AdminGlobalNotificationResponse {
    AdminGlobalNotificationResponse {
        id: model.id,
        title: model.title.clone(),
        html_content: model.html_content.clone(),
        target_audience: GlobalNotificationAudience::from_db(&model.target_audience),
        is_active: model.is_active,
        published_at: model.published_at.with_timezone(&Utc),
        expires_at: model.expires_at.map(|ts| ts.with_timezone(&Utc)),
        created_at: model.created_at.with_timezone(&Utc),
    }
}

/// Maps a `global_notifications::Model` + read flag into `GlobalAnnouncementResponse`.
pub fn map_global_announcement(
    model: &global_notifications::Model,
    is_read: bool,
) -> GlobalAnnouncementResponse {
    GlobalAnnouncementResponse {
        id: model.id,
        title: model.title.clone(),
        html_content: model.html_content.clone(),
        target_audience: GlobalNotificationAudience::from_db(&model.target_audience),
        published_at: model.published_at.with_timezone(&Utc),
        is_read,
    }
}

/// Maps a `media_files::Model` + R2 public URL into `MediaFileResponse`.
pub fn map_media_file_response(
    model: &media_files::Model,
    public_media_base_url: &str,
) -> MediaFileResponse {
    let public_url = if public_media_base_url.is_empty() {
        format!("/api/v1/media/blob/{}", model.id)
    } else {
        format!("{}/{}", public_media_base_url.trim_end_matches('/'), model.storage_key)
    };

    MediaFileResponse {
        id: model.id,
        entity_type: model.entity_type.clone(),
        entity_id: model.entity_id,
        field_name: model.field_name.clone(),
        storage_key: model.storage_key.clone(),
        xor_key: model.xor_key.clone(),
        content_type: model.content_type.clone(),
        file_size: model.file_size,
        public_url,
        created_at: model.created_at.with_timezone(&Utc),
    }
}

/// Parses a `serde_json::Value` permission JSONB array safely into `Vec<Permission>`.
pub fn parse_permissions_json(raw: &serde_json::Value) -> Vec<Permission> {
    serde_json::from_value::<Vec<Permission>>(raw.clone()).unwrap_or_default()
}
