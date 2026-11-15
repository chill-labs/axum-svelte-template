// =============================================================================
// DOMAIN MODULE EXPORTS
// =============================================================================
//
// Aggregates infrastructure envelopes and domain-specific DTOs exposed through
// Axum API routes and translated into TypeScript types by `gen-ts`.

/// Admin console DTOs (`users`, `hot-support`, `test-accounts`, `global-notifications`).
pub mod admin;
/// Domain error wrapper and Axum `ToResponse` trait.
pub mod error;
/// Typed `ErrorCode` enum parsed by `gen-ts`.
pub mod error_code;
/// Healthcheck status DTO.
pub mod healthcheck;
/// Protected media and optional R2 presign/confirm DTOs.
pub mod media;
/// Real-time in-app notifications, global announcements, and SSE events.
pub mod notification;
/// Granular workspace RBAC permissions (`PermResource` × `PermAction`).
pub mod permission;
/// Sanitization, pagination, sorting, and `deserialize_double_option`.
pub mod request;
/// Standardized `ApiResponse<T>` envelope.
pub mod response;
/// Todo CRUD, filter query, summary stats, and reorder DTOs.
pub mod todo;
/// Upload purpose enum and collision-free R2 key builder.
pub mod upload;
/// User session bootstrap, profile, onboarding, reauth, and password test-login DTOs.
pub mod user;
/// Email, slug, hex-color, and password validators.
pub mod validator;
/// Multi-tenant workspace and member collaboration DTOs.
pub mod workspace;
