// =============================================================================
// INTEGRATION TESTS: NOTIFICATIONS, GLOBAL ANNOUNCEMENTS & OPTIONAL R2 STORAGE
// =============================================================================
//
// Verifies per-user notification creation, read isolation, global announcement
// audience scoping (`all`, `authenticated`, `admins`), SSE broadcasts, and
// optional Cloudflare R2 `StorageNotConfigured` fallback vs configured presign/meta.

mod common;

// SECTION: IMPORTS

use common::TestApp;
use shared::domain::admin::{CreateGlobalNotificationRequest, UpdateGlobalNotificationRequest};
use shared::domain::error_code::ErrorCode;
use shared::domain::media::{ConfirmUploadRequest, PresignUploadRequest};
use shared::domain::notification::{GlobalNotificationAudience, RealtimeEvent};
use shared::domain::todo::TodoListQuery;
use shared::domain::upload::UploadPurpose;
use shared::domain::user::UserRole;

// SECTION: NOTIFICATIONS & GLOBAL ANNOUNCEMENT TESTS

#[tokio::test]
async fn notifications_and_global_announcements_audience_filtering() {
    let app = TestApp::new().await;
    let admin = app.create_admin("admin-notice@example.com", "Admin").await;
    let user = app.create_user("user-notice@example.com", "User").await;

    let mut sse_rx = app.state.notification_hub.subscribe();

    // Create personal notification for `user` and confirm SSE emission.
    let personal = app
        .state
        .notification_service
        .create_notification(
            user.id,
            user.active_workspace_id,
            "info",
            "Welcome",
            "Your account is ready.",
            Some("/todos".to_string()),
        )
        .await
        .expect("create personal notification");

    let env1 = sse_rx.recv().await.expect("receive personal SSE");
    assert_eq!(env1.target_user_id, Some(user.id));
    assert!(matches!(env1.event, RealtimeEvent::UserNotification(_)));

    // Another user (`admin`) cannot mark `user`'s personal notification read (`NotificationNotFound`).
    let cross_read = app
        .state
        .notification_service
        .mark_read(admin.id, personal.id)
        .await
        .unwrap_err();
    assert_eq!(cross_read.code(), ErrorCode::NotificationNotFound);

    // Admin publishes two global announcements: `All` and `Admins`.
    let public_notice = app
        .state
        .admin_service
        .create_global_notification(
            admin.id,
            CreateGlobalNotificationRequest {
                title: "Maintenance Window".to_string(),
                html_content: "<p>Planned upgrade tonight.</p>".to_string(),
                target_audience: GlobalNotificationAudience::All,
                is_active: Some(true),
            },
        )
        .await
        .expect("create public global notice");

    let env2 = sse_rx.recv().await.expect("receive global SSE");
    assert_eq!(env2.target_user_id, None);
    assert!(matches!(env2.event, RealtimeEvent::GlobalAnnouncement(_)));

    let admin_notice = app
        .state
        .admin_service
        .create_global_notification(
            admin.id,
            CreateGlobalNotificationRequest {
                title: "Admin Audit".to_string(),
                html_content: "<b>Staff only notice</b>".to_string(),
                target_audience: GlobalNotificationAudience::Admins,
                is_active: Some(true),
            },
        )
        .await
        .expect("create admin-only global notice");

    // Guest sees only `All` notice.
    let guest_feed = app
        .state
        .notification_service
        .get_feed(None, None)
        .await
        .expect("guest feed");
    assert_eq!(guest_feed.global_items.len(), 1);
    assert_eq!(guest_feed.global_items[0].id, public_notice.id);

    // Regular user sees personal + `All` notice (not `Admins` notice).
    let user_feed = app
        .state
        .notification_service
        .list_for_user(user.id, UserRole::User)
        .await
        .expect("user feed");
    assert_eq!(user_feed.items.len(), 1);
    assert_eq!(user_feed.global_items.len(), 1);
    assert_eq!(user_feed.unread_count, 2);

    // Admin sees both global notices (`All` + `Admins`).
    let admin_feed = app
        .state
        .notification_service
        .list_for_user(admin.id, UserRole::Admin)
        .await
        .expect("admin feed");
    assert_eq!(admin_feed.global_items.len(), 2);
    assert!(admin_feed
        .global_items
        .iter()
        .any(|g| g.id == admin_notice.id));

    // Mark all read for `user` -> unread count drops to 0.
    app.state
        .notification_service
        .mark_all_read(user.id, UserRole::User)
        .await
        .expect("mark all read");
    assert_eq!(
        app.state
            .notification_service
            .count_unread(user.id, UserRole::User)
            .await
            .expect("unread count"),
        0
    );

    // Admin can toggle & delete global notification.
    let toggled = app
        .state
        .admin_service
        .update_global_notification(
            admin_notice.id,
            UpdateGlobalNotificationRequest {
                title: None,
                html_content: None,
                target_audience: None,
                is_active: Some(false),
            },
        )
        .await
        .expect("deactivate global notice");
    assert!(!toggled.is_active);

    app.state
        .admin_service
        .delete_global_notification(admin_notice.id)
        .await
        .expect("delete global notice");
}

