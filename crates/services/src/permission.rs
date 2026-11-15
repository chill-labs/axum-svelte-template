// =============================================================================
// WORKSPACE RBAC PERMISSION SERVICE
// =============================================================================
//
// Enforces multi-tenant workspace isolation and granular `PermResource × PermAction`
// authorization backed by a Moka in-memory cache keyed by `(user_id, workspace_id)`.
//
// ## Key Concepts
//
// - **Owner Bypass**: `workspaces.owner_id == user_id` resolves to
//   `PermissionCheckResult::Owner`, granting full access to every resource/action.
// - **Member Permissions**: Collaborators in `workspace_members` hold a JSONB
//   permission set; `Workspace:View` is implicitly granted to active members.
// - **Cache Invalidation**: Call `invalidate(user_id, workspace_id)` whenever
//   a workspace is deleted or membership/permissions change.
//
// ## Architecture Notes
//
// Stored in `AppState` and injected into `WorkspaceService`, `TodoService`, and
// `UserService` bootstrap resolution.

// SECTION: IMPORTS

// `moka` - Async concurrent TTL/TTI cache for resolved workspace access.
// - `Cache` - Maps `(user_id, workspace_id)` to `PermissionCheckResult`
use moka::future::Cache;

// `sea_orm` - Asynchronous database query traits for workspaces and memberships.
// - `ColumnTrait` - Column equality filters (`WorkspaceId`, `UserId`)
// - `DatabaseConnection` - PostgreSQL pool handle
// - `EntityTrait` - `find_by_id` and `find` entry points
// - `QueryFilter` - Applies `.filter(...)` predicates
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

// `std::time::Duration` - Idle cache eviction timeout.
use std::time::Duration;

// `shared` - Configuration, error codes, and RBAC permission primitives.
// - `CacheConfig` - Cache capacity and TTL settings
// - `AppError` - Typed error wrapper
// - `ErrorCode` - `WorkspaceNotFound`, `PermissionDenied`, `Forbidden`
// - `all_permissions`, `has_any_permission`, `has_permission`, `PermAction`,
//   `Permission`, `PermissionSet`, `PermResource` - RBAC evaluation helpers
use shared::config::settings::CacheConfig;
use shared::domain::error::AppError;
use shared::domain::error_code::ErrorCode;
use shared::domain::permission::{
    all_permissions, has_any_permission, has_permission, PermAction, PermResource, Permission,
    PermissionSet,
};

// `crate::entities` - SeaORM entity definitions and JSONB permission parser.
// - `parse_permissions_json` - Deserializes `workspace_members.permissions` safely
// - `workspace_members` - Workspace collaborator join table
// - `workspaces` - Workspace table owning `owner_id`
use crate::entities::conversions::parse_permissions_json;
use crate::entities::{workspace_members, workspaces};

// SECTION: PERMISSION CHECK RESULT

/// Cached authorization relationship between a user and a workspace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PermissionCheckResult {
    /// User owns the workspace and bypasses granular permission checks.
    Owner,
    /// User is a collaborator holding a specific permission set.
    Member(PermissionSet),
    /// User has no access to the workspace.
    NotMember,
}

// SECTION: PERMISSION SERVICE

/// Workspace RBAC enforcement and resolution service with Moka caching.
#[derive(Clone, Debug)]
pub struct PermissionService {
    /// Database pool used on cache miss.
    db: DatabaseConnection,
    /// Cached `(user_id, workspace_id) -> PermissionCheckResult`.
    cache: Cache<(i32, i32), PermissionCheckResult>,
}

impl PermissionService {
    /// Creates a new `PermissionService` configured from `CacheConfig`.
    ///
    /// # Arguments
    ///
    /// * `db` - PostgreSQL connection pool
    /// * `cache_config` - Cache capacity and TTL settings
    pub fn new(db: DatabaseConnection, cache_config: &CacheConfig) -> Self {
        let cache = Cache::builder()
            .max_capacity(cache_config.max_capacity.max(100))
            .time_to_idle(Duration::from_secs(cache_config.user_ttl_secs.max(1)))
            .build();

        Self { db, cache }
    }

