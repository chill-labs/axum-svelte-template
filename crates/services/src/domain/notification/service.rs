// =============================================================================
// NOTIFICATION DOMAIN SERVICE
// =============================================================================
//
// Manages per-user real-time notifications (`notifications`) and admin-published
// global HTML announcements (`global_notifications` + `global_notification_reads`).
//
// ## Key Concepts
//
// - **Real-time Push**: Every created user notification immediately publishes a
//   `RealtimeEvent::UserNotification` onto `NotificationHub`.
// - **Audience Filtering**: Global announcements filter by `all`, `authenticated`,
//   and `admins` based on the caller's session and role.
// - **Idempotent Read Receipts**: Marking global announcements read handles
//   duplicate `(global_notification_id, user_id)` inserts gracefully.
//
// ## Architecture Notes
//
// Injected into `AppState` and also used by `WorkspaceService` and `TodoService`
// to notify collaborators on invitations and task assignments.

// SECTION: IMPORTS

// `chrono` - UTC timestamps for expiry checks and read receipt creation.
use chrono::Utc;

// `sea_orm` - Asynchronous PostgreSQL queries and bulk updates for notifications.
// - `ActiveModelTrait` - Inserts/updates notification records
// - `ColumnTrait` - Filter columns (`UserId`, `IsRead`, `IsActive`, etc.)
// - `Condition` - Combines audience and expiration filters
// - `DatabaseConnection` - PostgreSQL pool handle
// - `EntityTrait` - Query entry points
// - `PaginatorTrait` - Fast unread counts
// - `QueryFilter`, `QueryOrder`, `QuerySelect` - Filtering, ordering, limits
// - `Set` - Sets ActiveModel fields
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set,
};
use sea_orm::sea_query::Expr;

// `std::collections::HashSet` - Tracks global notification IDs already read by a user.
use std::collections::HashSet;

// `shared` - Notification DTOs, user roles, SSE events, and typed errors.
// - `AppError` - Standardized service error wrapper
// - `ErrorCode` - `NotificationNotFound`
// - `GlobalAnnouncementResponse`, `GlobalNotificationAudience`,
//   `NotificationFeedResponse`, `NotificationResponse`, `RealtimeEvent` - DTOs
// - `UserRole` - Determines `admins` audience visibility
use shared::domain::error::AppError;
use shared::domain::error_code::ErrorCode;
use shared::domain::notification::{
    GlobalAnnouncementResponse, GlobalNotificationAudience, NotificationFeedResponse,
    NotificationResponse, RealtimeEvent,
};
use shared::domain::user::UserRole;

// `crate::entities` - SeaORM entities and conversion helpers.
use crate::entities::conversions::{map_global_announcement, map_notification_response};
use crate::entities::{global_notification_reads, global_notifications, notifications};

// `crate::sse` - Real-time SSE broadcast hub.
use crate::sse::NotificationHub;

// SECTION: NOTIFICATION SERVICE

/// Service managing personal notifications, global announcements, and read receipts.
#[derive(Clone, Debug)]
pub struct NotificationService {
    /// PostgreSQL connection pool.
    db: DatabaseConnection,
    /// Real-time SSE hub for pushing new notifications.
    notification_hub: NotificationHub,
}

impl NotificationService {
    /// Creates a new `NotificationService`.
    pub fn new(db: DatabaseConnection, notification_hub: NotificationHub) -> Self {
        Self {
            db,
            notification_hub,
        }
    }

