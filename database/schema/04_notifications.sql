-- =============================================================================
-- SCHEMA 04: IN-APP & GLOBAL BROADCAST NOTIFICATIONS
-- =============================================================================
-- - `notifications`: Per-user real-time notifications pushed over SSE.
-- - `global_notifications`: Admin-published rich HTML announcements targeted at
--   `all`, `authenticated`, or `admins` with read-tracking per user.

CREATE TABLE IF NOT EXISTS notifications (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    workspace_id INTEGER REFERENCES workspaces(id) ON DELETE CASCADE,
    kind VARCHAR(50) NOT NULL DEFAULT 'info',
    title VARCHAR(200) NOT NULL,
    body TEXT NOT NULL,
    link_url VARCHAR(500),
    is_read BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS global_notifications (
    id SERIAL PRIMARY KEY,
    title VARCHAR(200) NOT NULL,
    html_content TEXT NOT NULL,
    target_audience VARCHAR(30) NOT NULL DEFAULT 'all',
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    published_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ,
    created_by INTEGER REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS global_notification_reads (
    id SERIAL PRIMARY KEY,
    global_notification_id INTEGER NOT NULL REFERENCES global_notifications(id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    read_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_global_notification_reads_user UNIQUE (global_notification_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_notifications_user_unread ON notifications(user_id, is_read, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_global_notifications_active ON global_notifications(is_active, published_at DESC);
