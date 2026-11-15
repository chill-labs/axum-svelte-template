// =============================================================================
// SSR-LITE SPA SHELL INJECTOR (`shell.rs`)
// =============================================================================
//
// Transforms SvelteKit `adapter-static` `200.html` into a personalized,
// zero-flash HTML response in sub-millisecond time:
//
// 1. **Instant Root Redirect (Crawler-Safe)**: Redirects interactive browsers
//    visiting `/` straight to `/todos` via `302 Found` while returning `200 OK`
//    HTML with OpenGraph tags to social preview crawlers.
// 2. **Zero-Flash Theme & Locale**: Injects `<html lang="vi|en" class="dark">`
//    and `<meta name="color-scheme">` from cookies before first paint.
// 3. **Route Modulepreload**: Scans `svelte-app/build/_app/immutable/entry/app.*.js`
//    when built to inject relevant `<link rel="modulepreload">` tags.
// 4. **XSS-Safe `window.__APP_BOOTSTRAP__`**: Embeds hydrated user session,
//    workspace RBAC, unread counts, and Hot Support state directly into `<head>`.

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::Arc;

// `axum` - Builds HTML & redirect HTTP responses cleanly.
use axum::extract::State;
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE, LOCATION, USER_AGENT, VARY};
use axum::http::{HeaderMap, HeaderValue, StatusCode, Uri};
use axum::response::{IntoResponse, Response};

// `serde` - Serializes `window.__APP_BOOTSTRAP__` contract.
use serde::Serialize;

// `tracing` - Logs missing build shell fallback once at startup.
use tracing::{info, warn};

use api::cookies::extract_cookie;
use services::middleware::auth::{has_hot_support_signal, resolve_session_user};
use services::state::AppState;
use shared::domain::user::SessionBootstrapData;

/// Fallback HTML shell rendered if `svelte-app/build/200.html` has not been built yet.
const FALLBACK_DEV_SHELL: &str = r#"<!doctype html>
<html lang="vi">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Axum + Svelte 5 Todo Template</title>
    <!--APP_BOOTSTRAP_SLOT-->
  </head>
  <body>
    <main style="font-family: system-ui, sans-serif; max-width: 640px; margin: 4rem auto; padding: 1.5rem;">
      <h1>Axum + Svelte 5 Todo Server is running</h1>
      <p>Run <code>cd svelte-app &amp;&amp; npm run build</code> (or run Vite dev server on port 5173) to serve the compiled Svelte 5 frontend.</p>
    </main>
  </body>
</html>"#;

/// Serialized JSON payload written to `window.__APP_BOOTSTRAP__`.
#[derive(Debug, Clone, Serialize)]
pub struct AppBootstrapPayload {
    pub session: Option<SessionBootstrapData>,
    pub lang: String,
    pub theme: Option<String>,
    pub r2_public_url: String,
    pub is_hot_support: bool,
}

/// Pre-parsed chunk preload table extracted from SvelteKit's `entry/app.*.js`.
#[derive(Debug, Clone, Default)]
pub struct SveltePreloadTable {
    pub immutable_chunks: Vec<String>,
}

impl SveltePreloadTable {
    /// Scans `site_root/_app/immutable/entry` to collect core entry chunks for `<link rel="modulepreload">`.
    pub fn load_from_site_root(site_root: &Path) -> Self {
        let entry_dir = site_root.join("_app/immutable/entry");
        let Ok(entries) = fs::read_dir(&entry_dir) else {
            return Self::default();
        };

        let mut chunks = Vec::new();
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".js") && (name.starts_with("start.") || name.starts_with("app.")) {
                chunks.push(format!("/_app/immutable/entry/{name}"));
            }
        }
        chunks.sort();
        Self {
            immutable_chunks: chunks,
        }
    }
}

/// Cached SPA HTML shell template + preload table shared across HTTP handlers.
#[derive(Debug, Clone)]
pub struct SpaShellTemplate {
    raw_html: Arc<String>,
    preloads: Arc<SveltePreloadTable>,
}

