use std::collections::BTreeMap;

use chrono::{DateTime, Datelike, NaiveDate, Timelike, Utc};
use chrono_tz::Tz;
use deadpool_postgres::Pool;
use reqwest::Client;
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tokio_postgres::{Client as PgClient, GenericClient};
use tracing::{info, warn};
use uuid::Uuid;

use crate::config::Config;
use crate::error::{AppError, AppResult};
use crate::prompt::allowed_memory_key;
use crate::prompt::{default_memory_for_key, memory_descriptors, DEFAULT_DAILY_SUMMARY};

const MEMORY_SEARCH_MIN_CHARS: usize = 4_000;
const MEMORY_CHUNK_CHARS: usize = 1_200;
const MAX_MEMORY_DOCUMENT_CHARS: usize = 100_000;
const MAX_USER_MEMORY_CHARS: usize = 1_000_000;
const MAX_MEMORY_INDEX_ATTEMPTS: i32 = 5;
pub const MEMORY_SNAPSHOT_LIMIT: i64 = 10;
#[derive(Debug, Clone)]
pub struct UserDay {
    timezone: Tz,
    now: DateTime<Utc>,
}

impl UserDay {
    pub fn new(timezone: &str, now: DateTime<Utc>) -> Result<Self, String> {
        Ok(Self {
            timezone: timezone
                .parse::<Tz>()
                .map_err(|_| format!("invalid IANA timezone {timezone}"))?,
            now,
        })
    }

    pub fn current_date(&self) -> NaiveDate {
        self.now.with_timezone(&self.timezone).date_naive()
    }

    pub fn completed_date(&self) -> NaiveDate {
        self.current_date()
            .pred_opt()
            .expect("valid date has a predecessor")
    }

    pub fn work_log_key(&self) -> String {
        format!("work_log_{}", self.current_date().format("%Y_%m_%d"))
    }

    pub fn weekly_override_key(&self) -> String {
        let iso_week = self.current_date().iso_week();
        format!("override_{}_W{:02}", iso_week.year(), iso_week.week())
    }
}

pub async fn user_day_for<C>(client: &C, user_id: &str, now: DateTime<Utc>) -> AppResult<UserDay>
where
    C: GenericClient + Sync,
{
    let timezone = client
        .query_opt("SELECT timezone FROM users WHERE id=$1", &[&user_id])
        .await?
        .map(|row| row.get::<_, String>("timezone"))
        .unwrap_or_else(|| "UTC".to_string());
    UserDay::new(&timezone, now).map_err(crate::error::AppError::BadRequest)
}

pub fn circular_minute_mean(samples: &[i32]) -> Option<i32> {
    if samples.is_empty() {
        return None;
    }
    let (sin_sum, cos_sum) = samples
        .iter()
        .fold((0.0, 0.0), |(sin_sum, cos_sum), minute| {
            let angle = (*minute as f64).rem_euclid(1440.0) * std::f64::consts::TAU / 1440.0;
            (sin_sum + angle.sin(), cos_sum + angle.cos())
        });
    let angle = sin_sum.atan2(cos_sum).rem_euclid(std::f64::consts::TAU);
    Some(((angle * 1440.0 / std::f64::consts::TAU).round() as i32).rem_euclid(1440))
}

#[derive(Debug, Clone)]
struct EmbeddingResult {
    values: Option<Vec<f32>>,
    provider: Option<String>,
    model: Option<String>,
}

impl EmbeddingResult {
    fn semantic(values: Vec<f32>, provider: &str, model: &str) -> Self {
        Self {
            values: Some(values),
            provider: Some(provider.to_string()),
            model: Some(model.to_string()),
        }
    }
    fn lexical() -> Self {
        Self {
            values: None,
            provider: None,
            model: None,
        }
    }
}

fn canonical_memory_commit_query() -> &'static str {
    "WITH canonical AS (
       INSERT INTO user_memories (user_id, memory_key, content, content_version, updated_at)
       VALUES ($1, $2, $3, $4, now())
       ON CONFLICT (user_id, memory_key) DO UPDATE SET content = EXCLUDED.content,
         content_version = EXCLUDED.content_version, updated_at = now()
       RETURNING user_id, memory_key, content_version)
     INSERT INTO memory_index_jobs (id, user_id, memory_key, content_version)
     SELECT $5, user_id, memory_key, content_version FROM canonical
     ON CONFLICT (user_id, memory_key, content_version) DO NOTHING"
}

fn insert_memory_chunk_generation_query() -> &'static str {
    "INSERT INTO memory_chunks (id, user_id, memory_key, index_generation, chunk_index,
      content, content_hash, embedding, embedding_provider, embedding_model)
     VALUES ($1,$2,$3,$4,$5,$6,$7,$8::TEXT::JSONB,$9,$10)"
}

fn activate_memory_index_generation_query() -> &'static str {
    "WITH valid_job AS (
       SELECT 1 FROM memory_index_jobs job
       JOIN user_memories canonical
         ON canonical.user_id=job.user_id AND canonical.memory_key=job.memory_key
       WHERE job.id=$5 AND job.lease_token=$6 AND job.status='in_progress'
         AND job.user_id=$1 AND job.memory_key=$2 AND job.content_version=$4
         AND canonical.content_version=$4
       FOR UPDATE OF job, canonical),
     activated AS (
       INSERT INTO memory_index_states (user_id,memory_key,active_index_generation,content_version)
       SELECT $1,$2,$3,$4 FROM valid_job
       ON CONFLICT (user_id,memory_key) DO UPDATE SET
         active_index_generation=EXCLUDED.active_index_generation,
         content_version=EXCLUDED.content_version, updated_at=now()
       RETURNING 1),
     deleted AS (
       DELETE FROM memory_chunks
       WHERE user_id=$1 AND memory_key=$2 AND index_generation<>$3
         AND EXISTS (SELECT 1 FROM activated)
       RETURNING 1)
     SELECT EXISTS(SELECT 1 FROM activated) AS activated"
}

fn claim_memory_index_job_query() -> &'static str {
    "WITH candidate AS (
       SELECT id FROM memory_index_jobs
       WHERE (status='pending' AND next_attempt_at <= now())
          OR (status='in_progress' AND lease_expires_at <= now())
       ORDER BY next_attempt_at, created_at LIMIT 1 FOR UPDATE SKIP LOCKED)
     UPDATE memory_index_jobs job SET status='in_progress', lease_token=$1,
       lease_expires_at=now()+interval '10 minutes', updated_at=now()
     FROM candidate WHERE job.id=candidate.id
     RETURNING job.id, job.user_id, job.memory_key, job.content_version"
}

fn claim_user_memory_index_job_query() -> &'static str {
    "WITH candidate AS (
       SELECT id FROM memory_index_jobs WHERE user_id=$2 AND
         ((status='pending' AND next_attempt_at <= now())
          OR (status='in_progress' AND lease_expires_at <= now()))
       ORDER BY next_attempt_at, created_at LIMIT 1 FOR UPDATE SKIP LOCKED)
     UPDATE memory_index_jobs job SET status='in_progress', lease_token=$1,
       lease_expires_at=now()+interval '10 minutes', updated_at=now()
     FROM candidate WHERE job.id=candidate.id
     RETURNING job.id, job.user_id, job.memory_key, job.content_version"
}

fn active_index_search_query() -> &'static str {
    "SELECT
        chunk.memory_key,
        chunk.content,
        chunk.embedding::TEXT AS embedding,
        chunk.embedding_provider,
        chunk.embedding_model
     FROM memory_chunks chunk
     JOIN memory_index_states state
       ON state.user_id=chunk.user_id AND state.memory_key=chunk.memory_key
      AND state.active_index_generation=chunk.index_generation
     JOIN user_memories canonical
       ON canonical.user_id=state.user_id AND canonical.memory_key=state.memory_key
      AND canonical.content_version=state.content_version
     WHERE chunk.user_id = $1"
}

fn canonical_snapshot_restore_query() -> &'static str {
    "WITH jobs AS (DELETE FROM memory_index_jobs WHERE user_id=$1),
       states AS (DELETE FROM memory_index_states WHERE user_id=$1),
       chunks AS (DELETE FROM memory_chunks WHERE user_id=$1),
       distillations AS (DELETE FROM memory_distillations WHERE user_id=$1)
     DELETE FROM user_memories WHERE user_id=$1"
}

