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

ALTER TABLE alarms ADD COLUMN IF NOT EXISTS series_id TEXT;
ALTER TABLE alarms ADD COLUMN IF NOT EXISTS generation BIGINT NOT NULL DEFAULT 1;
ALTER TABLE alarms ADD COLUMN IF NOT EXISTS delivery_token TEXT;
ALTER TABLE alarms ADD COLUMN IF NOT EXISTS delivery_lease_expires_at TIMESTAMPTZ;
ALTER TABLE alarms ADD COLUMN IF NOT EXISTS scheduled_local_id TEXT;
ALTER TABLE alarms ADD COLUMN IF NOT EXISTS scheduled_at TIMESTAMPTZ;
ALTER TABLE alarms ADD COLUMN IF NOT EXISTS cancellation_confirmed_at TIMESTAMPTZ;
UPDATE alarms SET series_id = id WHERE series_id IS NULL;
ALTER TABLE alarms ALTER COLUMN series_id SET NOT NULL;

CREATE INDEX IF NOT EXISTS alarms_device_status_fire_at_idx
    ON alarms (device_id, status, fire_at);

CREATE INDEX IF NOT EXISTS alarms_expires_at_idx
    ON alarms (expires_at);

CREATE INDEX IF NOT EXISTS alarms_device_delivery_lease_idx
    ON alarms (device_id, status, delivery_lease_expires_at, fire_at);

CREATE INDEX IF NOT EXISTS alarms_series_generation_idx
    ON alarms (series_id, generation, device_id);

CREATE INDEX IF NOT EXISTS alarms_device_cancellation_idx
    ON alarms (device_id, cancellation_confirmed_at)
    WHERE status = 'cancelled';

CREATE TABLE IF NOT EXISTS alarm_wake_outbox (
    id TEXT PRIMARY KEY,
    device_id TEXT NOT NULL REFERENCES devices(device_id) ON DELETE CASCADE,
    alarm_id TEXT NOT NULL REFERENCES alarms(id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'in_progress', 'completed')),
    lease_token TEXT,
    lease_expires_at TIMESTAMPTZ,
    attempts INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS alarm_wake_outbox_claim_idx
    ON alarm_wake_outbox (status, lease_expires_at, created_at);

CREATE TABLE IF NOT EXISTS alarm_action_replays (
    original_alarm_id TEXT NOT NULL REFERENCES alarms(id) ON DELETE CASCADE,
    action TEXT NOT NULL,
    parameters_hash TEXT NOT NULL,
    duration_minutes INTEGER NOT NULL,
    request_device_id TEXT NOT NULL REFERENCES devices(device_id) ON DELETE CASCADE,
    replacement_alarm_id TEXT NOT NULL REFERENCES alarms(id) ON DELETE CASCADE,
    replacement_series_id TEXT NOT NULL,
    replacement_generation BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (original_alarm_id, action)
);

CREATE INDEX IF NOT EXISTS alarm_action_replays_replacement_idx
    ON alarm_action_replays (replacement_series_id, replacement_generation);

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

ALTER TABLE users ADD COLUMN IF NOT EXISTS timezone TEXT NOT NULL DEFAULT 'UTC';

CREATE TABLE IF NOT EXISTS user_memories (
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    memory_key TEXT NOT NULL,
    content TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, memory_key)
);

ALTER TABLE user_memories ADD COLUMN IF NOT EXISTS content_version TEXT;
UPDATE user_memories SET content_version = md5(content) WHERE content_version IS NULL;
ALTER TABLE user_memories ALTER COLUMN content_version SET NOT NULL;

CREATE TABLE IF NOT EXISTS chat_messages (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role TEXT NOT NULL,
    content TEXT,
    tool_calls JSONB,
    tool_call_id TEXT,
    name TEXT,
    is_visible BOOLEAN NOT NULL DEFAULT TRUE,
    turn_id TEXT,
    request_id TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE chat_messages
    ADD COLUMN IF NOT EXISTS is_visible BOOLEAN NOT NULL DEFAULT TRUE;

ALTER TABLE chat_messages
    ADD COLUMN IF NOT EXISTS turn_id TEXT;

ALTER TABLE chat_messages
    ADD COLUMN IF NOT EXISTS request_id TEXT;

UPDATE chat_messages
SET is_visible = FALSE
WHERE is_visible = TRUE
  AND (role = 'tool' OR tool_calls IS NOT NULL OR tool_call_id IS NOT NULL);

CREATE INDEX IF NOT EXISTS chat_messages_user_id_created_at_idx
    ON chat_messages (user_id, created_at ASC);

CREATE INDEX IF NOT EXISTS chat_messages_visible_history_idx
    ON chat_messages (user_id, is_visible, created_at DESC, id DESC);

CREATE UNIQUE INDEX IF NOT EXISTS chat_messages_user_request_visible_assistant_idx
    ON chat_messages (user_id, request_id)
    WHERE request_id IS NOT NULL AND role = 'assistant' AND is_visible = TRUE;

CREATE UNIQUE INDEX IF NOT EXISTS chat_messages_user_request_visible_user_idx
    ON chat_messages (user_id, request_id)
    WHERE request_id IS NOT NULL AND role = 'user' AND is_visible = TRUE;

-- Exact visible-reply guarantees begin with chat_turns-backed turns. Legacy rows are
-- retained as conversation evidence, but ambiguous pre-migration tool turns cannot
-- be reconstructed into a curated reply and are excluded above when identifiable.
CREATE TABLE IF NOT EXISTS chat_turns (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    request_id TEXT NOT NULL,
    message_hash TEXT NOT NULL,
    user_message TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('processing', 'completed', 'failed')),
    lease_token TEXT NOT NULL,
    lease_generation BIGINT NOT NULL DEFAULT 1,
    lease_expires_at TIMESTAMPTZ NOT NULL,
    curated_reply TEXT,
    visible_reply TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, request_id)
);

