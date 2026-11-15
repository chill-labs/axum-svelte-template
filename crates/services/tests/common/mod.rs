// =============================================================================
// INTEGRATION TEST HARNESS (TEMPLATE-CLONED POSTGRESQL)
// =============================================================================
//
// Shared fixture and database isolation harness for all `it_*_domain.rs`
// integration test suites under `crates/services/tests/`.
//
// ## Key Concepts
//
// - **Template Database (`axum_svelte_it_template_v1`)**: Built once per test
//   binary via `tokio::sync::OnceCell`, applying `database/schema/01..05_*.sql`
//   and `database/triggers/1_example_workspace_triggers.sql`.
// - **Per-Test Cloning**: Each test clones a fresh `test_it_<uuid>` database
//   from the template under a PostgreSQL advisory lock + in-process mutex for
//   full parallel isolation with zero cross-test state leakage.
// - **Zero-Storage Default**: `TestApp::new()` initializes `AppState` with
//   `r2: None` (`storage: None`); `TestApp::with_settings(...)` enables custom
//   `GoogleConfig` or mock `R2Config` overrides per test.

// SECTION: IMPORTS

// `sea_orm` - Raw DDL execution and SeaORM fixture inserts against PostgreSQL.
use sea_orm::{
    ActiveModelTrait, ConnectOptions, ConnectionTrait, Database, DatabaseConnection, EntityTrait,
    Set, Statement,
};

// `services` - Backend state, entities, seeding, and shared test target services.
use services::entities::users;
use services::state::AppState;

// `shared` - Programmatic test configuration and admin/user DTOs.
use shared::config::settings::{R2Config, Settings};
use shared::domain::admin::{AdminTestAccountResponse, CreateTestAccountRequest};
use shared::domain::user::UserRole;

// `std` - Filesystem SQL reading, paths, and connection timeouts.
use std::fs;
use std::path::Path;
use std::time::Duration;

// `tokio` - Process-local async initialization and cloning lock.
use tokio::sync::{Mutex, OnceCell};

// `uuid` - Unique isolated database naming per test case.
use uuid::Uuid;

// SECTION: CONSTANTS & SYNCHRONIZATION

/// Template database initialized once per test binary.
const TEMPLATE_DB_NAME: &str = "axum_svelte_it_template_v1";

/// PostgreSQL advisory lock key serializing template creation & cloning across binaries.
const TEMPLATE_ADVISORY_LOCK_ID: i64 = 88_441_122;

/// Fallback PostgreSQL maintenance connection URLs tried if env/.env overrides are absent.
const FALLBACK_ADMIN_DB_URLS: &[&str] = &[
    "postgres://postgres:postgres@127.0.0.1:5432/postgres",
    "postgres://postgres:your_postgres_password@127.0.0.1:5432/postgres",
    "postgres://postgres@127.0.0.1:5432/postgres",
];

/// Cached resolved admin connection URL once template setup succeeds.
static TEMPLATE_ADMIN_URL: OnceCell<String> = OnceCell::const_new();

/// Process-local mutex serializing `CREATE DATABASE ... TEMPLATE`.
static DB_CLONE_MUTEX: Mutex<()> = Mutex::const_new(());

// SECTION: TEST APP CONTAINER

/// Isolated PostgreSQL + `AppState` test harness for a single integration test.
#[allow(dead_code)]
pub struct TestApp {
    /// Fully wired application state targeting the isolated test database.
    pub state: AppState,
    /// Name of the cloned PostgreSQL database (`test_it_<uuid>`).
    pub db_name: String,
    /// PostgreSQL connection string for the cloned database.
    pub db_url: String,
}

#[allow(dead_code)]
impl TestApp {
    /// Spins up a clean isolated database and `AppState` with default test settings (`r2: None`).
    pub async fn new() -> Self {
        Self::with_settings(|_| {}).await
    }

    /// Spins up a clean isolated database and lets the caller customize `Settings`
    /// (for example enabling `GoogleConfig` or mock `R2Config`).
    pub async fn with_settings(configure: impl FnOnce(&mut Settings)) -> Self {
        let admin_url = ensure_template_db().await;
        let db_name = format!("test_it_{}", Uuid::new_v4().simple());

        {
            let _guard = DB_CLONE_MUTEX.lock().await;
            let admin_db = connect_single(&admin_url).await;

            admin_db
                .execute_unprepared(&format!(
                    "SELECT pg_advisory_lock({TEMPLATE_ADVISORY_LOCK_ID});"
                ))
                .await
                .expect("acquire advisory lock before clone");

            admin_db
                .execute_unprepared(&format!(
                    "CREATE DATABASE \"{db_name}\" TEMPLATE {TEMPLATE_DB_NAME};"
                ))
                .await
                .unwrap_or_else(|err| panic!("failed cloning template db '{db_name}': {err:?}"));

            let _ = admin_db
                .execute_unprepared(&format!(
                    "SELECT pg_advisory_unlock({TEMPLATE_ADVISORY_LOCK_ID});"
                ))
                .await;
            let _ = admin_db.close().await;
        }

        let db_url = replace_db_name_in_url(&admin_url, &db_name);
        let mut settings = Settings::for_test(&db_url);
        configure(&mut settings);

        let db = AppState::connect_database(&settings)
            .await
            .unwrap_or_else(|err| panic!("failed connecting to isolated db '{db_name}': {err:?}"));
        let state = AppState::build(db, settings);

        Self {
            state,
            db_name,
            db_url,
        }
    }