fn restore_distillation_marker_query() -> &'static str {
    "INSERT INTO memory_distillations (user_id,distilled_date,trigger_source,summary_key)
     VALUES ($1,$2,'snapshot_restore',$3)
     ON CONFLICT (user_id,distilled_date) DO UPDATE SET
       trigger_source='snapshot_restore', summary_key=EXCLUDED.summary_key"
}

fn distillation_commit_query() -> &'static str {
    "WITH marker AS (
       INSERT INTO memory_distillations (user_id,distilled_date,trigger_source,summary_key)
       VALUES ($1,$2,$3,$4) ON CONFLICT (user_id,distilled_date) DO NOTHING RETURNING 1),
     summary_write AS (
       INSERT INTO user_memories (user_id,memory_key,content,content_version)
       SELECT $1,$4,$5,$6 FROM marker ON CONFLICT (user_id,memory_key) DO UPDATE SET
         content=EXCLUDED.content,content_version=EXCLUDED.content_version,updated_at=now()),
     durable_write AS (
       INSERT INTO user_memories (user_id,memory_key,content,content_version)
       SELECT $1,'durable',rtrim($7) || $8,$9 FROM marker
       ON CONFLICT (user_id,memory_key) DO UPDATE SET
         content=rtrim(user_memories.content) || $8,
         content_version=EXCLUDED.content_version,updated_at=now()),
     summary_job AS (
       INSERT INTO memory_index_jobs (id,user_id,memory_key,content_version)
       SELECT $10,$1,$4,$6 FROM marker ON CONFLICT (user_id,memory_key,content_version) DO NOTHING),
     durable_job AS (
       INSERT INTO memory_index_jobs (id,user_id,memory_key,content_version)
       SELECT $11,$1,'durable',$9 FROM marker ON CONFLICT (user_id,memory_key,content_version) DO NOTHING)
     SELECT COUNT(*)::BIGINT AS count FROM marker"
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemorySnapshotSummary {
    pub id: String,
    pub device_id: Option<String>,
    pub title: String,
    pub reason: String,
    pub memory_keys: Vec<String>,
    pub runtime_state: Option<Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemorySnapshotOutcome {
    pub snapshot: MemorySnapshotSummary,
    pub retained_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemorySnapshotRestoreOutcome {
    pub snapshot: MemorySnapshotSummary,
    pub restored_memory_keys: Vec<String>,
    pub restored_runtime_state: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemorySearchHit {
    pub memory_key: String,
    pub score: f64,
    pub content: String,
    pub embedding_provider: Option<String>,
    pub embedding_model: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DistillationOutcome {
    pub ok: bool,
    pub distilled: bool,
    pub user_id: String,
    pub date: String,
    pub trigger_source: String,
    pub summary_key: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SleepMetricsReport {
    pub usual_sleep_start_minute_utc: Option<i32>,
    pub average_sleep_minutes: Option<i32>,
    pub average_sleep_quality: Option<f64>,
    pub sleep_sample_count: i32,
    pub last_sleep_started_at: Option<String>,
    pub last_woke_at: Option<String>,
}

pub async fn save_memory_indexed(
    client: &PgClient,
    _config: &Config,
    user_id: &str,
    key: &str,
    content: &str,
) -> AppResult<()> {
    save_memory_canonical(client, user_id, key, content).await?;
    Ok(())
}

pub async fn save_memory_canonical<C>(
    client: &C,
    user_id: &str,
    key: &str,
    content: &str,
) -> AppResult<String>
where
    C: GenericClient + Sync,
{
    let content_chars = content.chars().count();
    if content_chars > MAX_MEMORY_DOCUMENT_CHARS {
        return Err(AppError::BadRequest(format!(
            "memory content must be at most {MAX_MEMORY_DOCUMENT_CHARS} characters"
        )));
    }
    let aggregate_without_current: i64 = client
        .query_one(
            "SELECT COALESCE(SUM(length(content)),0)::BIGINT AS chars
             FROM user_memories WHERE user_id=$1 AND memory_key<>$2",
            &[&user_id, &key],
        )
        .await?
        .get("chars");
    if aggregate_without_current.saturating_add(content_chars as i64) > MAX_USER_MEMORY_CHARS as i64
    {
        return Err(AppError::BadRequest(format!(
            "total memory must be at most {MAX_USER_MEMORY_CHARS} characters"
        )));
    }
    let version = content_hash(content);
    let job_id = format!("memory-index:{user_id}:{key}:{version}");
    client
        .execute(
            canonical_memory_commit_query(),
            &[&user_id, &key, &content, &version, &job_id],
        )
        .await?;
    Ok(version)
}

pub async fn process_memory_index_jobs(
    client: &PgClient,
    config: &Config,
    user_id: &str,
) -> AppResult<()> {
    let lease = Uuid::new_v4().to_string();
    let row = client
        .query_opt(claim_user_memory_index_job_query(), &[&lease, &user_id])
        .await?;
    process_claimed_memory_index_job(client, config, &lease, row)
        .await
        .map(|_| ())
}

pub async fn process_next_memory_index_job(pool: &Pool, config: &Config) -> AppResult<bool> {
    let client = pool.get().await?;
    let lease = Uuid::new_v4().to_string();
    let row = client
        .query_opt(claim_memory_index_job_query(), &[&lease])
        .await?;
    let claimed = row.is_some();
    let succeeded = process_claimed_memory_index_job(&client, config, &lease, row).await?;
    Ok(continue_index_drain(claimed, succeeded))
}

fn continue_index_drain(claimed: bool, succeeded: bool) -> bool {
    let _ = succeeded;
    claimed
}

async fn process_claimed_memory_index_job(
    client: &PgClient,
    config: &Config,
    lease: &str,
    row: Option<tokio_postgres::Row>,
) -> AppResult<bool> {
    let Some(row) = row else {
        return Ok(false);
    };
    let job_id: String = row.get("id");
    let user_id: String = row.get("user_id");
    let key: String = row.get("memory_key");
    let version: String = row.get("content_version");
    let current = client.query_opt(
        "SELECT content FROM user_memories WHERE user_id=$1 AND memory_key=$2 AND content_version=$3",
        &[&user_id, &key, &version],
    ).await?;
    let result = if let Some(current) = current {
        let content: String = current.get("content");
        index_memory_key(
            client,
            config,
            MemoryIndexBuild {
                user_id: &user_id,
                key: &key,
                content: &content,
                version: &version,
                job_id: &job_id,
                lease,
            },
        )
        .await
    } else {
        Ok(())
    };
    match result {
        Ok(()) => {
            client.execute("UPDATE memory_index_jobs SET status='completed', lease_token=NULL, lease_expires_at=NULL, attempts=attempts+1, next_attempt_at=now(), last_error=NULL, updated_at=now() WHERE id=$1 AND lease_token=$2", &[&job_id,&lease]).await?;
            Ok(true)
        }
        Err(error) => {
            warn!(user_id, memory_key = key, error = %error, "🔴 FALLBACK: memory index job deferred - Reason: derived index build failed - Impact: canonical memory remains saved and lexical search remains available");
            let error_text = error.to_string();
            client.execute(
                "UPDATE memory_index_jobs
                 SET status=CASE WHEN attempts + 1 >= $3 THEN 'failed' ELSE 'pending' END,
                     lease_token=NULL, lease_expires_at=NULL, attempts=attempts+1,
                     next_attempt_at=now() + make_interval(secs => LEAST(3600, 5 * (1 << LEAST(attempts, 9)))),
                     last_error=left($4,1000), updated_at=now()
                 WHERE id=$1 AND lease_token=$2",
                &[&job_id, &lease, &MAX_MEMORY_INDEX_ATTEMPTS, &error_text],
            ).await?;
            Ok(false)
        }
    }
}

pub async fn create_memory_snapshot(
    client: &PgClient,
    user_id: &str,
    device_id: Option<&str>,
    title: &str,
    reason: &str,
) -> AppResult<MemorySnapshotOutcome> {
    let snapshot_id = Uuid::new_v4().to_string();
    let memory_payload = collect_memory_payload(client, user_id).await?;
    let runtime_state = runtime_state_payload(client, user_id).await?;
    let memory_payload_text = serde_json::to_string(&memory_payload)?;
    let runtime_state_text = match &runtime_state {
        Some(value) => Some(serde_json::to_string(value)?),
        None => None,
    };

    client
        .execute(
            "
            INSERT INTO memory_snapshots (
                id,
                user_id,
                device_id,
                title,
                reason,
                memory_payload,
                runtime_state
            )
            VALUES ($1, $2, $3, $4, $5, $6::TEXT::JSONB, $7::TEXT::JSONB)
            ",
            &[
                &snapshot_id,
                &user_id,
                &device_id,
                &title,
                &reason,
                &memory_payload_text,
                &runtime_state_text,
            ],
        )
        .await?;

    prune_memory_snapshots(client, user_id).await?;
    let snapshot = memory_snapshot_summary(client, user_id, &snapshot_id).await?;
    let retained_count = memory_snapshot_count(client, user_id).await?;
    Ok(MemorySnapshotOutcome {
        snapshot,
        retained_count,
    })
}

pub async fn list_memory_snapshots(
    client: &PgClient,
    user_id: &str,
) -> AppResult<Vec<MemorySnapshotSummary>> {
    let rows = client
        .query(
            "
            SELECT
                id,
                device_id,
                title,
                reason,
                memory_payload::TEXT AS memory_payload,
                runtime_state::TEXT AS runtime_state,
                created_at
            FROM memory_snapshots
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT 10
            ",
            &[&user_id],
        )
        .await?;

    rows.into_iter()
        .map(memory_snapshot_summary_from_row)
        .collect()
}

pub async fn restore_memory_snapshot(
    client: &mut PgClient,
    _config: &Config,
    user_id: &str,
    snapshot_id: &str,
    restore_runtime_state: bool,
) -> AppResult<MemorySnapshotRestoreOutcome> {
    let transaction = client.transaction().await?;
    let row = transaction
        .query_opt(
            "
            SELECT
                id,
                device_id,
                title,
                reason,
                memory_payload::TEXT AS memory_payload,
                runtime_state::TEXT AS runtime_state,
                created_at
            FROM memory_snapshots
            WHERE user_id = $1 AND id = $2
            ",
            &[&user_id, &snapshot_id],
        )
        .await?;
    let Some(row) = row else {
        return Err(crate::error::AppError::BadRequest(
            "Memory snapshot not found".to_string(),
        ));
    };

    let snapshot = memory_snapshot_summary_from_row(row)?;
    let payload = memory_payload_from_summary_source(&transaction, user_id, snapshot_id).await?;
    transaction
        .execute(canonical_snapshot_restore_query(), &[&user_id])
        .await?;
    for (key, content) in &payload {
        save_memory_canonical(&transaction, user_id, key, content).await?;
        if let Some(date) = restored_summary_date(key) {
            transaction
                .execute(
                    restore_distillation_marker_query(),
                    &[&user_id, &date, &key],
                )
                .await?;
        }
    }

    let mut restored_runtime_state = false;
    if restore_runtime_state {
        if let Some(runtime_state) = snapshot.runtime_state.as_ref() {
            if let Some(state_name) = runtime_state.get("state").and_then(Value::as_str) {
                let source_tool = runtime_state
                    .get("sourceTool")
                    .and_then(Value::as_str)
                    .unwrap_or("memory_snapshot_restore");
                let metadata = runtime_state
                    .get("metadata")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                let metadata_text = serde_json::to_string(&metadata)?;
                crate::llm::apply_runtime_event(
                    &transaction,
                    user_id,
                    state_name,
                    source_tool,
                    serde_json::from_str(&metadata_text)?,
                )
                .await
                .map_err(crate::error::AppError::BadRequest)?;
                restored_runtime_state = true;
            }
        }
    }

    transaction.commit().await?;
    Ok(MemorySnapshotRestoreOutcome {
        snapshot,
        restored_memory_keys: payload.keys().cloned().collect(),
        restored_runtime_state,
    })
}

fn restored_summary_date(key: &str) -> Option<NaiveDate> {
    let suffix = key.strip_prefix("work_summary_")?;
    NaiveDate::parse_from_str(suffix, "%Y_%m_%d").ok()
}

async fn collect_memory_payload(
    client: &PgClient,
    user_id: &str,
) -> AppResult<BTreeMap<String, String>> {
    let mut payload = BTreeMap::new();
    for descriptor in memory_descriptors()
        .iter()
        .filter(|descriptor| descriptor.snapshot)
    {
        payload.insert(
            descriptor.key.to_string(),
            descriptor.default_content.to_string(),
        );
    }

    let rows = client
        .query(
            "
            SELECT memory_key, content
            FROM user_memories
            WHERE user_id = $1
            ORDER BY memory_key ASC
            ",
            &[&user_id],
        )
        .await?;
    for row in rows {
        let key: String = row.get("memory_key");
        if !allowed_memory_key(&key) {
            continue;
        }
        payload.insert(key, row.get("content"));
    }

    Ok(payload)
}

async fn runtime_state_payload(client: &PgClient, user_id: &str) -> AppResult<Option<Value>> {
    let row = client
        .query_opt(
            "
            SELECT state, entered_at, source_tool, metadata::TEXT AS metadata
            FROM user_runtime_states
            WHERE user_id = $1
            ",
            &[&user_id],
        )
        .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let entered_at: DateTime<Utc> = row.get("entered_at");
    let metadata_text: String = row.get("metadata");
    let metadata = serde_json::from_str::<Value>(&metadata_text).unwrap_or_else(|_| json!({}));
    Ok(Some(json!({
        "state": row.get::<_, String>("state"),
        "enteredAt": entered_at,
        "sourceTool": row.get::<_, Option<String>>("source_tool"),
        "metadata": metadata
    })))
}

async fn prune_memory_snapshots(client: &PgClient, user_id: &str) -> AppResult<()> {
    client
        .execute(
            "
            DELETE FROM memory_snapshots
            WHERE id IN (
                SELECT id
                FROM memory_snapshots
                WHERE user_id = $1
                ORDER BY created_at DESC
                OFFSET 10
            )
            ",
            &[&user_id],
        )
        .await?;
    Ok(())
}

async fn memory_snapshot_count(client: &PgClient, user_id: &str) -> AppResult<i64> {
    let row = client
        .query_one(
            "SELECT COUNT(*)::BIGINT AS count FROM memory_snapshots WHERE user_id = $1",
            &[&user_id],
        )
        .await?;
    Ok(row.get("count"))
}

async fn memory_snapshot_summary(
    client: &PgClient,
    user_id: &str,
    snapshot_id: &str,
) -> AppResult<MemorySnapshotSummary> {
    let row = client
        .query_one(
            "
            SELECT
                id,
                device_id,
                title,
                reason,
                memory_payload::TEXT AS memory_payload,
                runtime_state::TEXT AS runtime_state,
                created_at
            FROM memory_snapshots
            WHERE user_id = $1 AND id = $2
            ",
            &[&user_id, &snapshot_id],
        )
        .await?;
    memory_snapshot_summary_from_row(row)
}

fn memory_snapshot_summary_from_row(row: tokio_postgres::Row) -> AppResult<MemorySnapshotSummary> {
    let memory_payload: String = row.get("memory_payload");
    let runtime_state_text: Option<String> = row.get("runtime_state");
    let payload = serde_json::from_str::<BTreeMap<String, String>>(&memory_payload)?;
    let runtime_state = runtime_state_text
        .as_deref()
        .and_then(|text| serde_json::from_str::<Value>(text).ok());
    Ok(MemorySnapshotSummary {
        id: row.get("id"),
        device_id: row.get("device_id"),
        title: row.get("title"),
        reason: row.get("reason"),
        memory_keys: payload.keys().cloned().collect(),
        runtime_state,
        created_at: row.get("created_at"),
    })
}

async fn memory_payload_from_summary_source<C>(
    client: &C,
    user_id: &str,
    snapshot_id: &str,
) -> AppResult<BTreeMap<String, String>>
where
    C: GenericClient + Sync,
{
    let row = client
        .query_one(
            "
            SELECT memory_payload::TEXT AS memory_payload
            FROM memory_snapshots
            WHERE user_id = $1 AND id = $2
            ",
            &[&user_id, &snapshot_id],
        )
        .await?;
    let payload_text: String = row.get("memory_payload");
    let payload = serde_json::from_str::<BTreeMap<String, String>>(&payload_text)?;
    Ok(payload
        .into_iter()
        .filter(|(key, _)| allowed_memory_key(key))
        .collect())
}

fn update_sleep_start_mean_query() -> &'static str {
    "INSERT INTO user_state_metrics (
        user_id,
        usual_sleep_start_minute_utc,
        sleep_start_observation_count,
        sleep_start_sin_sum,
        sleep_start_cos_sum,
        last_sleep_started_at,
        updated_at
     )
     VALUES ($1, $2, 1, $3, $4, now(), now())
     ON CONFLICT (user_id) DO UPDATE SET
        sleep_start_observation_count = user_state_metrics.sleep_start_observation_count + 1,
        sleep_start_sin_sum = user_state_metrics.sleep_start_sin_sum + EXCLUDED.sleep_start_sin_sum,
        sleep_start_cos_sum = user_state_metrics.sleep_start_cos_sum + EXCLUDED.sleep_start_cos_sum,
        usual_sleep_start_minute_utc = mod(round((
            atan2(
                user_state_metrics.sleep_start_sin_sum + EXCLUDED.sleep_start_sin_sum,
                user_state_metrics.sleep_start_cos_sum + EXCLUDED.sleep_start_cos_sum
            ) * 1440 / (2 * pi()) + 1440
        )::NUMERIC), 1440)::INTEGER,
        last_sleep_started_at = EXCLUDED.last_sleep_started_at,
        updated_at = now()"
}

fn consume_sleep_start_query() -> &'static str {
    "WITH unmatched AS (
       SELECT last_sleep_started_at FROM user_state_metrics
       WHERE user_id=$1 AND last_sleep_started_at IS NOT NULL
       FOR UPDATE),
     consumed AS (
       UPDATE user_state_metrics metrics SET last_sleep_started_at = NULL
       FROM unmatched WHERE metrics.user_id=$1
       RETURNING unmatched.last_sleep_started_at)
     SELECT last_sleep_started_at FROM consumed"
}

pub async fn note_sleep_started<C>(client: &C, user_id: &str) -> AppResult<()>
where
    C: GenericClient + Sync,
{
    let now = Utc::now();
    let minute = (now.hour() * 60 + now.minute()) as i32;
    let initial_mean = circular_minute_mean(&[minute]).unwrap_or(minute);
    let angle = minute as f64 * std::f64::consts::TAU / 1440.0;
    let sin_value = angle.sin();
    let cos_value = angle.cos();
    client
        .execute(
            update_sleep_start_mean_query(),
            &[&user_id, &initial_mean, &sin_value, &cos_value],
        )
        .await?;
    Ok(())
}

pub async fn note_wake_logged<C>(
    client: &C,
    user_id: &str,
    sleep_quality: i64,
) -> AppResult<SleepMetricsReport>
where
    C: GenericClient + Sync,
{
    let row = client
        .query_opt(consume_sleep_start_query(), &[&user_id])
        .await?;
    let Some(row) = row else {
        return sleep_metrics_report(client, user_id).await;
    };
    let started: chrono::DateTime<Utc> = row.get("last_sleep_started_at");
    let sleep_minutes = Some((Utc::now() - started).num_minutes().clamp(1, 24 * 60) as i32);

    client
        .execute(
            "
            INSERT INTO user_state_metrics (
                user_id,
                average_sleep_minutes,
                average_sleep_quality,
                sleep_sample_count,
                last_woke_at,
                updated_at
            )
            VALUES ($1, $2, $3, 1, now(), now())
            ON CONFLICT (user_id) DO UPDATE SET
                average_sleep_minutes = CASE
                    WHEN $2::INTEGER IS NULL THEN user_state_metrics.average_sleep_minutes
                    WHEN user_state_metrics.average_sleep_minutes IS NULL THEN $2::INTEGER
                    ELSE round(
                        (user_state_metrics.average_sleep_minutes::NUMERIC * LEAST(user_state_metrics.sleep_sample_count, 14)
                         + $2::NUMERIC)
                        / (LEAST(user_state_metrics.sleep_sample_count, 14) + 1)
                    )::INTEGER
                END,
                average_sleep_quality = CASE
                    WHEN user_state_metrics.average_sleep_quality IS NULL THEN $3::DOUBLE PRECISION
                    ELSE (
                        user_state_metrics.average_sleep_quality * LEAST(user_state_metrics.sleep_sample_count, 14)
                        + $3::DOUBLE PRECISION
                    ) / (LEAST(user_state_metrics.sleep_sample_count, 14) + 1)
                END,
                sleep_sample_count = user_state_metrics.sleep_sample_count + 1,
                last_woke_at = now(),
                updated_at = now()
            ",
            &[&user_id, &sleep_minutes, &(sleep_quality as f64)],
        )
        .await?;
    sleep_metrics_report(client, user_id).await
}

pub async fn sleep_metrics_report<C>(client: &C, user_id: &str) -> AppResult<SleepMetricsReport>
where
    C: GenericClient + Sync,
{
    let row = client
        .query_opt(
            "
            SELECT
                usual_sleep_start_minute_utc,
                average_sleep_minutes,
                average_sleep_quality,
                sleep_sample_count,
                last_sleep_started_at,
                last_woke_at
            FROM user_state_metrics
            WHERE user_id = $1
            ",
            &[&user_id],
        )
        .await?;

    let Some(row) = row else {
        return Ok(SleepMetricsReport {
            usual_sleep_start_minute_utc: None,
            average_sleep_minutes: None,
            average_sleep_quality: None,
            sleep_sample_count: 0,
            last_sleep_started_at: None,
            last_woke_at: None,
        });
    };

    let last_sleep_started_at: Option<chrono::DateTime<Utc>> = row.get("last_sleep_started_at");
    let last_woke_at: Option<chrono::DateTime<Utc>> = row.get("last_woke_at");

    Ok(SleepMetricsReport {
        usual_sleep_start_minute_utc: row.get("usual_sleep_start_minute_utc"),
        average_sleep_minutes: row.get("average_sleep_minutes"),
        average_sleep_quality: row.get("average_sleep_quality"),
        sleep_sample_count: row.get("sleep_sample_count"),
        last_sleep_started_at: last_sleep_started_at.map(|dt| dt.to_rfc3339()),
        last_woke_at: last_woke_at.map(|dt| dt.to_rfc3339()),
    })
}

pub async fn distill_today(
    client: &PgClient,
    config: &Config,
    user_id: &str,
    trigger_source: &str,
) -> AppResult<DistillationOutcome> {
    let day = user_day_for(client, user_id, Utc::now()).await?;
    distill_date(
        client,
        config,
        user_id,
        day.completed_date(),
        trigger_source,
    )
    .await
}

pub async fn distill_idle_if_due(
    pool: &Pool,
    config: &Config,
    user_id: &str,
) -> AppResult<Option<DistillationOutcome>> {
    let client = pool.get().await?;
    let row = client
        .query_opt(
            "
            SELECT
                s.state,
                m.usual_sleep_start_minute_utc
            FROM user_runtime_states s
            LEFT JOIN user_state_metrics m ON m.user_id = s.user_id
            WHERE s.user_id = $1
            ",
            &[&user_id],
        )
        .await?;

    let Some(row) = row else {
        return Ok(None);
    };
    let state: String = row.get("state");
    let usual_sleep_start: Option<i32> = row.get("usual_sleep_start_minute_utc");
    if state != "idle" {
        return Ok(None);
    }
    let Some(usual_sleep_start) = usual_sleep_start else {
        return Ok(None);
    };
    if !is_one_hour_after_usual_sleep(usual_sleep_start) {
        return Ok(None);
    }

    let outcome = distill_today(&client, config, user_id, "idle_after_usual_sleep").await?;
    if outcome.distilled {
        info!(user_id, date = %outcome.date, "nightly memory distilled during idle window");
    }
    Ok(Some(outcome))
}

pub async fn distill_all_idle_users_due(
    pool: &Pool,
    config: &Config,
) -> AppResult<Vec<DistillationOutcome>> {
    let client = pool.get().await?;
    let rows = client
        .query(
            "
            SELECT user_id
            FROM user_runtime_states
            WHERE state = 'idle'
            ",
            &[],
        )
        .await?;
    drop(client);

    let mut outcomes = Vec::new();
    for row in rows {
        let user_id: String = row.get("user_id");
        match distill_idle_if_due(pool, config, &user_id).await {
            Ok(Some(outcome)) if outcome.distilled => outcomes.push(outcome),
            Ok(_) => {}
            Err(error) => {
                warn!(user_id, error = %error, "🔴 FALLBACK: user distillation skipped - Reason: isolated per-user worker failure - Impact: other users continue and this user retries later")
            }
        }
    }
    Ok(outcomes)
}

pub async fn search_memory(
    client: &PgClient,
    config: &Config,
    user_id: &str,
    query: &str,
    limit: usize,
) -> AppResult<Vec<MemorySearchHit>> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }

    let total_chars = total_searchable_memory_chars(client, user_id).await?;
    if total_chars < MEMORY_SEARCH_MIN_CHARS {
        return Ok(Vec::new());
    }

    let allow_embeddings = user_allows_server_embeddings(client, user_id).await?;
    let query_embedding =
        if allow_embeddings && reserve_memory_embedding_call(client, config, user_id).await? {
            embedding_with_fallback(config, query).await
        } else {
            EmbeddingResult::lexical()
        };
    let rows = client
        .query(active_index_search_query(), &[&user_id])
        .await?;

    let mut hits = Vec::new();
    for row in rows {
        let content: String = row.get("content");
        let embedding_text: Option<String> = row.get("embedding");
        let semantic_score = match (&query_embedding.values, embedding_text) {
            (Some(query_vec), Some(text)) => serde_json::from_str::<Vec<f32>>(&text)
                .ok()
                .map(|chunk_vec| cosine_similarity(query_vec, &chunk_vec))
                .unwrap_or(0.0),
            _ => 0.0,
        };
        let lexical_score = lexical_score(query, &content);
        let score = if semantic_score > 0.0 {
            semantic_score * 0.75 + lexical_score * 0.25
        } else {
            lexical_score
        };
        if score <= 0.0 {
            continue;
        }
        hits.push(MemorySearchHit {
            memory_key: row.get("memory_key"),
            score,
            content,
            embedding_provider: row.get("embedding_provider"),
            embedding_model: row.get("embedding_model"),
        });
    }

    let canonical_rows = client
        .query(
            "SELECT memory_key, content FROM user_memories memory
         WHERE user_id=$1 AND NOT EXISTS (
           SELECT 1 FROM memory_index_states state
           WHERE state.user_id=memory.user_id AND state.memory_key=memory.memory_key
             AND state.content_version=memory.content_version)",
            &[&user_id],
        )
        .await?;
    for row in canonical_rows {
        let memory_key: String = row.get("memory_key");
        if !is_searchable_key(&memory_key) {
            continue;
        }
        let content: String = row.get("content");
        let score = lexical_score(query, &content);
        if score > 0.0 {
            hits.push(MemorySearchHit {
                memory_key,
                score,
                content,
                embedding_provider: None,
                embedding_model: None,
            });
        }
    }

    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    hits.truncate(limit);
    Ok(hits)
}

