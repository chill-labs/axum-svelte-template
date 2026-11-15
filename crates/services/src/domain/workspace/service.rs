// =============================================================================
// WORKSPACE & COLLABORATOR DOMAIN SERVICE
// =============================================================================
//
// Manages multi-tenant workspaces, active workspace switching, collaborator
// membership, and granular `PermResource × PermAction` role assignments.
//
// ## Key Concepts
//
// - **Last-Workspace Guard**: Prevents deleting the owner's last workspace so
//   every user always retains an active working context (`CannotDeleteLastWorkspace`).
// - **Owner Protection**: Workspace owners bypass permission checks and cannot
//   be removed or stripped of ownership (`CannotRemoveWorkspaceOwner`).
// - **Cache Coherency**: Workspace/member mutations evict `PermissionService`,
//   `CacheService.user_workspaces`, and `CacheService.users` for affected members.
//
// ## Architecture Notes
//
// Enforces RBAC via `PermissionService` before reading or mutating any workspace.

// SECTION: IMPORTS

// `chrono` - UTC timestamps for updates.
use chrono::Utc;

// `sea_orm` - Asynchronous PostgreSQL transactions and batch aggregate queries.
// - `ActiveModelTrait` - Inserts, updates, deletes ActiveModels
// - `ColumnTrait` - Filter columns
// - `Condition` - Combines `OR` ownership/membership predicates
// - `DatabaseConnection` - PostgreSQL pool handle
// - `EntityTrait`, `PaginatorTrait` - Queries & count helpers
// - `QueryFilter`, `QueryOrder`, `TransactionTrait` - Filters, sorting, atomic tx
// - `Set` - Sets ActiveModel fields
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, Set, TransactionTrait,
};

// `std::collections::{HashMap, HashSet}` - Fast batch aggregation maps.
use std::collections::{HashMap, HashSet};

// `shared` - Workspace DTOs, RBAC primitives, validators, and error codes.
use shared::domain::error::AppError;
use shared::domain::error_code::ErrorCode;
use shared::domain::permission::{all_permissions, PermAction, PermResource, Permission};
use shared::domain::request::Sanitize;
use shared::domain::upload::UploadPurpose;
use shared::domain::validator::slugify;
use shared::domain::workspace::{
    AddWorkspaceMemberRequest, CreateWorkspaceRequest, MemberRolePreset, SwitchWorkspaceRequest,
    UpdateMemberPermissionsRequest, UpdateWorkspaceRequest, WorkspaceMemberResponse,
    WorkspaceResponse,
};

// `crate` - Caches, entities, media lookups, notifications, and RBAC service.
use crate::cache::CacheService;
use crate::domain::media::service::{MediaService, DEFAULT_MEDIA_FIELD};
use crate::domain::notification::service::NotificationService;
use crate::entities::conversions::parse_permissions_json;
use crate::entities::{todos, users, workspace_members, workspaces};
use crate::permission::PermissionService;
use crate::utils::token::generate_raw_token;

// SECTION: WORKSPACE SERVICE

/// Service implementing workspace CRUD, switching, and collaborator RBAC management.
#[derive(Clone)]
pub struct WorkspaceService {
    /// PostgreSQL connection pool.
    db: DatabaseConnection,
    /// Moka cache container.
    cache: CacheService,
    /// Workspace RBAC enforcement and resolution service.
    permission_service: PermissionService,
    /// Polymorphic encrypted media service for workspace logos & member avatars.
    media_service: MediaService,
    /// Real-time notification service for member invitation alerts.
    notification_service: NotificationService,
}

impl WorkspaceService {
    /// Constructs a new `WorkspaceService`.
    pub fn new(
        db: DatabaseConnection,
        cache: CacheService,
        permission_service: PermissionService,
        media_service: MediaService,
        notification_service: NotificationService,
    ) -> Self {
        Self {
            db,
            cache,
            permission_service,
            media_service,
            notification_service,
        }
    }

    // SECTION: WORKSPACE QUERIES & MUTATIONS

    /// Lists all workspaces owned by or shared with `user_id` (Moka cached).
    pub async fn list_workspaces(&self, user_id: i32) -> Result<Vec<WorkspaceResponse>, AppError> {
        self.cache
            .user_workspaces
            .get_or_load(user_id, || async move { self.load_user_workspaces_from_db(user_id).await })
            .await
    }