    /// Resolves a user's role, effective permission list, and owner flag for a workspace.
    ///
    /// Returns `Ok(None)` if the workspace does not exist or the user is not a member.
    ///
    /// # Arguments
    ///
    /// * `user_id` - Target user ID
    /// * `workspace_id` - Target workspace ID
    pub async fn resolve_user_workspace_access(
        &self,
        user_id: i32,
        workspace_id: i32,
    ) -> Result<Option<(String, Vec<Permission>, bool)>, AppError> {
        let Some(ws) = workspaces::Entity::find_by_id(workspace_id)
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed loading workspace: {err}")))?
        else {
            return Ok(None);
        };

        if ws.owner_id == user_id {
            self.cache
                .insert((user_id, workspace_id), PermissionCheckResult::Owner)
                .await;
            return Ok(Some(("owner".to_string(), all_permissions(), true)));
        }

        let member = workspace_members::Entity::find()
            .filter(workspace_members::Column::WorkspaceId.eq(workspace_id))
            .filter(workspace_members::Column::UserId.eq(user_id))
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed loading workspace member: {err}")))?;

        match member {
            Some(row) => {
                let perms = parse_permissions_json(&row.permissions);
                let perm_set: PermissionSet = perms.iter().copied().collect();
                self.cache
                    .insert((user_id, workspace_id), PermissionCheckResult::Member(perm_set))
                    .await;
                Ok(Some((row.role, perms, false)))
            }
            None => {
                self.cache
                    .insert((user_id, workspace_id), PermissionCheckResult::NotMember)
                    .await;
                Ok(None)
            }
        }
    }

    /// Verifies that `user_id` holds `(resource, action)` in `workspace_id`.
    ///
    /// # Errors
    ///
    /// - `ErrorCode::WorkspaceNotFound` (`404`) when the workspace does not exist
    /// - `ErrorCode::Forbidden` (`403`) when the user is not a member of the workspace
    /// - `ErrorCode::PermissionDenied` (`403`) when a member lacks the requested capability
    pub async fn check(
        &self,
        user_id: i32,
        workspace_id: i32,
        resource: PermResource,
        action: PermAction,
    ) -> Result<(), AppError> {
        let access = self.get_or_load(user_id, workspace_id).await?;
        match access {
            PermissionCheckResult::Owner => Ok(()),
            PermissionCheckResult::Member(set) => {
                if (resource == PermResource::Workspace && action == PermAction::View)
                    || has_permission(&set, resource, action)
                {
                    Ok(())
                } else {
                    Err(AppError::Code(ErrorCode::PermissionDenied))
                }
            }
            PermissionCheckResult::NotMember => Err(AppError::Code(ErrorCode::Forbidden)),
        }
    }

    /// Verifies that `user_id` holds at least one of `checks` inside `workspace_id`.
    ///
    /// # Errors
    ///
    /// Returns `WorkspaceNotFound`, `Forbidden`, or `PermissionDenied` on failure.
    pub async fn check_any(
        &self,
        user_id: i32,
        workspace_id: i32,
        checks: &[(PermResource, PermAction)],
    ) -> Result<(), AppError> {
        let access = self.get_or_load(user_id, workspace_id).await?;
        match access {
            PermissionCheckResult::Owner => Ok(()),
            PermissionCheckResult::Member(set) => {
                let implicit_view = checks
                    .iter()
                    .any(|(r, a)| *r == PermResource::Workspace && *a == PermAction::View);
                if implicit_view || has_any_permission(&set, checks) {
                    Ok(())
                } else {
                    Err(AppError::Code(ErrorCode::PermissionDenied))
                }
            }
            PermissionCheckResult::NotMember => Err(AppError::Code(ErrorCode::Forbidden)),
        }
    }

    /// Evicts cached permissions for `(user_id, workspace_id)` after membership or role changes.
    pub async fn invalidate(&self, user_id: i32, workspace_id: i32) {
        self.cache.invalidate(&(user_id, workspace_id)).await;
    }

    /// Reads cached `(user_id, workspace_id)` state or loads it from PostgreSQL.
    async fn get_or_load(
        &self,
        user_id: i32,
        workspace_id: i32,
    ) -> Result<PermissionCheckResult, AppError> {
        let key = (user_id, workspace_id);
        if let Some(cached) = self.cache.get(&key).await {
            return Ok(cached);
        }

        let loaded = self.load_from_db(user_id, workspace_id).await?;
        self.cache.insert(key, loaded.clone()).await;
        Ok(loaded)
    }

    /// Queries PostgreSQL for workspace ownership or collaborator permission matrix.
    async fn load_from_db(
        &self,
        user_id: i32,
        workspace_id: i32,
    ) -> Result<PermissionCheckResult, AppError> {
        let workspace = workspaces::Entity::find_by_id(workspace_id)
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed querying workspace: {err}")))?
            .ok_or(AppError::Code(ErrorCode::WorkspaceNotFound))?;

        if workspace.owner_id == user_id {
            return Ok(PermissionCheckResult::Owner);
        }

        let member = workspace_members::Entity::find()
            .filter(workspace_members::Column::WorkspaceId.eq(workspace_id))
            .filter(workspace_members::Column::UserId.eq(user_id))
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed querying membership: {err}")))?;

        match member {
            Some(row) => {
                let perms: PermissionSet = parse_permissions_json(&row.permissions)
                    .into_iter()
                    .collect();
                Ok(PermissionCheckResult::Member(perms))
            }
            None => Ok(PermissionCheckResult::NotMember),
        }
    }
}