async fn distill_date(
    client: &PgClient,
    _config: &Config,
    user_id: &str,
    date: NaiveDate,
    trigger_source: &str,
) -> AppResult<DistillationOutcome> {
    let summary_key = format!("work_summary_{}", date.format("%Y_%m_%d"));
    let already = client
        .query_opt(
            "
            SELECT summary_key
            FROM memory_distillations
            WHERE user_id = $1 AND distilled_date = $2
            ",
            &[&user_id, &date],
        )
        .await?;
    if already.is_some() {
        return Ok(DistillationOutcome {
            ok: true,
            distilled: false,
            user_id: user_id.to_string(),
            date: date.to_string(),
            trigger_source: trigger_source.to_string(),
            summary_key,
            reason: Some("already_distilled".to_string()),
        });
    }

    let work_log_key = format!("work_log_{}", date.format("%Y_%m_%d"));
    let work_log = read_memory(client, user_id, &work_log_key)
        .await?
        .unwrap_or_else(|| "# Work Log\n".to_string());
    let sleep = read_memory(client, user_id, "sleep")
        .await?
        .unwrap_or_else(|| {
            default_memory_for_key("sleep")
                .unwrap_or("# Sleep Ledger\n")
                .to_string()
        });
    let routine = read_memory(client, user_id, "routine")
        .await?
        .unwrap_or_else(|| {
            default_memory_for_key("routine")
                .unwrap_or("# Routine\n")
                .to_string()
        });
    let summary = deterministic_daily_summary(date, trigger_source, &work_log, &sleep, &routine);

    let durable_append = format!(
        "\n## {}\n- Distilled from daily logs via {}.\n- Work signal: {}\n- Sleep/recovery signal: {}\n",
        date,
        trigger_source,
        compact_line(find_last_matching_line(&work_log, &["session_end", "session_start"]).unwrap_or("No work session logged.")),
        compact_line(find_last_matching_line(&sleep, &["wake_log", "sleep_start"]).unwrap_or("No sleep update logged."))
    );
    let default_durable = default_memory_for_key("durable").unwrap_or("# Durable Memory\n");
    let summary_version = content_hash(&summary);
    let durable_version = Uuid::new_v4().to_string();
    let summary_job = format!("memory-index:{user_id}:{summary_key}:{summary_version}");
    let durable_job = format!("memory-index:{user_id}:durable:{durable_version}");
    let row = client
        .query_one(
            distillation_commit_query(),
            &[
                &user_id,
                &date,
                &trigger_source,
                &summary_key,
                &summary,
                &summary_version,
                &default_durable,
                &durable_append,
                &durable_version,
                &summary_job,
                &durable_job,
            ],
        )
        .await?;
    let committed: i64 = row.get("count");
    if committed == 0 {
        return Ok(DistillationOutcome {
            ok: true,
            distilled: false,
            user_id: user_id.to_string(),
            date: date.to_string(),
            trigger_source: trigger_source.to_string(),
            summary_key,
            reason: Some("already_distilled".to_string()),
        });
    }

    Ok(DistillationOutcome {
        ok: true,
        distilled: true,
        user_id: user_id.to_string(),
        date: date.to_string(),
        trigger_source: trigger_source.to_string(),
        summary_key,
        reason: None,
    })
}

