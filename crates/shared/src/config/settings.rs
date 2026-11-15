// =============================================================================
// APPLICATION SETTINGS LOADER
// =============================================================================
//
// Hierarchical environment configuration using `config` + `dotenvy`.
// Supports `APP__SECTION__KEY` overrides and validates critical security
// invariants in non-dev environments.

// `config` - Layered configuration builder from defaults and environment variables.
use config::{Config, ConfigError, Environment};

// `serde` - Automatic deserialization of nested configuration sections.
use serde::Deserialize;

/// Deployment environment classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ServerEnv {
    /// Local development (enables `/api/test/*` helpers and non-Secure localhost cookies).
    #[default]
    Dev,
    /// Staging environment matching production hardening with staging domain.
    Staging,
    /// Production environment with strict cookie security and TLS expectations.
    Prod,
}

impl ServerEnv {
    /// Returns `true` when running in local development mode.
    pub fn is_dev(self) -> bool {
        self == Self::Dev
    }

    /// Returns `true` when HTTPS `Secure` cookie attributes should be enforced.
    pub fn use_secure_cookies(self) -> bool {
        !self.is_dev()
    }
}

/// Root configuration structure loaded at server startup.
#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    /// Environment name (`dev`, `staging`, `prod`).
    #[serde(default)]
    pub env: ServerEnv,
    /// PostgreSQL connection configuration.
    pub database: DatabaseConfig,
    /// Tracing / log formatting settings.
    #[serde(default)]
    pub logging: LoggingConfig,
    /// HTTP bind host, port, and public canonical domain.
    #[serde(default)]
    pub server: ServerConfig,
    /// Symmetric signing / verification secret.
    pub jwt: JwtConfig,
    /// Bootstrap admin identity configuration.
    #[serde(default)]
    pub admin: AdminConfig,
    /// Google OAuth 2.0 application credentials.
    #[serde(default)]
    pub google: GoogleConfig,
    /// Moka in-memory cache TTL and capacity settings.
    #[serde(default)]
    pub cache: CacheConfig,
    /// Optional Cloudflare R2 / S3 storage settings.
    #[serde(default)]
    pub r2: Option<R2Config>,
}

/// PostgreSQL connection pool settings.
#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    /// PostgreSQL connection URI (`postgres://user:pass@host:port/db`).
    pub url: String,
    /// Maximum connection pool size.
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
}

fn default_max_connections() -> u32 {
    20
}

/// Tracing subscriber configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    /// EnvFilter directive string (e.g. `info,server=debug`).
    #[serde(default = "default_log_level")]
    pub level: String,
    /// Output format (`pretty` or `json`).
    #[serde(default = "default_log_format")]
    pub format: String,
    /// Optional log directory for daily rolling log files.
    #[serde(default)]
    pub file_dir: Option<String>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: default_log_format(),
            file_dir: None,
        }
    }
}

fn default_log_level() -> String {
    "info,server=debug,services=debug,api=debug".to_string()
}

fn default_log_format() -> String {
    "pretty".to_string()
}

/// HTTP server listener settings.
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    /// Bind host address.
    #[serde(default = "default_host")]
    pub host: String,
    /// Bind TCP port.
    #[serde(default = "default_port")]
    pub port: u16,
    /// Canonical public origin (used for OG metadata and OAuth links).
    #[serde(default = "default_domain")]
    pub domain: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            domain: default_domain(),
        }
    }
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    3000
}

fn default_domain() -> String {
    "http://localhost:3000".to_string()
}

/// JWT / HMAC secret configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct JwtConfig {
    /// Secret key (must be >= 32 characters in staging/prod).
    pub secret: String,
}

/// Seeded administrator account configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct AdminConfig {
    /// Admin email address provisioned on server boot.
    #[serde(default = "default_admin_email")]
    pub email: String,
    /// Display name for the seeded admin account.
    #[serde(default = "default_admin_name")]
    pub name: String,
}

impl Default for AdminConfig {
    fn default() -> Self {
        Self {
            email: default_admin_email(),
            name: default_admin_name(),
        }
    }
}

fn default_admin_email() -> String {
    "admin@example.com".to_string()
}

fn default_admin_name() -> String {
    "Template Admin".to_string()
}

/// Google OAuth 2.0 client settings.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct GoogleConfig {
    /// OAuth Client ID.
    #[serde(default)]
    pub client_id: String,
    /// OAuth Client Secret.
    #[serde(default)]
    pub client_secret: String,
    /// Registered OAuth callback URI.
    #[serde(default = "default_google_redirect_uri")]
    pub redirect_uri: String,
}

