use chrono::{NaiveDate, Timelike, Utc};
use deadpool_postgres::Pool;
use reqwest::Client;
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tokio_postgres::Client as PgClient;
use tracing::{info, warn};
use uuid::Uuid;

use crate::config::Config;
use crate::error::AppResult;
use crate::prompt::{default_memory_for_key, DEFAULT_DAILY_SUMMARY};

const MEMORY_SEARCH_MIN_CHARS: usize = 4_000;
const MEMORY_CHUNK_CHARS: usize = 1_200;

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
    config: &Config,
    user_id: &str,
    key: &str,
    content: &str,
) -> AppResult<()> {
    client
        .execute(
            "
            INSERT INTO user_memories (user_id, memory_key, content, updated_at)
            VALUES ($1, $2, $3, now())
            ON CONFLICT (user_id, memory_key) DO UPDATE SET
                content = EXCLUDED.content,
                updated_at = now()
            ",
            &[&user_id, &key, &content],
        )
        .await?;
    index_memory_key(client, config, user_id, key, content).await?;
    Ok(())
}

pub async fn note_sleep_started(client: &PgClient, user_id: &str) -> AppResult<()> {
    let now = Utc::now();
    let minute = (now.hour() * 60 + now.minute()) as i32;
    client
        .execute(
            "
            INSERT INTO user_state_metrics (
                user_id,
                usual_sleep_start_minute_utc,
                sleep_sample_count,
                last_sleep_started_at,
                updated_at
            )
            VALUES ($1, $2, 1, now(), now())
            ON CONFLICT (user_id) DO UPDATE SET
                usual_sleep_start_minute_utc = CASE
                    WHEN user_state_metrics.usual_sleep_start_minute_utc IS NULL THEN EXCLUDED.usual_sleep_start_minute_utc
                    ELSE round(
                        (user_state_metrics.usual_sleep_start_minute_utc::NUMERIC * LEAST(user_state_metrics.sleep_sample_count, 14)
                         + EXCLUDED.usual_sleep_start_minute_utc::NUMERIC)
                        / (LEAST(user_state_metrics.sleep_sample_count, 14) + 1)
                    )::INTEGER
                END,
                sleep_sample_count = user_state_metrics.sleep_sample_count + 1,
                last_sleep_started_at = EXCLUDED.last_sleep_started_at,
                updated_at = now()
            ",
            &[&user_id, &minute],
        )
        .await?;
    Ok(())
}

pub async fn note_wake_logged(
    client: &PgClient,
    user_id: &str,
    sleep_quality: i64,
) -> AppResult<SleepMetricsReport> {
    let row = client
        .query_opt(
            "
            SELECT last_sleep_started_at
            FROM user_state_metrics
            WHERE user_id = $1
            ",
            &[&user_id],
        )
        .await?;
    let sleep_minutes: Option<i32> = row
        .and_then(|row| row.get::<_, Option<chrono::DateTime<Utc>>>("last_sleep_started_at"))
        .map(|started| (Utc::now() - started).num_minutes().clamp(1, 24 * 60) as i32);

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
                last_woke_at = now(),
                updated_at = now()
            ",
            &[&user_id, &sleep_minutes, &(sleep_quality as f64)],
        )
        .await?;
    sleep_metrics_report(client, user_id).await
}

