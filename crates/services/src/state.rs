// =============================================================================
// APPLICATION STATE CONTAINER
// =============================================================================
//
// Aggregates PostgreSQL connection pool, runtime `Settings`, Moka caches,
// optional Cloudflare R2 `StorageService`, RBAC `PermissionService`, SSE
// `NotificationHub`, short-lived OAuth/reauth state caches, and all initialized
// domain services (`UserService`, `WorkspaceService`, `TodoService`,
// `NotificationService`, `MediaService`, `AdminService`).
//
// ## Key Concepts
//
// - **Infallible Optional Storage**: `AppState::build` constructs
//   `storage: Option<StorageService>` synchronously via `StorageService::from_settings`
//   and never fails startup when R2 credentials are omitted.
// - **Cheap Cloning**: All fields wrap `Arc`-backed handles (`DatabaseConnection`,
//   Moka `Cache`, `broadcast::Sender`, `reqwest::Client`), making `AppState::clone()`
//   zero-allocation for Axum `State<AppState>` extractors.
//
// ## Architecture Notes
//
// Built once in `crates/server/src/main.rs` and in isolated `TestApp` harnesses.

// SECTION: IMPORTS

// `moka` - Time-bounded TTL caches for pending OAuth state & password reauth tokens.
use moka::future::Cache;

// `sea_orm` - PostgreSQL pool configuration and connection handle.
// - `ConnectOptions`, `Database`, `DatabaseConnection`, `DbErr` - Pool setup
use sea_orm::{ConnectOptions, Database, DatabaseConnection, DbErr};

// `std::time::Duration` - Configures 10-minute TTLs and DB pool timeouts.
use std::time::Duration;

// `shared` - Validated application configuration.
use shared::config::settings::Settings;

// `crate` - Infrastructure & domain services wired into `AppState`.
use crate::cache::CacheService;
use crate::domain::admin::service::AdminService;
use crate::domain::media::service::MediaService;
use crate::domain::notification::service::NotificationService;
use crate::domain::todo::service::TodoService;
use crate::domain::user::service::UserService;
use crate::domain::workspace::service::WorkspaceService;
use crate::permission::PermissionService;
use crate::sse::NotificationHub;
use crate::storage::StorageService;

// SECTION: OAUTH & REAUTH CACHE RECORDS

/// Pending Google OAuth 2.0 `state` payload stored in `AppState::oauth_states` (10 min TTL).
#[derive(Debug, Clone)]
pub struct OAuthPendingState {
    /// Post-authentication frontend redirect destination (e.g. `"/todos"`).
    pub return_to: String,
    /// OAuth flow purpose (`"login"` or `"reauth"`).
    pub purpose: String,
    /// Optional lookup key into `AppState::pending_passwords` when `purpose == "reauth"`.
    pub reauth_token: Option<String>,
    /// Initiating `user_id` verified during `purpose == "reauth"`.
    pub user_id: Option<i32>,
}

/// Pending Argon2id password hash awaiting Google `purpose=reauth` confirmation (10 min TTL).
#[derive(Debug, Clone)]
pub struct PendingPassword {
    /// User requesting the password update.
    pub user_id: i32,
    /// Pre-computed PHC Argon2id hash ready to persist upon reauth completion.
    pub password_hash: String,
}

// SECTION: APPLICATION STATE

/// Shared backend state injected into Axum handlers, middleware, and integration tests.
#[derive(Clone)]
pub struct AppState {
    /// PostgreSQL SeaORM connection pool.
    pub db: DatabaseConnection,
    /// Validated application environment settings.
    pub settings: Settings,
    /// Moka session, user, workspace, and Todo caches.
    pub cache: CacheService,
    /// Optional Cloudflare R2 / S3 client (`None` when R2 env vars are blank).
    pub storage: Option<StorageService>,
    /// Cached workspace RBAC permission evaluator.
    pub permission_service: PermissionService,
    /// Real-time SSE broadcast hub for `/api/notifications/sse`.
    pub notification_hub: NotificationHub,
    /// 10-minute TTL cache storing pending Google OAuth CSRF & reauth states.
    pub oauth_states: Cache<String, OAuthPendingState>,
    /// 10-minute TTL cache storing pre-hashed passwords pending Google reauth.
    pub pending_passwords: Cache<String, PendingPassword>,
    /// Authentication, OAuth, onboarding, and profile domain service.
    pub user_service: UserService,
    /// Multi-tenant workspace and collaborator RBAC domain service.
    pub workspace_service: WorkspaceService,
    /// Workspace-scoped Todo CRUD, filtering, stats, and reorder service.
    pub todo_service: TodoService,
    /// Per-user notifications and global announcements feed service.
    pub notification_service: NotificationService,
    /// Polymorphic encrypted R2 media metadata & presigning service.
    pub media_service: MediaService,
    /// System administration, test accounts, global broadcasts, and hot support service.
    pub admin_service: AdminService,
}

impl AppState {
    /// Connects to PostgreSQL using `settings.database` pool parameters.
    pub async fn connect_database(settings: &Settings) -> Result<DatabaseConnection, DbErr> {
        let mut opts = ConnectOptions::new(settings.database.url.clone());
        opts.max_connections(settings.database.max_connections.max(1))
            .min_connections(1)
            .connect_timeout(Duration::from_secs(10))
            .acquire_timeout(Duration::from_secs(10))
            .sqlx_logging(false);

        Database::connect(opts).await
    }

    /// Synchronously builds `AppState` and wires every infrastructure and domain service.
    ///
    /// # Arguments
    ///
    /// * `db` - Connected SeaORM `DatabaseConnection` pool
    /// * `settings` - Application settings loaded from environment variables
    pub fn build(db: DatabaseConnection, settings: Settings) -> Self {
        let cache = CacheService::new(&settings.cache);
        let storage = StorageService::from_settings(&settings);
        let permission_service = PermissionService::new(db.clone(), &settings.cache);
        let notification_hub = NotificationHub::new();

        let oauth_ttl = Duration::from_secs(600);
        let oauth_states = Cache::builder()
            .max_capacity(10_000)
            .time_to_live(oauth_ttl)
            .build();

        let pending_passwords = Cache::builder()
            .max_capacity(10_000)
            .time_to_live(oauth_ttl)
            .build();

        let media_service =
            MediaService::new(db.clone(), storage.clone(), settings.public_media_url());
        let notification_service = NotificationService::new(db.clone(), notification_hub.clone());

        let user_service = UserService::new(
            db.clone(),
            settings.clone(),
            cache.clone(),
            media_service.clone(),
            oauth_states.clone(),
            pending_passwords.clone(),
            notification_service.clone(),
        );

        let workspace_service = WorkspaceService::new(
            db.clone(),
            cache.clone(),
            permission_service.clone(),
            media_service.clone(),
            notification_service.clone(),
        );

        let todo_service = TodoService::new(
            db.clone(),
            cache.clone(),
            permission_service.clone(),
            media_service.clone(),
            notification_hub.clone(),
            notification_service.clone(),
        );

        let admin_service = AdminService::new(db.clone(), cache.clone(), notification_hub.clone());

        Self {
            db,
            settings,
            cache,
            storage,
            permission_service,
            notification_hub,
            oauth_states,
            pending_passwords,
            user_service,
            workspace_service,
            todo_service,
            notification_service,
            media_service,
            admin_service,
        }
    }
}
