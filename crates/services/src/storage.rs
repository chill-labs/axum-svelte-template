// =============================================================================
// OPTIONAL CLOUDFLARE R2 / S3 OBJECT STORAGE SERVICE
// =============================================================================
//
// Wraps `aws-sdk-s3` configured for Cloudflare R2 S3-compatible APIs.
// Intentionally optional: when `APP__R2__*` credentials are empty or omitted,
// `StorageService::from_settings` returns `None` cleanly with zero startup errors.
//
// ## Key Concepts
//
// - **R2 Endpoint**: `https://{account_id}.r2.cloudflarestorage.com` with region
//   `"auto"` and `force_path_style(true)`.
// - **Presigned PUT**: Issues short-lived signed upload URLs so clients upload
//   XOR-obfuscated image payloads directly to R2 without buffering in Rust RAM.
//
// ## Architecture Notes
//
// Injected as `Option<StorageService>` inside `AppState` and `MediaService`.

// SECTION: IMPORTS

// `aws_config` - Standard AWS SDK behavior version selector.
// - `BehaviorVersion` - Selects latest SDK behavior defaults without IMDS probing
use aws_config::BehaviorVersion;

// `aws_sdk_s3` - S3-compatible client and presigned URL generator for R2.
// - `Client` - Asynchronous S3/R2 object storage client
// - `Config` - Explicit S3 client builder supporting synchronous construction
// - `Credentials` - Static access-key / secret-key credential provider
// - `Region` - Region wrapper configured to `"auto"` for Cloudflare R2
// - `PresigningConfig` - Expiration config for presigned PUT requests
use aws_sdk_s3::config::{Credentials, Region};
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::{Client, Config};

// `std::time::Duration` - Presigned URL TTL duration.
use std::time::Duration;

// `shared` - Environment settings and typed application error mapping.
// - `Settings` - Root settings containing optional `R2Config`
// - `AppError` - Service error container
use shared::config::settings::Settings;
use shared::domain::error::AppError;

// SECTION: STORAGE SERVICE

/// Optional Cloudflare R2 / S3 object storage wrapper.
#[derive(Clone, Debug)]
pub struct StorageService {
    /// Configured S3 SDK client pointing at Cloudflare R2.
    client: Client,
    /// Target R2 bucket name.
    bucket: String,
    /// Optional CDN or public bucket URL prefix (without trailing slash).
    public_url: String,
}

impl StorageService {
    /// Constructs `Some(StorageService)` if `settings.active_r2()` is configured, else `None`.
    ///
    /// Performs local configuration only and never blocks startup or fails when
    /// R2 environment variables are left blank.
    ///
    /// # Arguments
    ///
    /// * `settings` - Application settings loaded from environment variables
    pub fn from_settings(settings: &Settings) -> Option<Self> {
        let r2 = settings.active_r2()?;

        let endpoint_url = format!("https://{}.r2.cloudflarestorage.com", r2.account_id.trim());
        let credentials = Credentials::new(
            r2.access_key_id.trim(),
            r2.secret_access_key.trim(),
            None,
            None,
            "cloudflare-r2",
        );

        let s3_config = Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new("auto"))
            .endpoint_url(endpoint_url)
            .credentials_provider(credentials)
            .force_path_style(true)
            .build();

        let client = Client::from_conf(s3_config);

        Some(Self {
            client,
            bucket: r2.bucket_name.trim().to_string(),
            public_url: r2.public_url.trim().trim_end_matches('/').to_string(),
        })
    }

    /// Returns the configured R2 bucket name.
    pub fn bucket_name(&self) -> &str {
        &self.bucket
    }

    /// Returns the configured public CDN URL prefix (may be empty if private bucket).
    pub fn public_url(&self) -> &str {
        &self.public_url
    }

    /// Generates a short-lived presigned PUT URL for uploading an object directly to R2.
    ///
    /// # Arguments
    ///
    /// * `key` - Target object key inside the bucket (e.g. `todos/42/<uuid>.enc`)
    /// * `content_type` - Expected HTTP `Content-Type` header on the client PUT request
    /// * `expires_secs` - URL expiration window in seconds (clamped to `1..=604_800`)
    pub async fn presign_put(
        &self,
        key: &str,
        content_type: &str,
        expires_secs: u64,
    ) -> Result<String, AppError> {
        let ttl = Duration::from_secs(expires_secs.clamp(1, 604_800));
        let presign_config = PresigningConfig::expires_in(ttl)
            .map_err(|err| AppError::Internal(format!("Invalid R2 presign TTL: {err}")))?;

        let presigned = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type(content_type)
            .presigned(presign_config)
            .await
            .map_err(|err| AppError::Internal(format!("Failed to presign R2 PUT URL: {err}")))?;

        Ok(presigned.uri().to_string())
    }

    /// Deletes an object at `key` from the configured R2 bucket.
    ///
    /// # Arguments
    ///
    /// * `key` - Object key to remove from storage
    pub async fn delete_object(&self, key: &str) -> Result<(), AppError> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|err| {
                AppError::Internal(format!("Failed to delete R2 object '{key}': {err}"))
            })?;

        Ok(())
    }

    /// Downloads raw object bytes and optional content-type from R2 (used by blob proxy fallback).
    ///
    /// # Arguments
    ///
    /// * `key` - Target object key inside the bucket
    pub async fn get_object(&self, key: &str) -> Result<(Vec<u8>, Option<String>), AppError> {
        let output = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|err| {
                AppError::Internal(format!("Failed to fetch R2 object '{key}': {err}"))
            })?;

        let content_type = output.content_type().map(ToString::to_string);
        let collected = output
            .body
            .collect()
            .await
            .map_err(|err| AppError::Internal(format!("Failed reading R2 object stream: {err}")))?;

        Ok((collected.into_bytes().to_vec(), content_type))
    }
}

// SECTION: UNIT TESTS

#[cfg(test)]
mod tests {
    use super::*;
    use shared::config::settings::R2Config;

    #[test]
    fn from_settings_returns_none_when_r2_disabled_or_incomplete() {
        let mut settings = Settings::for_test("postgres://localhost/test");
        assert!(StorageService::from_settings(&settings).is_none());

        settings.r2 = Some(R2Config::default());
        assert!(StorageService::from_settings(&settings).is_none());
    }

    #[tokio::test]
    async fn from_settings_constructs_and_presigns_when_r2_configured() {
        let mut settings = Settings::for_test("postgres://localhost/test");
        settings.r2 = Some(R2Config {
            account_id: "acct123".to_string(),
            access_key_id: "key123".to_string(),
            secret_access_key: "secret123".to_string(),
            bucket_name: "my-bucket".to_string(),
            public_url: "https://cdn.example.com/".to_string(),
        });

        let storage = StorageService::from_settings(&settings).expect("R2 should be enabled");
        assert_eq!(storage.bucket_name(), "my-bucket");
        assert_eq!(storage.public_url(), "https://cdn.example.com");

        let url = storage
            .presign_put("todos/1/sample.enc", "application/octet-stream", 300)
            .await
            .expect("presign URL generation should succeed offline");
        assert!(url.contains("acct123.r2.cloudflarestorage.com/my-bucket/todos/1/sample.enc"));
    }
}
