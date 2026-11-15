// =============================================================================
// SESSION AUTHENTICATION MIDDLEWARE & HOT SUPPORT ISOLATION
// =============================================================================
//
// Resolves authenticated users from HttpOnly cookies (`auth-session` or isolated
// `hot-support-session`), backed by Moka session/user caches with PostgreSQL
// fallback and strict expiration enforcement.
//
// ## Key Concepts
//
// - **Cookie Isolation**: Normal browser traffic reads `auth-session`. Requests
//   carrying a Hot Support iframe signal (`X-Hot-Support: 1`, `?_hs=1`, or
//   `Referer` containing `_hs=1` exclusively on `/api/notifications/sse`) strictly
//   read `hot-support-session` with zero fallback to `auth-session`.
// - **Extension Guarantee**: `auth_middleware` inserts `Option<AuthenticatedUser>`
//   into request extensions on every request so downstream Axum handlers can
//   extract `Extension<Option<AuthenticatedUser>>` safely.
//
// ## Architecture Notes
//
// Mounted globally in `crates/server` via `axum::middleware::from_fn_with_state`.

// SECTION: IMPORTS

// `axum` - Request, response, header, and middleware types.
// - `Request` - Incoming HTTP request carrying URI, headers, and extensions
// - `State` - Shared `AppState` extractor
// - `COOKIE`, `REFERER`, `HeaderMap` - Standard HTTP header inspection types
// - `Next` - Middleware continuation chain
// - `Response` - Downstream HTTP response
use axum::extract::{Request, State};
use axum::http::header::{COOKIE, REFERER};
use axum::http::HeaderMap;
use axum::middleware::Next;
use axum::response::Response;

// `chrono` - UTC clock for checking PostgreSQL session expiration.
// - `Utc` - Current UTC timestamp
use chrono::Utc;

// `cookie` - RFC-compliant cookie parsing and Set-Cookie header formatting.
// - `Cookie` - Parses cookie pairs and formats `Set-Cookie` headers cleanly
// - `SameSite` - Enforces `SameSite=Lax` on session cookies
use cookie::{Cookie, SameSite};

// `sea_orm` - Queries `sessions` and `users` on Moka cache miss.
// - `EntityTrait` - `find_by_id` and `delete_by_id` queries
use sea_orm::EntityTrait;

// `crate::entities` - SeaORM entity definitions for `sessions` and `users`.
use crate::entities::{sessions, users};

// `crate::state` - Root application state containing DB pool and Moka caches.
use crate::state::AppState;

// `crate::utils::token` - SHA-256 session cookie digest helper.
use crate::utils::token::hash_token;

// SECTION: COOKIE & HEADER CONSTANTS

/// Primary HttpOnly session cookie for standard authenticated browser requests.
pub const AUTH_COOKIE_NAME: &str = "auth-session";

/// Isolated HttpOnly session cookie scoped strictly to Hot Support impersonation iframes.
pub const HOT_SUPPORT_COOKIE_NAME: &str = "hot-support-session";

/// Header injected by Hot Support iframe `fetch` wrappers to select `hot-support-session`.
pub const HOT_SUPPORT_HEADER_NAME: &str = "x-hot-support";

/// Query parameter appended to Hot Support iframe URLs and `EventSource` SSE links.
pub const HOT_SUPPORT_QUERY_PARAM: &str = "_hs";

/// SSE stream endpoint where `Referer` query `_hs=1` is accepted for `EventSource`.
const SSE_ENDPOINT_PATH: &str = "/api/notifications/sse";

// SECTION: AUTHENTICATED USER EXTENSION

/// Resolved authenticated user attached to Axum request extensions as `Option<AuthenticatedUser>`.
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    /// Hydrated user row from cache or PostgreSQL.
    pub user: users::Model,
    /// `true` when authenticated via the isolated `hot-support-session` cookie.
    pub is_hot_support: bool,
}

// SECTION: COOKIE & SIGNAL HELPERS

