# Database Schema, Triggers & SeaORM Entities

## 1. PostgreSQL Schema Files (`database/`)

SQL scripts execute deterministically in lexicographical order via `cargo xtask db schema` (`psql -v ON_ERROR_STOP=1`):

1. **`database/schema/01_users.sql`**:
   - `users`: `id`, `email`, `password_hash` (Argon2id for test accounts), `name`, `avatar_url`, `role` (`'user'|'admin'`), `email_verified`, `google_id`, `language` (`'vi'|'en'`), `is_test_account`, `active_workspace_id`, `onboarding_completed`, `last_login_at`.
   - `sessions`: `id` (64-char SHA-256 digest PK), `user_id` FK -> `users(id) ON DELETE CASCADE`, `expires_at`.
2. **`database/schema/02_workspaces.sql`**:
   - `workspaces`: `id`, `owner_id`, `name`, `slug` (unique), `description`, `color` (`#RRGGBB`).
   - Adds `fk_users_active_workspace` (`ON DELETE SET NULL`) on `users.active_workspace_id`.
   - `workspace_members`: `(workspace_id, user_id)` unique, `role`, `permissions` (`JSONB`).
3. **`database/schema/03_todos.sql`**:
   - `todos`: `workspace_id`, `created_by`, `assigned_to`, `title`, `description`, `status` (`'todo'|'in_progress'|'done'`), `priority` (`'low'|'medium'|'high'|'urgent'`), `due_date`, `is_completed`, `completed_at`, `position`.
4. **`database/schema/04_notifications.sql`**:
   - `notifications`: Per-user real-time notifications (`kind`, `title`, `body`, `link_url`, `is_read`).
   - `global_notifications`: Admin HTML broadcasts (`target_audience`: `'all'|'authenticated'|'admins'`, `is_active`, `published_at`, `expires_at`).
   - `global_notification_reads`: Per-user read receipt uniqueness (`uq_global_notification_reads_user`).
5. **`database/schema/05_media_files.sql`**:
   - `media_files`: Polymorphic encrypted attachment registry (`entity_type`, `entity_id`, `field_name`, `storage_key`, `xor_key`, `content_type`, `file_size`, `uploaded_by`), unique on `(entity_type, entity_id, field_name)`.

---

## 2. Auto-Provisioning Signup Trigger

`database/triggers/1_example_workspace_triggers.sql` installs `trg_create_default_workspace` (`AFTER INSERT ON users`):
- Creates `'Personal Workspace'` owned by `NEW.id` with a unique timestamped slug (`personal-<id>-<epoch>`).
- Inserts an `'owner'` row in `workspace_members`.
- Sets `users.active_workspace_id` to the newly created workspace.
- Seeds 5 onboarding Todo items covering workspace navigation, filtering, RBAC invitations, R2 setup, and trigger verification.

---

## 3. SeaORM Entities & Explicit Conversions

- Entities live in `crates/services/src/entities/*.rs`.
- Explicit mapping helpers live in `crates/services/src/entities/conversions.rs` so response DTOs always populate hydrated fields (such as `avatar_key`, `logo_key`, `attachment_key`, assignee names, and resolved RBAC permissions) instead of relying on raw `From` conversions.
- Runtime bootstrap (`SeedService`) ensures `APP__ADMIN__EMAIL` exists as `role = 'admin'` on server startup.
