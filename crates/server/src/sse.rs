// =============================================================================
// SERVER-SENT EVENTS (SSE) STREAM HANDLER
// =============================================================================
//
// Streams real-time user notifications, workspace todo updates, and global
// announcements over `GET /api/notifications/sse` using `BroadcastStream`.

use std::convert::Infallible;
use std::time::Duration;

// `axum` - SSE response types and keep-alive heartbeats.
use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};

// `futures::stream` - Adapts `BroadcastStream` to a filtered `Stream`.
use futures::stream::{self, Stream};

// `tokio_stream` - Async broadcast receiver stream wrapper.
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

// `tracing` - Logs client lag warnings cleanly.
use tracing::warn;

use api::extractors::OptionalAuthUser;
use services::state::AppState;
use shared::domain::notification::{GlobalNotificationAudience, RealtimeEvent};
use shared::domain::user::UserRole;

/// Streams filtered real-time SSE events for personal notifications, todos, and announcements.
pub async fn notifications_sse_handler(
    State(state): State<AppState>,
    OptionalAuthUser(maybe_auth): OptionalAuthUser,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let viewer_user_id = maybe_auth.as_ref().map(|a| a.user.id);
    let viewer_role = maybe_auth
        .as_ref()
        .map(|a| UserRole::from_db(&a.user.role));

    let rx = state.notification_hub.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(move |msg| match msg {
        Ok(envelope) => {
            if let Some(target_uid) = envelope.target_user_id {
                if Some(target_uid) != viewer_user_id {
                    return None;
                }
            }

            if let RealtimeEvent::GlobalAnnouncement(ref ga) = envelope.event {
                let allowed = match ga.target_audience {
                    GlobalNotificationAudience::All => true,
                    GlobalNotificationAudience::Authenticated => viewer_user_id.is_some(),
                    GlobalNotificationAudience::Admins => viewer_role == Some(UserRole::Admin),
                };
                if !allowed {
                    return None;
                }
            }

            let event_name = match &envelope.event {
                RealtimeEvent::UserNotification(_) => "user_notification",
                RealtimeEvent::GlobalAnnouncement(_) => "global_announcement",
                RealtimeEvent::TodoChanged { .. } => "todo_changed",
                RealtimeEvent::KeepAlive => "keep_alive",
            };

            match serde_json::to_string(&envelope.event) {
                Ok(json) => Some(Ok(Event::default().event(event_name).data(json))),
                Err(_) => None,
            }
        }
        Err(err) => {
            warn!(error = %err, "SSE subscriber lagged behind notification hub");
            None
        }
    });

    let initial = stream::once(async {
        Ok::<Event, Infallible>(Event::default().event("connected").data("{}"))
    });

    Sse::new(initial.chain(stream)).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(20))
            .text("keepalive"),
    )
}
