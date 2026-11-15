// =============================================================================
// NOTIFICATION & REAL-TIME SSE DTOs
// =============================================================================
//
// Per-user notifications, admin-published global announcements, and typed SSE
// events broadcast across `NotificationHub`.

// `chrono` - Notification timestamps.
use chrono::{DateTime, Utc};

// `serde` - JSON payloads for notification endpoints and SSE frames.
use serde::{Deserialize, Serialize};

/// Target audience for global announcement banners/notifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum GlobalNotificationAudience {
    #[default]
    All,
    Authenticated,
    Admins,
}

impl GlobalNotificationAudience {
    /// Converts stored DB audience string into enum variant.
    pub fn from_db(val: &str) -> Self {
        match val {
            "authenticated" => Self::Authenticated,
            "admins" => Self::Admins,
            _ => Self::All,
        }
    }

    /// Canonical DB value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Authenticated => "authenticated",
            Self::Admins => "admins",
        }
    }
}

/// In-app user notification record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationResponse {
    pub id: i32,
    pub user_id: i32,
    pub workspace_id: Option<i32>,
    pub kind: String,
    pub title: String,
    pub body: String,
    pub link_url: Option<String>,
    pub is_read: bool,
    pub created_at: DateTime<Utc>,
}

/// Combined notification dropdown feed (personal + active global broadcasts).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationFeedResponse {
    pub items: Vec<NotificationResponse>,
    pub global_items: Vec<GlobalAnnouncementResponse>,
    pub unread_count: u64,
}

/// Public/authenticated view of an active global announcement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalAnnouncementResponse {
    pub id: i32,
    pub title: String,
    pub html_content: String,
    pub target_audience: GlobalNotificationAudience,
    pub published_at: DateTime<Utc>,
    pub is_read: bool,
}

/// Typed real-time SSE event streamed on `/api/notifications/sse`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "payload", rename_all = "snake_case")]
pub enum RealtimeEvent {
    /// Newly delivered user-specific notification.
    UserNotification(NotificationResponse),
    /// Global HTML announcement published by admin.
    GlobalAnnouncement(GlobalAnnouncementResponse),
    /// Workspace Todo list mutated (triggers lightweight frontend refetch).
    TodoChanged { workspace_id: i32, todo_id: i32 },
    /// Periodic keep-alive heartbeat.
    KeepAlive,
}
