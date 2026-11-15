// =============================================================================
// ROUTES: TODOS CRUD, FILTERS, STATS & REORDERING
// =============================================================================

// `axum` - Extractors and router builder for workspace-scoped Todos.
use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch, post};
use axum::{Json, Router};

use services::state::AppState;
use shared::domain::error::ToResponse;
use shared::domain::response::ApiResponse;
use shared::domain::todo::{
    CreateTodoRequest, ReorderTodosRequest, TodoListQuery, UpdateTodoRequest,
};

use crate::extractors::AuthUser;

/// Mounts `/api/v1/workspaces/{workspace_id}/todos*`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/workspaces/{workspace_id}/todos",
            get(list_todos).post(create_todo),
        )
        .route(
            "/api/v1/workspaces/{workspace_id}/todos/reorder",
            post(reorder_todos),
        )
        .route(
            "/api/v1/workspaces/{workspace_id}/todos/{todo_id}",
            patch(update_todo).delete(delete_todo),
        )
}

/// Lists paginated workspace Todos alongside summary counters (`Todos:View`).
pub async fn list_todos(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
    Path(workspace_id): Path<i32>,
    Query(query): Query<TodoListQuery>,
) -> Response {
    match state
        .todo_service
        .list_todos(auth.user.id, workspace_id, query)
        .await
    {
        Ok((payload, pagination)) => ApiResponse::success_paginated(payload, pagination).into_response(),
        Err(err) => err.into_response(),
    }
}

/// Creates a new Todo item inside `workspace_id` (`Todos:Create`).
pub async fn create_todo(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
    Path(workspace_id): Path<i32>,
    Json(req): Json<CreateTodoRequest>,
) -> Response {
    state
        .todo_service
        .create_todo(auth.user.id, workspace_id, req)
        .await
        .to_response_created()
}

/// Patches an existing Todo (`Todos:Edit`), preserving `deserialize_double_option` clears.
pub async fn update_todo(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
    Path((workspace_id, todo_id)): Path<(i32, i32)>,
    Json(req): Json<UpdateTodoRequest>,
) -> Response {
    state
        .todo_service
        .update_todo(auth.user.id, workspace_id, todo_id, req)
        .await
        .to_response()
}

/// Deletes a Todo item inside `workspace_id` (`Todos:Delete`).
pub async fn delete_todo(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
    Path((workspace_id, todo_id)): Path<(i32, i32)>,
) -> Response {
    state
        .todo_service
        .delete_todo(auth.user.id, workspace_id, todo_id)
        .await
        .map(|()| true)
        .to_response()
}

/// Updates ordered display positions for Todos inside `workspace_id` (`Todos:Edit`).
pub async fn reorder_todos(
    State(state): State<AppState>,
    AuthUser(auth): AuthUser,
    Path(workspace_id): Path<i32>,
    Json(req): Json<ReorderTodosRequest>,
) -> Response {
    state
        .todo_service
        .reorder_todos(auth.user.id, workspace_id, req)
        .await
        .map(|()| true)
        .to_response()
}