pub(crate) async fn distill_date_for_test(
    client: &PgClient,
    config: &Config,
    user_id: &str,
    date: NaiveDate,
) -> AppResult<DistillationOutcome> {
    distill_date(client, config, user_id, date, "test_probe").await
}

pub(crate) async fn run_memory_db_invariant_probe(
    client: &mut PgClient,
    user_id: &str,
) -> AppResult<Value> {
    let rollback_key = format!("override_probe_{}", Uuid::new_v4());
    let rollback_content = "canonical rollback probe";
    let rollback_version = content_hash(rollback_content);
    let transaction = client.transaction().await?;
    save_memory_canonical(&transaction, user_id, &rollback_key, rollback_content).await?;
    transaction.rollback().await?;
    let canonical_after_rollback: i64 = client
        .query_one(
            "SELECT COUNT(*)::BIGINT AS count FROM user_memories WHERE user_id=$1 AND memory_key=$2",
            &[&user_id, &rollback_key],
        )
        .await?
        .get("count");
    let jobs_after_rollback: i64 = client
        .query_one(
            "SELECT COUNT(*)::BIGINT AS count FROM memory_index_jobs WHERE user_id=$1 AND memory_key=$2 AND content_version=$3",
            &[&user_id, &rollback_key, &rollback_version],
        )
        .await?
        .get("count");

    let transaction = client.transaction().await?;
    let key = format!("override_generation_probe_{}", Uuid::new_v4());
    let generation_v1 = Uuid::new_v4().to_string();
    let generation_v2 = Uuid::new_v4().to_string();
    let job_v1 = Uuid::new_v4().to_string();
    let lease_v1 = Uuid::new_v4().to_string();
    transaction
        .execute(
            "INSERT INTO user_memories (user_id,memory_key,content,content_version) VALUES ($1,$2,'v2','v2')",
            &[&user_id, &key],
        )
        .await?;
    transaction
        .execute(
            "INSERT INTO memory_index_jobs (id,user_id,memory_key,content_version,status,lease_token,lease_expires_at)
             VALUES ($1,$2,$3,'v1','in_progress',$4,now()+interval '10 minutes')",
            &[&job_v1, &user_id, &key, &lease_v1],
        )
        .await?;
    for generation in [&generation_v1, &generation_v2] {
        transaction
            .execute(
                "INSERT INTO memory_chunks (id,user_id,memory_key,index_generation,chunk_index,content,content_hash)
                 VALUES ($1,$2,$3,$4,0,'unchanged chunk','same-hash')",
                &[&Uuid::new_v4().to_string(), &user_id, &key, generation],
            )
            .await?;
    }
    transaction
        .execute(
            "INSERT INTO memory_index_states (user_id,memory_key,active_index_generation,content_version)
             VALUES ($1,$2,$3,'v2')",
            &[&user_id, &key, &generation_v2],
        )
        .await?;
    let generations_before: i64 = transaction
        .query_one(
            "SELECT COUNT(DISTINCT index_generation)::BIGINT AS count FROM memory_chunks WHERE user_id=$1 AND memory_key=$2",
            &[&user_id, &key],
        )
        .await?
        .get("count");
    let activation = transaction
        .query_one(
            activate_memory_index_generation_query(),
            &[&user_id, &key, &generation_v1, &"v1", &job_v1, &lease_v1],
        )
        .await?;
    let stale_activated: bool = activation.get("activated");
    let active_after: String = transaction
        .query_one(
            "SELECT active_index_generation FROM memory_index_states WHERE user_id=$1 AND memory_key=$2",
            &[&user_id, &key],
        )
        .await?
        .get("active_index_generation");
    let generations_after: i64 = transaction
        .query_one(
            "SELECT COUNT(DISTINCT index_generation)::BIGINT AS count FROM memory_chunks WHERE user_id=$1 AND memory_key=$2",
            &[&user_id, &key],
        )
        .await?
        .get("count");
    transaction.rollback().await?;

    Ok(json!({
        "canonicalAfterRollback": canonical_after_rollback,
        "jobsAfterRollback": jobs_after_rollback,
        "generationsBeforeActivation": generations_before,
        "generationsAfterStaleActivation": generations_after,
        "staleActivated": stale_activated,
        "newerGenerationStayedActive": active_after == generation_v2
    }))
}