    /// Creates a personal in-app notification and emits `RealtimeEvent::UserNotification`.
    ///
    /// # Arguments
    ///
    /// * `user_id` - Recipient user ID
    /// * `workspace_id` - Optional originating workspace ID
    /// * `kind` - Semantic notification tag (e.g. `"info"`, `"todo_assigned"`, `"member_added"`)
    /// * `title` - Short summary headline
    /// * `body` - Detailed message body
    /// * `link_url` - Optional deep link inside the app
    pub async fn create_notification(
        &self,
        user_id: i32,
        workspace_id: Option<i32>,
        kind: &str,
        title: &str,
        body: &str,
        link_url: Option<String>,
    ) -> Result<NotificationResponse, AppError> {
        let model = notifications::ActiveModel {
            user_id: Set(user_id),
            workspace_id: Set(workspace_id),
            kind: Set(kind.to_string()),
            title: Set(title.to_string()),
            body: Set(body.to_string()),
            link_url: Set(link_url),
            is_read: Set(false),
            ..Default::default()
        }
        .insert(&self.db)
        .await
        .map_err(|err| AppError::Internal(format!("Failed creating notification: {err}")))?;

        let dto = map_notification_response(&model);
        self.notification_hub
            .send_to_user(user_id, RealtimeEvent::UserNotification(dto.clone()));

        Ok(dto)
    }

