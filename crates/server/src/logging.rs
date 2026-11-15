// =============================================================================
// TRACING & LOGGING INITIALIZATION
// =============================================================================
//
// Configures `tracing_subscriber` with `EnvFilter`, optional daily rolling file
// logging (`tracing_appender`), and `pretty` (dev) or `json` (prod) formatting.

// `tracing_appender` - Non-blocking optional file appender guard.
use tracing_appender::non_blocking::WorkerGuard;

// `tracing_subscriber` - Composable log formatting and EnvFilter registry.
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use shared::config::settings::LoggingConfig;

/// Initializes global structured logging, returning an optional guard that must stay alive.
pub fn init_logging(cfg: &LoggingConfig) -> Option<WorkerGuard> {
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&cfg.level))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let is_json = cfg.format.eq_ignore_ascii_case("json");

    let mut file_guard: Option<WorkerGuard> = None;

    if let Some(ref dir) = cfg.file_dir {
        let trimmed = dir.trim();
        if !trimmed.is_empty() {
            let file_appender = tracing_appender::rolling::daily(trimmed, "server.log");
            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
            file_guard = Some(guard);

            if is_json {
                tracing_subscriber::registry()
                    .with(filter)
                    .with(fmt::layer().json())
                    .with(fmt::layer().json().with_writer(non_blocking))
                    .init();
            } else {
                tracing_subscriber::registry()
                    .with(filter)
                    .with(fmt::layer())
                    .with(fmt::layer().with_ansi(false).with_writer(non_blocking))
                    .init();
            }
            return file_guard;
        }
    }

    if is_json {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().json())
            .init();
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer())
            .init();
    }

    file_guard
}
