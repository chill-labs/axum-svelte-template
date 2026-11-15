// =============================================================================
// REACTIVE TOAST NOTIFICATION STORE
// =============================================================================

export type ToastKind = 'success' | 'error' | 'info' | 'warning';

export interface ToastItem {
  id: string;
  kind: ToastKind;
  message: string;
}

class ToastStore {
  items = $state<ToastItem[]>([]);

  push(kind: ToastKind, message: string, durationMs = 4000) {
    const id = `${Date.now()}-${Math.random().toString(36).slice(2, 7)}`;
    this.items = [...this.items, { id, kind, message }];
    if (durationMs > 0) {
      setTimeout(() => this.dismiss(id), durationMs);
    }
  }

  success(message: string) {
    this.push('success', message);
  }

  error(message: string) {
    this.push('error', message, 5500);
  }

  info(message: string) {
    this.push('info', message);
  }

  dismiss(id: string) {
    this.items = this.items.filter((item) => item.id !== id);
  }
}

export const toasts = new ToastStore();
