// =============================================================================
// HTTP COOKIE HELPERS
// =============================================================================
//
// Parses incoming `Cookie` headers and constructs hardened `Set-Cookie` header
// values respecting `ServerEnv` (`Secure` in staging/prod, `HttpOnly`, `SameSite=Lax`).

// `axum::http` - Extracts `COOKIE` header values from incoming requests.
use axum::http::{header::COOKIE, HeaderMap};

// `cookie` - Safe URL-aware cookie serialization and parsing.
use cookie::{Cookie, SameSite};

use shared::config::settings::Settings;

/// Extracts the value of the named cookie from an HTTP `HeaderMap`.
pub fn extract_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    for value in headers.get_all(COOKIE) {
        let Ok(raw) = value.to_str() else {
            continue;
        };
        for part in raw.split(';') {
            let trimmed = part.trim();
            if let Some((k, v)) = trimmed.split_once('=') {
                if k.trim() == name {
                    let val = v.trim();
                    if !val.is_empty() {
                        return Some(val.to_string());
                    }
                }
            }
        }
    }
    None
}

/// Builds an `HttpOnly`, `SameSite=Lax` `Set-Cookie` header string.
pub fn build_session_cookie(
    settings: &Settings,
    name: &str,
    value: &str,
    max_age_secs: i64,
) -> String {
    let mut cookie_builder = Cookie::build((name.to_string(), value.to_string()))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(cookie::time::Duration::seconds(max_age_secs));

    if settings.env.use_secure_cookies() {
        cookie_builder = cookie_builder.secure(true);
    }

    cookie_builder.build().to_string()
}

/// Builds an expired `Set-Cookie` header clearing the named cookie immediately.
pub fn clear_session_cookie(settings: &Settings, name: &str) -> String {
    let mut cookie_builder = Cookie::build((name.to_string(), String::new()))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(cookie::time::Duration::seconds(0));

    if settings.env.use_secure_cookies() {
        cookie_builder = cookie_builder.secure(true);
    }

    cookie_builder.build().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn extract_cookie_reads_matching_name() {
        let mut headers = HeaderMap::new();
        headers.insert(
            COOKIE,
            HeaderValue::from_static("theme=dark; auth-session=abc123; lang=en"),
        );
        assert_eq!(
            extract_cookie(&headers, "auth-session"),
            Some("abc123".to_string())
        );
        assert_eq!(extract_cookie(&headers, "missing"), None);
    }
}
