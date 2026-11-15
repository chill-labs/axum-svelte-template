// =============================================================================
// ENTITY: global_notification_reads
// =============================================================================

use sea_orm::entity::prelude::*;

/// SeaORM model for the `global_notification_reads` table (`04_notifications.sql`).
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "global_notification_reads")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub global_notification_id: i32,
    pub user_id: i32,
    pub read_at: DateTimeWithTimeZone,
}

/// Entity relations for `global_notification_reads`.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::global_notifications::Entity",
        from = "Column::GlobalNotificationId",
        to = "super::global_notifications::Column::Id",
        on_delete = "Cascade"
    )]
    GlobalNotification,
}

impl Related<super::global_notifications::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::GlobalNotification.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