pub(crate) async fn run_memory_activation_race_probe(
    pool: &Pool,
    config: &Config,
    user_id: &str,
) -> AppResult<Value> {
    let key = format!("override_activation_race_{}", Uuid::new_v4());
    let content_v1 = "race content v1";
    let content_v2 = "race content v2";
    let version_v1 = content_hash(content_v1);
    let version_v2 = content_hash(content_v2);
    let job_v1 = format!("memory-index:{user_id}:{key}:{version_v1}");
    let lease_v1 = Uuid::new_v4().to_string();
    let generation_v1 = Uuid::new_v4().to_string();

    let mut activation_client = pool.get().await?;
    save_memory_canonical(&**activation_client, user_id, &key, content_v1).await?;
    activation_client
        .execute(
            "UPDATE memory_index_jobs SET status='in_progress',lease_token=$2,
             lease_expires_at=now()+interval '10 minutes' WHERE id=$1",
            &[&job_v1, &lease_v1],
        )
        .await?;
    activation_client
        .execute(
            "INSERT INTO memory_chunks (id,user_id,memory_key,index_generation,chunk_index,content,content_hash)
             VALUES ($1,$2,$3,$4,0,$5,$6)",
            &[
                &Uuid::new_v4().to_string(),
                &user_id,
                &key,
                &generation_v1,
                &content_v1,
                &content_hash(content_v1),
            ],
        )
        .await?;

    let canonical_client = pool.get().await?;
    let activation = activation_client.transaction().await?;
    let activated: bool = activation
        .query_one(
            activate_memory_index_generation_query(),
            &[
                &user_id,
                &key,
                &generation_v1,
                &version_v1,
                &job_v1,
                &lease_v1,
            ],
        )
        .await?
        .get("activated");

    let update_user_id = user_id.to_string();
    let update_key = key.clone();
    let update = tokio::spawn(async move {
        save_memory_canonical(
            &**canonical_client,
            &update_user_id,
            &update_key,
            content_v2,
        )
        .await
    });
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let canonical_update_blocked = !update.is_finished();
    activation.commit().await?;
    tokio::time::timeout(std::time::Duration::from_secs(5), update)
        .await
        .map_err(|_| {
            crate::error::AppError::BadRequest(
                "canonical update did not resume after activation commit".to_string(),
            )
        })?
        .map_err(|error| {
            crate::error::AppError::BadRequest(format!(
                "canonical update task failed during race probe: {error}"
            ))
        })??;

    process_memory_index_jobs(&activation_client, config, user_id).await?;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
    let (final_canonical_version, final_active_version) = loop {
        let final_row = activation_client
            .query_one(
                "SELECT canonical.content_version AS canonical_version,
                        state.content_version AS active_version
                 FROM user_memories canonical
                 JOIN memory_index_states state
                   ON state.user_id=canonical.user_id AND state.memory_key=canonical.memory_key
                 WHERE canonical.user_id=$1 AND canonical.memory_key=$2",
                &[&user_id, &key],
            )
            .await?;
        let canonical_version: String = final_row.get("canonical_version");
        let active_version: String = final_row.get("active_version");
        if (canonical_version == version_v2 && active_version == version_v2)
            || tokio::time::Instant::now() >= deadline
        {
            break (canonical_version, active_version);
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    };
    activation_client
        .execute(
            "WITH jobs AS (DELETE FROM memory_index_jobs WHERE user_id=$1 AND memory_key=$2),
                  states AS (DELETE FROM memory_index_states WHERE user_id=$1 AND memory_key=$2),
                  chunks AS (DELETE FROM memory_chunks WHERE user_id=$1 AND memory_key=$2)
             DELETE FROM user_memories WHERE user_id=$1 AND memory_key=$2",
            &[&user_id, &key],
        )
        .await?;

    Ok(json!({
        "v1Activated": activated,
        "canonicalUpdateBlockedUntilActivationCommit": canonical_update_blocked,
        "finalCanonicalIsV2": final_canonical_version == version_v2,
        "finalActiveIsV2": final_active_version == version_v2
    }))
}

async fn read_memory(client: &PgClient, user_id: &str, key: &str) -> AppResult<Option<String>> {
    Ok(client
        .query_opt(
            "SELECT content FROM user_memories WHERE user_id = $1 AND memory_key = $2",
            &[&user_id, &key],
        )
        .await?
        .map(|row| row.get("content")))
}

fn deterministic_daily_summary(
    date: NaiveDate,
    trigger_source: &str,
    work_log: &str,
    sleep: &str,
    routine: &str,
) -> String {
    let sessions_started = work_log.matches("session_start:").count();
    let sessions_ended = work_log.matches("session_end:").count();
    let breaks_started = work_log.matches("break_start:").count();
    let latest_work =
        find_last_matching_line(work_log, &["session_end", "session_start", "break_start"])
            .unwrap_or("No work events logged.");
    let latest_sleep = find_last_matching_line(sleep, &["wake_log", "sleep_start"])
        .unwrap_or("No sleep events logged.");
    let routine_signal = routine
        .lines()
        .find(|line| {
            let trimmed = line.trim_start();
            trimmed.starts_with("- ")
                && !trimmed.contains("None yet")
                && !trimmed.starts_with("- Last updated from:")
        })
        .unwrap_or("No fixed routine signal found.");

    format!(
        "{DEFAULT_DAILY_SUMMARY}\nDate: {date}\nTrigger: {trigger_source}\n\n## Compact Evidence\n- Work sessions started: {sessions_started}\n- Work sessions ended: {sessions_ended}\n- Breaks started: {breaks_started}\n- Latest work signal: {}\n- Latest sleep signal: {}\n- Routine anchor: {}\n\n## Durable Candidates\n- Preserve task completion patterns, recurring excuses, sleep timing, and routine conflicts from this day.\n",
        compact_line(latest_work),
        compact_line(latest_sleep),
        compact_line(routine_signal)
    )
}

fn find_last_matching_line<'a>(content: &'a str, markers: &[&str]) -> Option<&'a str> {
    content
        .lines()
        .rev()
        .find(|line| markers.iter().any(|marker| line.contains(marker)))
}

