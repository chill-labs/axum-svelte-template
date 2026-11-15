// =============================================================================
// HTTP MIDDLEWARE MODULE
// =============================================================================
//
// Exports Axum middleware layers and session extraction helpers shared between
// `services` and `crates/server`.

// SECTION: PUBLIC SUBMODULES

/// Cookie-backed session resolution, Hot Support iframe isolation, and auth middleware.
pub mod auth;
