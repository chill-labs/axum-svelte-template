// =============================================================================
// SERVER BINARY ENTRYPOINT (`main.rs`)
// =============================================================================
//
// Initializes environment config, structured tracing, PostgreSQL pool,
// default admin seeding, background expired-session pruning, HTTP listener,
// and graceful `SIGINT`/`SIGTERM` shutdown.

use std::net::SocketAddr;
use std::path::PathBuf;

// `sea_orm` - Periodic expired-session cleanup query.
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

// `tokio` - Graceful Unix/Ctrl-C shutdown signal handling.
use tokio::net::TcpListener;

// `tokio_cron_scheduler` - Background job pruning expired PostgreSQL sessions.
use tokio_cron_scheduler::{Job, JobScheduler};

// `tracing` - Server startup lifecycle logs.
use tracing::{error, info};

use services::entities::sessions;
use services::seed::SeedService;
use services::state::AppState;
use shared::config::settings::Settings;

/// Tracing subscriber and rolling log file setup.
pub mod logging;
/// Main Axum router builder and static file middleware.
pub mod server;
/// SSR-lite SPA shell renderer and OpenGraph / bootstrap injector.
pub mod shell;
/// Server-Sent Events stream handler (`/api/notifications/sse`).
pub mod sse;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let settings = Settings::load()?;
    let _log_guard = logging::init_logging(&settings.logging);

    info!(
        env = ?settings.env,
        host = %settings.server.host,
        port = settings.server.port,
        "Starting axum-svelte-template server"
    );

    let db = AppState::connect_database(&settings).await?;
    SeedService::seed_default_admin(&db, &settings).await?;

    let state = AppState::build(db.clone(), settings.clone());
    let _scheduler = start_cleanup_scheduler(db).await;

    let site_root = PathBuf::from(
        std::env::var("SVELTE_SITE_ROOT").unwrap_or_else(|_| "svelte-app/build".to_string()),
    );
    let public_root =
        PathBuf::from(std::env::var("PUBLIC_ASSETS_ROOT").unwrap_or_else(|_| "public".to_string()));

    let app = server::build_app_router(state, site_root, public_root);
    let addr: SocketAddr = format!("{}:{}", settings.server.host, settings.server.port).parse()?;
    let listener = TcpListener::bind(addr).await?;
    info!(%addr, "HTTP server listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn start_cleanup_scheduler(db: sea_orm::DatabaseConnection) -> Option<JobScheduler> {
    let scheduler = JobScheduler::new().await.ok()?;
    let job = Job::new_async("0 0 * * * *", move |_uuid, _lock| {
        let db = db.clone();
        Box::pin(async move {
            let now = chrono::Utc::now();
            if let Err(err) = sessions::Entity::delete_many()
                .filter(sessions::Column::ExpiresAt.lt(now))
                .exec(&db)
                .await
            {
                error!(error = %err, "Failed cleaning expired sessions");
            }
        })
    })
    .ok()?;

    if scheduler.add(job).await.is_ok() && scheduler.start().await.is_ok() {
        Some(scheduler)
    } else {
        None
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut sigterm) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            sigterm.recv().await;
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    info!("Shutdown signal received, stopping HTTP server cleanly");
}