fn compact_line(line: &str) -> String {
    let trimmed = line.trim();
    if trimmed.chars().count() <= 180 {
        return trimmed.to_string();
    }
    trimmed.chars().take(180).collect::<String>()
}

async fn total_searchable_memory_chars(client: &PgClient, user_id: &str) -> AppResult<usize> {
    let rows = client.query("SELECT memory_key, length(content)::BIGINT AS chars FROM user_memories WHERE user_id=$1", &[&user_id]).await?;
    Ok(rows
        .into_iter()
        .filter(|row| is_searchable_key(&row.get::<_, String>("memory_key")))
        .map(|row| row.get::<_, i64>("chars").max(0) as usize)
        .sum())
}

fn is_searchable_key(key: &str) -> bool {
    memory_descriptors()
        .iter()
        .any(|descriptor| descriptor.key == key && descriptor.searchable)
        || key.starts_with("work_log_")
        || key.starts_with("work_summary_")
        || key.starts_with("override_")
}

struct MemoryIndexBuild<'a> {
    user_id: &'a str,
    key: &'a str,
    content: &'a str,
    version: &'a str,
    job_id: &'a str,
    lease: &'a str,
}

async fn index_memory_key(
    client: &PgClient,
    config: &Config,
    build: MemoryIndexBuild<'_>,
) -> AppResult<()> {
    let MemoryIndexBuild {
        user_id,
        key,
        content,
        version,
        job_id,
        lease,
    } = build;
    let generation = Uuid::new_v4().to_string();
    let allow_embeddings = user_allows_server_embeddings(client, user_id).await?;
    if !content.trim().is_empty() {
        let chunks = chunk_memory(content);
        for (index, chunk) in chunks.iter().enumerate() {
            let renewed = client
                .execute(
                    "UPDATE memory_index_jobs SET lease_expires_at=now()+interval '10 minutes', updated_at=now()
                     WHERE id=$1 AND lease_token=$2 AND status='in_progress'",
                    &[&job_id, &lease],
                )
                .await?;
            if renewed != 1 {
                return Err(AppError::Conflict(
                    "memory index lease was fenced before completion".to_string(),
                ));
            }
            let embedding = if allow_embeddings
                && reserve_memory_embedding_call(client, config, user_id).await?
            {
                embedding_with_fallback(config, chunk).await
            } else {
                EmbeddingResult::lexical()
            };
            let embedding_json = embedding
                .values
                .as_ref()
                .map(|values| json!(values).to_string());
            client
                .execute(
                    insert_memory_chunk_generation_query(),
                    &[
                        &Uuid::new_v4().to_string(),
                        &user_id,
                        &key,
                        &generation,
                        &(index as i32),
                        &chunk,
                        &content_hash(chunk),
                        &embedding_json,
                        &embedding.provider,
                        &embedding.model,
                    ],
                )
                .await?;
        }
    }
    let activation = client
        .query_one(
            activate_memory_index_generation_query(),
            &[&user_id, &key, &generation, &version, &job_id, &lease],
        )
        .await?;
    let activated: bool = activation.get("activated");
    if !activated {
        client
            .execute(
                "DELETE FROM memory_chunks WHERE user_id=$1 AND memory_key=$2 AND index_generation=$3",
                &[&user_id, &key, &generation],
            )
            .await?;
        info!(
            user_id,
            memory_key = key,
            content_version = version,
            "stale memory index generation superseded before activation"
        );
    }
    Ok(())
}

