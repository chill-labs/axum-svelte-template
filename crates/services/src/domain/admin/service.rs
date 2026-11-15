// =============================================================================
// ADMIN CONSOLE DOMAIN SERVICE
// =============================================================================
//
// Implements all 4 `/admin` console capabilities:
// 1. **Users**: Paginated directory (`list_users`) and role promotion/demotion
//    (`update_user_role`).
// 2. **Hot Support**: Isolated customer impersonation sessions
//    (`start_hot_support`, `stop_hot_support`).
// 3. **Test Accounts**: Password-enabled E2E/QA account CRUD (`list_test_accounts`,
//    `create_test_account`, `update_test_account`, `delete_test_account`).
// 4. **Global Notifications**: Rich HTML announcement CRUD + SSE broadcasts
//    (`list_global_notifications`, `create_global_notification`,
//    `update_global_notification`, `delete_global_notification`).
//
// ## Architecture Notes
//
// Exposed in `AppState::admin_service`. Route handlers enforce `UserRole::Admin`
// via `AdminService::ensure_admin` before invoking admin mutations.

// SECTION: IMPORTS

// `chrono` - Session expiry and timestamp updates.
use chrono::{Duration, Utc};

// `sea_orm` - Asynchronous PostgreSQL CRUD and case-insensitive `ILIKE` filtering.
// - `ActiveModelTrait` - Inserts/updates users, sessions, and global notifications
// - `ColumnTrait`, `Condition` - Query predicates
// - `DatabaseConnection` - PostgreSQL pool handle
// - `EntityTrait`, `PaginatorTrait` - Queries and paginated counts
// - `QueryFilter`, `QueryOrder`, `QuerySelect` - Filtering, ordering, limits/offsets
// - `Set` - ActiveModel field assignments
// - `PgExpr`, `Expr` - Case-insensitive `ILIKE` search expressions
use sea_orm::sea_query::extension::postgres::PgExpr;
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set,
};

// `std::collections::{HashMap, HashSet}` - Workspace count aggregation per user.
use std::collections::{HashMap, HashSet};

// `shared` - Admin DTOs, pagination, sanitization, and typed error codes.
use shared::domain::admin::{
    AdminGlobalNotificationResponse, AdminTestAccountResponse, AdminUpdateUserRoleRequest,
    AdminUserResponse, CreateGlobalNotificationRequest, CreateTestAccountRequest,
    GetAdminUsersQuery, HotSupportLoginRequest, HotSupportSessionInfo,
    UpdateGlobalNotificationRequest, UpdateTestAccountRequest,
};
use shared::domain::error::AppError;
use shared::domain::error_code::ErrorCode;
use shared::domain::notification::RealtimeEvent;
use shared::domain::request::{normalize_whitespace, PaginationQuery, Sanitize};
use shared::domain::response::Pagination;
use shared::domain::user::UserRole;

// `crate` - Caches, entity conversions, SSE hub, and crypto helpers.
use crate::cache::CacheService;
use crate::entities::conversions::{
    map_admin_global_notification, map_global_announcement, map_test_account_response,
};
use crate::entities::{global_notifications, sessions, users, workspace_members, workspaces};
use crate::sse::NotificationHub;
use crate::utils::password::hash_password;
use crate::utils::token::{generate_raw_token, hash_token};

// SECTION: CONSTANTS

/// Isolated Hot Support impersonation session lifetime (`4` hours).
pub const HOT_SUPPORT_TTL_HOURS: i64 = 4;

// SECTION: ADMIN SERVICE

/// Service implementing system administration, test accounts, global broadcasts, and Hot Support.
#[derive(Clone, Debug)]
pub struct AdminService {
    /// PostgreSQL connection pool.
    db: DatabaseConnection,
    /// Moka cache container for user/session invalidation.
    cache: CacheService,
    /// Real-time SSE hub broadcasting newly published/updated global notices.
    notification_hub: NotificationHub,
}

