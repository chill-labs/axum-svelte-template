// =============================================================================
// TODO DOMAIN SERVICE
// =============================================================================
//
// Manages workspace-scoped Todo items with RBAC enforcement, Moka list caching,
// three-state `deserialize_double_option` PATCH updates, batch attachment key
// resolution, reordering, and tenant-scoped SSE `TodoChanged` broadcasts.
//
// ## Key Concepts
//
// - **Tenant Isolation**: Every read/write enforces `PermissionService::check`
//   and filters by `todos.workspace_id == workspace_id` (`TodoNotFound` if
//   targeting a task in another workspace).
// - **Status <-> Completion Sync**: Updating `status` to `Done` or toggling
//   `is_completed` automatically synchronizes `completed_at`.
// - **Assignee Guard**: Setting `assigned_to` verifies the target user owns or
//   belongs to `workspace_id`, returning `ErrorCode::AssigneeNotInWorkspace`
//   otherwise.
//
// ## Architecture Notes
//
// Uses `CacheService.workspace_todos` (`CachedList<TodoResponse>`) and notifies
// workspace members over `NotificationHub` on every mutation.

// SECTION: IMPORTS

// `chrono` - UTC completion and update timestamps.
use chrono::Utc;

// `sea_orm` - Asynchronous PostgreSQL CRUD, transactions, and batch user lookups.
// - `ActiveModelTrait` - Inserts, updates, deletes Todo rows
// - `ColumnTrait` - Filter columns (`WorkspaceId`, `Id`, `Position`)
// - `DatabaseConnection` - PostgreSQL pool handle
// - `EntityTrait`, `QueryFilter`, `QueryOrder`, `TransactionTrait` - Query/tx APIs
// - `Set` - Sets ActiveModel fields
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
    TransactionTrait,
};

// `std::collections::{HashMap, HashSet}` - Fast creator/assignee name & member lookup.
use std::collections::{HashMap, HashSet};

// `shared` - Todo DTOs, RBAC primitives, pagination, and error codes.
use shared::domain::error::AppError;
use shared::domain::error_code::ErrorCode;
use shared::domain::notification::RealtimeEvent;
use shared::domain::permission::{PermAction, PermResource};
use shared::domain::request::{PaginationQuery, Sanitize, SortOrder};
use shared::domain::response::Pagination;
use shared::domain::todo::{
    CreateTodoRequest, ReorderTodosRequest, TodoListPayload, TodoListQuery, TodoPriority,
    TodoResponse, TodoStatus, TodoSummaryStats, UpdateTodoRequest,
};
use shared::domain::upload::UploadPurpose;

// `crate` - Caching, entities, media service, notifications, RBAC, and SSE hub.
use crate::cache::CacheService;
use crate::domain::media::service::{MediaService, DEFAULT_MEDIA_FIELD};
use crate::domain::notification::service::NotificationService;
use crate::entities::conversions::map_todo_response;
use crate::entities::{todos, users, workspace_members, workspaces};
use crate::permission::PermissionService;
use crate::sse::NotificationHub;

// SECTION: TODO SERVICE

/// Service implementing workspace-scoped Todo management and real-time broadcasts.
#[derive(Clone)]
pub struct TodoService {
    /// PostgreSQL connection pool.
    db: DatabaseConnection,
    /// Moka cache container holding `workspace_todos`.
    cache: CacheService,
    /// Workspace RBAC enforcement service.
    permission_service: PermissionService,
    /// Encrypted media lookup and cleanup service for Todo attachments.
    media_service: MediaService,
    /// Real-time SSE hub broadcasting `TodoChanged` to workspace members.
    notification_hub: NotificationHub,
    /// Personal notification service for assignee alerts.
    notification_service: NotificationService,
}