    /// Loads a single workspace summary after verifying `Workspace:View` permission.
    pub async fn get_workspace(
        &self,
        user_id: i32,
        workspace_id: i32,
    ) -> Result<WorkspaceResponse, AppError> {
        self.permission_service
            .check(user_id, workspace_id, PermResource::Workspace, PermAction::View)
            .await?;

        let ws = workspaces::Entity::find_by_id(workspace_id)
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed querying workspace: {err}")))?
            .ok_or(AppError::Code(ErrorCode::WorkspaceNotFound))?;

        self.build_workspace_response(user_id, ws).await
    }

    /// Creates a new workspace owned by `user_id`, registers owner membership,
    /// and evicts `user_workspaces` cache.
    pub async fn create_workspace(
        &self,
        user_id: i32,
        mut req: CreateWorkspaceRequest,
    ) -> Result<WorkspaceResponse, AppError> {
        req.sanitize();
        req.validate_input()?;

        let slug = match req.slug {
            Some(explicit) => {
                let exists = workspaces::Entity::find()
                    .filter(workspaces::Column::Slug.eq(&explicit))
                    .one(&self.db)
                    .await
                    .map_err(|err| {
                        AppError::Internal(format!("Failed checking workspace slug: {err}"))
                    })?
                    .is_some();
                if exists {
                    return Err(AppError::Code(ErrorCode::WorkspaceSlugTaken));
                }
                explicit
            }
            None => self.generate_unique_slug(&req.name).await?,
        };

        let color = req.color.unwrap_or_else(|| "#3B82F6".to_string());

        let txn = self
            .db
            .begin()
            .await
            .map_err(|err| AppError::Internal(format!("Failed starting workspace txn: {err}")))?;

        let created = workspaces::ActiveModel {
            owner_id: Set(user_id),
            name: Set(req.name),
            slug: Set(slug),
            description: Set(req.description),
            color: Set(color),
            ..Default::default()
        }
        .insert(&txn)
        .await
        .map_err(|err| AppError::Internal(format!("Failed creating workspace: {err}")))?;

        workspace_members::ActiveModel {
            workspace_id: Set(created.id),
            user_id: Set(user_id),
            role: Set("owner".to_string()),
            permissions: Set(serde_json::json!([])),
            ..Default::default()
        }
        .insert(&txn)
        .await
        .map_err(|err| AppError::Internal(format!("Failed inserting owner membership: {err}")))?;

        txn.commit()
            .await
            .map_err(|err| AppError::Internal(format!("Failed committing workspace txn: {err}")))?;

        self.permission_service.invalidate(user_id, created.id).await;
        self.cache.invalidate_workspace_list(user_id).await;

        self.build_workspace_response(user_id, created).await
    }