    /// Convenience builder returning a deterministic mock `R2Config` (offline presigning, no network).
    pub fn mock_r2_config() -> R2Config {
        R2Config {
            account_id: "test-account".to_string(),
            access_key_id: "test-access-key".to_string(),
            secret_access_key: "test-secret-key".to_string(),
            bucket_name: "test-bucket".to_string(),
            public_url: "https://cdn.test.local".to_string(),
        }
    }

    /// Inserts a standard verified user directly into `users` and reloads it with trigger fields.
    pub async fn create_user(&self, email: &str, name: &str) -> users::Model {
        let inserted = users::ActiveModel {
            email: Set(email.trim().to_lowercase()),
            name: Set(name.to_string()),
            role: Set(UserRole::User.as_str().to_string()),
            email_verified: Set(true),
            language: Set("vi".to_string()),
            is_test_account: Set(false),
            onboarding_completed: Set(true),
            ..Default::default()
        }
        .insert(&self.state.db)
        .await
        .expect("insert test user");

        users::Entity::find_by_id(inserted.id)
            .one(&self.state.db)
            .await
            .expect("reload test user")
            .expect("test user exists")
    }

    /// Inserts an administrator user directly into `users` and reloads it with trigger fields.
    pub async fn create_admin(&self, email: &str, name: &str) -> users::Model {
        let inserted = users::ActiveModel {
            email: Set(email.trim().to_lowercase()),
            name: Set(name.to_string()),
            role: Set(UserRole::Admin.as_str().to_string()),
            email_verified: Set(true),
            language: Set("en".to_string()),
            is_test_account: Set(false),
            onboarding_completed: Set(true),
            ..Default::default()
        }
        .insert(&self.state.db)
        .await
        .expect("insert test admin");

        users::Entity::find_by_id(inserted.id)
            .one(&self.state.db)
            .await
            .expect("reload test admin")
            .expect("test admin exists")
    }

    /// Creates a password-enabled Test Account via `AdminService::create_test_account`.
    pub async fn create_test_account(
        &self,
        email: &str,
        name: &str,
        password: &str,
    ) -> AdminTestAccountResponse {
        self.state
            .admin_service
            .create_test_account(CreateTestAccountRequest {
                email: email.to_string(),
                name: name.to_string(),
                password: password.to_string(),
                role: Some(UserRole::User),
                language: Some("en".to_string()),
            })
            .await
            .expect("create test account")
    }
}

// SECTION: TEMPLATE INITIALIZATION HELPERS

/// Replaces the database name segment in a PostgreSQL URL while preserving query parameters.
fn replace_db_name_in_url(base_url: &str, new_db_name: &str) -> String {
    let (without_query, query_suffix) = match base_url.split_once('?') {
        Some((prefix, query)) => (prefix, format!("?{query}")),
        None => (base_url, String::new()),
    };

    match without_query.rsplit_once('/') {
        Some((prefix, _)) => format!("{prefix}/{new_db_name}{query_suffix}"),
        None => format!("{without_query}/{new_db_name}{query_suffix}"),
    }
}

/// Opens a single-connection SeaORM handle for DDL operations.
async fn try_connect_single(url: &str) -> Result<DatabaseConnection, sea_orm::DbErr> {
    let mut opt = ConnectOptions::new(url.to_string());
    opt.max_connections(1)
        .min_connections(1)
        .connect_timeout(Duration::from_secs(5))
        .acquire_timeout(Duration::from_secs(5))
        .sqlx_logging(false);
    Database::connect(opt).await
}

/// Connects to PostgreSQL or panics with connection details.
async fn connect_single(url: &str) -> DatabaseConnection {
    try_connect_single(url)
        .await
        .unwrap_or_else(|err| panic!("Failed connecting to PostgreSQL '{url}': {err:?}"))
}

/// Reads an existing PostgreSQL URL from `.env` in `workspace_root` if present.
fn read_url_from_workspace_dotenv(workspace_root: &Path) -> Option<String> {
    let content = fs::read_to_string(workspace_root.join(".env")).ok()?;
    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, val)) = line.split_once('=') {
            if matches!(
                key.trim(),
                "TEST_ADMIN_DATABASE_URL" | "APP__DATABASE__URL" | "DATABASE_URL"
            ) {
                let cleaned = val.trim().trim_matches('"').trim_matches('\'');
                if !cleaned.is_empty() {
                    return Some(replace_db_name_in_url(cleaned, "postgres"));
                }
            }
        }
    }
    None
}