/// Extracts a named cookie value across all `Cookie` headers (handling HTTP/2 split headers
/// and tolerating non-ASCII sibling cookies).
///
/// # Arguments
///
/// * `headers` - HTTP request `HeaderMap`
/// * `target_name` - Exact cookie name to extract
pub fn extract_cookie_from_headers(headers: &HeaderMap, target_name: &str) -> Option<String> {
    for cookie_header in headers.get_all(COOKIE) {
        let raw = String::from_utf8_lossy(cookie_header.as_bytes());
        for segment in raw.split(';').map(str::trim) {
            if segment.is_empty() {
                continue;
            }
            if let Ok(parsed) = Cookie::parse(segment) {
                if parsed.name() == target_name {
                    return Some(parsed.value().to_string());
                }
            } else if let Some((name, val)) = segment.split_once('=') {
                if name.trim() == target_name {
                    return Some(val.trim().trim_matches('"').to_string());
                }
            }
        }
    }
    None
}

/// Formats a hardened `Set-Cookie` header value (`HttpOnly`, `Path=/`, `SameSite=Lax`).
///
/// # Arguments
///
/// * `cookie_name` - Target cookie name (`AUTH_COOKIE_NAME` or `HOT_SUPPORT_COOKIE_NAME`)
/// * `raw_token` - Plaintext session token sent to the client browser
/// * `max_age_secs` - Cookie lifetime in seconds
/// * `secure` - Whether to append the `Secure` attribute (true outside dev mode)
pub fn build_session_cookie_header(
    cookie_name: &str,
    raw_token: &str,
    max_age_secs: i64,
    secure: bool,
) -> String {
    let mut builder = Cookie::build((cookie_name.to_string(), raw_token.to_string()))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(cookie::time::Duration::seconds(max_age_secs));

    if secure {
        builder = builder.secure(true);
    }

    builder.build().to_string()
}

/// Formats an expired `Set-Cookie` header value to clear a browser session cookie.
pub fn build_clear_cookie_header(cookie_name: &str, secure: bool) -> String {
    build_session_cookie_header(cookie_name, "", 0, secure)
}

/// Checks whether `query` contains the `_hs=1` query parameter pair.
fn query_has_hot_support(query: Option<&str>) -> bool {
    let Some(q) = query else {
        return false;
    };
    let needle = format!("{HOT_SUPPORT_QUERY_PARAM}=1");
    q.split('&').any(|pair| pair == needle)
}

/// Checks whether the HTTP `Referer` header carries `_hs=1` in its query string.
fn referer_has_hot_support(headers: &HeaderMap) -> bool {
    let Some(referer) = headers.get(REFERER).and_then(|v| v.to_str().ok()) else {
        return false;
    };
    let Some((_, ref_query)) = referer.split_once('?') else {
        return false;
    };
    let clean_query = ref_query.split('#').next().unwrap_or(ref_query);
    query_has_hot_support(Some(clean_query))
}

/// Returns `true` when the request carries an explicit Hot Support signal:
/// - `X-Hot-Support: 1` header, or
/// - `_hs=1` query parameter.
///
/// Note: `Referer` `_hs=1` detection is intentionally scoped to `/api/notifications/sse`
/// via [`has_hot_support_signal_for_path`].
pub fn has_hot_support_signal(headers: &HeaderMap, uri_query: Option<&str>) -> bool {
    let has_header = headers
        .get(HOT_SUPPORT_HEADER_NAME)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.trim() == "1");

    has_header || query_has_hot_support(uri_query)
}

/// Evaluates [`has_hot_support_signal`] plus `Referer` `_hs=1` exclusively for `/api/notifications/sse`.
pub fn has_hot_support_signal_for_path(
    headers: &HeaderMap,
    uri_path: &str,
    uri_query: Option<&str>,
) -> bool {
    has_hot_support_signal(headers, uri_query)
        || (uri_path == SSE_ENDPOINT_PATH && referer_has_hot_support(headers))
}

// SECTION: SESSION RESOLUTION & MIDDLEWARE

