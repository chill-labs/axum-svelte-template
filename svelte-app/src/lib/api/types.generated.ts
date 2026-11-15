// =============================================================================
// AUTO-GENERATED TYPESCRIPT CONTRACTS — DO NOT EDIT MANUALLY
// Generated via: cargo run -p api --bin gen-ts
// Source: crates/shared/src/domain/*.rs
// =============================================================================

export const ERROR_CODES = [
  'UNAUTHORIZED',
  'INVALID_CREDENTIALS',
  'ACCOUNT_DISABLED',
  'FORBIDDEN',
  'ADMIN_REQUIRED',
  'PERMISSION_DENIED',
  'OAUTH_NOT_CONFIGURED',
  'OAUTH_EXCHANGE_FAILED',
  'HOT_SUPPORT_SESSION_INVALID',
  'VALIDATION_FAILED',
  'FIELD_REQUIRED',
  'INVALID_EMAIL',
  'INVALID_SLUG',
  'INVALID_HEX_COLOR',
  'PASSWORD_TOO_WEAK',
  'INVALID_PAGINATION',
  'NOT_FOUND',
  'CONFLICT',
  'INTERNAL_ERROR',
  'USER_NOT_FOUND',
  'EMAIL_ALREADY_EXISTS',
  'WORKSPACE_NOT_FOUND',
  'WORKSPACE_SLUG_TAKEN',
  'CANNOT_DELETE_LAST_WORKSPACE',
  'CANNOT_REMOVE_WORKSPACE_OWNER',
  'MEMBER_ALREADY_EXISTS',
  'MEMBER_NOT_FOUND',
  'TODO_NOT_FOUND',
  'TODO_TITLE_TOO_LONG',
  'ASSIGNEE_NOT_IN_WORKSPACE',
  'NOTIFICATION_NOT_FOUND',
  'STORAGE_NOT_CONFIGURED',
  'UNSUPPORTED_MEDIA_TYPE',
  'FILE_TOO_LARGE',
  'MEDIA_NOT_FOUND',
] as const;

export type ErrorCode = (typeof ERROR_CODES)[number];

export type Status =
  | { Success: 'Ok' | 'Created' }
  | { Error: 'BadRequest' | 'Unauthorized' | 'Forbidden' | 'NotFound' | 'Conflict' | 'ServiceUnavailable' | 'InternalServerError' };

export interface AddWorkspaceMemberRequest {
  email: string;
  role: MemberRolePreset;
  permissions?: Permission[] | null;
}

export interface AdminGlobalNotificationResponse {
  id: number;
  title: string;
  html_content: string;
  target_audience: GlobalNotificationAudience;
  is_active: boolean;
  published_at: string;
  expires_at?: string | null;
  created_at: string;
}

export interface AdminTestAccountResponse {
  id: number;
  email: string;
  name: string;
  role: UserRole;
  language: string;
  active_workspace_id?: number | null;
  last_login_at?: string | null;
  created_at: string;
}

export interface AdminUpdateUserRoleRequest {
  role: UserRole;
}

export interface AdminUserResponse {
  id: number;
  email: string;
  name: string;
  role: UserRole;
  is_test_account: boolean;
  onboarding_completed: boolean;
  workspace_count: number;
  last_login_at?: string | null;
  created_at: string;
}

export interface ApiResponse<T> {
  success: boolean;
  message: string;
  status: Status;
  data?: T | null;
  pagination?: Pagination | null;
  errors?: Record<string, string> | null;
}

export interface CompleteOnboardingRequest {
  name: string;
  language: string;
  first_workspace_name?: string | null;
}

export interface ConfirmUploadRequest {
  purpose: UploadPurpose;
  entity_id: number;
  field_name?: string | null;
  storage_key: string;
  xor_key: string;
  content_type: string;
  file_size: number;
}

export interface CreateGlobalNotificationRequest {
  title: string;
  html_content: string;
  target_audience: GlobalNotificationAudience;
  is_active?: boolean | null;
}

export interface CreateTestAccountRequest {
  email: string;
  name: string;
  password: string;
  role?: UserRole | null;
  language?: string | null;
}

export interface CreateTodoRequest {
  title: string;
  description?: string | null;
  status?: TodoStatus | null;
  priority?: TodoPriority | null;
  due_date?: string | null;
  assigned_to?: number | null;
}

export interface CreateWorkspaceRequest {
  name: string;
  slug?: string | null;
  description?: string | null;
  color?: string | null;
}

export type ErrorType = 'BadRequest' | 'Unauthorized' | 'Forbidden' | 'NotFound' | 'Conflict' | 'ServiceUnavailable' | 'InternalServerError';

export interface GetAdminUsersQuery {
  page?: number | null;
  page_size?: number | null;
  search?: string | null;
  role?: UserRole | null;
}

export interface GlobalAnnouncementResponse {
  id: number;
  title: string;
  html_content: string;
  target_audience: GlobalNotificationAudience;
  published_at: string;
  is_read: boolean;
}

export type GlobalNotificationAudience = 'all' | 'authenticated' | 'admins';

export interface HealthcheckResponse {
  status: string;
  version: string;
  database: boolean;
  storage_enabled: boolean;
  environment: string;
  timestamp: string;
}

export interface HotSupportLoginRequest {
  user_id: number;
}

export interface HotSupportSessionInfo {
  active: boolean;
  target_user_id?: number | null;
  target_user_name?: string | null;
  target_user_email?: string | null;
  launch_url?: string | null;
}

export interface InitiatePasswordReauthRequest {
  new_password: string;
}

export interface LoginPasswordRequest {
  email: string;
  password: string;
}

