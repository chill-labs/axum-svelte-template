// =============================================================================
// REAL-TIME SSE NOTIFICATION HUB
// =============================================================================
//
// In-process `tokio::sync::broadcast` hub fanning out targeted per-user events
// (`UserNotification`, workspace-scoped `TodoChanged`) and global announcements
// (`GlobalAnnouncement`) to connected `/api/notifications/sse` streams.
//
// ## Key Concepts
//
// - **RealtimeEnvelope**: Pairs an optional `target_user_id` with a serialized
//   `RealtimeEvent`. SSE streams discard envelopes targeted at other users.
// - **Fire-and-Forget**: Sending when zero SSE clients are connected succeeds
//   silently without blocking or failing domain mutations.
//
// ## Architecture Notes
//
// Owned by `AppState` and shared across `TodoService`, `NotificationService`,
// `AdminService`, and the Axum SSE handler in `crates/api`.

// SECTION: IMPORTS

// `tokio` - Multi-producer multi-consumer broadcast channel for SSE fan-out.
// - `broadcast` - Bounded ring buffer supporting multiple concurrent subscribers
use tokio::sync::broadcast;

// `shared` - Typed real-time SSE event enum.
// - `RealtimeEvent` - Enum variants pushed to connected browser `EventSource` clients
use shared::domain::notification::RealtimeEvent;

// SECTION: ENVELOPE & HUB

/// Bounded capacity for pending SSE messages in the broadcast ring buffer.
const HUB_CHANNEL_CAPACITY: usize = 1024;

/// Broadcast envelope carrying optional per-user routing metadata.
#[derive(Debug, Clone)]
pub struct RealtimeEnvelope {
    /// Recipient `user_id` (`Some(id)` for targeted events, `None` for global broadcasts).
    pub target_user_id: Option<i32>,
    /// Typed event payload serialized onto the SSE stream.
    pub event: RealtimeEvent,
}

/// Central publish/subscribe hub for real-time SSE events.
#[derive(Clone, Debug)]
pub struct NotificationHub {
    /// Broadcast sender cloned cheaply across domain services.
    sender: broadcast::Sender<RealtimeEnvelope>,
}

impl Default for NotificationHub {
    fn default() -> Self {
        Self::new()
    }
}

impl NotificationHub {
    /// Creates a new `NotificationHub` backed by a bounded broadcast channel.
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(HUB_CHANNEL_CAPACITY);
        Self { sender }
    }

    /// Subscribes a new SSE connection to receive future `RealtimeEnvelope` events.
    pub fn subscribe(&self) -> broadcast::Receiver<RealtimeEnvelope> {
        self.sender.subscribe()
    }

    /// Emits an event targeted specifically at `user_id`.
    ///
    /// Silently ignores errors when no active SSE clients are subscribed.
    pub fn send_to_user(&self, user_id: i32, event: RealtimeEvent) {
        let _ = self.sender.send(RealtimeEnvelope {
            target_user_id: Some(user_id),
            event,
        });
    }

    /// Broadcasts an event globally to all active SSE subscribers.
    ///
    /// Silently ignores errors when no active SSE clients are subscribed.
    pub fn broadcast_all(&self, event: RealtimeEvent) {
        let _ = self.sender.send(RealtimeEnvelope {
            target_user_id: None,
            event,
        });
    }
}

// SECTION: UNIT TESTS

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn hub_delivers_targeted_and_broadcast_events() {
        let hub = NotificationHub::new();
        let mut rx = hub.subscribe();

        hub.send_to_user(
            12,
            RealtimeEvent::TodoChanged {
                workspace_id: 3,
                todo_id: 99,
            },
        );
        let env1 = rx.recv().await.expect("receive targeted event");
        assert_eq!(env1.target_user_id, Some(12));
        assert!(matches!(
            env1.event,
            RealtimeEvent::TodoChanged {
                workspace_id: 3,
                todo_id: 99
            }
        ));

        hub.broadcast_all(RealtimeEvent::KeepAlive);
        let env2 = rx.recv().await.expect("receive broadcast event");
        assert_eq!(env2.target_user_id, None);
        assert!(matches!(env2.event, RealtimeEvent::KeepAlive));
    }
}