/// Resolves a working maintenance `postgres` database URL.
async fn resolve_admin_db_url(workspace_root: &Path) -> (String, DatabaseConnection) {
    if let Ok(explicit) = std::env::var("TEST_ADMIN_DATABASE_URL") {
        let conn = connect_single(explicit.trim()).await;
        return (explicit.trim().to_string(), conn);
    }

    let mut candidates = Vec::new();
    for key in ["APP__DATABASE__URL", "DATABASE_URL"] {
        if let Ok(val) = std::env::var(key) {
            if !val.trim().is_empty() {
                candidates.push(replace_db_name_in_url(val.trim(), "postgres"));
            }
        }
    }
    if let Some(dotenv_url) = read_url_from_workspace_dotenv(workspace_root) {
        candidates.push(dotenv_url);
    }
    for fallback in FALLBACK_ADMIN_DB_URLS {
        candidates.push((*fallback).to_string());
    }

    let mut last_err = None;
    for candidate in candidates {
        match try_connect_single(&candidate).await {
            Ok(conn) => return (candidate, conn),
            Err(err) => last_err = Some((candidate, err)),
        }
    }

    let (failed_url, err) = last_err.expect("at least one candidate PostgreSQL URL");
    panic!(
        "Failed connecting to local PostgreSQL (last tried '{failed_url}'): {err:?}. Set TEST_ADMIN_DATABASE_URL to override."
    );
}

/// Builds `axum_svelte_it_template_v1` once per test binary from `database/schema` + `database/triggers`.
async fn ensure_template_db() -> String {
    TEMPLATE_ADMIN_URL
        .get_or_init(|| async {
            let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
            let (admin_url, admin_db) = resolve_admin_db_url(&workspace_root).await;

            admin_db
                .execute_unprepared(&format!(
                    "SELECT pg_advisory_lock({TEMPLATE_ADVISORY_LOCK_ID});"
                ))
                .await
                .expect("acquire PostgreSQL advisory lock for template setup");

            // Sweep idle `test_it_%` databases left behind by previous test binaries.
            let stale_stmt = Statement::from_string(
                admin_db.get_database_backend(),
                "SELECT d.datname FROM pg_database d \
                 WHERE d.datname LIKE 'test_it_%' \
                   AND d.datistemplate = false \
                   AND NOT EXISTS ( \
                       SELECT 1 FROM pg_stat_activity a WHERE a.datname = d.datname \
                   );"
                .to_string(),
            );
            if let Ok(rows) = admin_db.query_all(stale_stmt).await {
                for row in rows {
                    if let Ok(stale_name) = row.try_get::<String>("", "datname") {
                        let _ = admin_db
                            .execute_unprepared(&format!(
                                "DROP DATABASE IF EXISTS \"{stale_name}\" WITH (FORCE);"
                            ))
                            .await;
                    }
                }
            }

            admin_db
                .execute_unprepared(&format!(
                    "DROP DATABASE IF EXISTS {TEMPLATE_DB_NAME} WITH (FORCE);"
                ))
                .await
                .expect("drop existing template database");

            admin_db
                .execute_unprepared(&format!("CREATE DATABASE {TEMPLATE_DB_NAME};"))
                .await
                .expect("create template database");

            let template_url = replace_db_name_in_url(&admin_url, TEMPLATE_DB_NAME);
            let template_db = connect_single(&template_url).await;

            let schema_dir = workspace_root.join("database/schema");
            let mut schema_files: Vec<_> = fs::read_dir(&schema_dir)
                .unwrap_or_else(|err| {
                    panic!("Failed reading schema dir '{}': {err:?}", schema_dir.display())
                })
                .filter_map(|entry| entry.ok().map(|e| e.path()))
                .filter(|path| path.extension().is_some_and(|ext| ext == "sql"))
                .collect();
            schema_files.sort();

            for sql_path in schema_files {
                let sql = fs::read_to_string(&sql_path).unwrap_or_else(|err| {
                    panic!("Failed reading schema '{}': {err:?}", sql_path.display())
                });
                template_db
                    .execute_unprepared(&sql)
                    .await
                    .unwrap_or_else(|err| {
                        panic!("Failed executing schema '{}': {err:?}", sql_path.display())
                    });
            }

            let trigger_path = workspace_root.join("database/triggers/1_example_workspace_triggers.sql");
            let trigger_sql = fs::read_to_string(&trigger_path).unwrap_or_else(|err| {
                panic!(
                    "Failed reading trigger '{}': {err:?}",
                    trigger_path.display()
                )
            });
            template_db
                .execute_unprepared(&trigger_sql)
                .await
                .unwrap_or_else(|err| {
                    panic!(
                        "Failed executing trigger '{}': {err:?}",
                        trigger_path.display()
                    )
                });

            let _ = template_db.close().await;
            let _ = admin_db
                .execute_unprepared(&format!(
                    "SELECT pg_advisory_unlock({TEMPLATE_ADVISORY_LOCK_ID});"
                ))
                .await;
            let _ = admin_db.close().await;

            admin_url
        })
        .await
        .clone()
}