    /// Updates workspace metadata using `deserialize_double_option` for `description`.
    pub async fn update_workspace(
        &self,
        user_id: i32,
        workspace_id: i32,
        mut req: UpdateWorkspaceRequest,
    ) -> Result<WorkspaceResponse, AppError> {
        req.sanitize();
        req.validate_input()?;

        self.permission_service
            .check(user_id, workspace_id, PermResource::Workspace, PermAction::Edit)
            .await?;

        let ws = workspaces::Entity::find_by_id(workspace_id)
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed querying workspace: {err}")))?
            .ok_or(AppError::Code(ErrorCode::WorkspaceNotFound))?;

        if let Some(ref new_slug) = req.slug {
            if new_slug != &ws.slug {
                let conflict = workspaces::Entity::find()
                    .filter(workspaces::Column::Slug.eq(new_slug))
                    .filter(workspaces::Column::Id.ne(workspace_id))
                    .one(&self.db)
                    .await
                    .map_err(|err| {
                        AppError::Internal(format!("Failed checking workspace slug conflict: {err}"))
                    })?
                    .is_some();
                if conflict {
                    return Err(AppError::Code(ErrorCode::WorkspaceSlugTaken));
                }
            }
        }

        let mut active: workspaces::ActiveModel = ws.into();
        if let Some(name) = req.name {
            active.name = Set(name);
        }
        if let Some(slug) = req.slug {
            active.slug = Set(slug);
        }
        if let Some(desc_opt) = req.description {
            active.description = Set(desc_opt);
        }
        if let Some(color) = req.color {
            active.color = Set(color);
        }
        active.updated_at = Set(Utc::now().into());

        let updated = active
            .update(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed updating workspace: {err}")))?;

        let affected_users = self.collect_workspace_user_ids(workspace_id, updated.owner_id).await?;
        for uid in affected_users {
            self.cache.invalidate_workspace_list(uid).await;
        }

        self.build_workspace_response(user_id, updated).await
    }

    /// Deletes a workspace unless it is the owner's last owned workspace.
    pub async fn delete_workspace(
        &self,
        user_id: i32,
        workspace_id: i32,
    ) -> Result<(), AppError> {
        self.permission_service
            .check(
                user_id,
                workspace_id,
                PermResource::Workspace,
                PermAction::Delete,
            )
            .await?;

        let ws = workspaces::Entity::find_by_id(workspace_id)
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed querying workspace: {err}")))?
            .ok_or(AppError::Code(ErrorCode::WorkspaceNotFound))?;

        let owned_count = workspaces::Entity::find()
            .filter(workspaces::Column::OwnerId.eq(ws.owner_id))
            .count(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed counting owned workspaces: {err}")))?;

        if owned_count <= 1 {
            return Err(AppError::Code(ErrorCode::CannotDeleteLastWorkspace));
        }

        let affected_users = self.collect_workspace_user_ids(workspace_id, ws.owner_id).await?;

        workspaces::Entity::delete_by_id(workspace_id)
            .exec(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed deleting workspace: {err}")))?;

        self.media_service
            .delete_media(
                UploadPurpose::WorkspaceLogo.entity_type(),
                workspace_id,
                DEFAULT_MEDIA_FIELD,
            )
            .await?;

        self.cache.invalidate_todos(workspace_id).await;
        for uid in affected_users {
            self.permission_service.invalidate(uid, workspace_id).await;
            self.cache.invalidate_workspace_list(uid).await;
            self.cache.invalidate_user(uid).await;
        }

        Ok(())
    }

    /// Switches `users.active_workspace_id` for `user_id` after verifying membership.
    pub async fn switch_active_workspace(
        &self,
        user_id: i32,
        req: SwitchWorkspaceRequest,
    ) -> Result<WorkspaceResponse, AppError> {
        self.permission_service
            .check(
                user_id,
                req.workspace_id,
                PermResource::Workspace,
                PermAction::View,
            )
            .await?;

        let user = users::Entity::find_by_id(user_id)
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed loading user for switch: {err}")))?
            .ok_or(AppError::Code(ErrorCode::UserNotFound))?;

        let mut active: users::ActiveModel = user.into();
        active.active_workspace_id = Set(Some(req.workspace_id));
        active.updated_at = Set(Utc::now().into());
        let updated = active
            .update(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed switching workspace: {err}")))?;

        self.cache.users.insert(updated.id, updated).await;
        self.get_workspace(user_id, req.workspace_id).await
    }

    // SECTION: COLLABORATOR MEMBERSHIP & PERMISSIONS

    /// Lists all collaborators (and owner) in `workspace_id`.
    pub async fn list_members(
        &self,
        user_id: i32,
        workspace_id: i32,
    ) -> Result<Vec<WorkspaceMemberResponse>, AppError> {
        self.permission_service
            .check(user_id, workspace_id, PermResource::Members, PermAction::View)
            .await?;

        let ws = workspaces::Entity::find_by_id(workspace_id)
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed querying workspace: {err}")))?
            .ok_or(AppError::Code(ErrorCode::WorkspaceNotFound))?;

        let rows = workspace_members::Entity::find()
            .filter(workspace_members::Column::WorkspaceId.eq(workspace_id))
            .order_by_asc(workspace_members::Column::JoinedAt)
            .order_by_asc(workspace_members::Column::Id)
            .all(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed loading workspace members: {err}")))?;

        let mut user_ids: Vec<i32> = rows.iter().map(|r| r.user_id).collect();
        if !user_ids.contains(&ws.owner_id) {
            user_ids.push(ws.owner_id);
        }

        let users_map: HashMap<i32, users::Model> = users::Entity::find()
            .filter(users::Column::Id.is_in(user_ids.clone()))
            .all(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed loading member users: {err}")))?
            .into_iter()
            .map(|u| (u.id, u))
            .collect();

        let avatars = self
            .media_service
            .get_media_batch(
                UploadPurpose::UserAvatar.entity_type(),
                &user_ids,
                DEFAULT_MEDIA_FIELD,
            )
            .await?;

        let mut result = Vec::with_capacity(rows.len() + 1);
        let mut owner_included = false;

        for row in rows {
            let Some(member_user) = users_map.get(&row.user_id) else {
                continue;
            };
            let is_owner = row.user_id == ws.owner_id;
            if is_owner {
                owner_included = true;
            }
            let permissions = if is_owner {
                all_permissions()
            } else {
                parse_permissions_json(&row.permissions)
            };

            result.push(WorkspaceMemberResponse {
                id: row.id,
                workspace_id,
                user_id: row.user_id,
                user_name: member_user.name.clone(),
                user_email: member_user.email.clone(),
                user_avatar_url: member_user.avatar_url.clone(),
                user_avatar_key: avatars.get(&row.user_id).map(|m| m.xor_key.clone()),
                role: if is_owner {
                    "owner".to_string()
                } else {
                    row.role
                },
                permissions,
                is_owner,
                joined_at: row.joined_at.with_timezone(&Utc),
            });
        }

        if !owner_included {
            if let Some(owner_user) = users_map.get(&ws.owner_id) {
                result.insert(
                    0,
                    WorkspaceMemberResponse {
                        id: 0,
                        workspace_id,
                        user_id: owner_user.id,
                        user_name: owner_user.name.clone(),
                        user_email: owner_user.email.clone(),
                        user_avatar_url: owner_user.avatar_url.clone(),
                        user_avatar_key: avatars.get(&owner_user.id).map(|m| m.xor_key.clone()),
                        role: "owner".to_string(),
                        permissions: all_permissions(),
                        is_owner: true,
                        joined_at: ws.created_at.with_timezone(&Utc),
                    },
                );
            }
        }

        Ok(result)
    }

    /// Invites an existing user by email into `workspace_id` with preset or custom permissions.
    pub async fn add_member(
        &self,
        actor_user_id: i32,
        workspace_id: i32,
        mut req: AddWorkspaceMemberRequest,
    ) -> Result<WorkspaceMemberResponse, AppError> {
        req.sanitize();
        req.validate_input()?;

        self.permission_service
            .check(
                actor_user_id,
                workspace_id,
                PermResource::Members,
                PermAction::Create,
            )
            .await?;

        let ws = workspaces::Entity::find_by_id(workspace_id)
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed querying workspace: {err}")))?
            .ok_or(AppError::Code(ErrorCode::WorkspaceNotFound))?;

        let target_user = users::Entity::find()
            .filter(users::Column::Email.eq(&req.email))
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed querying user by email: {err}")))?
            .ok_or(AppError::Code(ErrorCode::UserNotFound))?;

        if target_user.id == ws.owner_id {
            return Err(AppError::Code(ErrorCode::MemberAlreadyExists));
        }

        let existing = workspace_members::Entity::find()
            .filter(workspace_members::Column::WorkspaceId.eq(workspace_id))
            .filter(workspace_members::Column::UserId.eq(target_user.id))
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed checking existing member: {err}")))?;

        if existing.is_some() {
            return Err(AppError::Code(ErrorCode::MemberAlreadyExists));
        }

        let permissions: Vec<Permission> = match req.role {
            MemberRolePreset::Custom => req.permissions.unwrap_or_default(),
            preset => req
                .permissions
                .filter(|p| !p.is_empty())
                .or_else(|| preset.default_permissions())
                .unwrap_or_default(),
        };

        let perms_json = serde_json::to_value(&permissions).map_err(|err| {
            AppError::Internal(format!("Failed serializing permissions JSON: {err}"))
        })?;

        let inserted = workspace_members::ActiveModel {
            workspace_id: Set(workspace_id),
            user_id: Set(target_user.id),
            role: Set(req.role.as_str().to_string()),
            permissions: Set(perms_json),
            ..Default::default()
        }
        .insert(&self.db)
        .await
        .map_err(|err| AppError::Internal(format!("Failed inserting workspace member: {err}")))?;

        self.permission_service
            .invalidate(target_user.id, workspace_id)
            .await;
        self.cache.invalidate_workspace_list(target_user.id).await;
        self.cache.invalidate_workspace_list(actor_user_id).await;

        let _ = self
            .notification_service
            .create_notification(
                target_user.id,
                Some(workspace_id),
                "workspace_invite",
                &format!("Added to workspace {}", ws.name),
                &format!("You were granted {} access in {}.", req.role.as_str(), ws.name),
                Some("/workspaces".to_string()),
            )
            .await;

        let avatar = self
            .media_service
            .get_media(
                UploadPurpose::UserAvatar.entity_type(),
                target_user.id,
                DEFAULT_MEDIA_FIELD,
            )
            .await?;

        Ok(WorkspaceMemberResponse {
            id: inserted.id,
            workspace_id,
            user_id: target_user.id,
            user_name: target_user.name,
            user_email: target_user.email,
            user_avatar_url: target_user.avatar_url,
            user_avatar_key: avatar.map(|m| m.xor_key),
            role: inserted.role,
            permissions,
            is_owner: false,
            joined_at: inserted.joined_at.with_timezone(&Utc),
        })
    }

    /// Updates a collaborator's role preset and JSONB permission matrix.
    pub async fn update_member_permissions(
        &self,
        actor_user_id: i32,
        workspace_id: i32,
        target_user_id: i32,
        req: UpdateMemberPermissionsRequest,
    ) -> Result<WorkspaceMemberResponse, AppError> {
        self.permission_service
            .check(
                actor_user_id,
                workspace_id,
                PermResource::Members,
                PermAction::Edit,
            )
            .await?;

        let ws = workspaces::Entity::find_by_id(workspace_id)
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed querying workspace: {err}")))?
            .ok_or(AppError::Code(ErrorCode::WorkspaceNotFound))?;

        if target_user_id == ws.owner_id {
            return Err(AppError::Code(ErrorCode::CannotRemoveWorkspaceOwner));
        }

        let member = workspace_members::Entity::find()
            .filter(workspace_members::Column::WorkspaceId.eq(workspace_id))
            .filter(workspace_members::Column::UserId.eq(target_user_id))
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed querying member: {err}")))?
            .ok_or(AppError::Code(ErrorCode::MemberNotFound))?;

        let effective_permissions = if req.permissions.is_empty() && req.role != MemberRolePreset::Custom {
            req.role.default_permissions().unwrap_or_default()
        } else {
            req.permissions
        };

        let perms_json = serde_json::to_value(&effective_permissions).map_err(|err| {
            AppError::Internal(format!("Failed serializing permissions JSON: {err}"))
        })?;

        let mut active: workspace_members::ActiveModel = member.into();
        active.role = Set(req.role.as_str().to_string());
        active.permissions = Set(perms_json);
        let updated = active
            .update(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed updating member permissions: {err}")))?;

        self.permission_service
            .invalidate(target_user_id, workspace_id)
            .await;
        self.cache.invalidate_workspace_list(target_user_id).await;

        let target_user = users::Entity::find_by_id(target_user_id)
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed loading member user: {err}")))?
            .ok_or(AppError::Code(ErrorCode::UserNotFound))?;

        let avatar = self
            .media_service
            .get_media(
                UploadPurpose::UserAvatar.entity_type(),
                target_user.id,
                DEFAULT_MEDIA_FIELD,
            )
            .await?;

        Ok(WorkspaceMemberResponse {
            id: updated.id,
            workspace_id,
            user_id: target_user.id,
            user_name: target_user.name,
            user_email: target_user.email,
            user_avatar_url: target_user.avatar_url,
            user_avatar_key: avatar.map(|m| m.xor_key),
            role: updated.role,
            permissions: effective_permissions,
            is_owner: false,
            joined_at: updated.joined_at.with_timezone(&Utc),
        })
    }

    /// Removes a collaborator from `workspace_id` (owners cannot be removed).
    pub async fn remove_member(
        &self,
        actor_user_id: i32,
        workspace_id: i32,
        target_user_id: i32,
    ) -> Result<(), AppError> {
        self.permission_service
            .check(
                actor_user_id,
                workspace_id,
                PermResource::Members,
                PermAction::Delete,
            )
            .await?;

        let ws = workspaces::Entity::find_by_id(workspace_id)
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed querying workspace: {err}")))?
            .ok_or(AppError::Code(ErrorCode::WorkspaceNotFound))?;

        if target_user_id == ws.owner_id {
            return Err(AppError::Code(ErrorCode::CannotRemoveWorkspaceOwner));
        }

        let member = workspace_members::Entity::find()
            .filter(workspace_members::Column::WorkspaceId.eq(workspace_id))
            .filter(workspace_members::Column::UserId.eq(target_user_id))
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed querying member: {err}")))?
            .ok_or(AppError::Code(ErrorCode::MemberNotFound))?;

        workspace_members::Entity::delete_by_id(member.id)
            .exec(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed removing member: {err}")))?;

        // If removed user had this workspace selected, revert to their owned workspace.
        if let Some(target_user) = users::Entity::find_by_id(target_user_id)
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed loading removed user: {err}")))?
        {
            if target_user.active_workspace_id == Some(workspace_id) {
                let fallback_id = workspaces::Entity::find()
                    .filter(workspaces::Column::OwnerId.eq(target_user_id))
                    .order_by_asc(workspaces::Column::Id)
                    .one(&self.db)
                    .await
                    .map_err(|err| {
                        AppError::Internal(format!("Failed finding fallback workspace: {err}"))
                    })?
                    .map(|w| w.id);

                let mut active: users::ActiveModel = target_user.into();
                active.active_workspace_id = Set(fallback_id);
                active.updated_at = Set(Utc::now().into());
                let _ = active.update(&self.db).await;
                self.cache.invalidate_user(target_user_id).await;
            }
        }

        self.permission_service
            .invalidate(target_user_id, workspace_id)
            .await;
        self.cache.invalidate_workspace_list(target_user_id).await;
        self.cache.invalidate_workspace_list(actor_user_id).await;

        Ok(())
    }

    // SECTION: INTERNAL HELPERS

    /// Loads and enriches all workspaces accessible to `user_id`.
    async fn load_user_workspaces_from_db(
        &self,
        user_id: i32,
    ) -> Result<Vec<WorkspaceResponse>, AppError> {
        let memberships = workspace_members::Entity::find()
            .filter(workspace_members::Column::UserId.eq(user_id))
            .all(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed loading memberships: {err}")))?;

        let member_ws_ids: Vec<i32> = memberships.iter().map(|m| m.workspace_id).collect();
        let membership_by_ws: HashMap<i32, workspace_members::Model> = memberships
            .into_iter()
            .map(|m| (m.workspace_id, m))
            .collect();

        let condition = if member_ws_ids.is_empty() {
            Condition::any().add(workspaces::Column::OwnerId.eq(user_id))
        } else {
            Condition::any()
                .add(workspaces::Column::OwnerId.eq(user_id))
                .add(workspaces::Column::Id.is_in(member_ws_ids))
        };

        let ws_rows = workspaces::Entity::find()
            .filter(condition)
            .order_by_asc(workspaces::Column::CreatedAt)
            .order_by_asc(workspaces::Column::Id)
            .all(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed loading user workspaces: {err}")))?;

        if ws_rows.is_empty() {
            return Ok(Vec::new());
        }

        let ws_ids: Vec<i32> = ws_rows.iter().map(|w| w.id).collect();

        let all_members = workspace_members::Entity::find()
            .filter(workspace_members::Column::WorkspaceId.is_in(ws_ids.clone()))
            .all(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed counting workspace members: {err}")))?;

        let mut member_sets: HashMap<i32, HashSet<i32>> = HashMap::new();
        for w in &ws_rows {
            member_sets.entry(w.id).or_default().insert(w.owner_id);
        }
        for m in all_members {
            member_sets.entry(m.workspace_id).or_default().insert(m.user_id);
        }

        let all_todos = todos::Entity::find()
            .filter(todos::Column::WorkspaceId.is_in(ws_ids.clone()))
            .all(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed counting workspace todos: {err}")))?;

        let mut todo_totals: HashMap<i32, (i64, i64)> = HashMap::new();
        for t in all_todos {
            let entry = todo_totals.entry(t.workspace_id).or_insert((0, 0));
            entry.0 += 1;
            if t.is_completed {
                entry.1 += 1;
            }
        }

        let logos = self
            .media_service
            .get_media_batch(
                UploadPurpose::WorkspaceLogo.entity_type(),
                &ws_ids,
                DEFAULT_MEDIA_FIELD,
            )
            .await?;

        let mut out = Vec::with_capacity(ws_rows.len());
        for ws in ws_rows {
            let is_owner = ws.owner_id == user_id;
            let (my_role, my_permissions) = if is_owner {
                ("owner".to_string(), all_permissions())
            } else if let Some(m) = membership_by_ws.get(&ws.id) {
                (m.role.clone(), parse_permissions_json(&m.permissions))
            } else {
                ("member".to_string(), Vec::new())
            };

            let member_count = member_sets.get(&ws.id).map_or(1, |s| s.len() as i64);
            let (todo_count, completed_todo_count) = todo_totals.get(&ws.id).copied().unwrap_or((0, 0));

            out.push(WorkspaceResponse {
                id: ws.id,
                owner_id: ws.owner_id,
                name: ws.name,
                slug: ws.slug,
                description: ws.description,
                color: ws.color,
                logo_key: logos.get(&ws.id).map(|m| m.xor_key.clone()),
                is_owner,
                my_role,
                my_permissions,
                member_count,
                todo_count,
                completed_todo_count,
                created_at: ws.created_at.with_timezone(&Utc),
                updated_at: ws.updated_at.with_timezone(&Utc),
            });
        }

        Ok(out)
    }

    /// Hydrates a single `WorkspaceResponse` for `(user_id, ws)`.
    async fn build_workspace_response(
        &self,
        user_id: i32,
        ws: workspaces::Model,
    ) -> Result<WorkspaceResponse, AppError> {
        let is_owner = ws.owner_id == user_id;
        let (my_role, my_permissions) = if is_owner {
            ("owner".to_string(), all_permissions())
        } else {
            let member = workspace_members::Entity::find()
                .filter(workspace_members::Column::WorkspaceId.eq(ws.id))
                .filter(workspace_members::Column::UserId.eq(user_id))
                .one(&self.db)
                .await
                .map_err(|err| AppError::Internal(format!("Failed querying member role: {err}")))?;
            match member {
                Some(m) => (m.role, parse_permissions_json(&m.permissions)),
                None => ("viewer".to_string(), Vec::new()),
            }
        };

        let members = workspace_members::Entity::find()
            .filter(workspace_members::Column::WorkspaceId.eq(ws.id))
            .all(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed counting members: {err}")))?;
        let mut uids: HashSet<i32> = members.into_iter().map(|m| m.user_id).collect();
        uids.insert(ws.owner_id);

        let todo_count = todos::Entity::find()
            .filter(todos::Column::WorkspaceId.eq(ws.id))
            .count(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed counting todos: {err}")))?
            as i64;

        let completed_todo_count = todos::Entity::find()
            .filter(todos::Column::WorkspaceId.eq(ws.id))
            .filter(todos::Column::IsCompleted.eq(true))
            .count(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed counting completed todos: {err}")))?
            as i64;

        let logo = self
            .media_service
            .get_media(
                UploadPurpose::WorkspaceLogo.entity_type(),
                ws.id,
                DEFAULT_MEDIA_FIELD,
            )
            .await?;

        Ok(WorkspaceResponse {
            id: ws.id,
            owner_id: ws.owner_id,
            name: ws.name,
            slug: ws.slug,
            description: ws.description,
            color: ws.color,
            logo_key: logo.map(|m| m.xor_key),
            is_owner,
            my_role,
            my_permissions,
            member_count: uids.len() as i64,
            todo_count,
            completed_todo_count,
            created_at: ws.created_at.with_timezone(&Utc),
            updated_at: ws.updated_at.with_timezone(&Utc),
        })
    }

    /// Generates a collision-free kebab-case workspace slug from `name`.
    async fn generate_unique_slug(&self, name: &str) -> Result<String, AppError> {
        let base = slugify(name);
        let base_exists = workspaces::Entity::find()
            .filter(workspaces::Column::Slug.eq(&base))
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed checking base slug: {err}")))?
            .is_some();

        if !base_exists {
            return Ok(base);
        }

        let suffix = &generate_raw_token()[..6];
        let trimmed_base: String = base.chars().take(120).collect();
        Ok(format!("{}-{}", trimmed_base.trim_end_matches('-'), suffix))
    }

    /// Collects all distinct user IDs (owner + members) belonging to `workspace_id`.
    pub async fn collect_workspace_user_ids(
        &self,
        workspace_id: i32,
        owner_id: i32,
    ) -> Result<Vec<i32>, AppError> {
        let members = workspace_members::Entity::find()
            .filter(workspace_members::Column::WorkspaceId.eq(workspace_id))
            .all(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed loading workspace user IDs: {err}")))?;

        let mut set: HashSet<i32> = members.into_iter().map(|m| m.user_id).collect();
        set.insert(owner_id);
        Ok(set.into_iter().collect())
    }
}
