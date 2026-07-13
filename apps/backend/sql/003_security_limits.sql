CREATE TABLE IF NOT EXISTS provider_usage_daily (
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    usage_date DATE NOT NULL DEFAULT CURRENT_DATE,
    usage_kind TEXT NOT NULL,
    units BIGINT NOT NULL DEFAULT 0 CHECK (units >= 0),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, usage_date, usage_kind)
);

ALTER TABLE memory_index_jobs ADD COLUMN IF NOT EXISTS next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now();
ALTER TABLE memory_index_jobs ADD COLUMN IF NOT EXISTS last_error TEXT;
ALTER TABLE memory_index_jobs DROP CONSTRAINT IF EXISTS memory_index_jobs_status_check;
ALTER TABLE memory_index_jobs ADD CONSTRAINT memory_index_jobs_status_check
    CHECK (status IN ('pending','in_progress','completed','failed'));
CREATE INDEX IF NOT EXISTS memory_index_jobs_retry_idx
    ON memory_index_jobs (status,next_attempt_at,lease_expires_at,created_at);
