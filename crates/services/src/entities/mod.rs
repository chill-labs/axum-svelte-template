// =============================================================================
// SEAORM ENTITIES & DOMAIN CONVERSIONS
// =============================================================================
//
// Handwritten SeaORM 1.1 entities matching `database/schema/01..05_*.sql`
// paired with explicit DTO conversion helpers in `conversions.rs`.

/// Explicit SeaORM Model -> shared DTO mapping functions.
pub mod conversions;
/// Global notification broadcast read receipts (`global_notification_reads`).
pub mod global_notification_reads;
/// Admin-published global HTML announcements (`global_notifications`).
pub mod global_notifications;
/// Polymorphic encrypted R2 media attachments (`media_files`).
pub mod media_files;
/// Per-user real-time notifications (`notifications`).
pub mod notifications;
/// User authentication session hashes (`sessions`).
pub mod sessions;
/// Workspace-scoped Todo items (`todos`).
pub mod todos;
/// Registered user accounts and preferences (`users`).
pub mod users;
/// Workspace memberships and JSONB permission grants (`workspace_members`).
pub mod workspace_members;
/// Multi-tenant workspace entities (`workspaces`).
pub mod workspaces;
