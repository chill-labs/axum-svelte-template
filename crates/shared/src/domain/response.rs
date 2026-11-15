// =============================================================================
// STANDARD API RESPONSE ENVELOPE & PAGINATION
// =============================================================================
//
// Canonical JSON response wrapper returned by every `/api/*` endpoint.
// Preserves the exact shape expected by Svelte frontend clients and `gen-ts`.

use std::collections::HashMap;

// `axum` - Serializes envelope with appropriate HTTP status code.
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

// `serde` - Serialization and deserialization of API JSON envelopes.
use serde::{Deserialize, Serialize};

use crate::domain::error_code::ErrorCode;

/// Success sub-status categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SuccessType {
    Ok,
    Created,
}

/// Error sub-status categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorType {
    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict,
    ServiceUnavailable,
    InternalServerError,
}

/// Discriminated API status union (`Success` or `Error`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    Success(SuccessType),
    Error(ErrorType),
}

impl Status {
    /// Resolves the HTTP `StatusCode` associated with this status tag.
    pub const fn http_status(self) -> StatusCode {
        match self {
            Self::Success(SuccessType::Ok) => StatusCode::OK,
            Self::Success(SuccessType::Created) => StatusCode::CREATED,
            Self::Error(ErrorType::BadRequest) => StatusCode::BAD_REQUEST,
            Self::Error(ErrorType::Unauthorized) => StatusCode::UNAUTHORIZED,
            Self::Error(ErrorType::Forbidden) => StatusCode::FORBIDDEN,
            Self::Error(ErrorType::NotFound) => StatusCode::NOT_FOUND,
            Self::Error(ErrorType::Conflict) => StatusCode::CONFLICT,
            Self::Error(ErrorType::ServiceUnavailable) => StatusCode::SERVICE_UNAVAILABLE,
            Self::Error(ErrorType::InternalServerError) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<StatusCode> for ErrorType {
    fn from(value: StatusCode) -> Self {
        match value {
            StatusCode::BAD_REQUEST => Self::BadRequest,
            StatusCode::UNAUTHORIZED => Self::Unauthorized,
            StatusCode::FORBIDDEN => Self::Forbidden,
            StatusCode::NOT_FOUND => Self::NotFound,
            StatusCode::CONFLICT => Self::Conflict,
            StatusCode::SERVICE_UNAVAILABLE | StatusCode::BAD_GATEWAY => Self::ServiceUnavailable,
            _ => Self::InternalServerError,
        }
    }
}

/// Pagination metadata returned by list endpoints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pagination {
    /// Current 1-based page number.
    pub page: u64,
    /// Maximum items per page.
    pub page_size: u64,
    /// Total matching records across all pages.
    pub total_items: u64,
    /// Total computed page count.
    pub total_pages: u64,
}

impl Pagination {
    /// Constructs normalized pagination metadata.
    pub fn new(page: u64, page_size: u64, total_items: u64) -> Self {
        let safe_page_size = page_size.max(1);
        let total_pages = if total_items == 0 {
            1
        } else {
            total_items.div_ceil(safe_page_size)
        };
        Self {
            page: page.max(1),
            page_size: safe_page_size,
            total_items,
            total_pages,
        }
    }
}

/// Unified response wrapper for all JSON API handlers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    /// `true` on `2xx` responses, `false` otherwise.
    pub success: bool,
    /// Canonical `OK`, `CREATED`, or uppercase `ErrorCode` string.
    pub message: String,
    /// Structured status tag.
    pub status: Status,
    /// Payload data when successful.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    /// Optional pagination summary for list endpoints.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination: Option<Pagination>,
    /// Optional field-level validation error codes (`field -> ErrorCode`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<HashMap<String, String>>,
}

impl<T> ApiResponse<T> {
    /// Wraps payload data in a `200 OK` envelope.
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            message: "OK".to_string(),
            status: Status::Success(SuccessType::Ok),
            data: Some(data),
            pagination: None,
            errors: None,
        }
    }

    /// Wraps payload data in a `201 Created` envelope.
    pub fn success_created(data: T) -> Self {
        Self {
            success: true,
            message: "CREATED".to_string(),
            status: Status::Success(SuccessType::Created),
            data: Some(data),
            pagination: None,
            errors: None,
        }
    }

    /// Wraps paginated list data in a `200 OK` envelope.
    pub fn success_paginated(data: T, pagination: Pagination) -> Self {
        Self {
            success: true,
            message: "OK".to_string(),
            status: Status::Success(SuccessType::Ok),
            data: Some(data),
            pagination: Some(pagination),
            errors: None,
        }
    }

    /// Constructs a failure envelope from an `ErrorCode`.
    pub fn error(code: ErrorCode) -> Self {
        Self {
            success: false,
            message: code.as_str().to_string(),
            status: Status::Error(ErrorType::from(code.http_status())),
            data: None,
            pagination: None,
            errors: None,
        }
    }

    /// Constructs a failure envelope carrying per-field validation errors.
    pub fn error_with_fields(code: ErrorCode, errors: HashMap<String, String>) -> Self {
        Self {
            success: false,
            message: code.as_str().to_string(),
            status: Status::Error(ErrorType::from(code.http_status())),
            data: None,
            pagination: None,
            errors: Some(errors),
        }
    }
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        (self.status.http_status(), Json(self)).into_response()
    }
}
