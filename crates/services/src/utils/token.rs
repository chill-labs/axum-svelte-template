// =============================================================================
// SESSION & STATE TOKEN UTILITIES
// =============================================================================
//
// Cryptographically secure random token generation and SHA-256 digest hashing
// for browser session cookies (`auth-session`, `hot-support-session`), OAuth
// state parameters, and password-reauth confirmation tokens.
//
// ## Key Concepts
//
// - **Raw Token (`64` hex chars)**: 32 bytes from `rand::rng()` encoded as hex,
//   stored in HttpOnly cookies or single-use state caches.
// - **SHA-256 Digest (`64` hex chars)**: Stored in `sessions.id` (`VARCHAR(64)`)
//   and Moka session caches so database read leaks cannot hijack live sessions.
//
// ## Architecture Notes
//
// Never store raw session tokens in PostgreSQL or in-memory caches. Always pass
// cookies through `hash_token()` before lookup or persistence.

// SECTION: IMPORTS

// `hex` - Lowercase hexadecimal encoding for 32-byte tokens and SHA-256 digests.
// - `encode` - Encodes raw byte slices without heap overhead beyond output string

// `rand` - Cryptographic thread-local CSPRNG (`rand 0.9`).
// - `RngCore` - Trait providing `fill_bytes` for random byte generation
use rand::RngCore;

// `sha2` - Standard FIPS 180-4 SHA-256 digest computation.
// - `Digest` - Trait providing `new`, `update`, and `finalize`
// - `Sha256` - SHA-256 hash implementation
use sha2::{Digest, Sha256};

// SECTION: TOKEN FUNCTIONS

/// Generates a 256-bit cryptographically random token encoded as 64 hex chars.
pub fn generate_raw_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// Computes the lowercase 64-character SHA-256 hex digest of a raw token string.
///
/// # Arguments
///
/// * `raw_token` - Plaintext token extracted from a cookie or generated state
pub fn hash_token(raw_token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw_token.as_bytes());
    hex::encode(hasher.finalize())
}

// SECTION: UNIT TESTS

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_token_is_64_hex_chars_and_random() {
        let t1 = generate_raw_token();
        let t2 = generate_raw_token();
        assert_eq!(t1.len(), 64);
        assert_eq!(t2.len(), 64);
        assert_ne!(t1, t2);
        assert!(t1.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn hash_token_is_deterministic_64_hex_digest() {
        let raw = "example-session-token";
        let d1 = hash_token(raw);
        let d2 = hash_token(raw);
        assert_eq!(d1, d2);
        assert_eq!(d1.len(), 64);
        assert_ne!(d1, raw);
        assert_ne!(d1, hash_token("different-token"));
    }
}
