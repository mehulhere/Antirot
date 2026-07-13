ALTER TABLE devices ADD COLUMN IF NOT EXISTS session_version BIGINT NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS devices_session_validation_idx
    ON devices (device_id,user_id,session_version);
