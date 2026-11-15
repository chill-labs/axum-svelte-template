// =============================================================================
// WORKSPACE RBAC PERMISSIONS
// =============================================================================
//
// Granular `PermResource × PermAction` permission matrix for workspace members.
// Workspace owners bypass individual checks; collaborators hold explicit
// subsets stored in `workspace_members.permissions` (PostgreSQL JSONB).

use std::collections::HashSet;

// `serde` - Serializes permissions as structured JSON objects or snake_case tokens.
use serde::{Deserialize, Serialize};

/// Protected resource domains inside a workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermResource {
    /// Workspace settings (rename, edit description, delete workspace).
    Workspace,
    /// Workspace membership & collaborator role management.
    Members,
    /// Todos within the workspace.
    Todos,
}

/// Actions applicable to each `PermResource`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermAction {
    View,
    Create,
    Edit,
    Delete,
}

/// Single `(resource, action)` permission grant stored in `workspace_members.permissions`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Permission {
    pub resource: PermResource,
    pub action: PermAction,
}

impl Permission {
    /// Convenience constructor for permission pairs.
    pub const fn new(resource: PermResource, action: PermAction) -> Self {
        Self { resource, action }
    }
}

/// Effective set of workspace permissions resolved for a user.
pub type PermissionSet = HashSet<Permission>;

/// Checks whether a permission set grants `(resource, action)`.
pub fn has_permission(set: &PermissionSet, resource: PermResource, action: PermAction) -> bool {
    set.contains(&Permission::new(resource, action))
}

/// Checks whether a permission set grants at least one of the listed `(resource, action)` pairs.
pub fn has_any_permission(set: &PermissionSet, checks: &[(PermResource, PermAction)]) -> bool {
    checks
        .iter()
        .any(|(resource, action)| has_permission(set, *resource, *action))
}

/// Returns full permissions for workspace owners / admins.
pub fn all_permissions() -> Vec<Permission> {
    let mut list = Vec::new();
    for resource in [
        PermResource::Workspace,
        PermResource::Members,
        PermResource::Todos,
    ] {
        for action in [
            PermAction::View,
            PermAction::Create,
            PermAction::Edit,
            PermAction::Delete,
        ] {
            list.push(Permission::new(resource, action));
        }
    }
    list
}

/// Returns preset permissions for the `editor` workspace role.
pub fn editor_permissions() -> Vec<Permission> {
    vec![
        Permission::new(PermResource::Workspace, PermAction::View),
        Permission::new(PermResource::Members, PermAction::View),
        Permission::new(PermResource::Todos, PermAction::View),
        Permission::new(PermResource::Todos, PermAction::Create),
        Permission::new(PermResource::Todos, PermAction::Edit),
        Permission::new(PermResource::Todos, PermAction::Delete),
    ]
}

/// Returns default permissions for a standard workspace `member`.
pub fn default_member_permissions() -> Vec<Permission> {
    vec![
        Permission::new(PermResource::Workspace, PermAction::View),
        Permission::new(PermResource::Members, PermAction::View),
        Permission::new(PermResource::Todos, PermAction::View),
        Permission::new(PermResource::Todos, PermAction::Create),
        Permission::new(PermResource::Todos, PermAction::Edit),
    ]
}

/// Returns read-only permissions for the `viewer` workspace role.
pub fn viewer_permissions() -> Vec<Permission> {
    vec![
        Permission::new(PermResource::Workspace, PermAction::View),
        Permission::new(PermResource::Todos, PermAction::View),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_presets_match_expected_capabilities() {
        let member_set: PermissionSet = default_member_permissions().into_iter().collect();
        assert!(has_permission(&member_set, PermResource::Todos, PermAction::Create));
        assert!(!has_permission(&member_set, PermResource::Todos, PermAction::Delete));

        let viewer_set: PermissionSet = viewer_permissions().into_iter().collect();
        assert!(has_permission(&viewer_set, PermResource::Todos, PermAction::View));
        assert!(!has_permission(&viewer_set, PermResource::Todos, PermAction::Create));
        assert_eq!(all_permissions().len(), 12);
    }
}