ALTER TABLE chat_turns ADD COLUMN IF NOT EXISTS lease_token TEXT;
ALTER TABLE chat_turns ADD COLUMN IF NOT EXISTS lease_generation BIGINT NOT NULL DEFAULT 1;
ALTER TABLE chat_turns ADD COLUMN IF NOT EXISTS curated_reply TEXT;
UPDATE chat_turns SET lease_token = id WHERE lease_token IS NULL;
ALTER TABLE chat_turns ALTER COLUMN lease_token SET NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS chat_turns_one_processing_per_user_idx
    ON chat_turns (user_id)
    WHERE status = 'processing';

CREATE TABLE IF NOT EXISTS chat_tool_outcomes (
    turn_id TEXT NOT NULL REFERENCES chat_turns(id) ON DELETE CASCADE,
    call_fingerprint TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    arguments_hash TEXT NOT NULL,
    succeeded BOOLEAN NOT NULL,
    message TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (turn_id, call_fingerprint)
);

CREATE TABLE IF NOT EXISTS chat_effect_outbox (
    id TEXT PRIMARY KEY,
    turn_id TEXT NOT NULL REFERENCES chat_turns(id) ON DELETE CASCADE,
    effect_kind TEXT NOT NULL,
    payload JSONB NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'in_progress', 'completed', 'failed')),
    lease_token TEXT,
    lease_expires_at TIMESTAMPTZ,
    attempts INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE chat_effect_outbox ADD COLUMN IF NOT EXISTS lease_token TEXT;
ALTER TABLE chat_effect_outbox ADD COLUMN IF NOT EXISTS lease_expires_at TIMESTAMPTZ;
ALTER TABLE chat_effect_outbox DROP CONSTRAINT IF EXISTS chat_effect_outbox_status_check;
ALTER TABLE chat_effect_outbox ADD CONSTRAINT chat_effect_outbox_status_check
    CHECK (status IN ('pending', 'in_progress', 'completed', 'failed'));

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

ALTER TABLE user_runtime_states ADD COLUMN IF NOT EXISTS alarm_generation BIGINT NOT NULL DEFAULT 0;
ALTER TABLE user_runtime_states ADD COLUMN IF NOT EXISTS alarm_series_id TEXT;

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

ALTER TABLE user_state_metrics ADD COLUMN IF NOT EXISTS sleep_start_observation_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE user_state_metrics ADD COLUMN IF NOT EXISTS sleep_start_sin_sum DOUBLE PRECISION NOT NULL DEFAULT 0;
ALTER TABLE user_state_metrics ADD COLUMN IF NOT EXISTS sleep_start_cos_sum DOUBLE PRECISION NOT NULL DEFAULT 0;

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
    index_generation TEXT NOT NULL,
    chunk_index INTEGER NOT NULL,
    content TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    embedding JSONB,
    embedding_provider TEXT,
    embedding_model TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT memory_chunks_user_key_generation_chunk_hash_key
        UNIQUE (user_id, memory_key, index_generation, chunk_index, content_hash)
);

ALTER TABLE memory_chunks ADD COLUMN IF NOT EXISTS index_generation TEXT;
UPDATE memory_chunks SET index_generation = content_hash WHERE index_generation IS NULL;
ALTER TABLE memory_chunks ALTER COLUMN index_generation SET NOT NULL;
ALTER TABLE memory_chunks
    DROP CONSTRAINT IF EXISTS memory_chunks_user_id_memory_key_chunk_index_content_hash_key;
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'memory_chunks_user_key_generation_chunk_hash_key'
    ) THEN
        ALTER TABLE memory_chunks
            ADD CONSTRAINT memory_chunks_user_key_generation_chunk_hash_key
            UNIQUE (user_id, memory_key, index_generation, chunk_index, content_hash);
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS memory_index_states (
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    memory_key TEXT NOT NULL,
    active_index_generation TEXT NOT NULL,
    content_version TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, memory_key)
);

CREATE TABLE IF NOT EXISTS memory_index_jobs (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    memory_key TEXT NOT NULL,
    content_version TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'in_progress', 'completed')),
    lease_token TEXT,
    lease_expires_at TIMESTAMPTZ,
    attempts INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, memory_key, content_version)
);

CREATE INDEX IF NOT EXISTS memory_index_jobs_claim_idx
    ON memory_index_jobs (status, lease_expires_at, created_at);

CREATE INDEX IF NOT EXISTS memory_chunks_user_key_idx
    ON memory_chunks (user_id, memory_key);

-- Ensure fallback admin user exists for admin/device bypass tokens
INSERT INTO users (id, email, display_name)
VALUES ('admin', 'admin@antirot.org', 'Admin Bypass')
ON CONFLICT (id) DO NOTHING;
