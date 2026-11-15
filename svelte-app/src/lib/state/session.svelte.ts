// =============================================================================
// REACTIVE SESSION & RBAC STORE
// =============================================================================
//
// Hydrates immediately from `window.__APP_BOOTSTRAP__` (zero initial flicker),
// exposes workspace RBAC `can(resource, action)` checks, manages `LoginGateModal`,
// and stays synced on workspace switches and onboarding updates.

import { browser } from '$app/env';
import { goto } from '$app/navigation';
import { API_ENDPOINTS, apiRequest, isHotSupportActive } from '#lib/api/client.ts';
import type {
  PermAction,
  PermResource,
  SessionBootstrapData
} from '#lib/api/types.generated.ts';
import { ROUTES } from '#lib/config/routes.ts';
import { uiPrefs } from '#lib/state/theme.svelte.ts';
import { toasts } from '#lib/state/toast.svelte.ts';

function initialSession(): SessionBootstrapData | null {
  if (!browser) return null;
  return window.__APP_BOOTSTRAP__?.session ?? null;
}

class SessionStore {
  data = $state<SessionBootstrapData | null>(initialSession());
  loginGateOpen = $state(false);
  r2PublicUrl = $state(browser ? (window.__APP_BOOTSTRAP__?.r2_public_url ?? '') : '');
  loading = $state(false);

  user = $derived(this.data?.user ?? null);
  isAuthenticated = $derived(this.data !== null);
  isHotSupport = $derived(Boolean(this.data?.is_hot_support) || isHotSupportActive());
  isAdmin = $derived(this.data?.user.role === 'admin' && !this.isHotSupport);
  activeWorkspaceId = $derived(this.data?.active_workspace_id ?? null);
  activeWorkspaceName = $derived(this.data?.active_workspace_name ?? null);
  activeWorkspaceRole = $derived(this.data?.active_workspace_role ?? null);
  unreadCount = $derived(this.data?.unread_notifications ?? 0);

  can(resource: PermResource, action: PermAction): boolean {
    const perms = this.data?.permissions;
    if (!perms) return false;
    return perms.some((p) => p.resource === resource && p.action === action);
  }

  openLoginGate() {
    if (browser) {
      localStorage.setItem('return_to', window.location.pathname + window.location.search);
    }
    this.loginGateOpen = true;
  }

  closeLoginGate() {
    this.loginGateOpen = false;
  }

  requireAuth(callback?: () => void): boolean {
    if (this.isAuthenticated) {
      callback?.();
      return true;
    }
    this.openLoginGate();
    return false;
  }

  setSession(next: SessionBootstrapData | null) {
    this.data = next;
    if (next) {
      this.loginGateOpen = false;
    }
  }

  async refresh(): Promise<SessionBootstrapData | null> {
    this.loading = true;
    try {
      const res = await apiRequest<SessionBootstrapData | null>(API_ENDPOINTS.AUTH_GET_ME);
      if (res.success) {
        this.data = res.data ?? null;
      }
      return this.data;
    } finally {
      this.loading = false;
    }
  }

  async switchWorkspace(workspaceId: number): Promise<boolean> {
    const res = await apiRequest<SessionBootstrapData>(
      API_ENDPOINTS.WORKSPACES_SWITCH_WORKSPACE,
      { body: { workspace_id: workspaceId } }
    );
    if (res.success && res.data) {
      this.data = res.data;
      return true;
    }
    toasts.error(uiPrefs.err(res.message));
    return false;
  }

  async logout() {
    await apiRequest<boolean>(API_ENDPOINTS.AUTH_LOGOUT);
    this.data = null;
    if (browser) {
      await goto(ROUTES.LOGIN);
    }
  }

  decrementUnread() {
    if (this.data && this.data.unread_notifications > 0) {
      this.data = {
        ...this.data,
        unread_notifications: this.data.unread_notifications - 1
      };
    }
  }

  clearUnread() {
    if (this.data) {
      this.data = {
        ...this.data,
        unread_notifications: 0
      };
    }
  }

  incrementUnread() {
    if (this.data) {
      this.data = {
        ...this.data,
        unread_notifications: this.data.unread_notifications + 1
      };
    }
  }
}

export const sessionStore = new SessionStore();
