// =============================================================================
// ENTITY: global_notifications
// =============================================================================

use sea_orm::entity::prelude::*;

/// SeaORM model for the `global_notifications` table (`04_notifications.sql`).
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "global_notifications")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub title: String,
    pub html_content: String,
    pub target_audience: String,
    pub is_active: bool,
    pub published_at: DateTimeWithTimeZone,
    pub expires_at: Option<DateTimeWithTimeZone>,
    pub created_by: Option<i32>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

/// Entity relations for `global_notifications`.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::global_notification_reads::Entity")]
    Reads,
}

impl Related<super::global_notification_reads::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Reads.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
