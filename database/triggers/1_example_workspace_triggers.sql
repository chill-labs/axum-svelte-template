-- =============================================================================
-- TRIGGER: AUTO-PROVISION PERSONAL WORKSPACE & STARTER TODOS ON USER SIGNUP
-- =============================================================================
-- Ensures every newly inserted user immediately owns an active workspace
-- (`Personal Workspace`) and 5 helpful starter Todo items without extra RPC roundtrips.

CREATE OR REPLACE FUNCTION create_default_workspace_for_user()
RETURNS TRIGGER AS $$
DECLARE
    v_workspace_id INTEGER;
    v_slug VARCHAR(140);
BEGIN
    v_slug := 'personal-' || NEW.id::TEXT || '-' || FLOOR(EXTRACT(EPOCH FROM NOW()))::BIGINT::TEXT;

    INSERT INTO workspaces (owner_id, name, slug, description, color)
    VALUES (
        NEW.id,
        'Personal Workspace',
        v_slug,
        'Default personal workspace created automatically on signup.',
        '#3B82F6'
    )
    RETURNING id INTO v_workspace_id;

    INSERT INTO workspace_members (workspace_id, user_id, role, permissions)
    VALUES (
        v_workspace_id,
        NEW.id,
        'owner',
        '[]'::jsonb
    )
    ON CONFLICT (workspace_id, user_id) DO NOTHING;

    UPDATE users
    SET active_workspace_id = v_workspace_id
    WHERE id = NEW.id;

    INSERT INTO todos (workspace_id, created_by, assigned_to, title, description, status, priority, is_completed, completed_at, position)
    VALUES
        (v_workspace_id, NEW.id, NEW.id, 'Explore your new Axum + Svelte 5 workspace', 'Review the sidebar navigation, light/dark toggle, and bilingual vi/en switcher.', 'todo', 'medium', FALSE, NULL, 1),
        (v_workspace_id, NEW.id, NEW.id, 'Create or filter a task by status & priority', 'Try switching tabs between All, Active, Done, or High/Urgent priority.', 'in_progress', 'high', FALSE, NULL, 2),
        (v_workspace_id, NEW.id, NULL, 'Invite a collaborator to your workspace', 'Open Workspaces -> Members to assign granular View/Create/Edit/Delete permissions.', 'todo', 'low', FALSE, NULL, 3),
        (v_workspace_id, NEW.id, NEW.id, 'Configure optional Cloudflare R2 object storage', 'Fill APP__R2__* variables in .env when ready to enable encrypted image attachments.', 'todo', 'low', FALSE, NULL, 4),
        (v_workspace_id, NEW.id, NEW.id, 'Verify onboarding trigger provisioning', 'This starter task proves PostgreSQL AFTER INSERT trigger provisioned cleanly.', 'done', 'medium', TRUE, NOW(), 5);

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_create_default_workspace ON users;
CREATE TRIGGER trg_create_default_workspace
AFTER INSERT ON users
FOR EACH ROW
EXECUTE FUNCTION create_default_workspace_for_user();
