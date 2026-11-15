// =============================================================================
// ROUTES: WORKSPACES & COLLABORATOR RBAC MANAGEMENT
// =============================================================================

// `axum` - Path/State/Json extractors and RESTful router composition.
use axum::extract::{Path, State};
use axum::response::Response;
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};

use services::state::AppState;
use shared::domain::error::ToResponse;
use shared::domain::workspace::{
    AddWorkspaceMemberRequest, CreateWorkspaceRequest, SwitchWorkspaceRequest,
    UpdateMemberPermissionsRequest, UpdateWorkspaceRequest,
};

use crate::extractors::AuthUser;

/// Mounts `/api/v1/workspaces/*` endpoints.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/workspaces", get(list_workspaces).post(create_workspace))
        .route("/api/v1/workspaces/switch", post(switch_workspace))
        .route(
            "/api/v1/workspaces/{workspace_id}",
            get(get_workspace)
                .patch(update_workspace)
                .delete(delete_workspace),
        )
        .route(
            "/api/v1/workspaces/{workspace_id}/members",
            get(list_members).post(add_member),
        )
        .route(
            "/api/v1/workspaces/{workspace_id}/members/{member_user_id}",
            put(update_member_permissions).delete(remove_member),
        )
        .route(
            "/api/v1/workspaces/{workspace_id}/members/leave",
            delete(remove_self_stub),
        )
}

/// Lists all workspaces accessible to the signed-in user.
pub async fn list_workspaces(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
) -> Response {
    state
        .workspace_service
        .list_workspaces(auth.user.id)
        .await
        .to_response()
}

/// Creates a new workspace and switches active context to it.
pub async fn create_workspace(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
    Json(req): Json<CreateWorkspaceRequest>,
) -> Response {
    state
        .workspace_service
        .create_workspace(auth.user.id, req)
        .await
        .to_response_created()
}

/// Switches active workspace and returns freshly hydrated session bootstrap data.
pub async fn switch_workspace(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
    Json(req): Json<SwitchWorkspaceRequest>,
) -> Response {
    if let Err(err) = state
        .workspace_service
        .switch_active_workspace(auth.user.id, req)
        .await
    {
        return axum::response::IntoResponse::into_response(err);
    }

    state
        .user_service
        .get_session_bootstrap(
            auth.user.id,
            auth.is_hot_support,
            &state.permission_service,
        )
        .await
        .to_response()
}

/// Retrieves a single workspace summary if authorized (`Workspace:View`).
pub async fn get_workspace(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
    Path(workspace_id): Path<i32>,
) -> Response {
    state
        .workspace_service
        .get_workspace(auth.user.id, workspace_id)
        .await
        .to_response()
}

/// Updates workspace properties (`Workspace:Edit`), supporting `deserialize_double_option`.
pub async fn update_workspace(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
    Path(workspace_id): Path<i32>,
    Json(req): Json<UpdateWorkspaceRequest>,
) -> Response {
    state
        .workspace_service
        .update_workspace(auth.user.id, workspace_id, req)
        .await
        .to_response()
}

/// Deletes a workspace (`Workspace:Delete`), guarding against deleting the last workspace.
pub async fn delete_workspace(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
    Path(workspace_id): Path<i32>,
) -> Response {
    state
        .workspace_service
        .delete_workspace(auth.user.id, workspace_id)
        .await
        .map(|()| true)
        .to_response()
}

/// Lists all members and resolved permissions for `workspace_id`.
pub async fn list_members(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
    Path(workspace_id): Path<i32>,
) -> Response {
    state
        .workspace_service
        .list_members(auth.user.id, workspace_id)
        .await
        .to_response()
}

/// Invites an existing user by email with role preset and permissions (`Members:Create`).
pub async fn add_member(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
    Path(workspace_id): Path<i32>,
    Json(req): Json<AddWorkspaceMemberRequest>,
) -> Response {
    state
        .workspace_service
        .add_member(auth.user.id, workspace_id, req)
        .await
        .to_response_created()
}

/// Updates a collaborator's role preset and explicit RBAC permission matrix (`Members:Edit`).
pub async fn update_member_permissions(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
    Path((workspace_id, member_user_id)): Path<(i32, i32)>,
    Json(req): Json<UpdateMemberPermissionsRequest>,
) -> Response {
    state
        .workspace_service
        .update_member_permissions(auth.user.id, workspace_id, member_user_id, req)
        .await
        .to_response()
}

/// Removes a collaborator from the workspace (`Members:Delete`).
pub async fn remove_member(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
    Path((workspace_id, member_user_id)): Path<(i32, i32)>,
) -> Response {
    state
        .workspace_service
        .remove_member(auth.user.id, workspace_id, member_user_id)
        .await
        .map(|()| true)
        .to_response()
}

/// Convenience leave-workspace route delegating to `remove_member` for the caller.
pub async fn remove_self_stub(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
    Path(workspace_id): Path<i32>,
) -> Response {
    state
        .workspace_service
        .remove_member(auth.user.id, workspace_id, auth.user.id)
        .await
        .map(|()| true)
        .to_response()
}
