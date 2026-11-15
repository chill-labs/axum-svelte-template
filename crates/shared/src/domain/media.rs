// =============================================================================
// PROTECTED MEDIA & OPTIONAL R2 UPLOAD DTOs
// =============================================================================
//
// Presigned upload + confirmation payloads for client-side XOR obfuscated
// image uploads and metadata retrieval for `<ProtectedImage />`.

use std::collections::HashMap;

// `chrono` - Media creation timestamps.
use chrono::{DateTime, Utc};

// `serde` - Serialization for presign, confirm, and metadata lookup APIs.
use serde::{Deserialize, Serialize};

use crate::domain::error::AppError;
use crate::domain::error_code::ErrorCode;
use crate::domain::upload::{
    UploadPurpose, ALLOWED_IMAGE_CONTENT_TYPES, MAX_IMAGE_UPLOAD_BYTES,
};
use crate::domain::validator::finish_validation;

/// Stored media descriptor returned to clients so `ProtectedImage` can deobfuscate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaFileResponse {
    pub id: i32,
    pub entity_type: String,
    pub entity_id: i32,
    pub field_name: String,
    pub storage_key: String,
    pub xor_key: String,
    pub content_type: String,
    pub file_size: i64,
    pub public_url: String,
    pub created_at: DateTime<Utc>,
}

/// Request to obtain a short-lived R2 presigned PUT URL for an encrypted blob.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresignUploadRequest {
    pub purpose: UploadPurpose,
    pub entity_id: i32,
    pub content_type: String,
    pub file_size: i64,
}

impl PresignUploadRequest {
    /// Validates upload file size (`<= 5 MiB`) and allowed MIME types.
    pub fn validate_input(&self) -> Result<(), AppError> {
        let mut errors = HashMap::new();
        if self.entity_id <= 0 {
            errors.insert("entity_id".to_string(), ErrorCode::ValidationFailed);
        }
        if self.file_size <= 0 || self.file_size > MAX_IMAGE_UPLOAD_BYTES {
            errors.insert("file_size".to_string(), ErrorCode::FileTooLarge);
        }
        if !ALLOWED_IMAGE_CONTENT_TYPES.contains(&self.content_type.as_str()) {
            errors.insert("content_type".to_string(), ErrorCode::UnsupportedMediaType);
        }
        finish_validation(errors)
    }
}

/// Response containing presigned R2 PUT URL plus server-generated XOR key & object key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresignUploadResponse {
    pub upload_url: String,
    pub storage_key: String,
    pub xor_key: String,
    pub expires_in_secs: u64,
}

/// Request confirming a completed R2 upload to persist metadata into `media_files`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmUploadRequest {
    pub purpose: UploadPurpose,
    pub entity_id: i32,
    pub field_name: Option<String>,
    pub storage_key: String,
    pub xor_key: String,
    pub content_type: String,
    pub file_size: i64,
}

impl ConfirmUploadRequest {
    /// Validates non-empty keys and valid bounds.
    pub fn validate_input(&self) -> Result<(), AppError> {
        let mut errors = HashMap::new();
        if self.storage_key.trim().is_empty() {
            errors.insert("storage_key".to_string(), ErrorCode::FieldRequired);
        }
        if self.xor_key.trim().is_empty() {
            errors.insert("xor_key".to_string(), ErrorCode::FieldRequired);
        }
        if self.file_size <= 0 || self.file_size > MAX_IMAGE_UPLOAD_BYTES {
            errors.insert("file_size".to_string(), ErrorCode::FileTooLarge);
        }
        finish_validation(errors)
    }
}
