// =============================================================================
// INTEGRATION TESTS: WORKSPACE MULTI-TENANT CRUD & GRANULAR RBAC
// =============================================================================
//
// Verifies workspace creation, slug collision detection, double-option updates,
// last-workspace delete protection, active workspace switching, and multi-tenant
// collaborator RBAC matrix enforcement with Moka permission cache invalidation.

mod common;

// SECTION: IMPORTS

use common::TestApp;
use shared::domain::error_code::ErrorCode;
use shared::domain::permission::{PermAction, PermResource, Permission};
use shared::domain::workspace::{
    AddWorkspaceMemberRequest, CreateWorkspaceRequest, MemberRolePreset, SwitchWorkspaceRequest,
    UpdateMemberPermissionsRequest, UpdateWorkspaceRequest,
};

// SECTION: WORKSPACE CRUD & LAST-WORKSPACE GUARD

#[tokio::test]
async fn workspace_crud_slug_uniqueness_and_last_workspace_guard() {
    let app = TestApp::new().await;
    let owner = app.create_user("owner@example.com", "Owner").await;

    let initial_workspaces = app
        .state
        .workspace_service
        .list_workspaces(owner.id)
        .await
        .expect("list initial workspaces");
    assert_eq!(initial_workspaces.len(), 1);
    let personal_id = initial_workspaces[0].id;

    // Cannot delete the owner's last workspace.
    let err = app
        .state
        .workspace_service
        .delete_workspace(owner.id, personal_id)
        .await
        .unwrap_err();
    assert_eq!(err.code(), ErrorCode::CannotDeleteLastWorkspace);

    // Create a second workspace with explicit slug and color.
    let created = app
        .state
        .workspace_service
        .create_workspace(
            owner.id,
            CreateWorkspaceRequest {
                name: "  Product Roadmap  ".to_string(),
                slug: Some("product-roadmap".to_string()),
                description: Some("Q4 planning".to_string()),
                color: Some("#10B981".to_string()),
            },
        )
        .await
        .expect("create workspace");
    assert_eq!(created.name, "Product Roadmap");
    assert_eq!(created.slug, "product-roadmap");
    assert_eq!(created.description.as_deref(), Some("Q4 planning"));

    // Duplicate slug rejected with WorkspaceSlugTaken.
    let dup_err = app
        .state
        .workspace_service
        .create_workspace(
            owner.id,
            CreateWorkspaceRequest {
                name: "Another".to_string(),
                slug: Some("product-roadmap".to_string()),
                description: None,
                color: None,
            },
        )
        .await
        .unwrap_err();
    assert_eq!(dup_err.code(), ErrorCode::WorkspaceSlugTaken);

    // Double-option clears description explicitly when `Some(None)`.
    let updated = app
        .state
        .workspace_service
        .update_workspace(
            owner.id,
            created.id,
            UpdateWorkspaceRequest {
                name: Some("Roadmap 2026".to_string()),
                slug: None,
                description: Some(None),
                color: None,
            },
        )
        .await
        .expect("update workspace");
    assert_eq!(updated.name, "Roadmap 2026");
    assert_eq!(updated.description, None);

    // Switch active workspace and verify persisted active_workspace_id.
    let switched = app
        .state
        .workspace_service
        .switch_active_workspace(
            owner.id,
            SwitchWorkspaceRequest {
                workspace_id: created.id,
            },
        )
        .await
        .expect("switch active workspace");
    assert_eq!(switched.id, created.id);
    let reloaded_owner = app
        .state
        .user_service
        .get_user_by_id(owner.id)
        .await
        .expect("reload owner");
    assert_eq!(reloaded_owner.active_workspace_id, Some(created.id));

    // Deleting the second workspace now succeeds.
    app.state
        .workspace_service
        .delete_workspace(owner.id, created.id)
        .await
        .expect("delete second workspace");
    let remaining = app
        .state
        .workspace_service
        .list_workspaces(owner.id)
        .await
        .expect("list remaining workspaces");
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].id, personal_id);
}

