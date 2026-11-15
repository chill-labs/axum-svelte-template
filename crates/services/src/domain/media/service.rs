// =============================================================================
// MEDIA DOMAIN SERVICE
// =============================================================================
//
// Manages polymorphic encrypted media records in `media_files` alongside
// optional Cloudflare R2 object presigning and cleanup.
//
// ## Key Concepts
//
// - **Graceful Storage Fallback**: When `storage` is `None`, `presign_upload`
//   and `confirm_upload` return `Err(AppError::Code(ErrorCode::StorageNotConfigured))`
//   cleanly while batch metadata queries still return empty maps.
// - **Polymorphic Slot Uniqueness**: `(entity_type, entity_id, field_name)` is
//   unique (`05_media_files.sql`). Replacing a file deletes the stale R2 object
//   best-effort so orphaned blobs do not accumulate.
//
// ## Architecture Notes
//
// Called by `UserService`, `WorkspaceService`, `TodoService`, and `/api/v1/media/*`.

// SECTION: IMPORTS

// `chrono` - UTC timestamps for `media_files.updated_at`.
use chrono::Utc;

// `sea_orm` - Asynchronous database persistence for `media_files`.
// - `ActiveModelTrait` - Inserts and updates `media_files::ActiveModel`
// - `ColumnTrait` - Filter columns (`EntityType`, `EntityId`, `FieldName`)
// - `DatabaseConnection` - PostgreSQL pool handle
// - `EntityTrait` - Query entry point for `media_files::Entity`
// - `QueryFilter` - Applies `WHERE` filters
// - `Set` - Sets ActiveModel column values
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

// `std::collections::HashMap` - Maps `entity_id -> MediaFileResponse` for batch loads.
use std::collections::HashMap;

// `tracing` - Logs non-fatal R2 object cleanup failures during replacement/deletion.
use tracing::warn;

// `shared` - Media DTOs, object key generation, XOR keys, and error codes.
// - `AppError` - Standardized error enum
// - `ErrorCode` - `StorageNotConfigured`, `MediaNotFound`
// - `ConfirmUploadRequest`, `MediaFileResponse`, `PresignUploadRequest`,
//   `PresignUploadResponse` - Media API payloads
// - `generate_object_key` - Builds collision-free `folder/id/uuid.enc` R2 paths
// - `generate_key` - Generates 64-char hex XOR obfuscation keys
use shared::domain::error::AppError;
use shared::domain::error_code::ErrorCode;
use shared::domain::media::{
    ConfirmUploadRequest, MediaFileResponse, PresignUploadRequest, PresignUploadResponse,
};
use shared::domain::upload::generate_object_key;
use shared::utils::obfuscation::generate_key;

// `crate::entities` - SeaORM entity and DTO mapper for `media_files`.
use crate::entities::conversions::map_media_file_response;
use crate::entities::media_files;

// `crate::storage` - Optional Cloudflare R2 / S3 client.
use crate::storage::StorageService;

// SECTION: SERVICE DEFINITION

/// Default `field_name` stored in `media_files` when callers omit a slot name.
pub const DEFAULT_MEDIA_FIELD: &str = "image";

/// Presigned R2 upload URL expiration window (`5` minutes).
const PRESIGN_EXPIRY_SECS: u64 = 300;

/// Service coordinating `media_files` persistence and optional R2 operations.
#[derive(Clone, Debug)]
pub struct MediaService {
    /// PostgreSQL connection pool.
    db: DatabaseConnection,
    /// Optional Cloudflare R2 storage client (`None` when R2 is unconfigured).
    storage: Option<StorageService>,
    /// Public CDN base URL prefix (empty when using `/api/v1/media/blob/:id`).
    public_media_base_url: String,
}

impl MediaService {
    /// Constructs a new `MediaService`.
    ///
    /// # Arguments
    ///
    /// * `db` - PostgreSQL pool handle
    /// * `storage` - `Some(StorageService)` if R2 is configured, otherwise `None`
    /// * `public_media_base_url` - Public CDN base URL from `Settings::public_media_url()`
    pub fn new(
        db: DatabaseConnection,
        storage: Option<StorageService>,
        public_media_base_url: String,
    ) -> Self {
        Self {
            db,
            storage,
            public_media_base_url,
        }
    }

