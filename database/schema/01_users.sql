-- =============================================================================
-- SCHEMA 01: USERS & SESSIONS
-- =============================================================================
-- Authentication model:
-- - Standard users authenticate via Google OAuth 2.0 (`google_id`).
-- - Local Test Accounts (created by admins in `/admin`) have `password_hash`
--   (Argon2id) and `is_test_account = TRUE` for automated E2E / QA flows.
-- - Sessions store SHA-256 hex digests of raw `auth-session` / `hot-support-session`
--   cookie tokens so leaked DB rows cannot impersonate active users.

CREATE TABLE IF NOT EXISTS users (
    id SERIAL PRIMARY KEY,
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255),
    name VARCHAR(255) NOT NULL,
    avatar_url TEXT,
    role VARCHAR(50) NOT NULL DEFAULT 'user',
    email_verified BOOLEAN NOT NULL DEFAULT FALSE,
    google_id VARCHAR(255) UNIQUE,
    language VARCHAR(10) NOT NULL DEFAULT 'vi',
    is_test_account BOOLEAN NOT NULL DEFAULT FALSE,
    active_workspace_id INTEGER,
    onboarding_completed BOOLEAN NOT NULL DEFAULT FALSE,
    last_login_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS sessions (
    id VARCHAR(64) PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_users_google_id ON users(google_id);
CREATE INDEX IF NOT EXISTS idx_users_role ON users(role);
CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions(expires_at);