impl SpaShellTemplate {
    /// Loads `200.html` (or `index.html`) from `site_root`, falling back cleanly in dev mode.
    pub fn load(site_root: &Path) -> Self {
        let candidates = [site_root.join("200.html"), site_root.join("index.html")];
        let mut loaded_html = None;

        for candidate in &candidates {
            if let Ok(content) = fs::read_to_string(candidate) {
                info!(path = %candidate.display(), "Loaded compiled SvelteKit SPA shell");
                loaded_html = Some(content);
                break;
            }
        }

        let raw_html = match loaded_html {
            Some(html) => html,
            None => {
                warn!(
                    site_root = %site_root.display(),
                    "SvelteKit 200.html not found; serving dev fallback shell"
                );
                FALLBACK_DEV_SHELL.to_string()
            }
        };

        let preloads = SveltePreloadTable::load_from_site_root(site_root);
        Self {
            raw_html: Arc::new(raw_html),
            preloads: Arc::new(preloads),
        }
    }

    /// Creates an in-memory template directly for unit tests.
    pub fn from_raw(html: impl Into<String>) -> Self {
        Self {
            raw_html: Arc::new(html.into()),
            preloads: Arc::new(SveltePreloadTable::default()),
        }
    }

    /// Renders personalized HTML injecting zero-flash `<html lang/class>`, color-scheme,
    /// modulepreload hints, and XSS-escaped `window.__APP_BOOTSTRAP__`.
    pub fn render_html(&self, payload: &AppBootstrapPayload, path: &str) -> String {
        let is_dark = payload.theme.as_deref() == Some("dark");
        let color_scheme = match payload.theme.as_deref() {
            Some("dark") => "dark",
            Some("light") => "light",
            _ => "light dark",
        };

        let mut html = (*self.raw_html).clone();

        let html_tag_replacement = if is_dark {
            format!(r#"<html lang="{}" class="dark">"#, escape_attr(&payload.lang))
        } else {
            format!(r#"<html lang="{}">"#, escape_attr(&payload.lang))
        };

        if html.contains(r#"<html lang="vi">"#) {
            html = html.replacen(r#"<html lang="vi">"#, &html_tag_replacement, 1);
        } else if html.contains("<html>") {
            html = html.replacen("<html>", &html_tag_replacement, 1);
        }

        let mut head_injection = format!(
            "\n    <meta name=\"color-scheme\" content=\"{color_scheme}\" />\n"
        );

        let mut seen = HashSet::new();
        for chunk in &self.preloads.immutable_chunks {
            if seen.insert(chunk.clone()) {
                head_injection.push_str(&format!(
                    "    <link rel=\"modulepreload\" href=\"{}\" />\n",
                    escape_attr(chunk)
                ));
            }
        }

        if path == "/" || is_social_route(path) {
            head_injection.push_str(
                "    <meta property=\"og:type\" content=\"website\" />\n\
                 <meta property=\"og:title\" content=\"Axum + Svelte 5 Todo Template\" />\n\
                 <meta property=\"og:description\" content=\"Multi-workspace Todo starter template powered by Rust Axum, PostgreSQL, and Svelte 5 Runes.\" />\n\
                 <meta property=\"og:image\" content=\"/og-image.svg\" />\n",
            );
        }

        let bootstrap_json = escape_inline_json(
            &serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string()),
        );
        head_injection.push_str(&format!(
            "    <script>window.__APP_BOOTSTRAP__ = {bootstrap_json};</script>\n"
        ));

        if html.contains("<!--APP_BOOTSTRAP_SLOT-->") {
            html = html.replacen("<!--APP_BOOTSTRAP_SLOT-->", &head_injection, 1);
        } else if html.contains("</head>") {
            html = html.replacen("</head>", &format!("{head_injection}  </head>"), 1);
        }

        html
    }
}

/// Shared bundle containing `AppState` and compiled `SpaShellTemplate`.
#[derive(Clone)]
pub struct ServerShellState {
    pub app: AppState,
    pub shell: SpaShellTemplate,
}

/// Fast-path `/` handler: 302 redirect to `/todos` for browsers, 200 OG HTML for crawlers.
pub async fn handle_root(
    State(shell_state): State<ServerShellState>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if is_crawler_user_agent(&headers) {
        return render_spa_shell_response(&shell_state, &headers, &uri).await;
    }

    let dest = match uri.query() {
        Some(q) if !q.is_empty() => format!("/todos?{q}"),
        _ => "/todos".to_string(),
    };

    let mut resp = (StatusCode::FOUND, [(LOCATION, dest)]).into_response();
    resp.headers_mut()
        .insert(VARY, HeaderValue::from_static("User-Agent, Cookie"));
    resp
}

/// Serves the SPA HTML shell for every client-side route fallback.
pub async fn serve_spa_shell(
    State(shell_state): State<ServerShellState>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    render_spa_shell_response(&shell_state, &headers, &uri).await
}

async fn render_spa_shell_response(
    shell_state: &ServerShellState,
    headers: &HeaderMap,
    uri: &Uri,
) -> Response {
    let use_hot_support = has_hot_support_signal(headers, uri.query());
    let maybe_auth = resolve_session_user(&shell_state.app, headers, use_hot_support).await;

    let session = match maybe_auth {
        Some(auth) => shell_state
            .app
            .user_service
            .get_session_bootstrap(
                auth.user.id,
                auth.is_hot_support,
                &shell_state.app.permission_service,
            )
            .await
            .ok(),
        None => None,
    };

    let cookie_lang = extract_cookie(headers, "lang").and_then(|l| normalize_lang(&l));
    let session_lang = session
        .as_ref()
        .and_then(|s| normalize_lang(&s.user.language));
    let lang = cookie_lang
        .or(session_lang)
        .unwrap_or_else(|| "vi".to_string());

    let theme = extract_cookie(headers, "theme").and_then(|t| match t.as_str() {
        "dark" => Some("dark".to_string()),
        "light" => Some("light".to_string()),
        _ => None,
    });

    let payload = AppBootstrapPayload {
        is_hot_support: session.as_ref().is_some_and(|s| s.is_hot_support),
        session,
        lang,
        theme,
        r2_public_url: shell_state.app.settings.public_media_url(),
    };

    let html = shell_state.shell.render_html(&payload, uri.path());
    let mut resp = (StatusCode::OK, html).into_response();
    resp.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    resp.headers_mut().insert(
        CACHE_CONTROL,
        HeaderValue::from_static("no-cache, no-store, must-revalidate"),
    );
    resp.headers_mut()
        .insert(VARY, HeaderValue::from_static("Cookie"));
    resp
}

/// Returns `true` when User-Agent matches well-known link preview bots/crawlers.
pub fn is_crawler_user_agent(headers: &HeaderMap) -> bool {
    let Some(ua) = headers.get(USER_AGENT).and_then(|v| v.to_str().ok()) else {
        return false;
    };
    let lower = ua.to_lowercase();
    [
        "facebookexternalhit",
        "facebot",
        "zalo",
        "twitterbot",
        "telegrambot",
        "discordbot",
        "slackbot",
        "whatsapp",
        "linkedinbot",
        "googlebot",
    ]
    .iter()
    .any(|bot| lower.contains(bot))
}

fn is_social_route(path: &str) -> bool {
    matches!(path, "/" | "/todos" | "/auth/login")
}

fn normalize_lang(raw: &str) -> Option<String> {
    let trimmed = raw.trim().to_lowercase();
    if trimmed.starts_with("en") {
        Some("en".to_string())
    } else if trimmed.starts_with("vi") {
        Some("vi".to_string())
    } else {
        None
    }
}

fn escape_inline_json(raw: &str) -> String {
    raw.replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
        .replace('\'', "\\u0027")
}

fn escape_attr(val: &str) -> String {
    val.replace('"', "&quot;").replace('<', "&lt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crawler_detection_recognizes_social_bots() {
        let mut bot_headers = HeaderMap::new();
        bot_headers.insert(USER_AGENT, HeaderValue::from_static("ZaloPC/1.0"));
        assert!(is_crawler_user_agent(&bot_headers));

        let mut browser_headers = HeaderMap::new();
        browser_headers.insert(
            USER_AGENT,
            HeaderValue::from_static("Mozilla/5.0 Chrome/125.0"),
        );
        assert!(!is_crawler_user_agent(&browser_headers));
    }

    #[test]
    fn shell_injects_theme_lang_and_escapes_xss() {
        let tpl = SpaShellTemplate::from_raw(
            r#"<!doctype html><html lang="vi"><head><!--APP_BOOTSTRAP_SLOT--></head><body></body></html>"#,
        );
        let payload = AppBootstrapPayload {
            session: None,
            lang: "en".to_string(),
            theme: Some("dark".to_string()),
            r2_public_url: "https://cdn.example.com/</script><script>alert(1)".to_string(),
            is_hot_support: true,
        };

        let rendered = tpl.render_html(&payload, "/todos");
        assert!(rendered.contains(r#"<html lang="en" class="dark">"#));
        assert!(rendered.contains(r#"<meta name="color-scheme" content="dark" />"#));
        assert!(!rendered.contains("</script><script>alert(1)"));
        assert!(rendered.contains("\\u003c/script\\u003e"));
        assert!(rendered.contains("\"is_hot_support\":true"));
    }
}
