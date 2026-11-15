// =============================================================================
// SERVICES UTILITIES MODULE
// =============================================================================
//
// Cryptographic and security helper primitives for backend services:
// - `password`: Argon2id password hashing and constant-time verification
// - `token`: Cryptographic RNG token generation and SHA-256 session hashing
//
// ## Architecture Notes
//
// Kept separate from `shared::utils` because `argon2` and `sha2` are server-only
// dependencies not compiled into client bundles.

// SECTION: PUBLIC SUBMODULES

/// Argon2id password hashing and verification utilities.
pub mod password;

/// Session token generation and SHA-256 hashing utilities.
pub mod token;
