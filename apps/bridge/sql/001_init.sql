CREATE TABLE IF NOT EXISTS devices (
    device_id TEXT PRIMARY KEY,
    user_id TEXT,
    api_token_hash TEXT UNIQUE,
    platform TEXT NOT NULL,
    app_version TEXT NOT NULL,
    notification_capability TEXT NOT NULL,
    usage_capability TEXT NOT NULL,
    push_provider TEXT,
    push_token TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    display_name TEXT,
    avatar_url TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS auth_identities (
    provider TEXT NOT NULL,
    provider_subject TEXT NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    email TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (provider, provider_subject)
);

ALTER TABLE devices
    ADD COLUMN IF NOT EXISTS user_id TEXT;

ALTER TABLE devices
    ADD COLUMN IF NOT EXISTS api_token_hash TEXT;

ALTER TABLE devices
    ADD COLUMN IF NOT EXISTS workspace_id TEXT;

ALTER TABLE devices
    ADD COLUMN IF NOT EXISTS device_name TEXT;

ALTER TABLE devices
    ADD COLUMN IF NOT EXISTS paired_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS devices_user_id_idx
    ON devices (user_id);

CREATE INDEX IF NOT EXISTS devices_workspace_id_idx
    ON devices (workspace_id);

CREATE UNIQUE INDEX IF NOT EXISTS devices_api_token_hash_idx
    ON devices (api_token_hash)
    WHERE api_token_hash IS NOT NULL;

CREATE TABLE IF NOT EXISTS pairing_sessions (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    code_hash TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    used_at TIMESTAMPTZ,
    claimed_device_id TEXT,
    claimed_user_id TEXT,
    device_name TEXT,
    attempt_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS pairing_sessions_workspace_created_idx
    ON pairing_sessions (workspace_id, created_at DESC);

CREATE TABLE IF NOT EXISTS alarms (
    id TEXT PRIMARY KEY,
    device_id TEXT NOT NULL REFERENCES devices(device_id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    severity TEXT NOT NULL,
    title TEXT NOT NULL,
    message TEXT NOT NULL,
    fire_at TIMESTAMPTZ NOT NULL,
    hidden_buffer_applied BOOLEAN NOT NULL DEFAULT false,
    requires_acknowledgement BOOLEAN NOT NULL DEFAULT true,
    expires_at TIMESTAMPTZ,
    status TEXT NOT NULL DEFAULT 'pending',
    delivery_attempts INTEGER NOT NULL DEFAULT 0,
    last_delivered_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS alarms_device_status_fire_at_idx
    ON alarms (device_id, status, fire_at);

CREATE INDEX IF NOT EXISTS alarms_expires_at_idx
    ON alarms (expires_at);

CREATE TABLE IF NOT EXISTS alarm_events (
    id BIGSERIAL PRIMARY KEY,
    alarm_id TEXT NOT NULL REFERENCES alarms(id) ON DELETE CASCADE,
    device_id TEXT NOT NULL,
    action TEXT NOT NULL,
    minutes INTEGER,
    occurred_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS alarm_events_alarm_id_idx
    ON alarm_events (alarm_id, created_at DESC);

CREATE TABLE IF NOT EXISTS page_views (
    id TEXT PRIMARY KEY,
    count BIGINT NOT NULL DEFAULT 0
);

INSERT INTO page_views (id, count)
VALUES ('homepage', 0)
ON CONFLICT (id) DO NOTHING;

-- Standalone Antirot Orchestration, Subscriptions, and Memory updates
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS subscription_tier TEXT NOT NULL DEFAULT 'byok';

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS subscription_status TEXT NOT NULL DEFAULT 'inactive';

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS byok_api_key TEXT;

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS byok_provider TEXT;

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS subscription_active_until TIMESTAMPTZ;

CREATE TABLE IF NOT EXISTS user_memories (
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    memory_key TEXT NOT NULL,
    content TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, memory_key)
);

CREATE TABLE IF NOT EXISTS chat_messages (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role TEXT NOT NULL,
    content TEXT,
    tool_calls JSONB,
    tool_call_id TEXT,
    name TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS chat_messages_user_id_created_at_idx
    ON chat_messages (user_id, created_at ASC);

-- Ensure fallback admin user exists for admin/device bypass tokens
INSERT INTO users (id, email, display_name)
VALUES ('admin', 'admin@antirot.org', 'Admin Bypass')
ON CONFLICT (id) DO NOTHING;


