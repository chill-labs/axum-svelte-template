// =============================================================================
// UPLOAD PURPOSES & R2 OBJECT KEY GENERATOR
// =============================================================================
//
// Centralizes valid upload targets, maximum file sizes, MIME types, and
// deterministic R2 storage key paths (`media_files.storage_key`).

// `serde` - Serializes upload purposes across JSON presign requests.
use serde::{Deserialize, Serialize};

// `uuid` - Generates collision-free object names inside each entity prefix.
use uuid::Uuid;

/// Maximum permitted encrypted image upload size (`5 MiB`).
pub const MAX_IMAGE_UPLOAD_BYTES: i64 = 5 * 1024 * 1024;

/// Allowed client-compressed MIME types for XOR-protected image uploads.
pub const ALLOWED_IMAGE_CONTENT_TYPES: &[&str] = &[
    "image/webp",
    "image/png",
    "image/jpeg",
    "application/octet-stream",
];

/// Domain upload purpose matching `media_files.entity_type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UploadPurpose {
    UserAvatar,
    WorkspaceLogo,
    TodoAttachment,
}

impl UploadPurpose {
    /// Returns the canonical `entity_type` string stored in `media_files`.
    pub const fn entity_type(self) -> &'static str {
        match self {
            Self::UserAvatar => "user_avatar",
            Self::WorkspaceLogo => "workspace_logo",
            Self::TodoAttachment => "todo_attachment",
        }
    }

    /// Returns folder prefix inside Cloudflare R2.
    pub const fn folder_prefix(self) -> &'static str {
        match self {
            Self::UserAvatar => "avatars",
            Self::WorkspaceLogo => "workspaces",
            Self::TodoAttachment => "todos",
        }
    }
}

/// Builds a collision-free object key in R2 (e.g. `todos/42/<uuid>.enc`).
pub fn generate_object_key(purpose: UploadPurpose, entity_id: i32) -> String {
    format!(
        "{}/{}/{}.enc",
        purpose.folder_prefix(),
        entity_id,
        Uuid::new_v4()
    )
}