    /// Returns `true` when Cloudflare R2 object storage is configured.
    pub fn is_storage_enabled(&self) -> bool {
        self.storage.is_some()
    }

    /// Issues a short-lived R2 presigned PUT URL and fresh 64-hex XOR obfuscation key.
    ///
    /// # Errors
    ///
    /// - `ErrorCode::StorageNotConfigured` (`503`) if R2 storage is disabled
    /// - `AppError::FieldErrors` (`400`) if file size or MIME type is invalid
    pub async fn presign_upload(
        &self,
        req: PresignUploadRequest,
    ) -> Result<PresignUploadResponse, AppError> {
        let storage = self
            .storage
            .as_ref()
            .ok_or(AppError::Code(ErrorCode::StorageNotConfigured))?;

        req.validate_input()?;

        let storage_key = generate_object_key(req.purpose, req.entity_id);
        let xor_key = generate_key();
        let upload_url = storage
            .presign_put(&storage_key, &req.content_type, PRESIGN_EXPIRY_SECS)
            .await?;

        Ok(PresignUploadResponse {
            upload_url,
            storage_key,
            xor_key,
            expires_in_secs: PRESIGN_EXPIRY_SECS,
        })
    }

    /// Confirms a direct-to-R2 encrypted upload and persists metadata in `media_files`.
    ///
    /// # Arguments
    ///
    /// * `user_id` - Authenticated user confirming the upload
    /// * `req` - Uploaded storage key, XOR key, MIME type, and byte length
    pub async fn confirm_upload(
        &self,
        user_id: i32,
        req: ConfirmUploadRequest,
    ) -> Result<MediaFileResponse, AppError> {
        if self.storage.is_none() {
            return Err(AppError::Code(ErrorCode::StorageNotConfigured));
        }
        req.validate_input()?;

        let field_name = req
            .field_name
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .unwrap_or(DEFAULT_MEDIA_FIELD);

        self.upsert_media(
            req.purpose.entity_type(),
            req.entity_id,
            field_name,
            req.storage_key,
            req.xor_key,
            req.content_type,
            req.file_size,
            Some(user_id),
        )
        .await
    }

    /// Inserts or replaces a polymorphic `media_files` record for `(entity_type, entity_id, field_name)`.
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_media(
        &self,
        entity_type: &str,
        entity_id: i32,
        field_name: &str,
        storage_key: String,
        xor_key: String,
        content_type: String,
        file_size: i64,
        uploaded_by: Option<i32>,
    ) -> Result<MediaFileResponse, AppError> {
        let existing = media_files::Entity::find()
            .filter(media_files::Column::EntityType.eq(entity_type))
            .filter(media_files::Column::EntityId.eq(entity_id))
            .filter(media_files::Column::FieldName.eq(field_name))
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed querying media slot: {err}")))?;

        let saved = match existing {
            Some(row) => {
                let stale_key = (row.storage_key != storage_key).then(|| row.storage_key.clone());
                let mut active: media_files::ActiveModel = row.into();
                active.storage_key = Set(storage_key);
                active.xor_key = Set(xor_key);
                active.content_type = Set(content_type);
                active.file_size = Set(file_size);
                active.uploaded_by = Set(uploaded_by);
                active.updated_at = Set(Utc::now().into());

                let updated = active.update(&self.db).await.map_err(|err| {
                    AppError::Internal(format!("Failed updating media record: {err}"))
                })?;

                if let (Some(old_key), Some(storage)) = (stale_key, self.storage.as_ref()) {
                    if let Err(err) = storage.delete_object(&old_key).await {
                        warn!(key = %old_key, error = %err, "Failed deleting superseded R2 object");
                    }
                }
                updated
            }
            None => {
                let active = media_files::ActiveModel {
                    entity_type: Set(entity_type.to_string()),
                    entity_id: Set(entity_id),
                    field_name: Set(field_name.to_string()),
                    storage_key: Set(storage_key),
                    xor_key: Set(xor_key),
                    content_type: Set(content_type),
                    file_size: Set(file_size),
                    uploaded_by: Set(uploaded_by),
                    ..Default::default()
                };
                active.insert(&self.db).await.map_err(|err| {
                    AppError::Internal(format!("Failed inserting media record: {err}"))
                })?
            }
        };

        Ok(map_media_file_response(&saved, &self.public_media_base_url))
    }

