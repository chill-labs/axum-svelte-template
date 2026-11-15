// =============================================================================
// TYPED APPLICATION ERROR CODES
// =============================================================================
//
// Machine-readable error constants shared across Rust services, JSON responses,
// and generated TypeScript contracts (`gen-ts` parses `define_error_codes!`).
// Svelte UI translates each code using bilingual `vi` / `en` dictionaries.

// `axum` - HTTP status codes returned per error variant.
use axum::http::StatusCode;

// `serde` - Serializes error codes consistently in `SCREAMING_SNAKE_CASE`.
use serde::{Deserialize, Serialize};

macro_rules! define_error_codes {
    (
        $(
            $(#[$meta:meta])*
            $variant:ident => $code:literal, $status:expr
        ),* $(,)?
    ) => {
        /// Machine-readable domain and validation error identifiers.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
        pub enum ErrorCode {
            $(
                $(#[$meta])*
                $variant,
            )*
        }

        impl ErrorCode {
            /// Returns the canonical uppercase error code string.
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $code,)*
                }
            }

            /// Maps the error variant to its HTTP status code.
            pub const fn http_status(self) -> StatusCode {
                match self {
                    $(Self::$variant => $status,)*
                }
            }
        }
    };
}

define_error_codes! {
    // Authentication & Session
    Unauthorized => "UNAUTHORIZED", StatusCode::UNAUTHORIZED,
    InvalidCredentials => "INVALID_CREDENTIALS", StatusCode::UNAUTHORIZED,
    AccountDisabled => "ACCOUNT_DISABLED", StatusCode::FORBIDDEN,
    Forbidden => "FORBIDDEN", StatusCode::FORBIDDEN,
    AdminRequired => "ADMIN_REQUIRED", StatusCode::FORBIDDEN,
    PermissionDenied => "PERMISSION_DENIED", StatusCode::FORBIDDEN,
    OAuthNotConfigured => "OAUTH_NOT_CONFIGURED", StatusCode::SERVICE_UNAVAILABLE,
    OAuthExchangeFailed => "OAUTH_EXCHANGE_FAILED", StatusCode::BAD_GATEWAY,
    HotSupportSessionInvalid => "HOT_SUPPORT_SESSION_INVALID", StatusCode::UNAUTHORIZED,

    // Validation & Generic Requests
    ValidationFailed => "VALIDATION_FAILED", StatusCode::BAD_REQUEST,
    FieldRequired => "FIELD_REQUIRED", StatusCode::BAD_REQUEST,
    InvalidEmail => "INVALID_EMAIL", StatusCode::BAD_REQUEST,
    InvalidSlug => "INVALID_SLUG", StatusCode::BAD_REQUEST,
    InvalidHexColor => "INVALID_HEX_COLOR", StatusCode::BAD_REQUEST,
    PasswordTooWeak => "PASSWORD_TOO_WEAK", StatusCode::BAD_REQUEST,
    InvalidPagination => "INVALID_PAGINATION", StatusCode::BAD_REQUEST,
    NotFound => "NOT_FOUND", StatusCode::NOT_FOUND,
    Conflict => "CONFLICT", StatusCode::CONFLICT,
    InternalError => "INTERNAL_ERROR", StatusCode::INTERNAL_SERVER_ERROR,

    // Users
    UserNotFound => "USER_NOT_FOUND", StatusCode::NOT_FOUND,
    EmailAlreadyExists => "EMAIL_ALREADY_EXISTS", StatusCode::CONFLICT,

    // Workspaces & Members
    WorkspaceNotFound => "WORKSPACE_NOT_FOUND", StatusCode::NOT_FOUND,
    WorkspaceSlugTaken => "WORKSPACE_SLUG_TAKEN", StatusCode::CONFLICT,
    CannotDeleteLastWorkspace => "CANNOT_DELETE_LAST_WORKSPACE", StatusCode::BAD_REQUEST,
    CannotRemoveWorkspaceOwner => "CANNOT_REMOVE_WORKSPACE_OWNER", StatusCode::BAD_REQUEST,
    MemberAlreadyExists => "MEMBER_ALREADY_EXISTS", StatusCode::CONFLICT,
    MemberNotFound => "MEMBER_NOT_FOUND", StatusCode::NOT_FOUND,

    // Todos
    TodoNotFound => "TODO_NOT_FOUND", StatusCode::NOT_FOUND,
    TodoTitleTooLong => "TODO_TITLE_TOO_LONG", StatusCode::BAD_REQUEST,
    AssigneeNotInWorkspace => "ASSIGNEE_NOT_IN_WORKSPACE", StatusCode::BAD_REQUEST,

    // Notifications
    NotificationNotFound => "NOTIFICATION_NOT_FOUND", StatusCode::NOT_FOUND,

    // Media & Optional Cloudflare R2 Storage
    StorageNotConfigured => "STORAGE_NOT_CONFIGURED", StatusCode::SERVICE_UNAVAILABLE,
    UnsupportedMediaType => "UNSUPPORTED_MEDIA_TYPE", StatusCode::BAD_REQUEST,
    FileTooLarge => "FILE_TOO_LARGE", StatusCode::BAD_REQUEST,
    MediaNotFound => "MEDIA_NOT_FOUND", StatusCode::NOT_FOUND,
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_code_serializes_as_screaming_snake_case() {
        let json = serde_json::to_string(&ErrorCode::StorageNotConfigured).unwrap();
        assert_eq!(json, "\"STORAGE_NOT_CONFIGURED\"");
        assert_eq!(
            ErrorCode::StorageNotConfigured.http_status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
    }
}
