ALTER TABLE alarm_wake_outbox ADD COLUMN IF NOT EXISTS next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now();
ALTER TABLE alarm_wake_outbox ADD COLUMN IF NOT EXISTS last_error TEXT;
ALTER TABLE alarm_wake_outbox DROP CONSTRAINT IF EXISTS alarm_wake_outbox_status_check;
ALTER TABLE alarm_wake_outbox ADD CONSTRAINT alarm_wake_outbox_status_check
    CHECK (status IN ('pending','in_progress','completed','failed'));
CREATE INDEX IF NOT EXISTS alarm_wake_outbox_retry_idx
    ON alarm_wake_outbox (status,next_attempt_at,lease_expires_at,created_at);
