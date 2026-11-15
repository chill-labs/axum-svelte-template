// =============================================================================
// ENTITY: media_files
// =============================================================================

use sea_orm::entity::prelude::*;

/// SeaORM model for the `media_files` table (`05_media_files.sql`).
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "media_files")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub entity_type: String,
    pub entity_id: i32,
    pub field_name: String,
    pub storage_key: String,
    pub xor_key: String,
    pub content_type: String,
    pub file_size: i64,
    pub uploaded_by: Option<i32>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

/// Entity relations for `media_files`.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
