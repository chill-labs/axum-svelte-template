// =============================================================================
// ENTITY: notifications
// =============================================================================

use sea_orm::entity::prelude::*;

/// SeaORM model for the `notifications` table (`04_notifications.sql`).
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "notifications")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub user_id: i32,
    pub workspace_id: Option<i32>,
    pub kind: String,
    pub title: String,
    pub body: String,
    pub link_url: Option<String>,
    pub is_read: bool,
    pub created_at: DateTimeWithTimeZone,
}

/// Entity relations for `notifications`.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::UserId",
        to = "super::users::Column::Id",
        on_delete = "Cascade"
    )]
    User,
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
