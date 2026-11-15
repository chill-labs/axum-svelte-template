// =============================================================================
// AXUM REQUEST EXTRACTORS (`AuthUser`, `OptionalAuthUser`, `AdminUser`)
// =============================================================================
//
// Reads `Option<AuthenticatedUser>` injected by `auth_middleware` and enforces
// authentication or system-wide administrator guarantees.

// `axum` - Async `FromRequestParts` trait and `Extension` extraction.
use axum::extract::FromRequestParts;
use axum::http::request::Parts;

use services::middleware::auth::AuthenticatedUser;
use shared::domain::error::AppError;
use shared::domain::error_code::ErrorCode;
use shared::domain::user::UserRole;

/// Extracts an authenticated user from request extensions (`401 UNAUTHORIZED` otherwise).
#[derive(Debug, Clone)]
pub struct AuthUser(pub AuthenticatedUser);

impl<S: Send + Sync> FromRequestParts<S> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let maybe = parts
            .extensions
            .get::<Option<AuthenticatedUser>>()
            .cloned()
            .flatten();

        match maybe {
            Some(user) => Ok(Self(user)),
            None => Err(AppError::Code(ErrorCode::Unauthorized)),
        }
    }
}

/// Extracts `Option<AuthenticatedUser>` cleanly for mixed guest/authenticated endpoints.
#[derive(Debug, Clone)]
pub struct OptionalAuthUser(pub Option<AuthenticatedUser>);

impl<S: Send + Sync> FromRequestParts<S> for OptionalAuthUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let maybe = parts
            .extensions
            .get::<Option<AuthenticatedUser>>()
            .cloned()
            .flatten();
        Ok(Self(maybe))
    }
}

/// Extracts a system administrator, strictly rejecting Hot Support impersonation frames.
#[derive(Debug, Clone)]
pub struct AdminUser(pub AuthenticatedUser);

impl<S: Send + Sync> FromRequestParts<S> for AdminUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let maybe = parts
            .extensions
            .get::<Option<AuthenticatedUser>>()
            .cloned()
            .flatten();

        let Some(auth) = maybe else {
            return Err(AppError::Code(ErrorCode::Unauthorized));
        };

        if auth.is_hot_support || UserRole::from_db(&auth.user.role) != UserRole::Admin {
            return Err(AppError::Code(ErrorCode::AdminRequired));
        }

        Ok(Self(auth))
    }
}
