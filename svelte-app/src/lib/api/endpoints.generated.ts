// =============================================================================
// AUTO-GENERATED API ENDPOINT REGISTRY — DO NOT EDIT MANUALLY
// Generated via: cargo run -p api --bin gen-ts
// Source: crates/api/src/routes/*.rs
// =============================================================================

export type HttpMethod = 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE';

export interface ApiEndpointDef {
  readonly method: HttpMethod;
  readonly path: string;
}

export const API_ENDPOINTS = {
  ADMIN_CREATE_GLOBAL_NOTIFICATION: { method: 'POST' as HttpMethod, path: '/api/v1/admin/notifications' },
  ADMIN_CREATE_TEST_ACCOUNT: { method: 'POST' as HttpMethod, path: '/api/v1/admin/test-accounts' },
  ADMIN_DELETE_GLOBAL_NOTIFICATION: { method: 'DELETE' as HttpMethod, path: '/api/v1/admin/notifications/{id}' },
  ADMIN_DELETE_TEST_ACCOUNT: { method: 'DELETE' as HttpMethod, path: '/api/v1/admin/test-accounts/{id}' },
  ADMIN_HOT_SUPPORT_CLOSE: { method: 'POST' as HttpMethod, path: '/api/v1/admin/hot-support/close' },
  ADMIN_HOT_SUPPORT_LOGIN: { method: 'POST' as HttpMethod, path: '/api/v1/admin/hot-support/login' },
  ADMIN_HOT_SUPPORT_SESSION: { method: 'GET' as HttpMethod, path: '/api/v1/admin/hot-support/session' },
  ADMIN_LIST_ADMIN_USERS: { method: 'GET' as HttpMethod, path: '/api/v1/admin/users' },
  ADMIN_LIST_GLOBAL_NOTIFICATIONS: { method: 'GET' as HttpMethod, path: '/api/v1/admin/notifications' },
  ADMIN_LIST_TEST_ACCOUNTS: { method: 'GET' as HttpMethod, path: '/api/v1/admin/test-accounts' },
  ADMIN_UPDATE_GLOBAL_NOTIFICATION: { method: 'PUT' as HttpMethod, path: '/api/v1/admin/notifications/{id}' },
  ADMIN_UPDATE_TEST_ACCOUNT: { method: 'PUT' as HttpMethod, path: '/api/v1/admin/test-accounts/{id}' },
  ADMIN_UPDATE_USER_ROLE: { method: 'PUT' as HttpMethod, path: '/api/v1/admin/users/{id}/role' },
  AUTH_COMPLETE_ONBOARDING: { method: 'POST' as HttpMethod, path: '/api/v1/auth/onboarding' },
  AUTH_GET_ME: { method: 'GET' as HttpMethod, path: '/api/v1/auth/me' },
  AUTH_GOOGLE_CALLBACK: { method: 'GET' as HttpMethod, path: '/auth/google/callback' },
  AUTH_GOOGLE_LOGIN: { method: 'GET' as HttpMethod, path: '/api/v1/auth/google/login' },
  AUTH_LOGIN_PASSWORD: { method: 'POST' as HttpMethod, path: '/api/v1/auth/login-password' },
  AUTH_LOGOUT: { method: 'POST' as HttpMethod, path: '/api/v1/auth/logout' },
  AUTH_REAUTH_PASSWORD: { method: 'POST' as HttpMethod, path: '/api/v1/auth/reauth-password' },
  HEALTH_CHECK_HEALTH: { method: 'GET' as HttpMethod, path: '/api/v1/health' },
  MEDIA_CONFIRM_UPLOAD: { method: 'POST' as HttpMethod, path: '/api/v1/media/confirm' },
  MEDIA_GET_ENTITY_MEDIA: { method: 'GET' as HttpMethod, path: '/api/v1/media/{entity_type}/{entity_id}' },
  MEDIA_GET_MEDIA_BLOB: { method: 'GET' as HttpMethod, path: '/api/v1/media/blob/{media_id}' },
  MEDIA_PRESIGN_UPLOAD: { method: 'POST' as HttpMethod, path: '/api/v1/media/presign' },
  NOTIFICATIONS_GET_NOTIFICATIONS: { method: 'GET' as HttpMethod, path: '/api/v1/notifications' },
  NOTIFICATIONS_MARK_ALL_READ: { method: 'POST' as HttpMethod, path: '/api/v1/notifications/read-all' },
  NOTIFICATIONS_MARK_GLOBAL_READ: { method: 'POST' as HttpMethod, path: '/api/v1/notifications/global/{id}/read' },
  NOTIFICATIONS_MARK_ONE_READ: { method: 'POST' as HttpMethod, path: '/api/v1/notifications/{id}/read' },
  PROFILE_UPDATE_LANGUAGE: { method: 'PUT' as HttpMethod, path: '/api/v1/profile/language' },
  PROFILE_UPDATE_PROFILE: { method: 'PUT' as HttpMethod, path: '/api/v1/profile' },
  TODOS_CREATE_TODO: { method: 'POST' as HttpMethod, path: '/api/v1/workspaces/{workspace_id}/todos' },
  TODOS_DELETE_TODO: { method: 'DELETE' as HttpMethod, path: '/api/v1/workspaces/{workspace_id}/todos/{todo_id}' },
  TODOS_LIST_TODOS: { method: 'GET' as HttpMethod, path: '/api/v1/workspaces/{workspace_id}/todos' },
  TODOS_REORDER_TODOS: { method: 'POST' as HttpMethod, path: '/api/v1/workspaces/{workspace_id}/todos/reorder' },
  TODOS_UPDATE_TODO: { method: 'PATCH' as HttpMethod, path: '/api/v1/workspaces/{workspace_id}/todos/{todo_id}' },
  WORKSPACES_ADD_MEMBER: { method: 'POST' as HttpMethod, path: '/api/v1/workspaces/{workspace_id}/members' },
  WORKSPACES_CREATE_WORKSPACE: { method: 'POST' as HttpMethod, path: '/api/v1/workspaces' },
  WORKSPACES_DELETE_WORKSPACE: { method: 'DELETE' as HttpMethod, path: '/api/v1/workspaces/{workspace_id}' },
  WORKSPACES_GET_WORKSPACE: { method: 'GET' as HttpMethod, path: '/api/v1/workspaces/{workspace_id}' },
  WORKSPACES_LIST_MEMBERS: { method: 'GET' as HttpMethod, path: '/api/v1/workspaces/{workspace_id}/members' },
  WORKSPACES_LIST_WORKSPACES: { method: 'GET' as HttpMethod, path: '/api/v1/workspaces' },
  WORKSPACES_REMOVE_MEMBER: { method: 'DELETE' as HttpMethod, path: '/api/v1/workspaces/{workspace_id}/members/{member_user_id}' },
  WORKSPACES_REMOVE_SELF_STUB: { method: 'DELETE' as HttpMethod, path: '/api/v1/workspaces/{workspace_id}/members/leave' },
  WORKSPACES_SWITCH_WORKSPACE: { method: 'POST' as HttpMethod, path: '/api/v1/workspaces/switch' },
  WORKSPACES_UPDATE_MEMBER_PERMISSIONS: { method: 'PUT' as HttpMethod, path: '/api/v1/workspaces/{workspace_id}/members/{member_user_id}' },
  WORKSPACES_UPDATE_WORKSPACE: { method: 'PATCH' as HttpMethod, path: '/api/v1/workspaces/{workspace_id}' },
} as const;
