// =============================================================================
// DOMAIN SERVICES MODULE
// =============================================================================
//
// Aggregates all business-logic domain services for the Axum + Svelte 5
// workspace template.

// SECTION: PUBLIC DOMAIN SUBMODULES

/// System administrator operations (users, test accounts, global notices, hot support).
pub mod admin;

/// Polymorphic encrypted R2 media management (`media_files`).
pub mod media;

/// Per-user notifications, global announcements, and unread counters.
pub mod notification;

/// Workspace-scoped Todo CRUD, filtering, stats, and drag-reorder operations.
pub mod todo;

/// User authentication, OAuth, password reauth, onboarding, and profile management.
pub mod user;

/// Multi-tenant workspace CRUD, active workspace switching, and member RBAC management.
pub mod workspace;
