// =============================================================================
// HTTP ROUTER ASSEMBLY & STATIC ASSET SERVING (`server.rs`)
// =============================================================================
//
// Wires together `/api/v1/*`, SSE notifications, optional dev `/api/test/*`,
// immutable SvelteKit `_app` bundles, `public/` static assets, fast-path `/`
// handling, and SPA shell fallback under a single shared `AppState`.

use std::path::PathBuf;

// `axum` - Router composition, fallback handling, and middleware layers.
use axum::extract::Request;
use axum::http::header::CACHE_CONTROL;
use axum::http::{HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;

// `tower_http` - Static file serving with `.br`/`.gz` precompression and CORS/tracing.
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

use services::middleware::auth::auth_middleware;
use services::state::AppState;
use shared::domain::error_code::ErrorCode;
use shared::domain::response::ApiResponse;

use crate::shell::{handle_root, serve_spa_shell, ServerShellState, SpaShellTemplate};
use crate::sse::notifications_sse_handler;

/// Builds the complete production/development Axum HTTP router.
pub fn build_app_router(state: AppState, site_root: PathBuf, public_root: PathBuf) -> Router {
    let shell_template = SpaShellTemplate::load(&site_root);
    let shell_state = ServerShellState {
        app: state.clone(),
        shell: shell_template,
    };

    let immutable_app_dir = ServeDir::new(site_root.join("_app"))
        .precompressed_br()
        .precompressed_gzip();

    let immutable_router = Router::new()
        .nest_service("/_app", immutable_app_dir)
        .layer(axum::middleware::from_fn(set_immutable_cache_header));

    let public_assets_router = Router::new()
        .nest_service("/logo", ServeDir::new(public_root.join("logo")))
        .nest_service("/icons", ServeDir::new(public_root.join("icons")))
        .nest_service("/js", ServeDir::new(public_root.join("js")))
        .nest_service("/guides", ServeDir::new(public_root.join("guides")))
        .route_service("/robots.txt", ServeFile::new(public_root.join("robots.txt")))
        .route_service(
            "/sitemap.xml",
            ServeFile::new(public_root.join("sitemap.xml")),
        )
        .route_service(
            "/og-image.svg",
            ServeFile::new(public_root.join("og-image.svg")),
        );

    let mut api_router = Router::new()
        .merge(api::routes::api_v1_router())
        .route("/api/notifications/sse", get(notifications_sse_handler));

    if state.settings.env.is_dev() {
        api_router = api_router.merge(services::test_api::router(&state.settings));
    }

    let api_router = api_router.with_state(state.clone());

    let shell_router = Router::new()
        .route("/", get(handle_root))
        .fallback(spa_or_api_404)
        .with_state(shell_state);

    Router::new()
        .merge(immutable_router)
        .merge(public_assets_router)
        .merge(api_router)
        .merge(shell_router)
        .layer(axum::middleware::from_fn_with_state(state, auth_middleware))
        .layer(TraceLayer::new_for_http())
}

async fn set_immutable_cache_header(req: Request, next: Next) -> Response {
    let mut resp = next.run(req).await;
    if resp.status().is_success() {
        resp.headers_mut().insert(
            CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=31536000, immutable"),
        );
    }
    resp
}

async fn spa_or_api_404(
    state: axum::extract::State<ServerShellState>,
    headers: axum::http::HeaderMap,
    uri: axum::http::Uri,
) -> Response {
    if uri.path().starts_with("/api/") {
        return (
            StatusCode::NOT_FOUND,
            axum::Json(ApiResponse::<()>::error(ErrorCode::NotFound)),
        )
            .into_response();
    }
    serve_spa_shell(state, headers, uri).await
}
