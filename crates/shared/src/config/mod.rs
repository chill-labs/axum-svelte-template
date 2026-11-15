// =============================================================================
// CONFIGURATION MODULE
// =============================================================================
//
// Exports centralized route metadata and strongly typed environment settings.

/// Route definitions and auth classification (`Public`, `GuestOnly`, `Protected`).
pub mod routes;

/// Hierarchical configuration loaded from environment variables (`APP__*`).
pub mod settings;