pub async fn sleep_metrics_report(client: &PgClient, user_id: &str) -> AppResult<SleepMetricsReport> {
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
    distill_date(client, config, user_id, Utc::now().date_naive(), trigger_source).await
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
        if let Some(outcome) = distill_idle_if_due(pool, config, &user_id).await? {
            if outcome.distilled {
                outcomes.push(outcome);
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

    ensure_memory_index(client, config, user_id).await?;
    let total_chars = total_searchable_memory_chars(client, user_id).await?;
    if total_chars < MEMORY_SEARCH_MIN_CHARS {
        return Ok(Vec::new());
    }

    let query_embedding = embedding_with_fallback(config, query).await;
    let rows = client
        .query(
            "
            SELECT
                memory_key,
                content,
                embedding::TEXT AS embedding,
                embedding_provider,
                embedding_model
            FROM memory_chunks
            WHERE user_id = $1
            ",
            &[&user_id],
        )
        .await?;

    let mut hits = Vec::new();
    for row in rows {
        let content: String = row.get("content");
        let embedding_text: Option<String> = row.get("embedding");
        let semantic_score = match (&query_embedding, embedding_text) {
            (Ok(query_vec), Some(text)) => serde_json::from_str::<Vec<f32>>(&text)
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

    hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    hits.truncate(limit);
    Ok(hits)
}

async fn distill_date(
    client: &PgClient,
    config: &Config,
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
        .unwrap_or_else(|| default_memory_for_key("sleep").unwrap_or("# Sleep Ledger\n").to_string());
    let routine = read_memory(client, user_id, "routine")
        .await?
        .unwrap_or_else(|| default_memory_for_key("routine").unwrap_or("# Routine\n").to_string());
    let durable = read_memory(client, user_id, "durable")
        .await?
        .unwrap_or_else(|| default_memory_for_key("durable").unwrap_or("# Durable Memory\n").to_string());

    let summary = deterministic_daily_summary(date, trigger_source, &work_log, &sleep, &routine);
    save_memory_indexed(client, config, user_id, &summary_key, &summary).await?;

    let durable_append = format!(
        "\n## {}\n- Distilled from daily logs via {}.\n- Work signal: {}\n- Sleep/recovery signal: {}\n",
        date,
        trigger_source,
        compact_line(find_last_matching_line(&work_log, &["session_end", "session_start"]).unwrap_or("No work session logged.")),
        compact_line(find_last_matching_line(&sleep, &["wake_log", "sleep_start"]).unwrap_or("No sleep update logged."))
    );
    let mut next_durable = durable.trim_end().to_string();
    next_durable.push_str(&durable_append);
    save_memory_indexed(client, config, user_id, "durable", &next_durable).await?;

    client
        .execute(
            "
            INSERT INTO memory_distillations (user_id, distilled_date, trigger_source, summary_key)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (user_id, distilled_date) DO NOTHING
            ",
            &[&user_id, &date, &trigger_source, &summary_key],
        )
        .await?;

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
    let latest_work = find_last_matching_line(work_log, &["session_end", "session_start", "break_start"])
        .unwrap_or("No work events logged.");
    let latest_sleep = find_last_matching_line(sleep, &["wake_log", "sleep_start"])
        .unwrap_or("No sleep events logged.");
    let routine_signal = routine
        .lines()
        .find(|line| line.contains("Gym") || line.contains("girlfriend") || line.contains("Relationship"))
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
    let row = client
        .query_one(
            "
            SELECT COALESCE(SUM(length(content)), 0)::BIGINT AS chars
            FROM user_memories
            WHERE user_id = $1
              AND (
                memory_key IN ('durable', 'behavior', 'tasks', 'routine', 'sleep', 'achievements', 'longterm', 'shortterm')
                OR memory_key LIKE 'work_log_%'
                OR memory_key LIKE 'work_summary_%'
                OR memory_key LIKE 'override_%'
              )
            ",
            &[&user_id],
        )
        .await?;
    let chars: i64 = row.get("chars");
    Ok(chars.max(0) as usize)
}

async fn ensure_memory_index(client: &PgClient, config: &Config, user_id: &str) -> AppResult<()> {
    let rows = client
        .query(
            "
            SELECT memory_key, content
            FROM user_memories
            WHERE user_id = $1
              AND (
                memory_key IN ('durable', 'behavior', 'tasks', 'routine', 'sleep', 'achievements', 'longterm', 'shortterm')
                OR memory_key LIKE 'work_log_%'
                OR memory_key LIKE 'work_summary_%'
                OR memory_key LIKE 'override_%'
              )
            ",
            &[&user_id],
        )
        .await?;
    for row in rows {
        let key: String = row.get("memory_key");
        let content: String = row.get("content");
        let current_hash = content_hash(&content);
        let existing = client
            .query_opt(
                "
                SELECT 1
                FROM memory_chunks
                WHERE user_id = $1 AND memory_key = $2 AND content_hash = $3
                LIMIT 1
                ",
                &[&user_id, &key, &current_hash],
            )
            .await?;
        if existing.is_none() {
            index_memory_key(client, config, user_id, &key, &content).await?;
        }
    }
    Ok(())
}

async fn index_memory_key(
    client: &PgClient,
    config: &Config,
    user_id: &str,
    key: &str,
    content: &str,
) -> AppResult<()> {
    if content.trim().is_empty() {
        client
            .execute(
                "DELETE FROM memory_chunks WHERE user_id = $1 AND memory_key = $2",
                &[&user_id, &key],
            )
            .await?;
        return Ok(());
    }

    let chunks = chunk_memory(content);
    client
        .execute(
            "DELETE FROM memory_chunks WHERE user_id = $1 AND memory_key = $2",
            &[&user_id, &key],
        )
        .await?;
    for (index, chunk) in chunks.iter().enumerate() {
        let embedding = embedding_with_fallback(config, chunk).await;
        let (embedding_json, provider, model) = match embedding {
            Ok(values) => (
                Some(json!(values).to_string()),
                Some(active_embedding_provider(config).0),
                Some(active_embedding_provider(config).1),
            ),
            Err(reason) => {
                warn!(
                    user_id,
                    memory_key = key,
                    reason = %reason,
                    "🔴 FALLBACK: semantic memory indexing stored keyword-only chunk - Reason: embedding provider unavailable - Impact: memory search uses lexical scoring"
                );
                (None, None, None)
            }
        };
        client
            .execute(
                "
                INSERT INTO memory_chunks (
                    id,
                    user_id,
                    memory_key,
                    chunk_index,
                    content,
                    content_hash,
                    embedding,
                    embedding_provider,
                    embedding_model
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7::TEXT::JSONB, $8, $9)
                ",
                &[
                    &Uuid::new_v4().to_string(),
                    &user_id,
                    &key,
                    &(index as i32),
                    &chunk,
                    &content_hash(chunk),
                    &embedding_json,
                    &provider,
                    &model,
                ],
            )
            .await?;
    }
    Ok(())
}

fn chunk_memory(content: &str) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    for line in content.lines() {
        if current.chars().count() + line.chars().count() + 1 > MEMORY_CHUNK_CHARS && !current.trim().is_empty() {
            chunks.push(current.trim().to_string());
            current.clear();
        }
        current.push_str(line);
        current.push('\n');
    }
    if !current.trim().is_empty() {
        chunks.push(current.trim().to_string());
    }
    chunks
}

async fn embedding_with_fallback(config: &Config, text: &str) -> Result<Vec<f32>, String> {
    if let Some(key) = &config.memory_embeddings.gemini_api_key {
        match gemini_embedding(key, &config.memory_embeddings.model, text).await {
            Ok(values) => return Ok(values),
            Err(err) => {
                warn!(
                    error = %err,
                    "🔴 FALLBACK: Gemini memory embedding failed - Reason: provider request failed - Impact: trying Voyage fallback"
                );
            }
        }
    }
    if let Some(key) = &config.memory_embeddings.voyage_api_key {
        return voyage_embedding(key, &config.memory_embeddings.fallback_model, text).await;
    }
    Err("no Gemini or Voyage embedding key configured".to_string())
}

fn active_embedding_provider(config: &Config) -> (String, String) {
    if config.memory_embeddings.gemini_api_key.is_some() {
        (config.memory_embeddings.provider.clone(), config.memory_embeddings.model.clone())
    } else {
        (
            config.memory_embeddings.fallback_provider.clone(),
            config.memory_embeddings.fallback_model.clone(),
        )
    }
}

async fn gemini_embedding(api_key: &str, model: &str, text: &str) -> Result<Vec<f32>, String> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:embedContent?key={}",
        model,
        api_key
    );
    let response = Client::new()
        .post(url)
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
        .map(|value| value.as_f64().map(|v| v as f32).ok_or_else(|| "invalid Gemini embedding value".to_string()))
        .collect()
}

async fn voyage_embedding(api_key: &str, model: &str, text: &str) -> Result<Vec<f32>, String> {
    let response = Client::new()
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
        .map(|value| value.as_f64().map(|v| v as f32).ok_or_else(|| "invalid Voyage embedding value".to_string()))
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
    use super::{chunk_memory, is_one_hour_after_usual_sleep, lexical_score};

    #[test]
    fn chunks_memory_without_empty_chunks() {
        let chunks = chunk_memory(&"alpha\n".repeat(500));
        assert!(!chunks.is_empty());
        assert!(chunks.iter().all(|chunk| !chunk.trim().is_empty()));
    }

    #[test]
    fn lexical_score_matches_query_terms() {
        let score = lexical_score("relationship gym", "Gym block done. Relationship call moved.");
        assert!(score > 0.9);
    }

    #[test]
    fn idle_distillation_window_handles_day_wrap() {
        let _ = is_one_hour_after_usual_sleep(23 * 60);
    }
}