impl GoogleConfig {
    /// Returns `true` when non-placeholder OAuth credentials are configured.
    pub fn is_configured(&self) -> bool {
        !self.client_id.trim().is_empty()
            && !self.client_secret.trim().is_empty()
            && !self.client_id.starts_with("your-")
            && !self.client_secret.starts_with("your-")
    }
}

fn default_google_redirect_uri() -> String {
    "http://localhost:3000/auth/google/callback".to_string()
}

/// Moka cache configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct CacheConfig {
    /// Session cache time-to-live in seconds.
    #[serde(default = "default_ttl_secs")]
    pub session_ttl_secs: u64,
    /// User cache time-to-live in seconds.
    #[serde(default = "default_ttl_secs")]
    pub user_ttl_secs: u64,
    /// Maximum cached entry capacity per cache.
    #[serde(default = "default_max_capacity")]
    pub max_capacity: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            session_ttl_secs: default_ttl_secs(),
            user_ttl_secs: default_ttl_secs(),
            max_capacity: default_max_capacity(),
        }
    }
}

fn default_ttl_secs() -> u64 {
    300
}

fn default_max_capacity() -> u64 {
    10_000
}

/// Optional Cloudflare R2 / S3 object storage configuration.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct R2Config {
    /// Cloudflare Account ID.
    #[serde(default)]
    pub account_id: String,
    /// R2 Access Key ID.
    #[serde(default)]
    pub access_key_id: String,
    /// R2 Secret Access Key.
    #[serde(default)]
    pub secret_access_key: String,
    /// Target R2 Bucket name.
    #[serde(default)]
    pub bucket_name: String,
    /// Optional CDN / custom public URL prefix (`https://cdn.example.com`).
    #[serde(default)]
    pub public_url: String,
}

impl R2Config {
    /// Returns `true` only if all required S3/R2 fields are populated.
    pub fn is_complete(&self) -> bool {
        !self.account_id.trim().is_empty()
            && !self.access_key_id.trim().is_empty()
            && !self.secret_access_key.trim().is_empty()
            && !self.bucket_name.trim().is_empty()
    }
}

impl Settings {
    /// Loads `.env` (if present) and constructs `Settings` from `APP__*` env vars.
    pub fn load() -> Result<Self, ConfigError> {
        let _ = dotenvy::dotenv();

        let builder = Config::builder()
            .add_source(Environment::with_prefix("APP").separator("__"));

        let settings: Self = builder.build()?.try_deserialize()?;
        settings.validate_invariants()?;
        Ok(settings)
    }

    /// Builds `Settings` explicitly for isolated integration or unit tests.
    pub fn for_test(database_url: impl Into<String>) -> Self {
        Self {
            env: ServerEnv::Dev,
            database: DatabaseConfig {
                url: database_url.into(),
                max_connections: 5,
            },
            logging: LoggingConfig::default(),
            server: ServerConfig::default(),
            jwt: JwtConfig {
                secret: "test-only-secret-at-least-32-characters-long".to_string(),
            },
            admin: AdminConfig::default(),
            google: GoogleConfig::default(),
            cache: CacheConfig::default(),
            r2: None,
        }
    }

    /// Returns `Some(&R2Config)` only when object storage credentials are completely set.
    pub fn active_r2(&self) -> Option<&R2Config> {
        self.r2.as_ref().filter(|cfg| cfg.is_complete())
    }

    /// Resolves public media base URL (empty string when R2 is disabled).
    pub fn public_media_url(&self) -> String {
        self.active_r2()
            .map(|r2| r2.public_url.trim_end_matches('/').to_string())
            .unwrap_or_default()
    }

    fn validate_invariants(&self) -> Result<(), ConfigError> {
        if !self.env.is_dev() && self.jwt.secret.trim().len() < 32 {
            return Err(ConfigError::Message(
                "APP__JWT__SECRET must be at least 32 characters outside dev mode".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_r2_is_none_for_empty_fields() {
        let mut settings = Settings::for_test("postgres://localhost/test");
        settings.r2 = Some(R2Config::default());
        assert!(settings.active_r2().is_none());
        assert_eq!(settings.public_media_url(), "");
    }

    #[test]
    fn active_r2_returns_config_when_populated() {
        let mut settings = Settings::for_test("postgres://localhost/test");
        settings.r2 = Some(R2Config {
            account_id: "acct".into(),
            access_key_id: "key".into(),
            secret_access_key: "secret".into(),
            bucket_name: "bucket".into(),
            public_url: "https://cdn.example.com/".into(),
        });
        assert!(settings.active_r2().is_some());
        assert_eq!(settings.public_media_url(), "https://cdn.example.com");
    }
}
