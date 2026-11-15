-- =============================================================================
-- SCHEMA 05: POLYMORPHIC OBFUSCATED MEDIA FILES (OPTIONAL R2 STORAGE)
-- =============================================================================
-- Stores Cloudflare R2 object keys alongside per-file XOR obfuscation keys.
-- Used for user avatars, workspace badges, and Todo image attachments.

CREATE TABLE IF NOT EXISTS media_files (
    id SERIAL PRIMARY KEY,
    entity_type VARCHAR(50) NOT NULL,
    entity_id INTEGER NOT NULL,
    field_name VARCHAR(50) NOT NULL DEFAULT 'image',
    storage_key VARCHAR(512) NOT NULL,
    xor_key VARCHAR(128) NOT NULL,
    content_type VARCHAR(100) NOT NULL DEFAULT 'image/webp',
    file_size BIGINT NOT NULL DEFAULT 0,
    uploaded_by INTEGER REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_media_files_entity_field UNIQUE (entity_type, entity_id, field_name)
);

CREATE INDEX IF NOT EXISTS idx_media_files_entity ON media_files(entity_type, entity_id);
CREATE INDEX IF NOT EXISTS idx_media_files_uploaded_by ON media_files(uploaded_by);