// SECTION: OPTIONAL R2 STORAGE FALLBACK & CONFIGURED MEDIA FLOW

#[tokio::test]
async fn optional_r2_storage_disabled_vs_configured_media_keys() {
    // 1. Default TestApp (`r2: None`) -> `StorageService` disabled, `presign_upload` returns `StorageNotConfigured`.
    let disabled_app = TestApp::new().await;
    assert!(disabled_app.state.storage.is_none());
    assert!(!disabled_app.state.media_service.is_storage_enabled());

    let err = disabled_app
        .state
        .media_service
        .presign_upload(PresignUploadRequest {
            purpose: UploadPurpose::TodoAttachment,
            entity_id: 1,
            content_type: "image/webp".to_string(),
            file_size: 2048,
        })
        .await
        .unwrap_err();
    assert_eq!(err.code(), ErrorCode::StorageNotConfigured);

    // 2. Configured TestApp (`mock_r2_config`) -> `presign_upload` + `confirm_upload` + batch attachment key resolution.
    let enabled_app = TestApp::with_settings(|s| {
        s.r2 = Some(TestApp::mock_r2_config());
    })
    .await;
    assert!(enabled_app.state.storage.is_some());
    assert!(enabled_app.state.media_service.is_storage_enabled());

    let user = enabled_app
        .create_user("media@example.com", "Media User")
        .await;
    let ws_id = user.active_workspace_id.unwrap();
    let (todos_payload, _) = enabled_app
        .state
        .todo_service
        .list_todos(user.id, ws_id, TodoListQuery::default())
        .await
        .expect("list starter todos");
    let target_todo_id = todos_payload.items[0].id;

    let presign = enabled_app
        .state
        .media_service
        .presign_upload(PresignUploadRequest {
            purpose: UploadPurpose::TodoAttachment,
            entity_id: target_todo_id,
            content_type: "image/webp".to_string(),
            file_size: 4096,
        })
        .await
        .expect("presign with configured R2");
    assert_eq!(presign.xor_key.len(), 64);
    assert!(presign.upload_url.contains(&presign.storage_key));

    let confirmed = enabled_app
        .state
        .media_service
        .confirm_upload(
            user.id,
            ConfirmUploadRequest {
                purpose: UploadPurpose::TodoAttachment,
                entity_id: target_todo_id,
                field_name: None,
                storage_key: presign.storage_key.clone(),
                xor_key: presign.xor_key.clone(),
                content_type: "image/webp".to_string(),
                file_size: 4096,
            },
        )
        .await
        .expect("confirm upload");
    assert_eq!(
        confirmed.public_url,
        format!("https://cdn.test.local/{}", presign.storage_key)
    );

    // Cache invalidation ensures TodoResponse exposes `attachment_key = Some(xor_key)`.
    enabled_app.state.cache.invalidate_todos(ws_id).await;
    let fetched = enabled_app
        .state
        .todo_service
        .get_todo(user.id, ws_id, target_todo_id)
        .await
        .expect("fetch todo with attachment key");
    assert_eq!(fetched.attachment_key.as_deref(), Some(presign.xor_key.as_str()));
}
