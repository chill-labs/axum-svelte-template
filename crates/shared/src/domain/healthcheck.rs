// =============================================================================
// HEALTHCHECK DTO
// =============================================================================
//
// Liveness & readiness payload exposed at `/api/v1/health`.

// `serde` - Serializes health check telemetry for monitoring and `/healthcheck`.
use serde::{Deserialize, Serialize};

/// Health telemetry returned by `/api/v1/health`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthcheckResponse {
    /// Overall status (`"ok"` or `"degraded"`).
    pub status: String,
    /// Workspace crate version.
    pub version: String,
    /// PostgreSQL connectivity flag.
    pub database: bool,
    /// Whether optional Cloudflare R2 object storage is configured.
    pub storage_enabled: bool,
    /// Current environment (`dev`, `staging`, `prod`).
    pub environment: String,
    /// Current UTC timestamp ISO-8601 string.
    pub timestamp: String,
}
