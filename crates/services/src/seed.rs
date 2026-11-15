// =============================================================================
// DATABASE SEED SERVICE
// =============================================================================
//
// Provides idempotent startup seeding so the configured administrator email
// (`APP__ADMIN__EMAIL`) always exists with `role = 'admin'` and
// `onboarding_completed = TRUE` before serving HTTP traffic.
//
// ## Key Concepts
//
// - **Idempotent Bootstrap**: Safe to run on every server boot across multi-replica
//   or restarted environments without duplicating records or overwriting custom
//   user preferences when already provisioned.
// - **Trigger Coordination**: Creating a new admin user automatically fires
//   `trg_create_default_workspace`, provisioning `Personal Workspace` and 5
//   starter Todos; the inserted row is re-fetched to hydrate `active_workspace_id`.

// SECTION: IMPORTS

// `chrono` - UTC timestamps for updates.
use chrono::Utc;

// `sea_orm` - Queries and inserts/updates the `users` table.
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

// `tracing` - Logs admin seed creation/promotion events.
use tracing::info;

// `shared` - Settings and standardized `AppError`.
use shared::config::settings::Settings;
use shared::domain::error::AppError;
use shared::domain::user::UserRole;

// `crate::entities` - `users` SeaORM entity.
use crate::entities::users;

// SECTION: SEED SERVICE

/// Startup database seeding routines.
pub struct SeedService;

impl SeedService {
    /// Ensures `settings.admin.email` exists in `users` with `role = "admin"` and
    /// `onboarding_completed = true`.
    ///
    /// # Arguments
    ///
    /// * `db` - PostgreSQL pool handle
    /// * `settings` - Application settings containing `admin.email` and `admin.name`
    pub async fn seed_default_admin(
        db: &DatabaseConnection,
        settings: &Settings,
    ) -> Result<users::Model, AppError> {
        let email = settings.admin.email.trim().to_lowercase();
        let display_name = {
            let trimmed = settings.admin.name.trim();
            if trimmed.is_empty() {
                "Template Admin".to_string()
            } else {
                trimmed.to_string()
            }
        };

        if let Some(existing) = users::Entity::find()
            .filter(users::Column::Email.eq(&email))
            .one(db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed querying seed admin: {err}")))?
        {
            let needs_update = existing.role != UserRole::Admin.as_str()
                || !existing.onboarding_completed
                || !existing.email_verified;

            if !needs_update {
                return Ok(existing);
            }

            let mut active: users::ActiveModel = existing.into();
            active.role = Set(UserRole::Admin.as_str().to_string());
            active.onboarding_completed = Set(true);
            active.email_verified = Set(true);
            active.updated_at = Set(Utc::now().into());

            let updated = active
                .update(db)
                .await
                .map_err(|err| AppError::Internal(format!("Failed promoting seed admin: {err}")))?;

            info!(user_id = updated.id, email = %updated.email, "Promoted existing default admin account");
            return Ok(updated);
        }

        let inserted = users::ActiveModel {
            email: Set(email),
            name: Set(display_name),
            role: Set(UserRole::Admin.as_str().to_string()),
            email_verified: Set(true),
            language: Set("vi".to_string()),
            is_test_account: Set(false),
            onboarding_completed: Set(true),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|err| AppError::Internal(format!("Failed inserting seed admin: {err}")))?;

        // Re-fetch so `active_workspace_id` populated by `trg_create_default_workspace` is returned.
        let hydrated = users::Entity::find_by_id(inserted.id)
            .one(db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed reloading seed admin: {err}")))?
            .unwrap_or(inserted);

        info!(
            user_id = hydrated.id,
            email = %hydrated.email,
            active_workspace_id = ?hydrated.active_workspace_id,
            "Seeded default admin account"
        );

        Ok(hydrated)
    }
}