    /// Returns the combined notification feed (personal + eligible active global broadcasts)
    /// for either an authenticated user or an unauthenticated guest (`user_id = None`).
    pub async fn get_feed(
        &self,
        user_id: Option<i32>,
        user_role: Option<UserRole>,
    ) -> Result<NotificationFeedResponse, AppError> {
        let now = Utc::now();
        let allowed_audiences: Vec<&str> = match (user_id, user_role) {
            (Some(_), Some(UserRole::Admin)) => vec![
                GlobalNotificationAudience::All.as_str(),
                GlobalNotificationAudience::Authenticated.as_str(),
                GlobalNotificationAudience::Admins.as_str(),
            ],
            (Some(_), _) => vec![
                GlobalNotificationAudience::All.as_str(),
                GlobalNotificationAudience::Authenticated.as_str(),
            ],
            (None, _) => vec![GlobalNotificationAudience::All.as_str()],
        };

        let global_rows = global_notifications::Entity::find()
            .filter(global_notifications::Column::IsActive.eq(true))
            .filter(global_notifications::Column::TargetAudience.is_in(allowed_audiences))
            .filter(
                Condition::any()
                    .add(global_notifications::Column::ExpiresAt.is_null())
                    .add(global_notifications::Column::ExpiresAt.gt(now)),
            )
            .order_by_desc(global_notifications::Column::PublishedAt)
            .order_by_desc(global_notifications::Column::Id)
            .limit(20)
            .all(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed loading global notices: {err}")))?;

        let Some(uid) = user_id else {
            let global_items: Vec<GlobalAnnouncementResponse> = global_rows
                .iter()
                .map(|row| map_global_announcement(row, false))
                .collect();
            let unread_count = global_items.len() as u64;
            return Ok(NotificationFeedResponse {
                items: Vec::new(),
                global_items,
                unread_count,
            });
        };

        let personal_rows = notifications::Entity::find()
            .filter(notifications::Column::UserId.eq(uid))
            .order_by_desc(notifications::Column::CreatedAt)
            .order_by_desc(notifications::Column::Id)
            .limit(50)
            .all(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed loading notifications: {err}")))?;

        let global_ids: Vec<i32> = global_rows.iter().map(|g| g.id).collect();
        let read_ids: HashSet<i32> = if global_ids.is_empty() {
            HashSet::new()
        } else {
            global_notification_reads::Entity::find()
                .filter(global_notification_reads::Column::UserId.eq(uid))
                .filter(global_notification_reads::Column::GlobalNotificationId.is_in(global_ids))
                .all(&self.db)
                .await
                .map_err(|err| {
                    AppError::Internal(format!("Failed loading global read receipts: {err}"))
                })?
                .into_iter()
                .map(|r| r.global_notification_id)
                .collect()
        };

        let items: Vec<NotificationResponse> =
            personal_rows.iter().map(map_notification_response).collect();
        let global_items: Vec<GlobalAnnouncementResponse> = global_rows
            .iter()
            .map(|g| map_global_announcement(g, read_ids.contains(&g.id)))
            .collect();

        let unread_personal = notifications::Entity::find()
            .filter(notifications::Column::UserId.eq(uid))
            .filter(notifications::Column::IsRead.eq(false))
            .count(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed counting unread notices: {err}")))?;

        let unread_global = global_items.iter().filter(|g| !g.is_read).count() as u64;

        Ok(NotificationFeedResponse {
            items,
            global_items,
            unread_count: unread_personal + unread_global,
        })
    }

    /// Convenience wrapper loading the notification feed for an authenticated user.
    pub async fn list_for_user(
        &self,
        user_id: i32,
        user_role: UserRole,
    ) -> Result<NotificationFeedResponse, AppError> {
        self.get_feed(Some(user_id), Some(user_role)).await
    }

    /// Counts unread personal + active global notifications for session bootstrap.
    pub async fn count_unread(&self, user_id: i32, user_role: UserRole) -> Result<u64, AppError> {
        let feed = self.get_feed(Some(user_id), Some(user_role)).await?;
        Ok(feed.unread_count)
    }

    /// Marks a single personal notification as read (`user_id` ownership enforced).
    pub async fn mark_read(&self, user_id: i32, notification_id: i32) -> Result<(), AppError> {
        let model = notifications::Entity::find_by_id(notification_id)
            .filter(notifications::Column::UserId.eq(user_id))
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed querying notification: {err}")))?
            .ok_or(AppError::Code(ErrorCode::NotificationNotFound))?;

        if model.is_read {
            return Ok(());
        }

        let mut active: notifications::ActiveModel = model.into();
        active.is_read = Set(true);
        active
            .update(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed marking notification read: {err}")))?;

        Ok(())
    }

    /// Marks all personal notifications and active eligible global notices read for `user_id`.
    pub async fn mark_all_read(&self, user_id: i32, user_role: UserRole) -> Result<(), AppError> {
        notifications::Entity::update_many()
            .col_expr(notifications::Column::IsRead, Expr::value(true))
            .filter(notifications::Column::UserId.eq(user_id))
            .filter(notifications::Column::IsRead.eq(false))
            .exec(&self.db)
            .await
            .map_err(|err| {
                AppError::Internal(format!("Failed marking notifications read: {err}"))
            })?;

        let feed = self.get_feed(Some(user_id), Some(user_role)).await?;
        for global in feed.global_items.into_iter().filter(|g| !g.is_read) {
            self.mark_global_read(user_id, global.id).await?;
        }

        Ok(())
    }

    /// Records a read receipt for a global announcement idempotently.
    pub async fn mark_global_read(
        &self,
        user_id: i32,
        global_notification_id: i32,
    ) -> Result<(), AppError> {
        let exists = global_notifications::Entity::find_by_id(global_notification_id)
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed loading global notice: {err}")))?
            .is_some();

        if !exists {
            return Err(AppError::Code(ErrorCode::NotificationNotFound));
        }

        let already = global_notification_reads::Entity::find()
            .filter(global_notification_reads::Column::GlobalNotificationId.eq(global_notification_id))
            .filter(global_notification_reads::Column::UserId.eq(user_id))
            .one(&self.db)
            .await
            .map_err(|err| {
                AppError::Internal(format!("Failed checking global notice read state: {err}"))
            })?;

        if already.is_some() {
            return Ok(());
        }

        let active = global_notification_reads::ActiveModel {
            global_notification_id: Set(global_notification_id),
            user_id: Set(user_id),
            ..Default::default()
        };

        // Ignore unique constraint races from concurrent requests.
        if let Err(err) = active.insert(&self.db).await {
            let still_missing = global_notification_reads::Entity::find()
                .filter(
                    global_notification_reads::Column::GlobalNotificationId
                        .eq(global_notification_id),
                )
                .filter(global_notification_reads::Column::UserId.eq(user_id))
                .one(&self.db)
                .await
                .map_err(|q_err| {
                    AppError::Internal(format!("Failed verifying global read receipt: {q_err}"))
                })?
                .is_none();

            if still_missing {
                return Err(AppError::Internal(format!(
                    "Failed recording global read receipt: {err}"
                )));
            }
        }

        Ok(())
    }
}
