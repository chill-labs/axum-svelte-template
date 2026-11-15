// =============================================================================
// PASSWORD HASHING UTILITIES
// =============================================================================
//
// Provides memory-hard Argon2id password hashing and constant-time verification
// for admin-provisioned Test Accounts and Google-reauth local passwords.
//
// ## Key Concepts
//
// - **Argon2id**: Recommended OWASP password hash using per-password 128-bit OS
//   CSPRNG salts stored in standard PHC string format.
// - **Typed Errors**: Translates Argon2 failures directly into `AppError` so
//   domain services can propagate with `?`.
//
// ## Architecture Notes
//
// Used by `UserService`, `AdminService`, and dev-only `/api/test/hash-password`.

// SECTION: IMPORTS

// `argon2` - Argon2id password hashing and PHC verification traits.
// - `OsRng` - Operating system cryptographic RNG for unique per-hash salts
// - `PasswordHash` - Parsed PHC string wrapper for verification
// - `SaltString` - Base64-encoded random salt container
// - `Argon2` - Default Argon2id parameter profile
// - `PasswordHasher` - Trait providing `hash_password`
// - `PasswordVerifier` - Trait providing constant-time `verify_password`
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{Error as PasswordHashError, PasswordHash, SaltString};
use argon2::{Argon2, PasswordHasher, PasswordVerifier};

// `shared` - Standardized service error wrapper.
// - `AppError` - Wraps unexpected hashing failures as `Internal`
use shared::domain::error::AppError;

// SECTION: PASSWORD HASHING & VERIFICATION

/// Hashes a plaintext password with Argon2id and a unique random 128-bit salt.
///
/// # Arguments
///
/// * `password` - Plaintext secret to hash
///
/// # Returns
///
/// A PHC-formatted Argon2id hash string (`$argon2id$...`).
///
/// # Errors
///
/// Returns `AppError::Internal` if Argon2 hashing fails unexpectedly.
pub fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|err| AppError::Internal(format!("Password hashing failed: {err}")))?;
    Ok(hash.to_string())
}

/// Verifies a plaintext password against a stored PHC Argon2 hash string.
///
/// # Arguments
///
/// * `password` - Candidate plaintext password
/// * `hash` - Stored PHC-formatted hash string
///
/// # Returns
///
/// - `Ok(true)` when the password matches the stored hash
/// - `Ok(false)` when the password does not match
///
/// # Errors
///
/// Returns `AppError::Internal` if the stored PHC hash string is malformed or
/// verification encounters an internal algorithm error.
pub fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|err| AppError::Internal(format!("Invalid password hash format: {err}")))?;

    match Argon2::default().verify_password(password.as_bytes(), &parsed_hash) {
        Ok(()) => Ok(true),
        Err(PasswordHashError::Password) => Ok(false),
        Err(err) => Err(AppError::Internal(format!(
            "Password verification failed: {err}"
        ))),
    }
}

// SECTION: UNIT TESTS

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_and_verify_roundtrip() {
        let hash = hash_password("CorrectHorseBatteryStaple!99").expect("hashing should succeed");
        assert!(hash.starts_with("$argon2id$"));
        assert!(verify_password("CorrectHorseBatteryStaple!99", &hash).expect("verify ok"));
        assert!(!verify_password("WrongPassword123", &hash).expect("verify mismatch ok"));
    }

    #[test]
    fn unique_salts_produce_distinct_hashes() {
        let h1 = hash_password("SamePassword123").expect("hash 1");
        let h2 = hash_password("SamePassword123").expect("hash 2");
        assert_ne!(h1, h2);
        assert!(verify_password("SamePassword123", &h1).expect("verify 1"));
        assert!(verify_password("SamePassword123", &h2).expect("verify 2"));
    }

    #[test]
    fn malformed_hash_returns_internal_error() {
        let err = verify_password("any", "not-a-valid-phc-hash").unwrap_err();
        assert!(matches!(err, AppError::Internal(_)));
    }
}
