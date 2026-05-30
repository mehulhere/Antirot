CREATE TABLE IF NOT EXISTS devices (
    device_id TEXT PRIMARY KEY,
    platform TEXT NOT NULL,
    app_version TEXT NOT NULL,
    notification_capability TEXT NOT NULL,
    usage_capability TEXT NOT NULL,
    push_provider TEXT,
    push_token TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

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