// SECTION: MULTI-TENANT RBAC & COLLABORATOR PERMISSIONS

#[tokio::test]
async fn collaborator_rbac_isolation_and_permission_cache_invalidation() {
    let app = TestApp::new().await;
    let owner = app.create_user("boss@example.com", "Boss").await;
    let collab = app.create_user("collab@example.com", "Collaborator").await;
    let outsider = app.create_user("outsider@example.com", "Outsider").await;

    let ws = app
        .state
        .workspace_service
        .create_workspace(
            owner.id,
            CreateWorkspaceRequest {
                name: "Shared Engineering".to_string(),
                slug: None,
                description: None,
                color: None,
            },
        )
        .await
        .expect("create shared workspace");

    // Outsider cannot view workspace or check permissions (`Forbidden`).
    let outsider_err = app
        .state
        .permission_service
        .check(outsider.id, ws.id, PermResource::Todos, PermAction::View)
        .await
        .unwrap_err();
    assert_eq!(outsider_err.code(), ErrorCode::Forbidden);

    // Invite collaborator as `Viewer` -> can view Todos, denied `Todos:Create`.
    let added = app
        .state
        .workspace_service
        .add_member(
            owner.id,
            ws.id,
            AddWorkspaceMemberRequest {
                email: "collab@example.com".to_string(),
                role: MemberRolePreset::Viewer,
                permissions: None,
            },
        )
        .await
        .expect("add viewer member");
    assert_eq!(added.role, "viewer");

    app.state
        .permission_service
        .check(collab.id, ws.id, PermResource::Todos, PermAction::View)
        .await
        .expect("viewer can view todos");
    let create_denied = app
        .state
        .permission_service
        .check(collab.id, ws.id, PermResource::Todos, PermAction::Create)
        .await
        .unwrap_err();
    assert_eq!(create_denied.code(), ErrorCode::PermissionDenied);

    // Upgrade collaborator to custom `Todos:Create` + `Todos:Edit` -> cache invalidated immediately.
    let updated_member = app
        .state
        .workspace_service
        .update_member_permissions(
            owner.id,
            ws.id,
            collab.id,
            UpdateMemberPermissionsRequest {
                role: MemberRolePreset::Custom,
                permissions: vec![
                    Permission::new(PermResource::Todos, PermAction::View),
                    Permission::new(PermResource::Todos, PermAction::Create),
                    Permission::new(PermResource::Todos, PermAction::Edit),
                ],
            },
        )
        .await
        .expect("update custom permissions");
    assert_eq!(updated_member.role, "custom");

    app.state
        .permission_service
        .check(collab.id, ws.id, PermResource::Todos, PermAction::Create)
        .await
        .expect("updated permissions take effect immediately");
    assert_eq!(
        app.state
            .permission_service
            .check(collab.id, ws.id, PermResource::Todos, PermAction::Delete)
            .await
            .unwrap_err()
            .code(),
        ErrorCode::PermissionDenied
    );

    // Owner cannot be removed (`CannotRemoveWorkspaceOwner`).
    let remove_owner_err = app
        .state
        .workspace_service
        .remove_member(owner.id, ws.id, owner.id)
        .await
        .unwrap_err();
    assert_eq!(remove_owner_err.code(), ErrorCode::CannotRemoveWorkspaceOwner);

    // Remove collaborator -> permissions immediately revoked (`Forbidden`).
    app.state
        .workspace_service
        .remove_member(owner.id, ws.id, collab.id)
        .await
        .expect("remove collaborator");
    assert_eq!(
        app.state
            .permission_service
            .check(collab.id, ws.id, PermResource::Todos, PermAction::View)
            .await
            .unwrap_err()
            .code(),
        ErrorCode::Forbidden
    );
}
