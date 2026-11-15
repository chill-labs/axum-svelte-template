# Authentication, Sessions & Workspace RBAC

## 1. Authentication Methods

### Google OAuth 2.0 (Primary Login + Re-Authentication)
- Configured via `APP__GOOGLE__CLIENT_ID`, `APP__GOOGLE__CLIENT_SECRET`, and `APP__GOOGLE__REDIRECT_URI`.
- Login flow redirects to Google OAuth, exchanges code at `/auth/google/callback`, creates/updates the `users` row, and sets an `HttpOnly`, `SameSite=Lax` (`Secure` outside `APP__ENV=dev`) `auth-session` cookie.
- Sensitive operations use `purpose="reauth"` OAuth roundtrips before applying account changes (`/auth/reset-password`, `/auth/reset-success`).

### Admin-Provisioned Test Accounts (`/auth/login-password`)
- When Google OAuth is unconfigured locally or during Playwright E2E runs, admins can create/manage Test Accounts (`is_test_account = true`, Argon2id `password_hash`) in `/admin`.
- Standard OAuth users cannot bypass OAuth with `/auth/login-password`; the endpoint strictly verifies `is_test_account = true`.

### Session Storage & Security
- Raw random tokens sent in `auth-session` cookies are **never** stored in plaintext in PostgreSQL.
- `sessions.id` stores the `64`-character lowercase hex `SHA-256` digest of the cookie token.

### Isolated Hot Support Impersonation
- Admins (`role = 'admin'`) can start an isolated Hot Support session (`AdminHotSupportTab`) via `/api/v1/admin/hot-support/*`.
- Sets a separate `hot-support-session` cookie so the admin can inspect target workspaces without destroying their own `auth-session`.

---

## 2. Route Security Classification (`crates/shared/src/config/routes.rs`)

| RouteType | Behavior | Routes |
| :--- | :--- | :--- |
| `Public` | Accessible to guests & authenticated users | `/`, `/todos`, `/auth/google/callback`, `/healthcheck`, `/terms`, `/privacy` |
| `GuestOnly` | Redirects authenticated users to `/todos` | `/auth/login`, `/auth/register`, `/auth/login-password`, `/login-password` |
| `Protected` | Requires valid session (`admin: true` requires `UserRole::Admin`) | `/workspaces`, `/workspaces/members`, `/profile`, `/settings`, `/auth/onboarding`, `/auth/reset-password`, `/auth/reset-success`, `/admin` (`admin: true`) |

---

## 3. Workspace Multi-Tenancy & 12-Permission RBAC (`crates/shared/src/domain/permission.rs`)

- **Resources (`PermResource`)**: `Workspace`, `Members`, `Todos`
- **Actions (`PermAction`)**: `View`, `Create`, `Edit`, `Delete`
- **Owners**: Workspace owners (`workspace_members.role == 'owner'` or `workspaces.owner_id == user.id`) bypass individual permission checks.
- **Presets (`MemberRolePreset`)**:
  - `editor`: View Workspace/Members + View/Create/Edit/Delete Todos
  - `member`: View Workspace/Members + View/Create/Edit Todos
  - `viewer`: View Workspace + View Todos
  - `custom`: Arbitrary subset stored in `workspace_members.permissions` (`JSONB`)
- Backend handlers enforce permissions via `crates/services/src/permission.rs` before executing workspace mutations or reads.
