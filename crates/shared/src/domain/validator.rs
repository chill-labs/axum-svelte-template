// =============================================================================
// INPUT VALIDATORS
// =============================================================================
//
// Custom format validation helpers returning typed `ErrorCode` failures or
// field maps compatible with frontend localized validation display.

use std::collections::HashMap;
use std::sync::LazyLock;

// `regex` - Precompiled static regex patterns for slug and hex-color validation.
use regex::Regex;

use crate::domain::error::AppError;
use crate::domain::error_code::ErrorCode;

static SLUG_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z0-9]+(?:-[a-z0-9]+)*$").expect("valid slug regex"));

static HEX_COLOR_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^#[0-9A-Fa-f]{6}$").expect("valid hex color regex"));

static EMAIL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}$").expect("valid email regex")
});

/// Validates basic email syntax without heap-heavy allocations.
pub fn is_valid_email(email: &str) -> bool {
    let trimmed = email.trim();
    !trimmed.is_empty() && trimmed.len() <= 255 && EMAIL_REGEX.is_match(trimmed)
}

/// Validates URL-safe kebab-case slugs (`[a-z0-9-]`).
pub fn is_valid_slug(slug: &str) -> bool {
    !slug.is_empty() && slug.len() <= 140 && SLUG_REGEX.is_match(slug)
}

/// Validates `#RRGGBB` color values.
pub fn is_valid_hex_color(color: &str) -> bool {
    HEX_COLOR_REGEX.is_match(color)
}

/// Checks password minimum strength (at least 8 characters).
pub fn is_valid_password(password: &str) -> bool {
    password.len() >= 8 && password.len() <= 128
}

/// Generates a clean lowercase kebab-case slug base from an arbitrary display name.
pub fn slugify(input: &str) -> String {
    let mut slug = String::new();
    let mut prev_dash = false;
    for ch in input.trim().to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            prev_dash = false;
        } else if !prev_dash && !slug.is_empty() {
            slug.push('-');
            prev_dash = true;
        }
    }
    let cleaned = slug.trim_matches('-').to_string();
    if cleaned.is_empty() {
        "workspace".to_string()
    } else {
        cleaned
    }
}

/// Converts a field error map into `Result<(), AppError>` if non-empty.
pub fn finish_validation(errors: HashMap<String, ErrorCode>) -> Result<(), AppError> {
    if errors.is_empty() {
        Ok(())
    } else {
        Err(AppError::FieldErrors(errors))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validators_accept_and_reject_expected_inputs() {
        assert!(is_valid_email("teacher@example.com"));
        assert!(!is_valid_email("invalid-email"));
        assert!(is_valid_slug("team-alpha-2026"));
        assert!(!is_valid_slug("Team Alpha!"));
        assert!(is_valid_hex_color("#3B82F6"));
        assert!(!is_valid_hex_color("blue"));
        assert_eq!(slugify("  My First Workspace!! "), "my-first-workspace");
    }
}