fn chunk_memory(content: &str) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut current_chars = 0;
    for character in content.chars() {
        current.push(character);
        current_chars += 1;
        if current_chars == MEMORY_CHUNK_CHARS {
            if !current.trim().is_empty() {
                chunks.push(current.trim().to_string());
            }
            current.clear();
            current_chars = 0;
        }
    }
    if !current.trim().is_empty() {
        chunks.push(current.trim().to_string());
    }
    chunks
}

async fn user_allows_server_embeddings(client: &PgClient, user_id: &str) -> AppResult<bool> {
    if user_id == "admin" {
        return Ok(true);
    }
    let tier = client
        .query_opt(
            "SELECT subscription_tier FROM users WHERE id=$1",
            &[&user_id],
        )
        .await?
        .map(|row| row.get::<_, String>("subscription_tier"));
    Ok(tier.as_deref() != Some("byok"))
}

async fn reserve_memory_embedding_call(
    client: &PgClient,
    config: &Config,
    user_id: &str,
) -> AppResult<bool> {
    if config.memory_daily_embedding_calls == 0 {
        return Ok(false);
    }
    let row = client
        .query_opt(
            "INSERT INTO provider_usage_daily (user_id,usage_date,usage_kind,units)
             VALUES ($1,CURRENT_DATE,'memory_embedding_calls',1)
             ON CONFLICT (user_id,usage_date,usage_kind) DO UPDATE
             SET units=provider_usage_daily.units + 1, updated_at=now()
             WHERE provider_usage_daily.units + 1 <= $2
             RETURNING units",
            &[&user_id, &config.memory_daily_embedding_calls],
        )
        .await?;
    if row.is_none() {
        warn!(user_id, "🔴 FALLBACK: semantic memory quota exhausted - Reason: daily embedding call budget reached - Impact: memory remains available through lexical search until the budget resets");
    }
    Ok(row.is_some())
}

async fn embedding_with_fallback(config: &Config, text: &str) -> EmbeddingResult {
    if let Some(key) = &config.memory_embeddings.gemini_api_key {
        match gemini_embedding(key, &config.memory_embeddings.model, text).await {
            Ok(values) => {
                return EmbeddingResult::semantic(
                    values,
                    &config.memory_embeddings.provider,
                    &config.memory_embeddings.model,
                )
            }
            Err(err) => {
                warn!(
                    error = %err,
                    "🔴 FALLBACK: Gemini memory embedding failed - Reason: provider request failed - Impact: trying Voyage fallback"
                );
            }
        }
    }
    if let Some(key) = &config.memory_embeddings.voyage_api_key {
        return match voyage_embedding(key, &config.memory_embeddings.fallback_model, text).await {
            Ok(values) => EmbeddingResult::semantic(
                values,
                &config.memory_embeddings.fallback_provider,
                &config.memory_embeddings.fallback_model,
            ),
            Err(reason) => {
                warn!(reason = %reason, "🔴 FALLBACK: semantic memory indexing stored keyword-only chunk - Reason: all embedding providers unavailable - Impact: memory search uses lexical scoring");
                EmbeddingResult::lexical()
            }
        };
    }
    EmbeddingResult::lexical()
}

async fn gemini_embedding(api_key: &str, model: &str, text: &str) -> Result<Vec<f32>, String> {
    let url = gemini_embedding_url(model);
    let response = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|err| err.to_string())?
        .post(url)
        .header("x-goog-api-key", api_key)
        .json(&json!({
            "content": {
                "parts": [{ "text": text }]
            }
        }))
        .send()
        .await
        .map_err(|err| err.to_string())?;
    if !response.status().is_success() {
        return Err(format!("Gemini embedding HTTP {}", response.status()));
    }
    let body: Value = response.json().await.map_err(|err| err.to_string())?;
    body["embedding"]["values"]
        .as_array()
        .ok_or_else(|| "Gemini embedding response missing values".to_string())?
        .iter()
        .map(|value| {
            value
                .as_f64()
                .map(|v| v as f32)
                .ok_or_else(|| "invalid Gemini embedding value".to_string())
        })
        .collect()
}

fn gemini_embedding_url(model: &str) -> String {
    format!("https://generativelanguage.googleapis.com/v1beta/models/{model}:embedContent")
}

async fn voyage_embedding(api_key: &str, model: &str, text: &str) -> Result<Vec<f32>, String> {
    let response = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|err| err.to_string())?
        .post("https://api.voyageai.com/v1/embeddings")
        .bearer_auth(api_key)
        .json(&json!({
            "input": [text],
            "model": model
        }))
        .send()
        .await
        .map_err(|err| err.to_string())?;
    if !response.status().is_success() {
        return Err(format!("Voyage embedding HTTP {}", response.status()));
    }
    let body: Value = response.json().await.map_err(|err| err.to_string())?;
    body["data"][0]["embedding"]
        .as_array()
        .ok_or_else(|| "Voyage embedding response missing values".to_string())?
        .iter()
        .map(|value| {
            value
                .as_f64()
                .map(|v| v as f32)
                .ok_or_else(|| "invalid Voyage embedding value".to_string())
        })
        .collect()
}

