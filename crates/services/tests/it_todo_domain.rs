// =============================================================================
// INTEGRATION TESTS: TODO CRUD, DOUBLE-OPTION PATCH, FILTERING & REORDER
// =============================================================================
//
// Verifies workspace-scoped Todo creation, assignee membership validation,
// `deserialize_double_option` explicit `null` clearing (`UpdateTodoRequest`),
// status/completion synchronization, search/filter stats, drag-and-drop
// reordering, and SSE `TodoChanged` delivery.

mod common;

// SECTION: IMPORTS

use chrono::NaiveDate;
use common::TestApp;
use shared::domain::error::AppError;
use shared::domain::error_code::ErrorCode;
use shared::domain::notification::RealtimeEvent;
use shared::domain::request::SortOrder;
use shared::domain::todo::{
    CreateTodoRequest, ReorderTodosRequest, TodoListQuery, TodoPriority, TodoStatus,
    UpdateTodoRequest,
};

// SECTION: TODO CRUD, DOUBLE-OPTION & REORDER TESTS

#[tokio::test]
async fn todo_crud_double_option_patch_and_assignee_isolation() {
    let app = TestApp::new().await;
    let owner = app.create_user("todo-owner@example.com", "Todo Owner").await;
    let outsider = app.create_user("stranger@example.com", "Stranger").await;
    let ws_id = owner
        .active_workspace_id
        .expect("owner active workspace");

    let mut sse_rx = app.state.notification_hub.subscribe();

    // Cross-tenant assignee rejected (`AssigneeNotInWorkspace`).
    let bad_assignee = app
        .state
        .todo_service
        .create_todo(
            owner.id,
            ws_id,
            CreateTodoRequest {
                title: "Assign outsider".to_string(),
                description: None,
                status: None,
                priority: None,
                due_date: None,
                assigned_to: Some(outsider.id),
            },
        )
        .await
        .unwrap_err();
    assert_eq!(bad_assignee.code(), ErrorCode::AssigneeNotInWorkspace);

    // Over-length title returns typed validation field error (`TodoTitleTooLong`).
    let long_title = "x".repeat(201);
    let len_err = app
        .state
        .todo_service
        .create_todo(
            owner.id,
            ws_id,
            CreateTodoRequest {
                title: long_title,
                description: None,
                status: None,
                priority: None,
                due_date: None,
                assigned_to: None,
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(len_err, AppError::FieldErrors(_)));

    // Create Todo with description, due_date, and assignee.
    let due = NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();
    let created = app
        .state
        .todo_service
        .create_todo(
            owner.id,
            ws_id,
            CreateTodoRequest {
                title: "  Ship   Phase 2  ".to_string(),
                description: Some("Verify sea-orm services".to_string()),
                status: Some(TodoStatus::InProgress),
                priority: Some(TodoPriority::Urgent),
                due_date: Some(due),
                assigned_to: Some(owner.id),
            },
        )
        .await
        .expect("create todo");
    assert_eq!(created.title, "Ship Phase 2");
    assert_eq!(created.due_date, Some(due));
    assert_eq!(created.assigned_to, Some(owner.id));
    assert!(!created.is_completed);

    let sse_msg = sse_rx.recv().await.expect("receive TodoChanged SSE");
    assert_eq!(sse_msg.target_user_id, Some(owner.id));
    assert!(matches!(
        sse_msg.event,
        RealtimeEvent::TodoChanged { workspace_id, todo_id }
            if workspace_id == ws_id && todo_id == created.id
    ));

    // Patch using JSON `deserialize_double_option`: omit `due_date`, explicitly clear `description` & `assigned_to`.
    let patch: UpdateTodoRequest = serde_json::from_str(
        r#"{"description": null, "assigned_to": null, "is_completed": true}"#,
    )
    .expect("deserialize double option patch");
    assert_eq!(patch.description, Some(None));
    assert_eq!(patch.assigned_to, Some(None));
    assert_eq!(patch.due_date, None);

    let updated = app
        .state
        .todo_service
        .update_todo(owner.id, ws_id, created.id, patch)
        .await
        .expect("apply double option patch");

    assert_eq!(updated.description, None);
    assert_eq!(updated.assigned_to, None);
    assert_eq!(updated.due_date, Some(due)); // Omitted -> preserved!
    assert_eq!(updated.status, TodoStatus::Done);
    assert!(updated.is_completed);
    assert!(updated.completed_at.is_some());

    // Cross-workspace isolation: outsider's workspace cannot access owner's todo.
    let outsider_ws = outsider.active_workspace_id.unwrap();
    let iso_err = app
        .state
        .todo_service
        .get_todo(outsider.id, outsider_ws, created.id)
        .await
        .unwrap_err();
    assert_eq!(iso_err.code(), ErrorCode::TodoNotFound);
}

#[tokio::test]
async fn todo_filtering_stats_pagination_and_reorder() {
    let app = TestApp::new().await;
    let owner = app.create_user("order@example.com", "Order Owner").await;
    let ws_id = owner.active_workspace_id.unwrap();

    let t1 = app
        .state
        .todo_service
        .create_todo(
            owner.id,
            ws_id,
            CreateTodoRequest {
                title: "Alpha Urgent".to_string(),
                description: None,
                status: Some(TodoStatus::Todo),
                priority: Some(TodoPriority::Urgent),
                due_date: None,
                assigned_to: None,
            },
        )
        .await
        .expect("create t1");

    let t2 = app
        .state
        .todo_service
        .create_todo(
            owner.id,
            ws_id,
            CreateTodoRequest {
                title: "Beta Low".to_string(),
                description: Some("Keyword search target".to_string()),
                status: Some(TodoStatus::Todo),
                priority: Some(TodoPriority::Low),
                due_date: None,
                assigned_to: None,
            },
        )
        .await
        .expect("create t2");

    // Reorder `t2` ahead of `t1`.
    app.state
        .todo_service
        .reorder_todos(
            owner.id,
            ws_id,
            ReorderTodosRequest {
                ordered_ids: vec![t2.id, t1.id],
            },
        )
        .await
        .expect("reorder todos");

    let (filtered_payload, page_meta) = app
        .state
        .todo_service
        .list_todos(
            owner.id,
            ws_id,
            TodoListQuery {
                search: Some("keyword".to_string()),
                status: Some(TodoStatus::Todo),
                priority: None,
                assigned_to: None,
                page: Some(1),
                page_size: Some(10),
                sort_order: Some(SortOrder::Asc),
            },
        )
        .await
        .expect("filtered search");
    assert_eq!(page_meta.total_items, 1);
    assert_eq!(filtered_payload.items[0].id, t2.id);
    // Workspace summary stats cover all 7 tasks (5 starter + 2 created).
    assert_eq!(filtered_payload.stats.total, 7);
    assert_eq!(filtered_payload.stats.urgent_count, 1);

    // Delete t1 and confirm total drops to 6.
    app.state
        .todo_service
        .delete_todo(owner.id, ws_id, t1.id)
        .await
        .expect("delete t1");
    let (after_delete, _) = app
        .state
        .todo_service
        .list_todos(owner.id, ws_id, TodoListQuery::default())
        .await
        .expect("list after delete");
    assert_eq!(after_delete.stats.total, 6);
    assert_eq!(after_delete.items[0].id, t2.id);
}
