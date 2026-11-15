// =============================================================================
// TODO DOMAIN DTOs
// =============================================================================
//
// Defines Todo workflow status/priority enums, list filter queries, summary
// counters, create/patch requests (`deserialize_double_option`), and reordering.

use std::collections::HashMap;

// `chrono` - Calendar `NaiveDate` due dates and UTC completion timestamps.
use chrono::{DateTime, NaiveDate, Utc};

// `serde` - JSON payloads for Todo API endpoints.
use serde::{Deserialize, Serialize};

use crate::domain::error::AppError;
use crate::domain::error_code::ErrorCode;
use crate::domain::request::{
    deserialize_double_option, normalize_whitespace, sanitize_double_opt, sanitize_opt, Sanitize,
    SortOrder,
};
use crate::domain::validator::finish_validation;

/// Todo workflow column status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TodoStatus {
    #[default]
    Todo,
    InProgress,
    Done,
}

impl TodoStatus {
    /// Parses stored DB string into `TodoStatus`.
    pub fn from_db(val: &str) -> Self {
        match val {
            "in_progress" => Self::InProgress,
            "done" => Self::Done,
            _ => Self::Todo,
        }
    }

    /// Canonical snake_case DB string.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Todo => "todo",
            Self::InProgress => "in_progress",
            Self::Done => "done",
        }
    }
}

/// Todo priority tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TodoPriority {
    Low,
    #[default]
    Medium,
    High,
    Urgent,
}

impl TodoPriority {
    /// Parses stored DB string into `TodoPriority`.
    pub fn from_db(val: &str) -> Self {
        match val {
            "low" => Self::Low,
            "high" => Self::High,
            "urgent" => Self::Urgent,
            _ => Self::Medium,
        }
    }

    /// Canonical snake_case DB string.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Urgent => "urgent",
        }
    }
}

/// Todo response item returned to Svelte UI (includes optional `attachment_key`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoResponse {
    pub id: i32,
    pub workspace_id: i32,
    pub created_by: i32,
    pub created_by_name: String,
    pub assigned_to: Option<i32>,
    pub assigned_to_name: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub status: TodoStatus,
    pub priority: TodoPriority,
    pub due_date: Option<NaiveDate>,
    pub is_completed: bool,
    pub completed_at: Option<DateTime<Utc>>,
    pub position: i32,
    pub attachment_key: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Aggregate counters for the active workspace Todo dashboard.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TodoSummaryStats {
    pub total: i64,
    pub todo_count: i64,
    pub in_progress_count: i64,
    pub done_count: i64,
    pub urgent_count: i64,
}

/// Combined list + summary response payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoListPayload {
    pub items: Vec<TodoResponse>,
    pub stats: TodoSummaryStats,
}

/// Query filter params for `GET /api/v1/workspaces/:workspace_id/todos`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TodoListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub search: Option<String>,
    pub status: Option<TodoStatus>,
    pub priority: Option<TodoPriority>,
    pub assigned_to: Option<i32>,
    pub sort_order: Option<SortOrder>,
}

/// Request payload to create a new Todo item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTodoRequest {
    pub title: String,
    pub description: Option<String>,
    pub status: Option<TodoStatus>,
    pub priority: Option<TodoPriority>,
    pub due_date: Option<NaiveDate>,
    pub assigned_to: Option<i32>,
}

impl Sanitize for CreateTodoRequest {
    fn sanitize(&mut self) {
        self.title = normalize_whitespace(&self.title);
        sanitize_opt(&mut self.description);
    }
}

impl CreateTodoRequest {
    /// Validates required title length (`1..=200`).
    pub fn validate_input(&self) -> Result<(), AppError> {
        let mut errors = HashMap::new();
        if self.title.is_empty() {
            errors.insert("title".to_string(), ErrorCode::FieldRequired);
        } else if self.title.chars().count() > 200 {
            errors.insert("title".to_string(), ErrorCode::TodoTitleTooLong);
        }
        finish_validation(errors)
    }
}

/// Patch request supporting explicit `null` clearing on nullable columns.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateTodoRequest {
    pub title: Option<String>,
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub description: Option<Option<String>>,
    pub status: Option<TodoStatus>,
    pub priority: Option<TodoPriority>,
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub due_date: Option<Option<NaiveDate>>,
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub assigned_to: Option<Option<i32>>,
    pub is_completed: Option<bool>,
}

impl Sanitize for UpdateTodoRequest {
    fn sanitize(&mut self) {
        if let Some(ref mut title) = self.title {
            *title = normalize_whitespace(title);
        }
        sanitize_double_opt(&mut self.description);
    }
}

impl UpdateTodoRequest {
    /// Validates non-empty title length if provided.
    pub fn validate_input(&self) -> Result<(), AppError> {
        let mut errors = HashMap::new();
        if let Some(ref title) = self.title {
            if title.is_empty() {
                errors.insert("title".to_string(), ErrorCode::FieldRequired);
            } else if title.chars().count() > 200 {
                errors.insert("title".to_string(), ErrorCode::TodoTitleTooLong);
            }
        }
        finish_validation(errors)
    }
}

/// Request payload to reorder Todo positions inside a workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReorderTodosRequest {
    pub ordered_ids: Vec<i32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_todo_supports_double_option_null_clear() {
        let payload: UpdateTodoRequest = serde_json::from_str(
            r#"{"description": null, "due_date": "2026-12-01"}"#,
        )
        .unwrap();
        assert_eq!(payload.description, Some(None));
        assert!(matches!(payload.due_date, Some(Some(_))));
        assert_eq!(payload.assigned_to, None);
    }
}