impl TodoService {
    /// Constructs a new `TodoService`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        db: DatabaseConnection,
        cache: CacheService,
        permission_service: PermissionService,
        media_service: MediaService,
        notification_hub: NotificationHub,
        notification_service: NotificationService,
    ) -> Self {
        Self {
            db,
            cache,
            permission_service,
            media_service,
            notification_hub,
            notification_service,
        }
    }

    // SECTION: QUERIES

    /// Lists filtered & paginated Todos plus aggregate `TodoSummaryStats` for `workspace_id`.
    pub async fn list_todos(
        &self,
        user_id: i32,
        workspace_id: i32,
        query: TodoListQuery,
    ) -> Result<(TodoListPayload, Pagination), AppError> {
        self.permission_service
            .check(user_id, workspace_id, PermResource::Todos, PermAction::View)
            .await?;

        let all_items = self
            .cache
            .workspace_todos
            .get_or_load(workspace_id, || async move {
                self.load_workspace_todos_from_db(workspace_id).await
            })
            .await?;

        let stats = compute_stats(&all_items);

        let search_needle = query
            .search
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_lowercase());

        let mut filtered: Vec<TodoResponse> = all_items
            .into_iter()
            .filter(|item| {
                if let Some(status) = query.status {
                    if item.status != status {
                        return false;
                    }
                }
                if let Some(priority) = query.priority {
                    if item.priority != priority {
                        return false;
                    }
                }
                if let Some(assigned_to) = query.assigned_to {
                    if item.assigned_to != Some(assigned_to) {
                        return false;
                    }
                }
                if let Some(ref needle) = search_needle {
                    let title_hit = item.title.to_lowercase().contains(needle);
                    let desc_hit = item
                        .description
                        .as_deref()
                        .is_some_and(|d| d.to_lowercase().contains(needle));
                    if !title_hit && !desc_hit {
                        return false;
                    }
                }
                true
            })
            .collect();

        match query.sort_order {
            Some(SortOrder::Desc) => {
                filtered.sort_by(|a, b| b.position.cmp(&a.position).then_with(|| b.id.cmp(&a.id)));
            }
            Some(SortOrder::Asc) | None => {
                filtered.sort_by(|a, b| a.position.cmp(&b.position).then_with(|| a.id.cmp(&b.id)));
            }
        }

        let (page, page_size) = PaginationQuery {
            page: query.page,
            page_size: query.page_size,
        }
        .normalized();

        let total_items = filtered.len() as u64;
        let pagination = Pagination::new(page, page_size, total_items);
        let offset = ((pagination.page - 1) * pagination.page_size) as usize;
        let page_items = filtered
            .into_iter()
            .skip(offset)
            .take(pagination.page_size as usize)
            .collect();

        Ok((
            TodoListPayload {
                items: page_items,
                stats,
            },
            pagination,
        ))
    }

    /// Loads a single Todo item scoped strictly to `workspace_id`.
    pub async fn get_todo(
        &self,
        user_id: i32,
        workspace_id: i32,
        todo_id: i32,
    ) -> Result<TodoResponse, AppError> {
        self.permission_service
            .check(user_id, workspace_id, PermResource::Todos, PermAction::View)
            .await?;

        let model = todos::Entity::find_by_id(todo_id)
            .filter(todos::Column::WorkspaceId.eq(workspace_id))
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed querying todo: {err}")))?
            .ok_or(AppError::Code(ErrorCode::TodoNotFound))?;

        self.enrich_single_todo(model).await
    }

    // SECTION: MUTATIONS

    /// Creates a new Todo item inside `workspace_id`, updates caches, and notifies members.
    pub async fn create_todo(
        &self,
        user_id: i32,
        workspace_id: i32,
        mut req: CreateTodoRequest,
    ) -> Result<TodoResponse, AppError> {
        req.sanitize();
        req.validate_input()?;

        self.permission_service
            .check(
                user_id,
                workspace_id,
                PermResource::Todos,
                PermAction::Create,
            )
            .await?;

        if let Some(assignee_id) = req.assigned_to {
            self.verify_workspace_member(workspace_id, assignee_id)
                .await?;
        }

        let max_pos = todos::Entity::find()
            .filter(todos::Column::WorkspaceId.eq(workspace_id))
            .order_by_desc(todos::Column::Position)
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed computing todo position: {err}")))?
            .map_or(0, |t| t.position);

        let status = req.status.unwrap_or_default();
        let priority = req.priority.unwrap_or_default();
        let is_completed = status == TodoStatus::Done;
        let completed_at = if is_completed {
            Some(Utc::now().into())
        } else {
            None
        };

        let created = todos::ActiveModel {
            workspace_id: Set(workspace_id),
            created_by: Set(user_id),
            assigned_to: Set(req.assigned_to),
            title: Set(req.title),
            description: Set(req.description),
            status: Set(status.as_str().to_string()),
            priority: Set(priority.as_str().to_string()),
            due_date: Set(req.due_date),
            is_completed: Set(is_completed),
            completed_at: Set(completed_at),
            position: Set(max_pos + 1),
            ..Default::default()
        }
        .insert(&self.db)
        .await
        .map_err(|err| AppError::Internal(format!("Failed creating todo: {err}")))?;

        self.invalidate_and_notify(workspace_id, created.id).await;

        if let Some(assignee_id) = created.assigned_to {
            if assignee_id != user_id {
                let _ = self
                    .notification_service
                    .create_notification(
                        assignee_id,
                        Some(workspace_id),
                        "todo_assigned",
                        "Task assigned to you",
                        &format!("You were assigned '{}'.", created.title),
                        Some("/todos".to_string()),
                    )
                    .await;
            }
        }

        self.enrich_single_todo(created).await
    }

    /// Applies partial PATCH updates (`deserialize_double_option` for `description`, `due_date`,
    /// and `assigned_to`) and keeps `status`, `is_completed`, and `completed_at` synchronized.
    pub async fn update_todo(
        &self,
        user_id: i32,
        workspace_id: i32,
        todo_id: i32,
        mut req: UpdateTodoRequest,
    ) -> Result<TodoResponse, AppError> {
        req.sanitize();
        req.validate_input()?;

        self.permission_service
            .check(user_id, workspace_id, PermResource::Todos, PermAction::Edit)
            .await?;

        let existing = todos::Entity::find_by_id(todo_id)
            .filter(todos::Column::WorkspaceId.eq(workspace_id))
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed querying todo: {err}")))?
            .ok_or(AppError::Code(ErrorCode::TodoNotFound))?;

        let old_assignee = existing.assigned_to;
        let existing_status = existing.status.clone();
        let existing_completed_at = existing.completed_at;
        let existing_is_completed = existing.is_completed;

        let mut active: todos::ActiveModel = existing.into();
        if let Some(title) = req.title {
            active.title = Set(title);
        }
        if let Some(desc_opt) = req.description {
            active.description = Set(desc_opt);
        }
        if let Some(due_opt) = req.due_date {
            active.due_date = Set(due_opt);
        }
        if let Some(assignee_opt) = req.assigned_to {
            if let Some(assignee_id) = assignee_opt {
                self.verify_workspace_member(workspace_id, assignee_id)
                    .await?;
            }
            active.assigned_to = Set(assignee_opt);
        }
        if let Some(priority) = req.priority {
            active.priority = Set(priority.as_str().to_string());
        }

        // Synchronize `status`, `is_completed`, and `completed_at`.
        match (req.status, req.is_completed) {
            (Some(new_status), Some(completed_flag)) => {
                active.status = Set(new_status.as_str().to_string());
                active.is_completed = Set(completed_flag);
                active.completed_at = Set(if completed_flag {
                    existing_completed_at.or_else(|| Some(Utc::now().into()))
                } else {
                    None
                });
            }
            (Some(new_status), None) => {
                let completed_flag = new_status == TodoStatus::Done;
                active.status = Set(new_status.as_str().to_string());
                active.is_completed = Set(completed_flag);
                active.completed_at = Set(if completed_flag {
                    existing_completed_at.or_else(|| Some(Utc::now().into()))
                } else {
                    None
                });
            }
            (None, Some(completed_flag)) => {
                active.is_completed = Set(completed_flag);
                if completed_flag {
                    if existing_status != TodoStatus::Done.as_str() {
                        active.status = Set(TodoStatus::Done.as_str().to_string());
                    }
                    active.completed_at =
                        Set(existing_completed_at.or_else(|| Some(Utc::now().into())));
                } else {
                    if existing_status == TodoStatus::Done.as_str() {
                        active.status = Set(TodoStatus::Todo.as_str().to_string());
                    }
                    active.completed_at = Set(None);
                }
            }
            (None, None) => {
                active.is_completed = Set(existing_is_completed);
            }
        }

        active.updated_at = Set(Utc::now().into());

        let updated = active
            .update(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed updating todo: {err}")))?;

        self.invalidate_and_notify(workspace_id, updated.id).await;

        if let Some(new_assignee) = updated.assigned_to {
            if Some(new_assignee) != old_assignee && new_assignee != user_id {
                let _ = self
                    .notification_service
                    .create_notification(
                        new_assignee,
                        Some(workspace_id),
                        "todo_assigned",
                        "Task assigned to you",
                        &format!("You were assigned '{}'.", updated.title),
                        Some("/todos".to_string()),
                    )
                    .await;
            }
        }

        self.enrich_single_todo(updated).await
    }

    /// Deletes a Todo item in `workspace_id`, removes any R2 attachment, and broadcasts SSE update.
    pub async fn delete_todo(
        &self,
        user_id: i32,
        workspace_id: i32,
        todo_id: i32,
    ) -> Result<(), AppError> {
        self.permission_service
            .check(
                user_id,
                workspace_id,
                PermResource::Todos,
                PermAction::Delete,
            )
            .await?;

        let existing = todos::Entity::find_by_id(todo_id)
            .filter(todos::Column::WorkspaceId.eq(workspace_id))
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed querying todo to delete: {err}")))?
            .ok_or(AppError::Code(ErrorCode::TodoNotFound))?;

        todos::Entity::delete_by_id(existing.id)
            .exec(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed deleting todo: {err}")))?;

        self.media_service
            .delete_media(
                UploadPurpose::TodoAttachment.entity_type(),
                todo_id,
                DEFAULT_MEDIA_FIELD,
            )
            .await?;

        self.invalidate_and_notify(workspace_id, todo_id).await;
        Ok(())
    }

    /// Updates sequential `position` values for `req.ordered_ids` inside `workspace_id`,
    /// keeping omitted workspace todos trailing cleanly in their prior relative order.
    pub async fn reorder_todos(
        &self,
        user_id: i32,
        workspace_id: i32,
        req: ReorderTodosRequest,
    ) -> Result<(), AppError> {
        self.permission_service
            .check(user_id, workspace_id, PermResource::Todos, PermAction::Edit)
            .await?;

        let mut seen = HashSet::with_capacity(req.ordered_ids.len());
        let mut unique_ordered_ids = Vec::with_capacity(req.ordered_ids.len());
        for id in req.ordered_ids {
            if seen.insert(id) {
                unique_ordered_ids.push(id);
            }
        }

        if unique_ordered_ids.is_empty() {
            return Ok(());
        }

        // Load every todo in the workspace in display order so omitted items can be
        // shifted cleanly behind `unique_ordered_ids` without colliding on `position = 1`.
        let existing_rows = todos::Entity::find()
            .filter(todos::Column::WorkspaceId.eq(workspace_id))
            .order_by_asc(todos::Column::Position)
            .order_by_asc(todos::Column::Id)
            .all(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed loading todos to reorder: {err}")))?;

        let ordered_existing_ids: Vec<i32> = existing_rows.iter().map(|r| r.id).collect();
        let mut row_map: HashMap<i32, todos::Model> =
            existing_rows.into_iter().map(|r| (r.id, r)).collect();

        for id in &unique_ordered_ids {
            if !row_map.contains_key(id) {
                return Err(AppError::Code(ErrorCode::TodoNotFound));
            }
        }

        let mut final_order = unique_ordered_ids;
        for id in ordered_existing_ids {
            if !seen.contains(&id) {
                final_order.push(id);
            }
        }

        let txn = self
            .db
            .begin()
            .await
            .map_err(|err| AppError::Internal(format!("Failed starting reorder txn: {err}")))?;

        let now = Utc::now();
        for (idx, todo_id) in final_order.iter().enumerate() {
            let new_position = (idx as i32) + 1;
            if let Some(row) = row_map.remove(todo_id) {
                if row.position != new_position {
                    let mut active: todos::ActiveModel = row.into();
                    active.position = Set(new_position);
                    active.updated_at = Set(now.into());
                    active.update(&txn).await.map_err(|err| {
                        AppError::Internal(format!("Failed updating todo position: {err}"))
                    })?;
                }
            }
        }

        txn.commit()
            .await
            .map_err(|err| AppError::Internal(format!("Failed committing reorder txn: {err}")))?;

        self.invalidate_and_notify(workspace_id, 0).await;
        Ok(())
    }

    // SECTION: INTERNAL HELPERS

    /// Verifies that `candidate_user_id` is either the workspace owner or a member.
    async fn verify_workspace_member(
        &self,
        workspace_id: i32,
        candidate_user_id: i32,
    ) -> Result<(), AppError> {
        let ws = workspaces::Entity::find_by_id(workspace_id)
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed loading workspace: {err}")))?
            .ok_or(AppError::Code(ErrorCode::WorkspaceNotFound))?;

        if ws.owner_id == candidate_user_id {
            return Ok(());
        }

        let is_member = workspace_members::Entity::find()
            .filter(workspace_members::Column::WorkspaceId.eq(workspace_id))
            .filter(workspace_members::Column::UserId.eq(candidate_user_id))
            .one(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed checking assignee membership: {err}")))?
            .is_some();

        if is_member {
            Ok(())
        } else {
            Err(AppError::Code(ErrorCode::AssigneeNotInWorkspace))
        }
    }

    /// Loads all Todos for `workspace_id` with creator/assignee names and attachment keys.
    async fn load_workspace_todos_from_db(
        &self,
        workspace_id: i32,
    ) -> Result<Vec<TodoResponse>, AppError> {
        let rows = todos::Entity::find()
            .filter(todos::Column::WorkspaceId.eq(workspace_id))
            .order_by_asc(todos::Column::Position)
            .order_by_asc(todos::Column::Id)
            .all(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed loading workspace todos: {err}")))?;

        if rows.is_empty() {
            return Ok(Vec::new());
        }

        let mut user_ids: HashSet<i32> = HashSet::new();
        let mut todo_ids: Vec<i32> = Vec::with_capacity(rows.len());
        for row in &rows {
            todo_ids.push(row.id);
            user_ids.insert(row.created_by);
            if let Some(assignee) = row.assigned_to {
                user_ids.insert(assignee);
            }
        }

        let name_map: HashMap<i32, String> = users::Entity::find()
            .filter(users::Column::Id.is_in(user_ids.into_iter().collect::<Vec<_>>()))
            .all(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed loading todo user names: {err}")))?
            .into_iter()
            .map(|u| (u.id, u.name))
            .collect();

        let attachments = self
            .media_service
            .get_media_batch(
                UploadPurpose::TodoAttachment.entity_type(),
                &todo_ids,
                DEFAULT_MEDIA_FIELD,
            )
            .await?;

        let items = rows
            .into_iter()
            .map(|row| {
                let created_by_name = name_map
                    .get(&row.created_by)
                    .cloned()
                    .unwrap_or_else(|| "Unknown".to_string());
                let assigned_to_name = row
                    .assigned_to
                    .and_then(|aid| name_map.get(&aid).cloned());
                let attachment_key = attachments.get(&row.id).map(|m| m.xor_key.clone());

                map_todo_response(&row, created_by_name, assigned_to_name, attachment_key)
            })
            .collect();

        Ok(items)
    }

    /// Enriches a single `todos::Model` with creator/assignee names and optional attachment key.
    async fn enrich_single_todo(&self, model: todos::Model) -> Result<TodoResponse, AppError> {
        let mut uids = vec![model.created_by];
        if let Some(aid) = model.assigned_to {
            uids.push(aid);
        }

        let name_map: HashMap<i32, String> = users::Entity::find()
            .filter(users::Column::Id.is_in(uids))
            .all(&self.db)
            .await
            .map_err(|err| AppError::Internal(format!("Failed loading todo user names: {err}")))?
            .into_iter()
            .map(|u| (u.id, u.name))
            .collect();

        let attachment = self
            .media_service
            .get_media(
                UploadPurpose::TodoAttachment.entity_type(),
                model.id,
                DEFAULT_MEDIA_FIELD,
            )
            .await?;

        let created_by_name = name_map
            .get(&model.created_by)
            .cloned()
            .unwrap_or_else(|| "Unknown".to_string());
        let assigned_to_name = model
            .assigned_to
            .and_then(|aid| name_map.get(&aid).cloned());

        Ok(map_todo_response(
            &model,
            created_by_name,
            assigned_to_name,
            attachment.map(|m| m.xor_key),
        ))
    }

    /// Evicts cached Todos and workspace stats, then emits `RealtimeEvent::TodoChanged`
    /// to every user belonging to `workspace_id`.
    async fn invalidate_and_notify(&self, workspace_id: i32, todo_id: i32) {
        self.cache.invalidate_todos(workspace_id).await;

        let mut recipients: HashSet<i32> = HashSet::new();
        if let Ok(Some(ws)) = workspaces::Entity::find_by_id(workspace_id)
            .one(&self.db)
            .await
        {
            recipients.insert(ws.owner_id);
        }
        if let Ok(members) = workspace_members::Entity::find()
            .filter(workspace_members::Column::WorkspaceId.eq(workspace_id))
            .all(&self.db)
            .await
        {
            for m in members {
                recipients.insert(m.user_id);
            }
        }

        for uid in recipients {
            self.cache.invalidate_workspace_list(uid).await;
            self.notification_hub.send_to_user(
                uid,
                RealtimeEvent::TodoChanged {
                    workspace_id,
                    todo_id,
                },
            );
        }
    }
}

/// Computes `TodoSummaryStats` across all workspace Todos.
fn compute_stats(items: &[TodoResponse]) -> TodoSummaryStats {
    let mut stats = TodoSummaryStats {
        total: items.len() as i64,
        ..Default::default()
    };

    for item in items {
        match item.status {
            TodoStatus::Todo => stats.todo_count += 1,
            TodoStatus::InProgress => stats.in_progress_count += 1,
            TodoStatus::Done => stats.done_count += 1,
        }
        if item.priority == TodoPriority::Urgent {
            stats.urgent_count += 1;
        }
    }

    stats
}
