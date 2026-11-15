// =============================================================================
// REAL-TIME SSE CLIENT STORE
// =============================================================================
//
// Connects to `/api/notifications/sse` (including `?_hs=1` in Hot Support
// iframes), increments unread badges, pushes toast notifications, and notifies
// active workspace views whenever a collaborator mutates a Todo.

import { browser } from '$app/env';
import type { GlobalAnnouncementResponse, NotificationResponse } from '#lib/api/types.generated.ts';
import { isHotSupportActive } from '#lib/api/client.ts';
import { sessionStore } from '#lib/state/session.svelte.ts';
import { toasts } from '#lib/state/toast.svelte.ts';

type TodoChangedListener = (payload: { workspace_id: number; todo_id: number }) => void;
type NotificationListener = () => void;

class SseClient {
  private source: EventSource | null = null;
  private todoListeners = new Set<TodoChangedListener>();
  private notificationListeners = new Set<NotificationListener>();
  connected = $state(false);

  connect() {
    if (!browser || this.source) return;
    const url = isHotSupportActive()
      ? '/api/notifications/sse?_hs=1'
      : '/api/notifications/sse';

    const es = new EventSource(url);
    this.source = es;

    es.addEventListener('connected', () => {
      this.connected = true;
    });

    es.addEventListener('user_notification', (ev) => {
      try {
        const parsed = JSON.parse(ev.data) as {
          event: 'user_notification';
          payload: NotificationResponse;
        };
        sessionStore.incrementUnread();
        toasts.info(parsed.payload.title);
        for (const cb of this.notificationListeners) cb();
      } catch {
        // Ignore malformed frame
      }
    });

    es.addEventListener('global_announcement', (ev) => {
      try {
        const parsed = JSON.parse(ev.data) as {
          event: 'global_announcement';
          payload: GlobalAnnouncementResponse;
        };
        sessionStore.incrementUnread();
        toasts.info(parsed.payload.title);
        for (const cb of this.notificationListeners) cb();
      } catch {
        // Ignore malformed frame
      }
    });

    es.addEventListener('todo_changed', (ev) => {
      try {
        const parsed = JSON.parse(ev.data) as {
          event: 'todo_changed';
          payload: { workspace_id: number; todo_id: number };
        };
        for (const cb of this.todoListeners) {
          cb(parsed.payload);
        }
      } catch {
        // Ignore malformed frame
      }
    });

    es.onerror = () => {
      this.connected = false;
    };
  }

  disconnect() {
    if (this.source) {
      this.source.close();
      this.source = null;
      this.connected = false;
    }
  }

  onTodoChanged(cb: TodoChangedListener): () => void {
    this.todoListeners.add(cb);
    return () => this.todoListeners.delete(cb);
  }

  onNotification(cb: NotificationListener): () => void {
    this.notificationListeners.add(cb);
    return () => this.notificationListeners.delete(cb);
  }
}

export const sseClient = new SseClient();