impl AdminService {
    /// Constructs a new `AdminService`.
    pub fn new(
        db: DatabaseConnection,
        cache: CacheService,
        notification_hub: NotificationHub,
    ) -> Self {
        Self {
            db,
            cache,
            notification_hub,
        }
    }

    /// Verifies that `actor` holds `UserRole::Admin`, returning `ErrorCode::AdminRequired` otherwise.
    pub fn ensure_admin(actor: &users::Model) -> Result<(), AppError> {
        if UserRole::from_db(&actor.role) == UserRole::Admin {
            Ok(())
        } else {
            Err(AppError::Code(ErrorCode::AdminRequired))
        }
    }

    // SECTION: TAB 1 — USERS DIRECTORY & ROLE MANAGEMENT

    /// Lists paginated users matching optional search text and role filter.
    pub async fn list_users(
        &self,
        query: GetAdminUsersQuery,
    ) -> Result<(Vec<AdminUserResponse>, Pagination), AppError> {
        let mut select = users::Entity::find();

        if let Some(ref raw_search) = query.search {
            let trimmed = raw_search.trim();
            if !trimmed.is_empty() {
                let pattern = format!("%{trimmed}%");
                select = select.filter(
                    Condition::any()
                        .add(Expr::col(users::Column::Email).ilike(pattern.clone()))
                        .add(Expr::col(users::Column::Name).ilike(pattern)),
                );
            }
        }

        if let Some(role) = query.role {
            select = select.filter(users::Column::Role.eq(role.as_str()));
        }

        let (page, page_size) = PaginationQuery {
            page: query.page,
            page_size: query.page_size,
        }
        .normalized();

        let total_items = select
            .clone()
            .count(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed counting admin users: {err}")))?;

        let offset = (page - 1) * page_size;
        let rows = select
            .order_by_desc(users::Column::CreatedAt)
            .order_by_desc(users::Column::Id)
            .offset(offset)
            .limit(page_size)
            .all(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed loading admin users: {err}")))?;

        let ws_counts = self.compute_workspace_counts(&rows).await?;

        let items = rows
            .into_iter()
            .map(|u| {
                let ws_count = ws_counts.get(&u.id).copied().unwrap_or(0);
                AdminUserResponse {
                    id: u.id,
                    email: u.email,
                    name: u.name,
                    role: UserRole::from_db(&u.role),
                    is_test_account: u.is_test_account,
                    onboarding_completed: u.onboarding_completed,
                    workspace_count: ws_count,
                    last_login_at: u.last_login_at.map(|ts| ts.with_timezone(&Utc)),
                    created_at: u.created_at.with_timezone(&Utc),
                }
            })
            .collect();

        Ok((items, Pagination::new(page, page_size, total_items)))
    }

    /// Updates a user's system role (`user` <-> `admin`), blocking self-demotion.
    pub async fn update_user_role(
        &self,
        actor_user_id: i32,
        target_user_id: i32,
        req: AdminUpdateUserRoleRequest,
    ) -> Result<AdminUserResponse, AppError> {
        if actor_user_id == target_user_id && req.role != UserRole::Admin {
            return Err(AppError::Code(ErrorCode::Forbidden));
        }

        let target = users::Entity::find_by_id(target_user_id)
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed querying user: {err}")))?
            .ok_or(AppError::Code(ErrorCode::UserNotFound))?;

        let mut active: users::ActiveModel = target.into();
        active.role = Set(req.role.as_str().to_string());
        active.updated_at = Set(Utc::now().into());

        let updated = active
            .update(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed updating user role: {err}")))?;

        self.cache.users.insert(updated.id, updated.clone()).await;

        let ws_counts = self
            .compute_workspace_counts(std::slice::from_ref(&updated))
            .await?;

        Ok(AdminUserResponse {
            id: updated.id,
            email: updated.email,
            name: updated.name,
            role: UserRole::from_db(&updated.role),
            is_test_account: updated.is_test_account,
            onboarding_completed: updated.onboarding_completed,
            workspace_count: ws_counts.get(&updated.id).copied().unwrap_or(0),
            last_login_at: updated.last_login_at.map(|ts| ts.with_timezone(&Utc)),
            created_at: updated.created_at.with_timezone(&Utc),
        })
    }

    // SECTION: TAB 2 — HOT SUPPORT IMPERSONATION

    /// Starts an isolated Hot Support impersonation session (`hot-support-session`) for `req.user_id`.
    ///
    /// # Returns
    ///
    /// `(HotSupportSessionInfo, raw_hot_support_token)`.
    pub async fn start_hot_support(
        &self,
        admin_user_id: i32,
        req: HotSupportLoginRequest,
    ) -> Result<(HotSupportSessionInfo, String), AppError> {
        let admin = users::Entity::find_by_id(admin_user_id)
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed loading admin user: {err}")))?
            .ok_or(AppError::Code(ErrorCode::UserNotFound))?;
        Self::ensure_admin(&admin)?;

        let target = users::Entity::find_by_id(req.user_id)
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed loading target user: {err}")))?
            .ok_or(AppError::Code(ErrorCode::UserNotFound))?;

        let raw_token = generate_raw_token();
        let hashed = hash_token(&raw_token);
        let expires_at = Utc::now() + Duration::hours(HOT_SUPPORT_TTL_HOURS);

        sessions::ActiveModel {
            id: Set(hashed.clone()),
            user_id: Set(target.id),
            expires_at: Set(expires_at.into()),
            ..Default::default()
        }
        .insert(&self.db)
        .await
        .map_err(|err| AppError::Internal(format!("Failed creating hot support session: {err}")))?;

        self.cache.sessions.insert(hashed, target.id).await;
        self.cache.users.insert(target.id, target.clone()).await;

        let info = HotSupportSessionInfo {
            active: true,
            target_user_id: Some(target.id),
            target_user_name: Some(target.name),
            target_user_email: Some(target.email),
            launch_url: Some("/todos?_hs=1".to_string()),
        };

        Ok((info, raw_token))
    }

    /// Terminates an isolated `hot-support-session` if present.
    pub async fn stop_hot_support(&self, raw_hot_support_token: Option<&str>) -> Result<(), AppError> {
        let Some(raw) = raw_hot_support_token.map(str::trim).filter(|s| !s.is_empty()) else {
            return Ok(());
        };
        let hashed = hash_token(raw);
        sessions::Entity::delete_by_id(&hashed)
            .exec(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed closing hot support session: {err}")))?;
        self.cache.invalidate_session(&hashed).await;
        Ok(())
    }

    // SECTION: TAB 3 — TEST ACCOUNTS CRUD

    /// Lists all admin-provisioned password-enabled Test Accounts.
    pub async fn list_test_accounts(&self) -> Result<Vec<AdminTestAccountResponse>, AppError> {
        let rows = users::Entity::find()
            .filter(users::Column::IsTestAccount.eq(true))
            .order_by_desc(users::Column::CreatedAt)
            .order_by_desc(users::Column::Id)
            .all(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed listing test accounts: {err}")))?;

        Ok(rows.iter().map(map_test_account_response).collect())
    }

    /// Creates a password-enabled Test Account (triggers automatic Personal Workspace provisioning).
    pub async fn create_test_account(
        &self,
        mut req: CreateTestAccountRequest,
    ) -> Result<AdminTestAccountResponse, AppError> {
        req.sanitize();
        req.validate_input()?;

        let duplicate = users::Entity::find()
            .filter(users::Column::Email.eq(&req.email))
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed checking duplicate email: {err}")))?
            .is_some();

        if duplicate {
            return Err(AppError::Code(ErrorCode::EmailAlreadyExists));
        }

        let password_hash = hash_password(&req.password)?;
        let role = req.role.unwrap_or(UserRole::User);
        let language = req
            .language
            .filter(|l| l == "vi" || l == "en")
            .unwrap_or_else(|| "vi".to_string());

        let inserted = users::ActiveModel {
            email: Set(req.email),
            password_hash: Set(Some(password_hash)),
            name: Set(req.name),
            role: Set(role.as_str().to_string()),
            email_verified: Set(true),
            language: Set(language),
            is_test_account: Set(true),
            onboarding_completed: Set(true),
            ..Default::default()
        }
        .insert(&self.db)
        .await
        .map_err(|err| AppError::Internal(format!("Failed creating test account: {err}")))?;

        // Re-fetch to capture `active_workspace_id` populated by `trg_create_default_workspace`.
        let hydrated = users::Entity::find_by_id(inserted.id)
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed reloading test account: {err}")))?
            .ok_or(AppError::Code(ErrorCode::UserNotFound))?;

        self.cache.users.insert(hydrated.id, hydrated.clone()).await;
        Ok(map_test_account_response(&hydrated))
    }

    /// Updates a Test Account's name, password, role, or preferred language.
    pub async fn update_test_account(
        &self,
        user_id: i32,
        mut req: UpdateTestAccountRequest,
    ) -> Result<AdminTestAccountResponse, AppError> {
        req.sanitize();
        req.validate_input()?;

        let user = users::Entity::find_by_id(user_id)
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed loading test account: {err}")))?
            .ok_or(AppError::Code(ErrorCode::UserNotFound))?;

        if !user.is_test_account {
            return Err(AppError::Code(ErrorCode::Forbidden));
        }

        let mut active: users::ActiveModel = user.into();
        if let Some(name) = req.name {
            active.name = Set(name);
        }
        if let Some(password) = req.password {
            active.password_hash = Set(Some(hash_password(&password)?));
        }
        if let Some(role) = req.role {
            active.role = Set(role.as_str().to_string());
        }
        if let Some(language) = req.language {
            if language == "vi" || language == "en" {
                active.language = Set(language);
            }
        }
        active.updated_at = Set(Utc::now().into());

        let updated = active
            .update(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed updating test account: {err}")))?;

        self.cache.users.insert(updated.id, updated.clone()).await;
        Ok(map_test_account_response(&updated))
    }

    /// Deletes a Test Account (`is_test_account == true` enforced).
    pub async fn delete_test_account(&self, user_id: i32) -> Result<(), AppError> {
        let user = users::Entity::find_by_id(user_id)
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed loading test account: {err}")))?
            .ok_or(AppError::Code(ErrorCode::UserNotFound))?;

        if !user.is_test_account {
            return Err(AppError::Code(ErrorCode::Forbidden));
        }

        users::Entity::delete_by_id(user_id)
            .exec(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed deleting test account: {err}")))?;

        self.cache.invalidate_user(user_id).await;
        self.cache.invalidate_workspace_list(user_id).await;
        Ok(())
    }

    // SECTION: TAB 4 — GLOBAL NOTIFICATIONS CRUD & SSE BROADCAST

    /// Lists all global HTML announcements for the Admin Console.
    pub async fn list_global_notifications(
        &self,
    ) -> Result<Vec<AdminGlobalNotificationResponse>, AppError> {
        let rows = global_notifications::Entity::find()
            .order_by_desc(global_notifications::Column::PublishedAt)
            .order_by_desc(global_notifications::Column::Id)
            .all(&self.db)
            .await
            .map_err(|err| {
                AppError::Internal(format!("Failed listing global notifications: {err}"))
            })?;

        Ok(rows.iter().map(map_admin_global_notification).collect())
    }

    /// Publishes a new global HTML announcement and broadcasts via `NotificationHub` when active.
    pub async fn create_global_notification(
        &self,
        admin_user_id: i32,
        mut req: CreateGlobalNotificationRequest,
    ) -> Result<AdminGlobalNotificationResponse, AppError> {
        req.sanitize();
        req.validate_input()?;

        let is_active = req.is_active.unwrap_or(true);
        let now = Utc::now();

        let created = global_notifications::ActiveModel {
            title: Set(req.title),
            html_content: Set(req.html_content),
            target_audience: Set(req.target_audience.as_str().to_string()),
            is_active: Set(is_active),
            published_at: Set(now.into()),
            created_by: Set(Some(admin_user_id)),
            ..Default::default()
        }
        .insert(&self.db)
        .await
        .map_err(|err| AppError::Internal(format!("Failed creating global notification: {err}")))?;

        if created.is_active {
            self.notification_hub
                .broadcast_all(RealtimeEvent::GlobalAnnouncement(map_global_announcement(
                    &created, false,
                )));
        }

        Ok(map_admin_global_notification(&created))
    }

    /// Updates a global announcement and broadcasts via `NotificationHub` if active.
    pub async fn update_global_notification(
        &self,
        notification_id: i32,
        req: UpdateGlobalNotificationRequest,
    ) -> Result<AdminGlobalNotificationResponse, AppError> {
        let existing = global_notifications::Entity::find_by_id(notification_id)
            .one(&self.db)
            .await
            .map_err(|err| {
                AppError::Internal(format!("Failed loading global notification: {err}"))
            })?
            .ok_or(AppError::Code(ErrorCode::NotificationNotFound))?;

        let mut active: global_notifications::ActiveModel = existing.into();

        if let Some(raw_title) = req.title {
            let clean = normalize_whitespace(&raw_title);
            if clean.is_empty() || clean.len() > 200 {
                let mut errs = HashMap::new();
                errs.insert("title".to_string(), ErrorCode::FieldRequired);
                return Err(AppError::FieldErrors(errs));
            }
            active.title = Set(clean);
        }
        if let Some(raw_html) = req.html_content {
            let clean = raw_html.trim().to_string();
            if clean.is_empty() {
                let mut errs = HashMap::new();
                errs.insert("html_content".to_string(), ErrorCode::FieldRequired);
                return Err(AppError::FieldErrors(errs));
            }
            active.html_content = Set(clean);
        }
        if let Some(audience) = req.target_audience {
            active.target_audience = Set(audience.as_str().to_string());
        }
        if let Some(is_active) = req.is_active {
            active.is_active = Set(is_active);
        }
        active.updated_at = Set(Utc::now().into());

        let updated = active.update(&self.db).await.map_err(|err| {
            AppError::Internal(format!("Failed updating global notification: {err}"))
        })?;

        if updated.is_active {
            self.notification_hub
                .broadcast_all(RealtimeEvent::GlobalAnnouncement(map_global_announcement(
                    &updated, false,
                )));
        }

        Ok(map_admin_global_notification(&updated))
    }

    /// Deletes a global announcement by ID (`ErrorCode::NotificationNotFound` if missing).
    pub async fn delete_global_notification(&self, notification_id: i32) -> Result<(), AppError> {
        let res = global_notifications::Entity::delete_by_id(notification_id)
            .exec(&self.db)
            .await
            .map_err(|err| {
                AppError::Internal(format!("Failed deleting global notification: {err}"))
            })?;

        if res.rows_affected == 0 {
            return Err(AppError::Code(ErrorCode::NotificationNotFound));
        }
        Ok(())
    }

    // SECTION: INTERNAL HELPERS

    /// Computes total distinct accessible workspace counts per user ID.
    async fn compute_workspace_counts(
        &self,
        user_rows: &[users::Model],
    ) -> Result<HashMap<i32, i64>, AppError> {
        if user_rows.is_empty() {
            return Ok(HashMap::new());
        }

        let user_ids: Vec<i32> = user_rows.iter().map(|u| u.id).collect();
        let owned = workspaces::Entity::find()
            .filter(workspaces::Column::OwnerId.is_in(user_ids.clone()))
            .all(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed counting owned workspaces: {err}")))?;

        let memberships = workspace_members::Entity::find()
            .filter(workspace_members::Column::UserId.is_in(user_ids))
            .all(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed counting memberships: {err}")))?;

        let mut per_user: HashMap<i32, HashSet<i32>> = HashMap::new();
        for w in owned {
            per_user.entry(w.owner_id).or_default().insert(w.id);
        }
        for m in memberships {
            per_user.entry(m.user_id).or_default().insert(m.workspace_id);
        }

        Ok(per_user
            .into_iter()
            .map(|(uid, set)| (uid, set.len() as i64))
            .collect())
    }
}