/// Resolves an `AuthenticatedUser` from either `hot-support-session` (strictly when
/// `use_hot_support` is `true`) or `auth-session` (when `false`).
///
/// Uses Moka `sessions` and `users` caches first, falling back to PostgreSQL with
/// explicit `expires_at` verification on cache misses.
pub async fn resolve_session_user(
    state: &AppState,
    headers: &HeaderMap,
    use_hot_support: bool,
) -> Option<AuthenticatedUser> {
    let cookie_name = if use_hot_support {
        HOT_SUPPORT_COOKIE_NAME
    } else {
        AUTH_COOKIE_NAME
    };

    let raw_token = extract_cookie_from_headers(headers, cookie_name)?;
    let trimmed = raw_token.trim();
    if trimmed.is_empty() {
        return None;
    }

    let hashed_token = hash_token(trimmed);

    if let Some(user_id) = state.cache.sessions.get(&hashed_token).await {
        if let Some(user) = load_user_cached_or_db(state, user_id).await {
            return Some(AuthenticatedUser {
                user,
                is_hot_support: use_hot_support,
            });
        }
        state.cache.invalidate_session(&hashed_token).await;
        return None;
    }

    let session = sessions::Entity::find_by_id(&hashed_token)
        .one(&state.db)
        .await
        .ok()??;

    if session.expires_at.with_timezone(&Utc) <= Utc::now() {
        let _ = sessions::Entity::delete_by_id(&hashed_token)
            .exec(&state.db)
            .await;
        return None;
    }

    let user = load_user_cached_or_db(state, session.user_id).await?;
    state.cache.sessions.insert(hashed_token, user.id).await;

    Some(AuthenticatedUser {
        user,
        is_hot_support: use_hot_support,
    })
}

/// Looks up a user row in Moka cache or falls back to PostgreSQL and populates the cache.
async fn load_user_cached_or_db(state: &AppState, user_id: i32) -> Option<users::Model> {
    if let Some(cached) = state.cache.users.get(&user_id).await {
        return Some(cached);
    }

    let user = users::Entity::find_by_id(user_id)
        .one(&state.db)
        .await
        .ok()??;
    state.cache.users.insert(user.id, user.clone()).await;
    Some(user)
}

/// Global Axum authentication middleware inserting `Option<AuthenticatedUser>` on every request.
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Response {
    let use_hot_support = has_hot_support_signal_for_path(
        req.headers(),
        req.uri().path(),
        req.uri().query(),
    );
    let resolved = resolve_session_user(&state, req.headers(), use_hot_support).await;
    req.extensions_mut().insert(resolved);
    next.run(req).await
}

// SECTION: UNIT TESTS

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn extract_cookie_handles_multiple_cookies_and_split_headers() {
        let mut headers = HeaderMap::new();
        headers.append(
            COOKIE,
            HeaderValue::from_bytes(b"pref=Ti\xe1\xba\xbfng; theme=dark").unwrap(),
        );
        headers.append(
            COOKIE,
            HeaderValue::from_static("auth-session=abc123; hot-support-session=hs999"),
        );

        assert_eq!(
            extract_cookie_from_headers(&headers, AUTH_COOKIE_NAME).as_deref(),
            Some("abc123")
        );
        assert_eq!(
            extract_cookie_from_headers(&headers, HOT_SUPPORT_COOKIE_NAME).as_deref(),
            Some("hs999")
        );
    }

    #[test]
    fn hot_support_signal_and_sse_referer_scoping() {
        let empty = HeaderMap::new();
        assert!(!has_hot_support_signal(&empty, None));
        assert!(has_hot_support_signal(&empty, Some("foo=1&_hs=1")));
        assert!(!has_hot_support_signal(&empty, Some("no_hs=1")));

        let mut hdr = HeaderMap::new();
        hdr.insert("X-Hot-Support", HeaderValue::from_static("1"));
        assert!(has_hot_support_signal(&hdr, None));

        let mut ref_hdr = HeaderMap::new();
        ref_hdr.insert(
            REFERER,
            HeaderValue::from_static("http://localhost:3000/todos?_hs=1"),
        );
        assert!(!has_hot_support_signal_for_path(
            &ref_hdr,
            "/api/v1/todos",
            None
        ));
        assert!(has_hot_support_signal_for_path(
            &ref_hdr,
            "/api/notifications/sse",
            None
        ));
    }
}
