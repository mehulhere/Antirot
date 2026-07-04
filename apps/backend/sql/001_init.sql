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

CREATE TABLE IF NOT EXISTS user_reports (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id TEXT,
    title TEXT NOT NULL,
    window_start TIMESTAMPTZ NOT NULL,
    window_end TIMESTAMPTZ NOT NULL,
    report_markdown TEXT NOT NULL,
    events JSONB NOT NULL DEFAULT '[]'::JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS user_reports_user_created_idx
    ON user_reports (user_id, created_at DESC);

CREATE TABLE IF NOT EXISTS memory_snapshots (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id TEXT,
    title TEXT NOT NULL,
    reason TEXT NOT NULL,
    memory_payload JSONB NOT NULL,
    runtime_state JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS memory_snapshots_user_created_idx
    ON memory_snapshots (user_id, created_at DESC);

CREATE TABLE IF NOT EXISTS user_runtime_states (
    user_id TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    state TEXT NOT NULL CHECK (state IN ('onboarding', 'idle', 'working', 'sleeping', 'break', 'vacation')),
    entered_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    source_tool TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::JSONB
);

CREATE TABLE IF NOT EXISTS user_state_metrics (
    user_id TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    usual_sleep_start_minute_utc INTEGER,
    average_sleep_minutes INTEGER,
    average_sleep_quality DOUBLE PRECISION,
    sleep_sample_count INTEGER NOT NULL DEFAULT 0,
    last_sleep_started_at TIMESTAMPTZ,
    last_woke_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE user_state_metrics
    ALTER COLUMN average_sleep_quality TYPE DOUBLE PRECISION
    USING average_sleep_quality::DOUBLE PRECISION;

CREATE TABLE IF NOT EXISTS memory_distillations (
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    distilled_date DATE NOT NULL,
    trigger_source TEXT NOT NULL,
    summary_key TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, distilled_date)
);

CREATE TABLE IF NOT EXISTS memory_chunks (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    memory_key TEXT NOT NULL,
    chunk_index INTEGER NOT NULL,
    content TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    embedding JSONB,
    embedding_provider TEXT,
    embedding_model TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, memory_key, chunk_index, content_hash)
);

CREATE INDEX IF NOT EXISTS memory_chunks_user_key_idx
    ON memory_chunks (user_id, memory_key);

-- Ensure fallback admin user exists for admin/device bypass tokens
INSERT INTO users (id, email, display_name)
VALUES ('admin', 'admin@antirot.org', 'Admin Bypass')
ON CONFLICT (id) DO NOTHING;