    /// Fetches a single media descriptor for `(entity_type, entity_id, field_name)`.
    pub async fn get_media(
        &self,
        entity_type: &str,
        entity_id: i32,
        field_name: &str,
    ) -> Result<Option<MediaFileResponse>, AppError> {
        let found = media_files::Entity::find()
            .filter(media_files::Column::EntityType.eq(entity_type))
            .filter(media_files::Column::EntityId.eq(entity_id))
            .filter(media_files::Column::FieldName.eq(field_name))
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed loading media file: {err}")))?;

        Ok(found.map(|m| map_media_file_response(&m, &self.public_media_base_url)))
    }

    /// Fetches a single media descriptor by its primary key ID.
    pub async fn get_media_by_id(
        &self,
        media_id: i32,
    ) -> Result<Option<MediaFileResponse>, AppError> {
        let found = media_files::Entity::find_by_id(media_id)
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed loading media by id: {err}")))?;

        Ok(found.map(|m| map_media_file_response(&m, &self.public_media_base_url)))
    }

    /// Batch-loads `entity_id -> MediaFileResponse` for lists without N+1 queries.
    pub async fn get_media_batch(
        &self,
        entity_type: &str,
        entity_ids: &[i32],
        field_name: &str,
    ) -> Result<HashMap<i32, MediaFileResponse>, AppError> {
        if entity_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let rows = media_files::Entity::find()
            .filter(media_files::Column::EntityType.eq(entity_type))
            .filter(media_files::Column::EntityId.is_in(entity_ids.to_vec()))
            .filter(media_files::Column::FieldName.eq(field_name))
            .all(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed batch loading media files: {err}")))?;

        let map = rows
            .into_iter()
            .map(|row| {
                let id = row.entity_id;
                let dto = map_media_file_response(&row, &self.public_media_base_url);
                (id, dto)
            })
            .collect();

        Ok(map)
    }

    /// Deletes the media slot from PostgreSQL and removes any stored R2 object best-effort.
    pub async fn delete_media(
        &self,
        entity_type: &str,
        entity_id: i32,
        field_name: &str,
    ) -> Result<(), AppError> {
        let existing = media_files::Entity::find()
            .filter(media_files::Column::EntityType.eq(entity_type))
            .filter(media_files::Column::EntityId.eq(entity_id))
            .filter(media_files::Column::FieldName.eq(field_name))
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed querying media to delete: {err}")))?;

        let Some(row) = existing else {
            return Ok(());
        };

        let storage_key = row.storage_key.clone();
        media_files::Entity::delete_by_id(row.id)
            .exec(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed deleting media row: {err}")))?;

        if let Some(storage) = self.storage.as_ref() {
            if let Err(err) = storage.delete_object(&storage_key).await {
                warn!(key = %storage_key, error = %err, "Failed deleting R2 object");
            }
        }

        Ok(())
    }

    /// Downloads raw obfuscated bytes for a `media_files` record from R2.
    pub async fn fetch_blob(&self, media_id: i32) -> Result<(Vec<u8>, String), AppError> {
        let storage = self
            .storage
            .as_ref()
            .ok_or(AppError::Code(ErrorCode::StorageNotConfigured))?;

        let row = media_files::Entity::find_by_id(media_id)
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed loading media blob: {err}")))?
            .ok_or(AppError::Code(ErrorCode::MediaNotFound))?;

        let (bytes, r2_ct) = storage.get_object(&row.storage_key).await?;
        let content_type = r2_ct.unwrap_or(row.content_type);
        Ok((bytes, content_type))
    }
}