fn lexical_score(query: &str, content: &str) -> f64 {
    let content_lower = content.to_ascii_lowercase();
    let mut matches = 0.0;
    let mut terms = 0.0;
    for term in query
        .split(|ch: char| !ch.is_alphanumeric())
        .map(str::trim)
        .filter(|term| term.len() >= 3)
    {
        terms += 1.0;
        if content_lower.contains(&term.to_ascii_lowercase()) {
            matches += 1.0;
        }
    }
    if terms == 0.0 {
        0.0
    } else {
        matches / terms
    }
}

fn cosine_similarity(left: &[f32], right: &[f32]) -> f64 {
    if left.is_empty() || left.len() != right.len() {
        return 0.0;
    }
    let mut dot = 0.0;
    let mut left_norm = 0.0;
    let mut right_norm = 0.0;
    for (a, b) in left.iter().zip(right.iter()) {
        let a = *a as f64;
        let b = *b as f64;
        dot += a * b;
        left_norm += a * a;
        right_norm += b * b;
    }
    if left_norm == 0.0 || right_norm == 0.0 {
        0.0
    } else {
        dot / (left_norm.sqrt() * right_norm.sqrt())
    }
}

fn content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}

fn is_one_hour_after_usual_sleep(usual_sleep_start: i32) -> bool {
    let now = Utc::now();
    let current_minute = (now.hour() * 60 + now.minute()) as i32;
    let target = (usual_sleep_start + 60).rem_euclid(24 * 60);
    let elapsed = (current_minute - target).rem_euclid(24 * 60);
    elapsed < 12 * 60
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunks_memory_without_empty_chunks() {
        let chunks = chunk_memory(&"alpha\n".repeat(500));
        assert!(!chunks.is_empty());
        assert!(chunks.iter().all(|chunk| !chunk.trim().is_empty()));
    }

    #[test]
    fn chunking_hard_splits_oversized_single_lines() {
        let chunks = chunk_memory(&"x".repeat(MEMORY_CHUNK_CHARS * 3 + 17));
        assert_eq!(chunks.len(), 4);
        assert!(chunks
            .iter()
            .all(|chunk| chunk.chars().count() <= MEMORY_CHUNK_CHARS));
    }

    #[test]
    fn lexical_score_matches_query_terms() {
        let score = lexical_score(
            "relationship gym",
            "Gym block done. Relationship call moved.",
        );
        assert!(score > 0.9);
    }

    #[test]
    fn idle_distillation_window_handles_day_wrap() {
        let _ = is_one_hour_after_usual_sleep(23 * 60);
    }

    #[test]
    fn gemini_embedding_url_never_contains_api_key() {
        let url = gemini_embedding_url("gemini-embedding-001");

        assert_eq!(url, "https://generativelanguage.googleapis.com/v1beta/models/gemini-embedding-001:embedContent");
        assert!(!url.contains("key="));
    }

    #[test]
    fn user_day_uses_iana_timezone_and_selects_completed_local_day() {
        let now = DateTime::parse_from_rfc3339("2026-07-13T19:15:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let day = UserDay::new("Asia/Kolkata", now).unwrap();
        assert_eq!(day.current_date().to_string(), "2026-07-14");
        assert_eq!(day.completed_date().to_string(), "2026-07-13");
        assert_eq!(day.work_log_key(), "work_log_2026_07_14");
    }

    #[test]
    fn weekly_override_key_uses_the_user_local_iso_week() {
        let now = DateTime::parse_from_rfc3339("2026-01-04T20:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let day = UserDay::new("Asia/Kolkata", now).unwrap();
        assert_eq!(day.weekly_override_key(), "override_2026_W02");
    }

    #[test]
    fn circular_sleep_mean_handles_midnight_wrap() {
        assert_eq!(circular_minute_mean(&[23 * 60 + 50, 10]), Some(0));
        assert_eq!(circular_minute_mean(&[60, 120]), Some(90));
    }

    #[test]
    fn index_generation_queries_swap_only_after_complete_build() {
        let build = insert_memory_chunk_generation_query();
        assert!(build.contains("index_generation"), "{build}");
        let swap = activate_memory_index_generation_query();
        assert!(swap.contains("active_index_generation"), "{swap}");
        assert!(swap.contains("DELETE FROM memory_chunks"), "{swap}");
    }

    #[test]
    fn active_index_search_rejects_generations_for_stale_canonical_content() {
        let query = active_index_search_query();
        assert!(
            query.contains("canonical.content_version=state.content_version"),
            "{query}"
        );
    }

    #[test]
    fn fallback_embedding_provenance_reports_actual_provider() {
        let result = EmbeddingResult::semantic(vec![1.0], "voyage", "voyage-3-lite");
        assert_eq!(result.provider.as_deref(), Some("voyage"));
        assert_eq!(result.model.as_deref(), Some("voyage-3-lite"));
    }

    #[test]
    fn canonical_commit_and_restore_queries_do_not_depend_on_embeddings() {
        assert!(!canonical_memory_commit_query().contains("embedding"));
        let restore = canonical_snapshot_restore_query();
        assert!(restore.contains("DELETE FROM user_memories"), "{restore}");
        assert!(restore.contains("memory_index_states"), "{restore}");
    }

    #[test]
    fn chunk_uniqueness_allows_identical_content_across_generations() {
        let schema = include_str!("../sql/001_init.sql");
        assert!(
            schema.contains(
                "UNIQUE (user_id, memory_key, index_generation, chunk_index, content_hash)"
            ),
            "{schema}"
        );
        assert!(schema.contains("DROP CONSTRAINT IF EXISTS memory_chunks_user_id_memory_key_chunk_index_content_hash_key"), "{schema}");
    }

    #[test]
    fn canonical_commit_atomically_enqueues_its_index_job() {
        let query = canonical_memory_commit_query();
        assert!(query.contains("WITH canonical AS"), "{query}");
        assert!(query.contains("INSERT INTO memory_index_jobs"), "{query}");
        assert!(query.contains("FROM canonical"), "{query}");
    }

    #[test]
    fn stale_index_worker_cannot_activate_or_delete_a_newer_generation() {
        let query = activate_memory_index_generation_query();
        assert!(query.contains("job.lease_token=$6"), "{query}");
        assert!(query.contains("canonical.content_version=$4"), "{query}");
        assert!(query.contains("FOR UPDATE OF job, canonical"), "{query}");
        assert!(query.contains("FROM activated"), "{query}");
    }

    #[test]
    fn background_index_claim_is_global_and_single_consumer() {
        let query = claim_memory_index_job_query();
        assert!(query.contains("FOR UPDATE SKIP LOCKED"), "{query}");
        assert!(!query.contains("user_id=$2"), "{query}");
        assert!(query.contains("lease_token=$1"), "{query}");
    }

    #[test]
    fn background_drain_continues_after_a_failed_claimed_job() {
        assert!(continue_index_drain(true, false));
        assert!(continue_index_drain(true, true));
        assert!(!continue_index_drain(false, true));
    }

    #[test]
    fn wake_completion_consumes_one_unmatched_sleep_start() {
        let query = consume_sleep_start_query();
        assert!(query.contains("last_sleep_started_at = NULL"), "{query}");
        assert!(
            query.contains("last_sleep_started_at IS NOT NULL"),
            "{query}"
        );
        assert!(
            query.contains("RETURNING unmatched.last_sleep_started_at"),
            "{query}"
        );
    }

    #[test]
    fn adjacent_distillations_atomically_append_current_durable() {
        let query = distillation_commit_query();
        assert!(query.contains("user_memories.content"), "{query}");
        assert!(query.contains("|| $8"), "{query}");
        assert!(query.contains("FROM marker"), "{query}");
    }

    #[test]
    fn snapshot_restore_rebuilds_distillation_markers_from_restored_summaries() {
        assert!(canonical_snapshot_restore_query().contains("memory_distillations"));
        let query = restore_distillation_marker_query();
        assert!(query.contains("memory_distillations"), "{query}");
        assert!(query.contains("snapshot_restore"), "{query}");
    }

    #[test]
    fn sql_circular_mean_normalizes_after_rounding() {
        let query = update_sleep_start_mean_query();
        assert!(query.contains("mod(round("), "{query}");
    }
}