export interface MediaFileResponse {
  id: number;
  entity_type: string;
  entity_id: number;
  field_name: string;
  storage_key: string;
  xor_key: string;
  content_type: string;
  file_size: number;
  public_url: string;
  created_at: string;
}

export type MemberRolePreset = 'editor' | 'member' | 'viewer' | 'custom';

export interface NotificationFeedResponse {
  items: NotificationResponse[];
  global_items: GlobalAnnouncementResponse[];
  unread_count: number;
}

export interface NotificationResponse {
  id: number;
  user_id: number;
  workspace_id?: number | null;
  kind: string;
  title: string;
  body: string;
  link_url?: string | null;
  is_read: boolean;
  created_at: string;
}

export interface Pagination {
  page: number;
  page_size: number;
  total_items: number;
  total_pages: number;
}

export interface PaginationQuery {
  page?: number | null;
  page_size?: number | null;
}

export type PermAction = 'view' | 'create' | 'edit' | 'delete';

export type PermResource = 'workspace' | 'members' | 'todos';

export interface Permission {
  resource: PermResource;
  action: PermAction;
}

export interface PresignUploadRequest {
  purpose: UploadPurpose;
  entity_id: number;
  content_type: string;
  file_size: number;
}

export interface PresignUploadResponse {
  upload_url: string;
  storage_key: string;
  xor_key: string;
  expires_in_secs: number;
}

export type RealtimeEvent =
  | { event: 'user_notification'; payload: NotificationResponse }
  | { event: 'global_announcement'; payload: GlobalAnnouncementResponse }
  | { event: 'todo_changed'; payload: { workspace_id: number; todo_id: number } }
  | { event: 'keep_alive' };

export interface ReauthRedirectResponse {
  redirect_url: string;
}

export interface ReorderTodosRequest {
  ordered_ids: number[];
}

export interface SessionBootstrapData {
  user: UserResponse;
  active_workspace_id?: number | null;
  active_workspace_name?: string | null;
  active_workspace_role?: string | null;
  permissions: Permission[];
  unread_notifications: number;
  is_hot_support: boolean;
}

export type SortOrder = 'asc' | 'desc';

export type SuccessType = 'Ok' | 'Created';

export interface SwitchWorkspaceRequest {
  workspace_id: number;
}

export interface TodoListPayload {
  items: TodoResponse[];
  stats: TodoSummaryStats;
}

export interface TodoListQuery {
  page?: number | null;
  page_size?: number | null;
  search?: string | null;
  status?: TodoStatus | null;
  priority?: TodoPriority | null;
  assigned_to?: number | null;
  sort_order?: SortOrder | null;
}

export type TodoPriority = 'low' | 'medium' | 'high' | 'urgent';

export interface TodoResponse {
  id: number;
  workspace_id: number;
  created_by: number;
  created_by_name: string;
  assigned_to?: number | null;
  assigned_to_name?: string | null;
  title: string;
  description?: string | null;
  status: TodoStatus;
  priority: TodoPriority;
  due_date?: string | null;
  is_completed: boolean;
  completed_at?: string | null;
  position: number;
  attachment_key?: string | null;
  created_at: string;
  updated_at: string;
}

export type TodoStatus = 'todo' | 'in_progress' | 'done';

export interface TodoSummaryStats {
  total: number;
  todo_count: number;
  in_progress_count: number;
  done_count: number;
  urgent_count: number;
}

export interface UpdateGlobalNotificationRequest {
  title?: string | null;
  html_content?: string | null;
  target_audience?: GlobalNotificationAudience | null;
  is_active?: boolean | null;
}

export interface UpdateLanguageRequest {
  language: string;
}

export interface UpdateMemberPermissionsRequest {
  role: MemberRolePreset;
  permissions: Permission[];
}

export interface UpdateProfileRequest {
  name: string;
  language?: string | null;
  avatar_url?: string | null;
}

export interface UpdateTestAccountRequest {
  name?: string | null;
  password?: string | null;
  role?: UserRole | null;
  language?: string | null;
}

export interface UpdateTodoRequest {
  title?: string | null;
  description?: string | null;
  status?: TodoStatus | null;
  priority?: TodoPriority | null;
  due_date?: string | null;
  assigned_to?: number | null;
  is_completed?: boolean | null;
}

export interface UpdateWorkspaceRequest {
  name?: string | null;
  slug?: string | null;
  description?: string | null;
  color?: string | null;
}

export type UploadPurpose = 'user_avatar' | 'workspace_logo' | 'todo_attachment';

export interface UserResponse {
  id: number;
  email: string;
  name: string;
  avatar_url?: string | null;
  avatar_key?: string | null;
  role: UserRole;
  email_verified: boolean;
  has_google: boolean;
  language: string;
  is_test_account: boolean;
  active_workspace_id?: number | null;
  onboarding_completed: boolean;
  last_login_at?: string | null;
  created_at: string;
}

export type UserRole = 'user' | 'admin';

export interface WorkspaceMemberResponse {
  id: number;
  workspace_id: number;
  user_id: number;
  user_name: string;
  user_email: string;
  user_avatar_url?: string | null;
  user_avatar_key?: string | null;
  role: string;
  permissions: Permission[];
  is_owner: boolean;
  joined_at: string;
}

export interface WorkspaceResponse {
  id: number;
  owner_id: number;
  name: string;
  slug: string;
  description?: string | null;
  color: string;
  logo_key?: string | null;
  is_owner: boolean;
  my_role: string;
  my_permissions: Permission[];
  member_count: number;
  todo_count: number;
  completed_todo_count: number;
  created_at: string;
  updated_at: string;
}

