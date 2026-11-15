// =============================================================================
// ENTITY: users
// =============================================================================

use sea_orm::entity::prelude::*;

/// SeaORM model for the `users` table (`01_users.sql`).
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub email: String,
    pub password_hash: Option<String>,
    pub name: String,
    pub avatar_url: Option<String>,
    pub role: String,
    pub email_verified: bool,
    #[sea_orm(unique)]
    pub google_id: Option<String>,
    pub language: String,
    pub is_test_account: bool,
    pub active_workspace_id: Option<i32>,
    pub onboarding_completed: bool,
    pub last_login_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

/// Entity relations for `users`.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::sessions::Entity")]
    Sessions,
    #[sea_orm(has_many = "super::workspaces::Entity")]
    OwnedWorkspaces,
    #[sea_orm(has_many = "super::workspace_members::Entity")]
    WorkspaceMemberships,
    #[sea_orm(has_many = "super::notifications::Entity")]
    Notifications,
}

impl Related<super::sessions::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Sessions.def()
    }
}

impl Related<super::workspaces::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OwnedWorkspaces.def()
    }
}

impl Related<super::workspace_members::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WorkspaceMemberships.def()
    }
}

impl Related<super::notifications::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Notifications.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
