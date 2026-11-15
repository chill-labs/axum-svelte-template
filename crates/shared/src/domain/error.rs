// =============================================================================
// DOMAIN ERROR WRAPPER & HTTP RESPONSE ADAPTER
// =============================================================================
//
// Converts typed service errors (`AppError`) into consistent `ApiResponse<T>`
// JSON payloads paired with the matching HTTP status code.

use std::collections::HashMap;

// `axum` - Translates `AppError` into Axum HTTP responses.
use axum::response::{IntoResponse, Response};

// `thiserror` - Ergonomic error enum derivation for services and handlers.
use thiserror::Error;

use crate::domain::error_code::ErrorCode;
use crate::domain::response::ApiResponse;

/// Standardized error type returned across `services` and `api`.
#[derive(Debug, Error)]
pub enum AppError {
    /// Typed error code mapped directly to its HTTP status.
    #[error("{0}")]
    Code(ErrorCode),

    /// Validation failure containing per-field error codes for inline UI rendering.
    #[error("VALIDATION_FAILED")]
    FieldErrors(HashMap<String, ErrorCode>),

    /// Domain error paired with contextual diagnostic message (logged, code sent cleanly).
    #[error("{code}: {detail}")]
    WithDetail { code: ErrorCode, detail: String },

    /// Unexpected internal error converted to `500 INTERNAL_ERROR`.
    #[error("INTERNAL_ERROR: {0}")]
    Internal(String),
}

impl AppError {
    /// Extracts the machine-readable `ErrorCode`.
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::Code(code) => *code,
            Self::FieldErrors(_) => ErrorCode::ValidationFailed,
            Self::WithDetail { code, .. } => *code,
            Self::Internal(_) => ErrorCode::InternalError,
        }
    }

    /// Converts an optional field error map into `ApiResponse::error`.
    pub fn into_api_response<T>(self) -> ApiResponse<T> {
        match self {
            Self::Code(code) => ApiResponse::error(code),
            Self::WithDetail { code, .. } => ApiResponse::error(code),
            Self::Internal(_) => ApiResponse::error(ErrorCode::InternalError),
            Self::FieldErrors(fields) => {
                let mapped = fields
                    .into_iter()
                    .map(|(k, v)| (k, v.as_str().to_string()))
                    .collect();
                ApiResponse::error_with_fields(ErrorCode::ValidationFailed, mapped)
            }
        }
    }
}

impl From<ErrorCode> for AppError {
    fn from(value: ErrorCode) -> Self {
        Self::Code(value)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.code().http_status();
        let body: ApiResponse<()> = self.into_api_response();
        (status, axum::Json(body)).into_response()
    }
}

/// Helper trait turning `Result<T, AppError>` directly into Axum `(StatusCode, Json<ApiResponse<T>>)` responses.
pub trait ToResponse<T> {
    /// Converts `Ok(T)` -> `200 OK`, `Err(AppError)` -> matching status + JSON envelope.
    fn to_response(self) -> Response;

    /// Converts `Ok(T)` -> `201 Created`, `Err(AppError)` -> matching status + JSON envelope.
    fn to_response_created(self) -> Response;
}

impl<T: serde::Serialize> ToResponse<T> for Result<T, AppError> {
    fn to_response(self) -> Response {
        match self {
            Ok(data) => ApiResponse::success(data).into_response(),
            Err(err) => err.into_response(),
        }
    }

    fn to_response_created(self) -> Response {
        match self {
            Ok(data) => ApiResponse::success_created(data).into_response(),
            Err(err) => err.into_response(),
        }
    }
}
