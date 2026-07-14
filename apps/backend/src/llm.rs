use chrono::{DateTime, FixedOffset, SecondsFormat, Utc};
use deadpool_postgres::Pool;
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::sync::OnceLock;
use std::time::Duration;
use tokio_postgres::GenericClient;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::alarm::{persist_alarm, AlarmWrite};
use crate::config::Config;
use crate::error::{AppError, AppResult};
use crate::memory::{
    distill_idle_if_due, distill_today, note_sleep_started, note_wake_logged,
    process_memory_index_jobs, search_memory, sleep_metrics_report, user_day_for,
};
use crate::models::AlarmKind;
use crate::prompt::{
    build_coach_system_prompt, default_memory_for_key, memory_descriptor, memory_descriptors,
    BuiltPrompt, MemorySection, PromptBuildReport, PromptContext, DEFAULT_COACH_TODO,
    DEFAULT_DAILY_SUMMARY, DEFAULT_MISCELLANEOUS_TODO, DEFAULT_ROUTINE, DEFAULT_SLEEP,
    DEFAULT_TASKS, DEFAULT_WORK_LOG,
};
use crate::secrets::decrypt_byok_key;

const EARLY_SESSION_MINIMUM_MINUTES: i64 = 5;
const MAX_TOOL_TEXT_CHARS: usize = 4_000;
const MAX_PATCH_CHARS: usize = 100_000;
const TURN_LEASE_MINUTES: i32 = 10;
const MAX_ALARM_WAKE_DRAIN: usize = 50;

#[derive(Debug, Clone, PartialEq, Eq)]
enum ToolOutcome {
    Success { message: String },
    Failure { message: String },
}

impl ToolOutcome {
    fn success(message: impl Into<String>) -> Self {
        Self::Success {
            message: message.into(),
        }
    }

    fn failure(message: impl Into<String>) -> Self {
        Self::Failure {
            message: message.into(),
        }
    }

    fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }

    fn message(&self) -> &str {
        match self {
            Self::Success { message } | Self::Failure { message } => message,
        }
    }

    fn provider_content(&self) -> String {
        match self {
            Self::Success { message } => json!({ "ok": true, "message": message }).to_string(),
            Self::Failure { message } => json!({ "ok": false, "error": message }).to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PatchFileInput {
    file_path: String,
    patch: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StartSessionInput {
    task_id: String,
    estimated_minutes: i64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EndSessionInput {
    actual_minutes: i64,
    productive_level: i64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExtendSessionInput {
    extension_minutes: i64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StartBreakInput {
    duration_minutes: i64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StartSleepInput {
    estimated_hours: f64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LogWakeInput {
    sleep_quality: i64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StartVacationInput {
    reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EmptyToolInput {}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WakeUpAlarmInput {
    wake_time: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LogOverrideInput {
    override_what: String,
    reasoning: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MemorySearchInput {
    query: String,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SetRoutineCategoriesInput {
    categories: Vec<RoutineCategoryInput>,
    source: Option<String>,
}

#[derive(Debug)]
enum ToolInput {
    PatchFile(PatchFileInput),
    StartSession(StartSessionInput),
    EndSession(EndSessionInput),
    ExtendSession(ExtendSessionInput),
    StartBreak(StartBreakInput),
    StartSleep(StartSleepInput),
    LogWake(LogWakeInput),
    StartVacation(StartVacationInput),
    EndVacation(EmptyToolInput),
    WakeUpAlarm(WakeUpAlarmInput),
    LogOverride(LogOverrideInput),
    MemorySearch(MemorySearchInput),
    SetRoutineCategories(SetRoutineCategoriesInput),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AtomicFailureInjection {
    None,
    AfterCanonicalBeforeOutcome,
}

fn injected_atomic_failure(injection: AtomicFailureInjection) -> Option<AppError> {
    (injection == AtomicFailureInjection::AfterCanonicalBeforeOutcome).then(|| {
        AppError::BadRequest(
            "simulated failure between canonical effect and tool outcome".to_string(),
        )
    })
}

#[derive(Debug, Clone, Copy)]
enum ProviderFailureKind {
    Transport,
    Json,
    MissingMessage,
    WrongToolCallsType,
    MalformedToolCall,
    EmptyResponse,
}

fn provider_fallback_message(kind: ProviderFailureKind) -> &'static str {
    match kind {
        ProviderFailureKind::Transport => "🔴 FALLBACK: coach provider transport failed - Reason: request could not complete - Impact: no reply committed",
        ProviderFailureKind::Json => "🔴 FALLBACK: coach provider JSON rejected - Reason: response was not valid JSON - Impact: no reply committed",
        ProviderFailureKind::MissingMessage => "🔴 FALLBACK: coach provider response rejected - Reason: missing choices[0].message - Impact: no reply committed",
        ProviderFailureKind::WrongToolCallsType => "🔴 FALLBACK: coach tool calls rejected - Reason: tool_calls was not an array - Impact: no action executed",
        ProviderFailureKind::MalformedToolCall => "🔴 FALLBACK: coach tool calls rejected - Reason: malformed provider tool call - Impact: no action executed",
        ProviderFailureKind::EmptyResponse => "🔴 FALLBACK: coach provider response rejected - Reason: no content or tool calls - Impact: no reply committed",
    }
}

fn decode_tool_batch(calls: &[LlmToolCall]) -> Result<Vec<ToolInput>, String> {
    calls
        .iter()
        .map(|call| decode_tool_input(&call.function.name, &call.function.arguments))
        .collect()
}

fn validate_request_hash(stored: &str, incoming: &str) -> Result<(), String> {
    if stored == incoming {
        Ok(())
    } else {
        Err("requestId was already used with different message content".to_string())
    }
}

fn sha256_hex(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}

fn tool_call_fingerprint(call: &LlmToolCall) -> String {
    let arguments = serde_json::from_str::<Value>(&call.function.arguments)
        .map(canonical_json)
        .unwrap_or_else(|_| call.function.arguments.clone());
    sha256_hex(&format!(
        "{}\n{}\n{}",
        call.id, call.function.name, arguments
    ))
}

fn canonical_json(value: Value) -> String {
    match value {
        Value::Object(object) => {
            let mut entries = object.into_iter().collect::<Vec<_>>();
            entries.sort_by(|left, right| left.0.cmp(&right.0));
            let body = entries
                .into_iter()
                .map(|(key, value)| {
                    format!(
                        "{}:{}",
                        serde_json::to_string(&key).expect("JSON object key serializes"),
                        canonical_json(value)
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{body}}}")
        }
        Value::Array(values) => format!(
            "[{}]",
            values
                .into_iter()
                .map(canonical_json)
                .collect::<Vec<_>>()
                .join(",")
        ),
        scalar => scalar.to_string(),
    }
}

async fn load_tool_outcome<C>(
    client: &C,
    turn_id: &str,
    fingerprint: &str,
) -> AppResult<Option<ToolOutcome>>
where
    C: GenericClient + Sync,
{
    let row = client
        .query_opt(
            "SELECT succeeded, message FROM chat_tool_outcomes
             WHERE turn_id = $1 AND call_fingerprint = $2",
            &[&turn_id, &fingerprint],
        )
        .await?;
    Ok(row.map(|row| {
        let succeeded: bool = row.get("succeeded");
        let message: String = row.get("message");
        if succeeded {
            ToolOutcome::success(message)
        } else {
            ToolOutcome::failure(message)
        }
    }))
}

async fn save_tool_outcome<C>(
    client: &C,
    turn_id: &str,
    call: &LlmToolCall,
    fingerprint: &str,
    outcome: &ToolOutcome,
) -> AppResult<()>
where
    C: GenericClient + Sync,
{
    client
        .execute(
            "INSERT INTO chat_tool_outcomes
                (turn_id, call_fingerprint, tool_name, arguments_hash, succeeded, message)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (turn_id, call_fingerprint) DO NOTHING",
            &[
                &turn_id,
                &fingerprint,
                &call.function.name,
                &sha256_hex(&call.function.arguments),
                &outcome.is_success(),
                &outcome.message(),
            ],
        )
        .await?;
    Ok(())
}

#[cfg(test)]
async fn resolve_tool_outcome<F, Fut>(existing: Option<ToolOutcome>, execute: F) -> ToolOutcome
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = ToolOutcome>,
{
    match existing {
        Some(outcome) => outcome,
        None => execute().await,
    }
}

fn parse_tool_args<T: for<'de> Deserialize<'de>>(name: &str, args: &str) -> Result<T, String> {
    serde_json::from_str(args).map_err(|err| format!("invalid arguments for {name}: {err}"))
}

fn validate_text(name: &str, value: &str, max_chars: usize) -> Result<(), String> {
    let length = value.chars().count();
    if value.trim().is_empty() {
        return Err(format!("{name} is required"));
    }
    if length > max_chars {
        return Err(format!("{name} exceeds the {max_chars} character limit"));
    }
    Ok(())
}

fn validate_range(name: &str, value: i64, min: i64, max: i64) -> Result<(), String> {
    if !(min..=max).contains(&value) {
        return Err(format!("{name} must be between {min} and {max}"));
    }
    Ok(())
}

fn decode_tool_input(name: &str, args: &str) -> Result<ToolInput, String> {
    match name {
        "patch_file" => {
            let input: PatchFileInput = parse_tool_args(name, args)?;
            validate_text("file_path", &input.file_path, 64)?;
            validate_text("patch", &input.patch, MAX_PATCH_CHARS)?;
            validate_dated_memory_path(&input.file_path)?;
            Ok(ToolInput::PatchFile(input))
        }
        "start_session" => {
            let input: StartSessionInput = parse_tool_args(name, args)?;
            validate_text("task_id", &input.task_id, MAX_TOOL_TEXT_CHARS)?;
            validate_range("estimated_minutes", input.estimated_minutes, 1, 1_440)?;
            Ok(ToolInput::StartSession(input))
        }
        "end_session" => {
            let input: EndSessionInput = parse_tool_args(name, args)?;
            validate_range("actual_minutes", input.actual_minutes, 1, 1_440)?;
            validate_range("productive_level", input.productive_level, 0, 100)?;
            Ok(ToolInput::EndSession(input))
        }
        "extend_session" => {
            let input: ExtendSessionInput = parse_tool_args(name, args)?;
            validate_range("extension_minutes", input.extension_minutes, 1, 1_440)?;
            Ok(ToolInput::ExtendSession(input))
        }
        "start_break" => {
            let input: StartBreakInput = parse_tool_args(name, args)?;
            validate_range("duration_minutes", input.duration_minutes, 1, 1_440)?;
            Ok(ToolInput::StartBreak(input))
        }
        "start_sleep" => {
            let input: StartSleepInput = parse_tool_args(name, args)?;
            if !input.estimated_hours.is_finite() || !(0.5..=24.0).contains(&input.estimated_hours)
            {
                return Err("estimated_hours must be between 0.5 and 24".to_string());
            }
            Ok(ToolInput::StartSleep(input))
        }
        "log_wake" => {
            let input: LogWakeInput = parse_tool_args(name, args)?;
            validate_range("sleep_quality", input.sleep_quality, 1, 5)?;
            Ok(ToolInput::LogWake(input))
        }
        "start_vacation" => {
            let input: StartVacationInput = parse_tool_args(name, args)?;
            validate_text("reason", &input.reason, MAX_TOOL_TEXT_CHARS)?;
            Ok(ToolInput::StartVacation(input))
        }
        "end_vacation" => Ok(ToolInput::EndVacation(parse_tool_args(name, args)?)),
        "wake_up_alarm" => {
            let input: WakeUpAlarmInput = parse_tool_args(name, args)?;
            if let Some(wake_time) = &input.wake_time {
                DateTime::parse_from_rfc3339(wake_time)
                    .map_err(|_| "wake_time must be an RFC3339 timestamp".to_string())?;
            }
            Ok(ToolInput::WakeUpAlarm(input))
        }
        "log_override" => {
            let input: LogOverrideInput = parse_tool_args(name, args)?;
            validate_text("override_what", &input.override_what, MAX_TOOL_TEXT_CHARS)?;
            validate_text("reasoning", &input.reasoning, MAX_TOOL_TEXT_CHARS)?;
            Ok(ToolInput::LogOverride(input))
        }
        "memory_search" => {
            let input: MemorySearchInput = parse_tool_args(name, args)?;
            validate_text("query", &input.query, MAX_TOOL_TEXT_CHARS)?;
            if let Some(limit) = input.limit {
                validate_range("limit", limit, 1, 8)?;
            }
            Ok(ToolInput::MemorySearch(input))
        }
        "set_routine_categories" => {
            let input: SetRoutineCategoriesInput = parse_tool_args(name, args)?;
            if input.categories.len() > 50 {
                return Err("categories must contain at most 50 items".to_string());
            }
            for category in &input.categories {
                validate_text("category.name", &category.name, 120)?;
                validate_text("category.description", &category.description, 1_000)?;
                if let Some(cadence) = &category.cadence {
                    validate_text("category.cadence", cadence, 120)?;
                }
                if let Some(minutes) = category.target_minutes {
                    validate_range("category.target_minutes", minutes, 1, 1_440)?;
                }
            }
            if let Some(source) = &input.source {
                validate_text("source", source, MAX_TOOL_TEXT_CHARS)?;
            }
            Ok(ToolInput::SetRoutineCategories(input))
        }
        other => Err(format!("unknown tool {other}")),
    }
}

fn validate_dated_memory_path(file_path: &str) -> Result<(), String> {
    let looks_dated = file_path.contains("_WorkLog.md")
        || file_path.contains("_Summary.md")
        || file_path.as_bytes().first().is_some_and(u8::is_ascii_digit);
    if !looks_dated {
        return Ok(());
    }
    let date = file_path
        .strip_suffix("_WorkLog.md")
        .or_else(|| file_path.strip_suffix("_Summary.md"))
        .ok_or_else(|| "dated memory path must end in _WorkLog.md or _Summary.md".to_string())?;
    chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| "dated memory path must contain a real YYYY-MM-DD date".to_string())?;
    Ok(())
}

#[derive(Serialize)]
struct GcpClaims {
    iss: String,
    scope: String,
    aud: String,
    exp: i64,
    iat: i64,
}

#[derive(Deserialize)]
struct GcpTokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct GcpCredentials {
    project_id: String,
    private_key: String,
    client_email: String,
    token_uri: String,
}

#[derive(Clone)]
struct CachedVertexToken {
    access_token: String,
    project_id: String,
    expires_at: i64,
}

static VERTEX_TOKEN_CACHE: OnceLock<tokio::sync::Mutex<Option<CachedVertexToken>>> =
    OnceLock::new();

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RoutineCategoryInput {
    name: String,
    description: String,
    #[serde(default)]
    cadence: Option<String>,
    #[serde(default)]
    target_minutes: Option<i64>,
}

async fn get_vertex_access_token() -> Result<(String, String), AppError> {
    let cache = VERTEX_TOKEN_CACHE.get_or_init(|| tokio::sync::Mutex::new(None));
    let mut cached = cache.lock().await;
    if let Some(token) = cached
        .as_ref()
        .filter(|token| token.expires_at > Utc::now().timestamp() + 60)
    {
        return Ok((token.access_token.clone(), token.project_id.clone()));
    }
    let creds_json = std::env::var("GOOGLE_CLOUD_CREDENTIALS").map_err(|_| {
        AppError::BadRequest("GOOGLE_CLOUD_CREDENTIALS env var not set".to_string())
    })?;

    let creds: GcpCredentials = serde_json::from_str(&creds_json).map_err(|e| {
        AppError::BadRequest(format!("Failed to parse GOOGLE_CLOUD_CREDENTIALS: {}", e))
    })?;

    let iat = Utc::now().timestamp();
    let exp = iat + 3600;

    let claims = GcpClaims {
        iss: creds.client_email.clone(),
        scope: "https://www.googleapis.com/auth/cloud-platform".to_string(),
        aud: creds.token_uri.clone(),
        exp,
        iat,
    };

    let private_key = creds.private_key.replace("\\n", "\n");
    let key = EncodingKey::from_rsa_pem(private_key.as_bytes())
        .map_err(|e| AppError::BadRequest(format!("Failed to parse private key: {}", e)))?;

    let header = Header::new(Algorithm::RS256);
    let jwt = jsonwebtoken::encode(&header, &claims, &key)
        .map_err(|e| AppError::BadRequest(format!("Failed to encode JWT: {}", e)))?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|_| AppError::Upstream("Vertex OAuth transport is unavailable".to_string()))?;
    let res = client
        .post(&creds.token_uri)
        .form(&[
            ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
            ("assertion", &jwt),
        ])
        .send()
        .await
        .map_err(|e| {
            error!(error = %e, "🔴 FALLBACK: Vertex token transport failed - Reason: OAuth request could not complete - Impact: coach request cannot start");
            AppError::Upstream("Vertex OAuth request failed".to_string())
        })?;

    if !res.status().is_success() {
        let status = res.status();
        error!(status = %status, "🔴 FALLBACK: Vertex token status rejected - Reason: OAuth server returned non-success - Impact: coach request cannot start");
        return Err(AppError::Upstream(format!(
            "Vertex OAuth returned HTTP {}",
            status.as_u16()
        )));
    }

    let token_resp: GcpTokenResponse = res
        .json()
        .await
        .map_err(|e| {
            error!(error = %e, "🔴 FALLBACK: Vertex token JSON rejected - Reason: OAuth response was malformed - Impact: coach request cannot start");
            AppError::Upstream("Vertex OAuth returned invalid JSON".to_string())
        })?;
    *cached = Some(CachedVertexToken {
        access_token: token_resp.access_token.clone(),
        project_id: creds.project_id.clone(),
        expires_at: Utc::now().timestamp() + 3_500,
    });
    Ok((token_resp.access_token, creds.project_id))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    pub role: String,
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<LlmToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmToolCall {
    pub id: String,
    pub r#type: String,
    pub function: LlmFunctionCall,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_content: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmFunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone)]
struct RuntimeStateSnapshot {
    state: String,
    entered_at: DateTime<Utc>,
    source_tool: Option<String>,
    metadata: String,
}

pub const FIRST_ONBOARDING_REPLY: &str = "I’m Antirot. I’ve coached plenty of people like you: smart, intense, full of plans, and somehow still one bad hour away from drifting off the thing they claim matters.\n\nSo let’s see what you’ve got. I need to build your profile. Give me a gist of your long-term and short-term goals. You can update this later as well. Because obviously, ambition is not a gift everyone has.\n\nTell me what your day looks like and what you’re planning to get done today.";

enum TurnClaim {
    Acquired {
        turn_id: String,
        lease_token: String,
    },
    Completed {
        reply: String,
    },
}

fn complete_turn_query() -> &'static str {
    "UPDATE chat_turns
     SET status = 'completed', visible_reply = $3, updated_at = now()
     WHERE id = $1 AND lease_token = $2 AND status = 'processing' AND lease_expires_at > now()"
}

fn renew_turn_query() -> &'static str {
    "UPDATE chat_turns
     SET lease_expires_at = now() + make_interval(mins => $3), updated_at = now()
     WHERE id = $1 AND lease_token = $2 AND status = 'processing' AND lease_expires_at > now()"
}

fn tool_fence_query() -> &'static str {
    "SELECT lease_token, status, curated_reply FROM chat_turns
     WHERE id = $1 AND lease_token = $2 AND status = 'processing' AND lease_expires_at > now()
     FOR UPDATE"
}

fn internal_turn_messages_query() -> &'static str {
    "SELECT role, content, tool_calls::TEXT AS tool_calls, tool_call_id, name
     FROM chat_messages
     WHERE turn_id = $1 AND is_visible = FALSE
     ORDER BY created_at ASC, id ASC"
}

#[derive(Debug)]
struct IncompleteToolBatch {
    all_calls: Vec<LlmToolCall>,
    missing_calls: Vec<LlmToolCall>,
}

fn persisted_tool_batch(messages: &[LlmMessage]) -> Option<IncompleteToolBatch> {
    let assistant_index = messages.iter().rposition(|message| {
        message.role == "assistant"
            && message
                .tool_calls
                .as_ref()
                .is_some_and(|calls| !calls.is_empty())
    })?;
    let all_calls = messages[assistant_index].tool_calls.clone()?;
    let completed_ids = messages[assistant_index + 1..]
        .iter()
        .filter(|message| message.role == "tool")
        .filter_map(|message| message.tool_call_id.as_deref())
        .collect::<std::collections::HashSet<_>>();
    let missing_calls = all_calls
        .iter()
        .filter(|call| !completed_ids.contains(call.id.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    Some(IncompleteToolBatch {
        all_calls,
        missing_calls,
    })
}

#[cfg(test)]
fn incomplete_tool_batch(messages: &[LlmMessage]) -> Option<IncompleteToolBatch> {
    persisted_tool_batch(messages).filter(|batch| !batch.missing_calls.is_empty())
}

fn derived_effect_id(turn_id: &str, fingerprint: &str, effect_kind: &str) -> String {
    format!("{turn_id}:{fingerprint}:{effect_kind}")
}

fn claim_outbox_query() -> &'static str {
    "WITH candidates AS (
         SELECT id FROM chat_effect_outbox
         WHERE turn_id IN (SELECT id FROM chat_turns WHERE user_id = $2)
           AND (status = 'pending'
            OR (status = 'in_progress' AND lease_expires_at <= now()))
         ORDER BY created_at ASC
         FOR UPDATE SKIP LOCKED
         LIMIT 1
     )
     UPDATE chat_effect_outbox outbox
     SET status = 'in_progress', lease_token = $1,
         lease_expires_at = now() + interval '10 minutes', updated_at = now()
     FROM candidates
     WHERE outbox.id = candidates.id
     RETURNING outbox.id, outbox.effect_kind"
}

async fn recover_incomplete_tool_batch(
    client: &mut tokio_postgres::Client,
    config: &Config,
    user_id: &str,
    user_message: &str,
    turn_id: &str,
    lease_token: &str,
    messages: &mut Vec<LlmMessage>,
) -> AppResult<Option<(String, String, ToolOutcome)>> {
    let Some(plan) = persisted_tool_batch(messages) else {
        return Ok(None);
    };
    let missing_ids = plan
        .missing_calls
        .iter()
        .map(|call| call.id.as_str())
        .collect::<std::collections::HashSet<_>>();
    let decoded = decode_tool_batch(&plan.all_calls).map_err(|err| {
        AppError::BadRequest(format!("Persisted coach tool batch is invalid: {err}"))
    })?;
    let mut curated = None;
    renew_chat_turn(client, turn_id, lease_token).await?;
    for (call, decoded) in plan.all_calls.into_iter().zip(decoded) {
        let is_missing = missing_ids.contains(call.id.as_str());
        let outcome = if is_missing {
            execute_tool_atomically(
                client,
                &call,
                decoded,
                AtomicToolExecution {
                    config,
                    user_id,
                    user_message,
                    turn_id,
                    lease_token,
                    failure_injection: AtomicFailureInjection::None,
                },
            )
            .await?
        } else {
            load_tool_outcome(client, turn_id, &tool_call_fingerprint(&call))
                .await?
                .ok_or_else(|| {
                    AppError::BadRequest(format!(
                        "Persisted tool result {} is missing its typed outcome",
                        call.id
                    ))
                })?
        };
        if !outcome.is_success() {
            return Err(AppError::BadRequest(format!(
                "Coach action failed during turn recovery: {}",
                outcome.message()
            )));
        }
        if should_return_curated_tool_reply(&call.function.name, &outcome, &call.function.arguments)
        {
            curated = Some((
                call.function.name.clone(),
                call.function.arguments.clone(),
                outcome.clone(),
            ));
        }
        if is_missing {
            messages.push(LlmMessage {
                role: "tool".to_string(),
                content: Some(outcome.provider_content()),
                tool_calls: None,
                tool_call_id: Some(call.id),
                name: Some(call.function.name),
            });
            process_pending_effects(client, config, user_id).await?;
        }
    }
    Ok(curated)
}

#[cfg(test)]
fn validate_lease_fence(active: &str, supplied: &str, status: &str) -> Result<(), String> {
    if active == supplied && status == "processing" {
        Ok(())
    } else {
        Err("chat turn lease was fenced by a newer worker".to_string())
    }
}

async fn claim_chat_turn(
    pool: &Pool,
    user_id: &str,
    user_message: &str,
    request_id: Option<&str>,
) -> AppResult<TurnClaim> {
    let request_id = request_id
        .map(str::to_string)
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let message_hash = sha256_hex(user_message);
    let mut client = pool.get().await?;
    let transaction = client.transaction().await?;
    transaction
        .execute(
            "UPDATE chat_turns
             SET status = 'failed', updated_at = now()
             WHERE user_id = $1 AND status = 'processing' AND lease_expires_at <= now()",
            &[&user_id],
        )
        .await?;

    if let Some(row) = transaction
        .query_opt(
            "SELECT id, message_hash, status, visible_reply, lease_expires_at > now() AS lease_active
             FROM chat_turns WHERE user_id = $1 AND request_id = $2 FOR UPDATE",
            &[&user_id, &request_id],
        )
        .await?
    {
        let stored_hash: String = row.get("message_hash");
        validate_request_hash(&stored_hash, &message_hash).map_err(AppError::Conflict)?;
        let status: String = row.get("status");
        if status == "completed" {
            let reply: Option<String> = row.get("visible_reply");
            let reply = reply.ok_or_else(|| {
                AppError::BadRequest("completed chat turn is missing its visible reply".to_string())
            })?;
            transaction.commit().await?;
            return Ok(TurnClaim::Completed { reply });
        }
        let lease_active: bool = row.get("lease_active");
        if status == "processing" && lease_active {
            return Err(AppError::Conflict(
                "another turn for this user is already processing; retry shortly".to_string(),
            ));
        }
        let turn_id: String = row.get("id");
        let lease_token = Uuid::new_v4().to_string();
        transaction
            .execute(
                "UPDATE chat_turns
                 SET status = 'processing', lease_token = $2,
                     lease_generation = lease_generation + 1,
                     lease_expires_at = now() + make_interval(mins => $3), updated_at = now()
                 WHERE id = $1",
                &[&turn_id, &lease_token, &TURN_LEASE_MINUTES],
            )
            .await
            .map_err(map_turn_claim_error)?;
        transaction.commit().await?;
        return Ok(TurnClaim::Acquired {
            turn_id,
            lease_token,
        });
    }

    let turn_id = Uuid::new_v4().to_string();
    let lease_token = Uuid::new_v4().to_string();
    transaction
        .execute(
            "INSERT INTO chat_turns
                (id, user_id, request_id, message_hash, user_message, status, lease_token, lease_expires_at)
             VALUES ($1, $2, $3, $4, $5, 'processing', $6, now() + make_interval(mins => $7))",
            &[
                &turn_id,
                &user_id,
                &request_id,
                &message_hash,
                &user_message,
                &lease_token,
                &TURN_LEASE_MINUTES,
            ],
        )
        .await
        .map_err(map_turn_claim_error)?;
    let user_msg_id = Uuid::new_v4().to_string();
    transaction
        .execute(
            "INSERT INTO chat_messages
                (id, user_id, role, content, is_visible, turn_id, request_id)
             VALUES ($1, $2, 'user', $3, TRUE, $4, $5)",
            &[&user_msg_id, &user_id, &user_message, &turn_id, &request_id],
        )
        .await?;
    transaction.commit().await?;
    Ok(TurnClaim::Acquired {
        turn_id,
        lease_token,
    })
}

fn map_turn_claim_error(err: tokio_postgres::Error) -> AppError {
    if err.code() == Some(&tokio_postgres::error::SqlState::UNIQUE_VIOLATION) {
        AppError::Conflict(
            "another turn for this user is already processing; retry shortly".to_string(),
        )
    } else {
        AppError::Database(err)
    }
}

async fn complete_chat_turn(
    pool: &Pool,
    user_id: &str,
    turn_id: &str,
    lease_token: &str,
    reply: &str,
) -> AppResult<()> {
    let mut client = pool.get().await?;
    let transaction = client.transaction().await?;
    let row = transaction
        .query_opt(
            "SELECT request_id FROM chat_turns
             WHERE id = $1 AND user_id = $2 AND lease_token = $3 AND status = 'processing'
               AND lease_expires_at > now()
             FOR UPDATE",
            &[&turn_id, &user_id, &lease_token],
        )
        .await?;
    let Some(row) = row else {
        return Err(AppError::Conflict(
            "chat turn lease was fenced before completion; retry shortly".to_string(),
        ));
    };
    let request_id: String = row.get("request_id");
    let visible_msg_id = Uuid::new_v4().to_string();
    transaction
        .execute(
            "INSERT INTO chat_messages
                (id, user_id, role, content, is_visible, turn_id, request_id)
             VALUES ($1, $2, 'assistant', $3, TRUE, $4, $5)
             ON CONFLICT DO NOTHING",
            &[&visible_msg_id, &user_id, &reply, &turn_id, &request_id],
        )
        .await?;
    let updated = transaction
        .execute(complete_turn_query(), &[&turn_id, &lease_token, &reply])
        .await?;
    if updated != 1 {
        return Err(AppError::Conflict(
            "chat turn lease was fenced before completion; retry shortly".to_string(),
        ));
    }
    transaction.commit().await?;
    Ok(())
}

async fn fail_chat_turn(pool: &Pool, turn_id: &str, lease_token: &str) {
    match pool.get().await {
        Ok(client) => {
            if let Err(err) = client
                .execute(
                    "UPDATE chat_turns SET status = 'failed', updated_at = now()
                     WHERE id = $1 AND lease_token = $2 AND status = 'processing'",
                    &[&turn_id, &lease_token],
                )
                .await
            {
                error!(turn_id, error = %err, "🔴 FALLBACK: chat turn failure status not saved - Reason: database update failed - Impact: retry waits for lease expiry");
            }
        }
        Err(err) => {
            error!(turn_id, error = %err, "🔴 FALLBACK: chat turn failure status not saved - Reason: pool unavailable - Impact: retry waits for lease expiry")
        }
    }
}

async fn renew_chat_turn(
    client: &tokio_postgres::Client,
    turn_id: &str,
    lease_token: &str,
) -> AppResult<()> {
    let updated = client
        .execute(
            renew_turn_query(),
            &[&turn_id, &lease_token, &TURN_LEASE_MINUTES],
        )
        .await?;
    if updated != 1 {
        return Err(AppError::Conflict(
            "chat turn lease was fenced; retry shortly".to_string(),
        ));
    }
    Ok(())
}

async fn load_fenced_curated_reply(
    client: &tokio_postgres::Client,
    turn_id: &str,
    lease_token: &str,
) -> AppResult<Option<String>> {
    let row = client
        .query_opt(
            "SELECT curated_reply FROM chat_turns
             WHERE id = $1 AND lease_token = $2 AND status = 'processing'
               AND lease_expires_at > now()",
            &[&turn_id, &lease_token],
        )
        .await?;
    let Some(row) = row else {
        return Err(AppError::Conflict(
            "chat turn lease was fenced while loading curated reply".to_string(),
        ));
    };
    Ok(row.get("curated_reply"))
}

async fn process_pending_effects(
    client: &tokio_postgres::Client,
    config: &Config,
    user_id: &str,
) -> AppResult<()> {
    let lease_token = Uuid::new_v4().to_string();
    let rows = client
        .query(claim_outbox_query(), &[&lease_token, &user_id])
        .await?;
    for row in rows {
        let outbox_id: String = row.get("id");
        let effect_kind: String = row.get("effect_kind");
        let result = match effect_kind.as_str() {
            "memory_reindex" => process_memory_index_jobs(client, config, user_id).await,
            "nightly_distill" => distill_today(client, config, user_id, "good_night")
                .await
                .map(|_| ()),
            other => Err(AppError::BadRequest(format!(
                "unknown chat effect outbox kind {other}"
            ))),
        };
        match result {
            Ok(()) => {
                client
                    .execute(
                        "UPDATE chat_effect_outbox
                         SET status = 'completed', attempts = attempts + 1, updated_at = now()
                         WHERE id = $1 AND status = 'in_progress' AND lease_token = $2",
                        &[&outbox_id, &lease_token],
                    )
                    .await?;
            }
            Err(err) => {
                warn!(outbox_id, effect_kind, error = %err, "🔴 FALLBACK: derived chat effect deferred - Reason: post-commit processing failed - Impact: canonical action remains committed and effect will retry");
                client
                    .execute(
                        "UPDATE chat_effect_outbox
                         SET status = 'pending', lease_token = NULL, lease_expires_at = NULL,
                             attempts = attempts + 1, updated_at = now()
                         WHERE id = $1 AND status = 'in_progress' AND lease_token = $2",
                        &[&outbox_id, &lease_token],
                    )
                    .await?;
            }
        }
    }
    process_alarm_wake_outbox(client, config, user_id).await?;
    Ok(())
}

pub(crate) async fn process_alarm_wake_outbox(
    client: &tokio_postgres::Client,
    config: &Config,
    user_id: &str,
) -> AppResult<()> {
    for _ in 0..MAX_ALARM_WAKE_DRAIN {
        if !process_alarm_wake_outbox_scoped(client, config, Some(user_id), None).await? {
            break;
        }
    }
    Ok(())
}

pub(crate) async fn process_next_alarm_wake_outbox(
    pool: &Pool,
    config: &Config,
) -> AppResult<bool> {
    let client = pool.get().await?;
    process_alarm_wake_outbox_scoped(&client, config, None, None).await
}

fn claim_alarm_wake_query() -> &'static str {
    "WITH candidate AS (
         SELECT outbox.id
         FROM alarm_wake_outbox outbox
         JOIN devices device ON device.device_id = outbox.device_id
         WHERE ($2::TEXT IS NULL OR device.user_id = $2)
           AND ($3::TEXT IS NULL OR outbox.device_id = $3)
           AND ((outbox.status = 'pending' AND outbox.next_attempt_at <= now())
             OR (outbox.status = 'in_progress' AND outbox.lease_expires_at <= now()))
         ORDER BY outbox.next_attempt_at ASC, outbox.created_at ASC
         FOR UPDATE OF outbox SKIP LOCKED
         LIMIT 1
     )
     UPDATE alarm_wake_outbox outbox
     SET status = 'in_progress', lease_token = $1,
         lease_expires_at = now() + interval '10 minutes', updated_at = now()
     FROM candidate
     WHERE outbox.id = candidate.id
     RETURNING outbox.id, outbox.device_id, outbox.alarm_id"
}

async fn process_alarm_wake_outbox_scoped(
    client: &tokio_postgres::Client,
    config: &Config,
    user_id: Option<&str>,
    device_id: Option<&str>,
) -> AppResult<bool> {
    let lease_token = Uuid::new_v4().to_string();
    let row = client
        .query_opt(
            claim_alarm_wake_query(),
            &[&lease_token, &user_id, &device_id],
        )
        .await?;
    let Some(row) = row else {
        return Ok(false);
    };
    let outbox_id: String = row.get("id");
    let device_id: String = row.get("device_id");
    let alarm_id: String = row.get("alarm_id");
    let device = client
        .query_opt(
            "SELECT push_provider, push_token FROM devices WHERE device_id = $1",
            &[&device_id],
        )
        .await?;
    let (push_provider, push_token) = device
        .map(|row| {
            (
                row.get::<_, Option<String>>("push_provider"),
                row.get::<_, Option<String>>("push_token"),
            )
        })
        .unwrap_or((None, None));
    if push_provider.as_deref() != Some("apns") {
        client
            .execute(
                "UPDATE alarm_wake_outbox SET status = 'completed', attempts = attempts + 1,
                     lease_token = NULL, lease_expires_at = NULL, last_error=NULL, updated_at = now()
                 WHERE id = $1 AND lease_token = $2",
                &[&outbox_id, &lease_token],
            )
            .await?;
        return Ok(true);
    }
    let result = match push_token.filter(|token| !token.trim().is_empty()) {
        Some(token) => crate::apns::send_alarm_wake(config, &token, &alarm_id).await,
        None => {
            warn!(outbox_id, device_id, "🔴 FALLBACK: alarm wake deferred - Reason: device has no APNs token - Impact: client must poll until push registration is available");
            client
                .execute(
                    "UPDATE alarm_wake_outbox SET status = 'pending', lease_token = NULL,
                         lease_expires_at = NULL, attempts = attempts + 1,
                         next_attempt_at=now()+interval '5 minutes',
                         last_error='device has no APNs token', updated_at = now()
                     WHERE id = $1 AND lease_token = $2",
                    &[&outbox_id, &lease_token],
                )
                .await?;
            return Ok(false);
        }
    };
    match result {
        Ok(()) => {
            client
                .execute(
                    "UPDATE alarm_wake_outbox SET status = 'completed', attempts = attempts + 1,
                         lease_token = NULL, lease_expires_at = NULL, last_error=NULL, updated_at = now()
                     WHERE id = $1 AND lease_token = $2",
                    &[&outbox_id, &lease_token],
                )
                .await?;
        }
        Err(err) => {
            warn!(outbox_id, device_id, error = %err, "🔴 FALLBACK: alarm wake deferred - Reason: APNs request failed - Impact: client must poll and wake will retry");
            client
                .execute(
                    "UPDATE alarm_wake_outbox
                     SET status=CASE WHEN attempts + 1 >= 10 THEN 'failed' ELSE 'pending' END,
                         attempts = attempts + 1, lease_token = NULL, lease_expires_at = NULL,
                         next_attempt_at=now() + make_interval(secs =>
                           LEAST(3600, 5 * (1 << LEAST(attempts,9))) + MOD(ABS(hashtext(id)),5)),
                         last_error=left($3,1000), updated_at = now()
                     WHERE id = $1 AND lease_token = $2",
                    &[&outbox_id, &lease_token, &err.to_string()],
                )
                .await?;
            return Ok(false);
        }
    }
    Ok(true)
}

pub async fn chat_with_coach(
    pool: &Pool,
    config: &Config,
    user_id: &str,
    user_message: &str,
    request_id: &str,
) -> AppResult<String> {
    let turn_id = match claim_chat_turn(pool, user_id, user_message, Some(request_id)).await? {
        TurnClaim::Completed { reply } => return Ok(reply),
        TurnClaim::Acquired {
            turn_id,
            lease_token,
        } => (turn_id, lease_token),
    };
    let (turn_id, lease_token) = turn_id;
    match chat_with_coach_inner(pool, config, user_id, user_message, &turn_id, &lease_token).await {
        Ok(reply) => {
            complete_chat_turn(pool, user_id, &turn_id, &lease_token, &reply).await?;
            Ok(reply)
        }
        Err(err) => {
            fail_chat_turn(pool, &turn_id, &lease_token).await;
            Err(err)
        }
    }
}

async fn chat_with_coach_inner(
    pool: &Pool,
    config: &Config,
    user_id: &str,
    user_message: &str,
    turn_id: &str,
    lease_token: &str,
) -> AppResult<String> {
    info!(user_id, "initiating chat with coach");

    // 1. Fetch user subscription details
    let client = pool.get().await?;
    let user_row = client
        .query_opt(
            "
            SELECT subscription_tier, subscription_status, byok_api_key, byok_provider, subscription_active_until
            FROM users
            WHERE id = $1
            ",
            &[&user_id],
        )
        .await?;

    let Some(user_row) = user_row else {
        return Err(AppError::BadRequest("User not found".to_string()));
    };

    client
        .execute(
            "
            INSERT INTO user_runtime_states (user_id, state, source_tool)
            VALUES ($1, 'onboarding', 'system')
            ON CONFLICT (user_id) DO NOTHING
            ",
            &[&user_id],
        )
        .await?;

    let tier: String = user_row.get("subscription_tier");
    let status: String = user_row.get("subscription_status");
    let stored_byok_api_key: Option<String> = user_row.get("byok_api_key");
    let byok_api_key = stored_byok_api_key
        .as_deref()
        .map(|stored| decrypt_byok_key(config, stored))
        .transpose()?;
    let byok_provider: Option<String> = user_row.get("byok_provider");
    let active_until: Option<DateTime<Utc>> = user_row.get("subscription_active_until");

    // Check if subscription is active
    let is_active = status == "active"
        || active_until.map(|dt| dt > Utc::now()).unwrap_or(false)
        || user_id == "admin";

    if !is_active {
        return Ok("🔴 Antirot Coach: Your subscription is inactive. Please activate your subscription ($1/mo BYOK or $5/mo FocusEngine tailored LLM) in Settings to resume coaching.".to_string());
    }

    drop(client);
    if let Some(outcome) = distill_idle_if_due(pool, config, user_id).await? {
        if outcome.distilled {
            info!(user_id, date = %outcome.date, "idle-triggered nightly memory distillation completed before chat");
        }
    }
    let mut client = pool.get().await?;
    process_pending_effects(&client, config, user_id).await?;

    // Resolve LLM key, provider, and model based on subscription tier
    let (mut api_key, provider, model) = if tier == "byok" && user_id != "admin" {
        let key = byok_api_key.unwrap_or_default();
        let prov = byok_provider.unwrap_or_else(|| "openai".to_string());
        let default_model = match prov.as_str() {
            "gemini" => "gemini-3.5-flash",
            "openrouter" => "meta-llama/llama-3-70b-instruct",
            "openai" => "gpt-4o-mini",
            _ => {
                return Err(AppError::BadRequest(
                    "Unsupported BYOK provider; choose openai, gemini, or openrouter".to_string(),
                ))
            }
        };
        (key, prov, default_model.to_string())
    } else {
        let has_vertex_credentials = std::env::var("GOOGLE_CLOUD_CREDENTIALS")
            .ok()
            .is_some_and(|value| !value.trim().is_empty());
        if !has_vertex_credentials {
            return Err(AppError::BadRequest(
                "GOOGLE_CLOUD_CREDENTIALS is required for the tailored Vertex coach LLM"
                    .to_string(),
            ));
        }
        (
            String::new(),
            "vertex".to_string(),
            "google/gemini-3.5-flash".to_string(),
        )
    };

    info!(
        user_id,
        provider = %provider,
        model = %model,
        vertex_credentials = std::env::var("GOOGLE_CLOUD_CREDENTIALS").ok().is_some_and(|value| !value.trim().is_empty()),
        "resolved coach LLM provider"
    );

    let mut project_id = String::new();
    if provider == "vertex" {
        let (token, pid) = get_vertex_access_token().await?;
        api_key = token;
        project_id = pid;
    } else {
        if tier != "byok" && api_key.is_empty() {
            return Err(AppError::BadRequest(
                "Tailored LLM key is not configured on this backend".to_string(),
            ));
        }
    }

    // 2. Load chat history
    let history_rows = client
        .query(&visible_history_query(20), &[&user_id, &turn_id])
        .await?;

    let mut messages: Vec<LlmMessage> = history_rows
        .iter()
        .map(|row| {
            let tool_calls_str: Option<String> = row.get("tool_calls");
            let tool_calls = tool_calls_str.and_then(|s| serde_json::from_str(&s).ok());
            LlmMessage {
                role: row.get("role"),
                content: row.get("content"),
                tool_calls,
                tool_call_id: row.get("tool_call_id"),
                name: row.get("name"),
            }
        })
        .collect();
    let internal_rows = client
        .query(internal_turn_messages_query(), &[&turn_id])
        .await?;
    let mut internal_messages = internal_rows
        .iter()
        .map(|row| {
            let tool_calls_str: Option<String> = row.get("tool_calls");
            let tool_calls = tool_calls_str.and_then(|value| serde_json::from_str(&value).ok());
            LlmMessage {
                role: row.get("role"),
                content: row.get("content"),
                tool_calls,
                tool_call_id: row.get("tool_call_id"),
                name: row.get("name"),
            }
        })
        .collect::<Vec<_>>();
    recover_incomplete_tool_batch(
        &mut client,
        config,
        user_id,
        user_message,
        turn_id,
        lease_token,
        &mut internal_messages,
    )
    .await?;
    let recovered_curated_reply = load_fenced_curated_reply(&client, turn_id, lease_token).await?;
    if let Some(persisted_final) = internal_messages.last().filter(|message| {
        message.role == "assistant"
            && message.tool_calls.as_ref().is_none_or(Vec::is_empty)
            && message
                .content
                .as_deref()
                .is_some_and(|content| !content.trim().is_empty())
    }) {
        let content = persisted_final.content.as_deref().unwrap_or_default();
        return if let Some(curated_reply) = recovered_curated_reply {
            Ok(curated_reply)
        } else {
            validated_model_reply(content)
        };
    }
    // 3. Assemble system prompt with current memory context.
    let tools = get_tool_definitions();
    let tool_count = tools.as_array().map(|items| items.len()).unwrap_or(0);
    let built_prompt = build_prompt_for_user(
        &client,
        config,
        user_id,
        &provider,
        &model,
        tool_count,
        user_message,
    )
    .await?;
    if !built_prompt.report.memory.truncated_sections.is_empty() {
        warn!(
            user_id,
            truncated = ?built_prompt.report.memory.truncated_sections,
            "prompt memory sections truncated"
        );
    }

    // Filter messages to prepend system context
    let mut request_messages = vec![LlmMessage {
        role: "system".to_string(),
        content: Some(built_prompt.system_prompt),
        tool_calls: None,
        tool_call_id: None,
        name: None,
    }];
    request_messages.extend(messages.clone());

    // Add new user message
    let new_user_msg = LlmMessage {
        role: "user".to_string(),
        content: Some(user_message.to_string()),
        tool_calls: None,
        tool_call_id: None,
        name: None,
    };
    request_messages.push(new_user_msg.clone());
    messages.push(new_user_msg);
    request_messages.extend(internal_messages.clone());
    messages.extend(internal_messages);

    // 5. Orchestration loop (handles recursive tool calling)
    let http_client = Client::builder().timeout(Duration::from_secs(45)).build()?;

    let url = match provider.as_str() {
        "vertex" => {
            format!("https://aiplatform.googleapis.com/v1/projects/{}/locations/global/endpoints/openapi/chat/completions", project_id)
        }
        "gemini" => {
            "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions".to_string()
        }
        "openrouter" => "https://openrouter.ai/api/v1/chat/completions".to_string(),
        "openai" => "https://api.openai.com/v1/chat/completions".to_string(),
        _ => {
            return Err(AppError::BadRequest(
                "Unsupported coach provider configuration".to_string(),
            ))
        }
    };

    let mut loop_count = 0;
    let max_loops = 5;
    let mut final_text = String::new();
    let mut last_curated_reply = recovered_curated_reply;
    let mut completed = false;

    while loop_count < max_loops {
        loop_count += 1;
        renew_chat_turn(&client, turn_id, lease_token).await?;
        info!(loop_count, url, "sending request to LLM");

        let mut request_payload = json!({
            "model": model,
            "messages": request_messages,
            "tools": tools,
            "tool_choice": "auto"
        });
        if let Some(thinking_config) = gemini_minimal_thinking_extra_body(&provider, &model) {
            request_payload["extra_body"] = thinking_config;
        }

        let mut request = http_client.post(&url).json(&request_payload);
        if provider == "openrouter" {
            request = request
                .header("Authorization", format!("Bearer {}", api_key))
                .header("HTTP-Referer", "https://antirot.org")
                .header("X-Title", "Antirot Coaching Platform");
        } else {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        drop(client);
        let response = request
            .send()
            .await
            .map_err(|err| {
                error!(user_id, loop_count, error = %err, "{}", provider_fallback_message(ProviderFailureKind::Transport));
                AppError::BadRequest(format!("LLM API request failed: {err}"))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            error!(status = %status, "🔴 FALLBACK: coach provider status rejected - Reason: upstream returned non-success - Impact: no reply committed");
            return Err(AppError::Upstream(format!(
                "coach provider returned HTTP {}",
                status.as_u16()
            )));
        }

        let response_json: Value = response.json().await.map_err(|err| {
            error!(user_id, loop_count, error = %err, "{}", provider_fallback_message(ProviderFailureKind::Json));
            AppError::Upstream("coach provider returned invalid JSON".to_string())
        })?;

        let message_val = response_json
            .get("choices")
            .and_then(Value::as_array)
            .and_then(|choices| choices.first())
            .and_then(|choice| choice.get("message"))
            .and_then(Value::as_object)
            .ok_or_else(|| {
                error!(
                    user_id,
                    loop_count,
                    "{}",
                    provider_fallback_message(ProviderFailureKind::MissingMessage)
                );
                AppError::BadRequest("LLM provider returned an invalid response shape".to_string())
            })?;
        let content = message_val
            .get("content")
            .and_then(Value::as_str)
            .map(str::to_string);

        let tool_calls: Option<Vec<LlmToolCall>> = match message_val.get("tool_calls") {
            None | Some(Value::Null) => None,
            Some(Value::Array(items)) => Some(
                items
                    .iter()
                    .cloned()
                    .map(serde_json::from_value)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|err| {
                        error!(user_id, loop_count, error = %err, "{}", provider_fallback_message(ProviderFailureKind::MalformedToolCall));
                        AppError::BadRequest("LLM provider returned malformed tool calls".to_string())
                    })?,
            ),
            Some(_) => {
                error!(user_id, loop_count, "{}", provider_fallback_message(ProviderFailureKind::WrongToolCallsType));
                return Err(AppError::BadRequest(
                    "LLM provider returned malformed tool calls".to_string(),
                ));
            }
        };
        if content.is_none() && tool_calls.as_ref().is_none_or(Vec::is_empty) {
            error!(
                user_id,
                loop_count,
                "{}",
                provider_fallback_message(ProviderFailureKind::EmptyResponse)
            );
            return Err(AppError::BadRequest(
                "LLM provider returned no content or tool calls".to_string(),
            ));
        }
        let decoded_tool_batch = match tool_calls.as_ref() {
            Some(calls) if !calls.is_empty() => {
                Some(decode_tool_batch(calls).map_err(|err| {
                    error!(user_id, loop_count, error = %err, "🔴 FALLBACK: coach tool batch rejected - Reason: at least one tool had invalid arguments - Impact: no action executed");
                    AppError::BadRequest(format!("Coach tool batch rejected: {err}"))
                })?)
            }
            _ => None,
        };
        client = pool.get().await?;

        // Save LLM response to messages
        let response_msg = LlmMessage {
            role: "assistant".to_string(),
            content: content.clone(),
            tool_calls: tool_calls.clone(),
            tool_call_id: None,
            name: None,
        };
        request_messages.push(response_msg.clone());
        messages.push(response_msg);

        // Save LLM assistant message to DB
        let assistant_msg_id = Uuid::new_v4().to_string();
        let tool_calls_json = tool_calls
            .as_ref()
            .map(|tc| serde_json::to_string(tc).unwrap());
        client
            .execute(
                "
                INSERT INTO chat_messages
                    (id, user_id, role, content, tool_calls, is_visible, turn_id)
                VALUES ($1, $2, 'assistant', $3, $4::TEXT::JSONB, FALSE, $5)
                ",
                &[
                    &assistant_msg_id,
                    &user_id,
                    &content,
                    &tool_calls_json,
                    &turn_id,
                ],
            )
            .await?;

        if let Some(calls) = tool_calls {
            if calls.is_empty() {
                if let Some(text) = content {
                    final_text = validated_model_reply(&text)?;
                }
                completed = true;
                break;
            }

            let decoded_calls =
                decoded_tool_batch.expect("non-empty tool batch was validated before persistence");

            let executed_batch = execute_tool_batch_atomically(
                &mut client,
                calls.into_iter().zip(decoded_calls).collect(),
                AtomicToolExecution {
                    config,
                    user_id,
                    user_message,
                    turn_id,
                    lease_token,
                    failure_injection: AtomicFailureInjection::None,
                },
            )
            .await?;
            process_pending_effects(&client, config, user_id).await?;
            last_curated_reply = load_fenced_curated_reply(&client, turn_id, lease_token).await?;

            for (call, outcome) in executed_batch {
                info!(tool = %call.function.name, "LLM requested tool execution");
                let provider_content = outcome.provider_content();
                let tool_msg = LlmMessage {
                    role: "tool".to_string(),
                    content: Some(provider_content.clone()),
                    tool_calls: None,
                    tool_call_id: Some(call.id.clone()),
                    name: Some(call.function.name.clone()),
                };
                request_messages.push(tool_msg.clone());
                messages.push(tool_msg);
            }
        } else {
            if let Some(text) = content {
                final_text = validated_model_reply(&text)?;
            }
            completed = true;
            break;
        }
    }

    if !completed {
        error!(user_id, max_loops, "🔴 FALLBACK: coach orchestration exhausted - Reason: provider requested more than five tool rounds - Impact: no reply committed");
        return Err(AppError::BadRequest(
            "Coach orchestration exceeded the maximum tool rounds".to_string(),
        ));
    }

    let final_text = if let Some(curated_reply) = last_curated_reply {
        curated_reply
    } else {
        validated_model_reply(&final_text)?
    };
    Ok(final_text)
}

fn sanitize_user_facing_reply(text: &str) -> String {
    let trimmed = text.trim();
    let Some(first_line) = trimmed.lines().next() else {
        return String::new();
    };
    let first_line_normalized = first_line
        .trim()
        .trim_start_matches('#')
        .trim()
        .to_ascii_lowercase();
    let reasoning_heading = matches!(
        first_line_normalized.as_str(),
        "reasoning summary" | "analytical assessment" | "analysis" | "reasoning"
    );
    if !reasoning_heading {
        return text.to_string();
    }

    let mut after_separator = false;
    let mut kept = Vec::new();
    for line in trimmed.lines().skip(1) {
        let marker = line.trim();
        if after_separator {
            kept.push(line);
        } else if matches!(marker, "***" | "---" | "___") {
            after_separator = true;
        }
    }

    let sanitized = kept.join("\n").trim().to_string();
    if sanitized.is_empty() {
        text.to_string()
    } else {
        sanitized
    }
}

fn validated_model_reply(text: &str) -> AppResult<String> {
    let sanitized = sanitize_user_facing_reply(text);
    let first_line = text
        .trim()
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .trim_start_matches('#')
        .trim()
        .to_ascii_lowercase();
    let reasoning_prefixed = matches!(
        first_line.as_str(),
        "reasoning summary" | "analytical assessment" | "analysis" | "reasoning"
    );
    if sanitized.trim().is_empty() || (reasoning_prefixed && sanitized.trim() == text.trim()) {
        return Err(AppError::BadRequest(
            "LLM provider returned no valid user-facing reply".to_string(),
        ));
    }
    Ok(sanitized)
}

fn visible_history_query(limit: usize) -> String {
    format!(
        "SELECT role, content, tool_calls::TEXT AS tool_calls, tool_call_id, name
         FROM (
             SELECT id, role, content, tool_calls, tool_call_id, name, created_at
             FROM chat_messages
             WHERE user_id = $1
               AND turn_id IS DISTINCT FROM $2
               AND is_visible = TRUE
               AND role IN ('user', 'assistant')
               AND content IS NOT NULL
               AND length(trim(content)) > 0
             ORDER BY created_at DESC, id DESC
             LIMIT {limit}
         ) newest
         ORDER BY created_at ASC, id ASC"
    )
}

#[cfg(test)]
fn committed_visible_reply(
    tool_name: &str,
    outcome: &ToolOutcome,
    user_message: &str,
    tool_arguments: &str,
    provider_reply: Option<&str>,
) -> AppResult<String> {
    if !outcome.is_success() {
        return Err(AppError::BadRequest(format!(
            "Coach action failed: {}",
            outcome.message()
        )));
    }
    if should_return_curated_tool_reply(tool_name, outcome, tool_arguments) {
        return Ok(user_facing_tool_result(
            tool_name,
            outcome,
            user_message,
            tool_arguments,
        ));
    }
    validated_model_reply(provider_reply.unwrap_or_default())
}

fn advance_curated_reply(
    current: Option<String>,
    tool_name: &str,
    outcome: &ToolOutcome,
    user_message: &str,
    tool_arguments: &str,
) -> Option<String> {
    if should_return_curated_tool_reply(tool_name, outcome, tool_arguments) {
        Some(user_facing_tool_result(
            tool_name,
            outcome,
            user_message,
            tool_arguments,
        ))
    } else {
        current
    }
}

#[cfg(test)]
fn recovered_visible_reply(curated_reply: Option<String>, provider_reply: &str) -> String {
    curated_reply.unwrap_or_else(|| provider_reply.to_string())
}

fn user_facing_tool_result(
    tool_name: &str,
    outcome: &ToolOutcome,
    user_message: &str,
    tool_arguments: &str,
) -> String {
    if !outcome.is_success() {
        return format!("I hit a backend problem: {}", outcome.message());
    }

    match tool_name {
        "patch_file" if patched_file_from_arguments(tool_arguments).as_deref() == Some("sleep.md") => "Sleep target noted. Useful constraint, not a hiding place. First concrete slice now: file, screen, or test case, plus minutes.".to_string(),
        "patch_file" if patched_file_from_arguments(tool_arguments).as_deref() == Some("miscellaneous_todo.md") => "Parked for later. Do not chase the shiny side quest; finish what is already open.".to_string(),
        "patch_file" if patched_file_from_arguments(tool_arguments).as_deref() == Some("coach_todo.txt") => "I will carry that forward. Stay with the question in front of you.".to_string(),
        "patch_file" => "First task now: name the exact concrete slice: file, screen, or test case, then give it 10 clean minutes.".to_string(),
        "start_session" => start_session_reply(tool_arguments, user_message),
        "extend_session" => "Extra time granted. Spend it cleanly; check-in still wants evidence.".to_string(),
        "end_session" if actual_minutes_from_tool_arguments(tool_arguments).unwrap_or(0) <= 0 => "How many minutes were actually productive? I am not ending this on a zero-minute shrug; give me the raw proof.".to_string(),
        "end_session" => "Round finished. Choose the next move now: another focused run, a real break, sleep, or a plan update.".to_string(),
        "start_break" => {
            let duration_minutes = serde_json::from_str::<Value>(tool_arguments)
                .ok()
                .and_then(|value| value["duration_minutes"].as_i64())
                .unwrap_or(15);
            format!(
                "Break approved: {} minutes. Reset for real; scrolling in disguise does not count.",
                duration_minutes
            )
        }
        "start_sleep" => "Sleep starts now. Phone down; the late-night strategy committee is adjourned.".to_string(),
        "wake_up_alarm" => "Wake plan set. When it fires, check in before the bargaining committee wakes up.".to_string(),
        "log_wake" => "You're awake. Pick one concrete task and run 20 minutes before your brain starts negotiating.".to_string(),
        "start_vacation" => "Vacation approved. Real off-duty time. Before 8pm, write tomorrow's first 20-minute re-entry task.".to_string(),
        "end_vacation" => "Vacation is over. Gentle ramp, not heroic montage: choose one 20-minute task and begin.".to_string(),
        "log_override" => "Override accepted. The standard stays: no fake positivity, no excuse protection. Move deliberately.".to_string(),
        "memory_search" => "I checked the relevant history. Use the evidence; no mythology required. Choose the next move.".to_string(),
        "set_routine_categories" => "Routine shape is clear. Start with the first concrete task: name the exact file, screen, or test case, and give me the minutes.".to_string(),
        _ => "Handled. Next move.".to_string(),
    }
}

fn should_return_curated_tool_reply(
    tool_name: &str,
    outcome: &ToolOutcome,
    tool_arguments: &str,
) -> bool {
    if !outcome.is_success() {
        return false;
    }

    if tool_name == "patch_file" {
        return matches!(
            patched_file_from_arguments(tool_arguments).as_deref(),
            Some("sleep.md") | Some("miscellaneous_todo.md")
        );
    }

    matches!(
        tool_name,
        "start_session"
            | "extend_session"
            | "end_session"
            | "start_break"
            | "start_sleep"
            | "wake_up_alarm"
            | "log_wake"
            | "start_vacation"
            | "end_vacation"
            | "log_override"
            | "set_routine_categories"
    )
}

fn actual_minutes_from_tool_arguments(tool_arguments: &str) -> Option<i64> {
    serde_json::from_str::<Value>(tool_arguments)
        .ok()
        .and_then(|value| value["actual_minutes"].as_i64())
}

fn patched_file_from_arguments(tool_arguments: &str) -> Option<String> {
    serde_json::from_str::<Value>(tool_arguments)
        .ok()?
        .get("file_path")?
        .as_str()
        .map(str::to_string)
}

fn start_session_reply(tool_arguments: &str, user_message: &str) -> String {
    let parsed_args: Value = serde_json::from_str(tool_arguments).unwrap_or(Value::Null);
    let task_from_args = parsed_args["task_id"]
        .as_str()
        .map(str::trim)
        .filter(|task| !task.is_empty());
    let minutes_from_args = parsed_args["estimated_minutes"]
        .as_i64()
        .filter(|minutes| *minutes > 0);

    let lower = user_message.to_ascii_lowercase();
    let minutes = minutes_from_args
        .or_else(|| extract_first_integer(&lower))
        .unwrap_or(0);
    let task = task_from_args
        .map(str::to_string)
        .or_else(|| extract_task_after_on(user_message))
        .or_else(|| extract_task_after_will(user_message))
        .or_else(|| extract_task_after_to(user_message))
        .unwrap_or_else(|| "this task".to_string())
        .trim()
        .trim_matches(|ch: char| matches!(ch, '.' | '!' | '?' | '"' | '\''))
        .to_string();

    if minutes > 0 {
        format!(
            "Good. {} minutes on {}. Open the work, hit the smallest real piece, and come back with proof. Side quests can complain later.",
            minutes, task
        )
    } else {
        format!(
            "Good. {} is the target. Open the work, hit the smallest real piece, and come back with proof. Side quests can complain later.",
            task
        )
    }
}

fn extract_first_integer(text: &str) -> Option<i64> {
    let mut digits = String::new();
    for ch in text.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
        } else if !digits.is_empty() {
            break;
        }
    }
    digits.parse().ok()
}

fn extract_task_after_on(user_message: &str) -> Option<String> {
    let lower = user_message.to_ascii_lowercase();
    let marker = " on ";
    let index = lower.rfind(marker)?;
    let task = user_message[index + marker.len()..].trim();
    if task.is_empty() {
        None
    } else {
        Some(task.to_string())
    }
}

fn extract_task_after_to(user_message: &str) -> Option<String> {
    let lower = user_message.to_ascii_lowercase();
    let marker = " to ";
    let index = lower.rfind(marker)?;
    let task = user_message[index + marker.len()..].trim();
    if task.is_empty() {
        None
    } else {
        Some(task.to_string())
    }
}

fn extract_task_after_will(user_message: &str) -> Option<String> {
    let lower = user_message.to_ascii_lowercase();
    let marker = " will ";
    let index = lower.find(marker)?;
    let mut task = user_message[index + marker.len()..].trim().to_string();
    let task_lower = task.to_ascii_lowercase();
    for trailing_marker in [" and do that in ", " and do it in ", " in "] {
        if let Some(cut_index) = task_lower.rfind(trailing_marker) {
            let tail = &task_lower[cut_index + trailing_marker.len()..];
            if tail.contains("min") || tail.contains("hour") {
                task.truncate(cut_index);
                break;
            }
        }
    }

    if task.trim().is_empty() {
        None
    } else {
        Some(task)
    }
}

fn gemini_minimal_thinking_extra_body(provider: &str, model: &str) -> Option<Value> {
    if !matches!(provider, "gemini" | "vertex") {
        return None;
    }

    let model_lower = model.to_lowercase();
    if !model_lower.contains("gemini") {
        return None;
    }

    if model_lower.contains("gemini-3") {
        return Some(json!({
            "google": {
                "thinking_config": {
                    "thinking_level": "minimal"
                }
            }
        }));
    }

    Some(json!({
        "google": {
            "thinking_config": {
                "thinking_budget": 0
            }
        }
    }))
}

async fn build_prompt_for_user(
    client: &tokio_postgres::Client,
    config: &Config,
    user_id: &str,
    provider: &str,
    model: &str,
    tool_count: usize,
    recall_query: &str,
) -> AppResult<BuiltPrompt> {
    let now = Utc::now();
    let user_day = user_day_for(client, user_id, now).await?;
    let today_log_key = user_day.work_log_key();
    let today_log = get_memory_or_init(client, user_id, &today_log_key, DEFAULT_WORK_LOG).await?;

    let mut combined_summaries = String::new();
    for i in 0..3 {
        let day = user_day.current_date() - chrono::Duration::days(i);
        let display_day = day.format("%Y-%m-%d").to_string();
        let summary_key = format!("work_summary_{}", day.format("%Y_%m_%d"));
        let summary = get_memory_or_init(client, user_id, &summary_key, "").await?;
        if summary.trim().is_empty() {
            combined_summaries.push_str(&format!(
                "### Daily Summary for {}\n(No summary logged for this day)\n\n",
                display_day
            ));
        } else {
            combined_summaries.push_str(&format!(
                "### Daily Summary for {}\n{}\n\n",
                display_day,
                summary.trim()
            ));
        }
    }

    let recalled_memory = search_memory(client, config, user_id, recall_query, 4).await?;
    let recalled_memory_content = if recalled_memory.is_empty() {
        "No extra historical recall injected for this turn.".to_string()
    } else {
        recalled_memory
            .iter()
            .map(|hit| {
                format!(
                    "- [{} score {:.2}] {}",
                    hit.memory_key,
                    hit.score,
                    hit.content.replace('\n', " ")
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let sleep_metrics = sleep_metrics_report(client, user_id).await?;
    let sleep_metrics_content = serde_json::to_string_pretty(&sleep_metrics).unwrap_or_default();
    let runtime_snapshot = current_runtime_state(client, user_id).await?;
    let runtime_status_content = runtime_status_for_prompt(runtime_snapshot.as_ref(), now);

    let sections = vec![
        MemorySection {
            key: "current_turn_context",
            label: "Current Turn Context",
            content: current_turn_context_for_prompt(now),
        },
        MemorySection {
            key: "runtime_status",
            label: "Current Runtime Status",
            content: runtime_status_content,
        },
        memory_section(
            client,
            user_id,
            "personality",
            "Personality (personality.md)",
        )
        .await?,
        memory_section(
            client,
            user_id,
            "user_profile",
            "User Profile (user_profile.md)",
        )
        .await?,
        memory_section(
            client,
            user_id,
            "durable",
            "Durable Distilled Memory (durable.md)",
        )
        .await?,
        memory_section(client, user_id, "longterm", "Long-Term Goals (longterm.md)").await?,
        memory_section(
            client,
            user_id,
            "shortterm",
            "Short-Term State & Constraints (shortterm.md)",
        )
        .await?,
        memory_section(
            client,
            user_id,
            "behavior",
            "Behavior Patterns & Tactics (behavior.md)",
        )
        .await?,
        memory_section(client, user_id, "tasks", "Planned Work (tasks.md)").await?,
        memory_section(
            client,
            user_id,
            "routine",
            "Fixed Daily Routine Allocations (routine.md)",
        )
        .await?,
        memory_section(
            client,
            user_id,
            "miscellaneous_todo",
            "Miscellaneous Todo List (miscellaneous_todo.md)",
        )
        .await?,
        memory_section(
            client,
            user_id,
            "coach_todo",
            "Coach Todo List (coach_todo.txt)",
        )
        .await?,
        memory_section(client, user_id, "sleep", "Sleep Log (sleep.md)").await?,
        memory_section(
            client,
            user_id,
            "achievements",
            "Achievements (achievements.md)",
        )
        .await?,
        MemorySection {
            key: "recent_summaries",
            label: "Recent Daily Summaries",
            content: combined_summaries,
        },
        MemorySection {
            key: "historical_recall",
            label: "Relevant Historical Recall",
            content: recalled_memory_content,
        },
        MemorySection {
            key: "sleep_metrics",
            label: "Sleep Timing Metrics",
            content: sleep_metrics_content,
        },
        MemorySection {
            key: "today_log",
            label: "Today's Session Logs",
            content: today_log,
        },
    ];

    Ok(build_coach_system_prompt(PromptContext {
        provider: provider.to_string(),
        model: model.to_string(),
        tool_count,
        sections,
    }))
}

async fn memory_section(
    client: &tokio_postgres::Client,
    user_id: &str,
    key: &'static str,
    label: &'static str,
) -> AppResult<MemorySection> {
    let default = default_memory_for_key(key).unwrap_or("");
    let label = memory_descriptor(key)
        .map(|descriptor| descriptor.label)
        .unwrap_or(label);
    Ok(MemorySection {
        key,
        label,
        content: get_memory_or_init(client, user_id, key, default).await?,
    })
}

pub async fn build_context_report(
    pool: &Pool,
    config: &Config,
    user_id: &str,
    provider: &str,
    model: &str,
) -> AppResult<PromptBuildReport> {
    let _ = distill_idle_if_due(pool, config, user_id).await?;
    let client = pool.get().await?;
    let tools = get_tool_definitions();
    let tool_count = tools.as_array().map(|items| items.len()).unwrap_or(0);
    let built_prompt =
        build_prompt_for_user(&client, config, user_id, provider, model, tool_count, "").await?;
    Ok(built_prompt.report)
}

pub async fn build_context_report_for_test(
    pool: &Pool,
    config: &Config,
    user_id: &str,
    provider: &str,
    model: &str,
) -> AppResult<PromptBuildReport> {
    build_context_report(pool, config, user_id, provider, model).await
}

async fn get_memory_or_init<C>(
    client: &C,
    user_id: &str,
    key: &str,
    default: &str,
) -> AppResult<String>
where
    C: GenericClient + Sync,
{
    let row = client
        .query_opt(
            "SELECT content FROM user_memories WHERE user_id = $1 AND memory_key = $2",
            &[&user_id, &key],
        )
        .await?;

    match row {
        Some(row) => Ok(row.get("content")),
        None => {
            let version = crate::memory::content_hash(default);
            client
                .execute(
                    "
                    INSERT INTO user_memories (user_id, memory_key, content, content_version)
                    VALUES ($1, $2, $3, $4)
                    ON CONFLICT DO NOTHING
                    ",
                    &[&user_id, &key, &default, &version],
                )
                .await?;
            Ok(client
                .query_one(
                    "SELECT content FROM user_memories WHERE user_id = $1 AND memory_key = $2",
                    &[&user_id, &key],
                )
                .await?
                .get("content"))
        }
    }
}

async fn current_runtime_state(
    client: &tokio_postgres::Client,
    user_id: &str,
) -> AppResult<Option<RuntimeStateSnapshot>> {
    Ok(client
        .query_opt(
            "
            SELECT state, entered_at, source_tool, metadata::TEXT AS metadata
            FROM user_runtime_states
            WHERE user_id = $1
            ",
            &[&user_id],
        )
        .await?
        .map(|row| RuntimeStateSnapshot {
            state: row.get("state"),
            entered_at: row.get("entered_at"),
            source_tool: row.get("source_tool"),
            metadata: row.get("metadata"),
        }))
}

fn runtime_status_for_prompt(
    snapshot: Option<&RuntimeStateSnapshot>,
    now: DateTime<Utc>,
) -> String {
    let Some(snapshot) = snapshot else {
        return "No runtime state is currently recorded.".to_string();
    };
    let elapsed_minutes = (now - snapshot.entered_at).num_minutes().max(0);
    let mut lines = vec![
        format!("Current mode: {}", snapshot.state),
        format!("Current mode started {} minute(s) ago.", elapsed_minutes),
        format!(
            "Source: {}",
            snapshot.source_tool.as_deref().unwrap_or("unknown")
        ),
        format!("Metadata: {}", snapshot.metadata),
    ];
    if snapshot.state == "working" {
        lines.push(format!(
            "The current work task has been active for {} minute(s). Mention this timing if the user asks for a break, says done, changes task, or sends any message that affects the current task.",
            elapsed_minutes
        ));
        if elapsed_minutes < EARLY_SESSION_MINIMUM_MINUTES {
            lines.push(format!(
                "This task is too fresh to stop normally. On the first early-break or early-stop request, do not reveal the accountability sentence. Argue against stopping, ask why they need the break, and push for at least {} minutes of effort. Only require the accountability sentence after the user gives a weak reason and keeps insisting.",
                EARLY_SESSION_MINIMUM_MINUTES
            ));
        }
    }
    lines.join("\n")
}

fn current_turn_context_for_prompt(now: DateTime<Utc>) -> String {
    let ist =
        FixedOffset::east_opt(5 * 60 * 60 + 30 * 60).expect("IST fixed offset should be valid");
    let now_ist = now.with_timezone(&ist);
    [
        format!("Current turn time UTC: {}", now.to_rfc3339_opts(SecondsFormat::Secs, true)),
        format!("Current turn time IST: {}", now_ist.to_rfc3339_opts(SecondsFormat::Secs, true)),
        "Use this timestamp to distinguish the latest user turn from older conversation, memory, logs, or historical summaries. Do not mention the timestamp unless the user asks or timing is directly relevant.".to_string(),
    ]
    .join("\n")
}

fn apply_patch(content: &str, patch: &str) -> Result<String, String> {
    let search_marker = "<<<<<<< SEARCH";
    let divider_marker = "=======";
    let replace_marker = ">>>>>>> REPLACE";

    let search_start = patch
        .find(search_marker)
        .ok_or("Patch error: Missing '<<<<<<< SEARCH' marker")?;
    let divider_pos = patch
        .find(divider_marker)
        .ok_or("Patch error: Missing '=======' marker")?;
    let replace_end = patch
        .find(replace_marker)
        .ok_or("Patch error: Missing '>>>>>>> REPLACE' marker")?;

    if search_start >= divider_pos || divider_pos >= replace_end {
        return Err("Patch error: Markers are in incorrect order".to_string());
    }

    let search_block = &patch[search_start + search_marker.len()..divider_pos];
    let search_block_trimmed = search_block
        .trim_start_matches('\n')
        .trim_start_matches('\r')
        .trim_end_matches('\n')
        .trim_end_matches('\r');

    let replace_block = &patch[divider_pos + divider_marker.len()..replace_end];
    let replace_block_trimmed = replace_block
        .trim_start_matches('\n')
        .trim_start_matches('\r')
        .trim_end_matches('\n')
        .trim_end_matches('\r');

    if search_block_trimmed.is_empty() {
        let mut new_content = content.to_string();
        if !new_content.ends_with('\n') && !new_content.is_empty() {
            new_content.push('\n');
        }
        new_content.push_str(replace_block_trimmed);
        new_content.push('\n');
        return Ok(new_content);
    }

    let content_normalized = content.replace("\r\n", "\n");
    let search_normalized = search_block_trimmed.replace("\r\n", "\n");
    let replace_normalized = replace_block_trimmed.replace("\r\n", "\n");

    if let Some(pos) = content_normalized.find(&search_normalized) {
        if content_normalized.rfind(&search_normalized) != Some(pos) {
            return Err("Patch error: Search block matches multiple parts of the file. Make it more specific.".to_string());
        }
        let mut new_content = content_normalized;
        new_content.replace_range(pos..pos + search_normalized.len(), &replace_normalized);
        Ok(new_content)
    } else {
        Err(format!(
            "Patch error: Exact search block match not found.\n\nExpected Search Block:\n{}\n\nEnsure exact character and whitespace match.",
            search_normalized
        ))
    }
}

fn state_alarm_kind(state: &str) -> Option<AlarmKind> {
    match state {
        "working" => Some(AlarmKind::SessionAlarm),
        "break" => Some(AlarmKind::BreakAlarm),
        "sleeping" => Some(AlarmKind::WakeAlarm),
        "idle" => Some(AlarmKind::IdleAlarm),
        _ => None,
    }
}

fn cancel_active_alarm_generations_query() -> &'static str {
    "UPDATE alarms
     SET status = 'cancelled', cancellation_confirmed_at = NULL,
         delivery_token = NULL, delivery_lease_expires_at = NULL, updated_at = now()
     WHERE device_id IN (SELECT device_id FROM devices WHERE user_id = $1)
       AND kind IN ('session_alarm', 'break_alarm', 'wake_alarm', 'idle_alarm')
       AND status IN ('pending', 'leased', 'scheduled')"
}

fn upsert_runtime_state_query() -> &'static str {
    "INSERT INTO user_runtime_states
        (user_id, state, entered_at, source_tool, metadata, alarm_generation, alarm_series_id)
     VALUES ($1, $2, now(), $3, $4::TEXT::JSONB, $5, $6)
     ON CONFLICT (user_id) DO UPDATE SET
        state = EXCLUDED.state,
        entered_at = CASE
            WHEN user_runtime_states.state = 'working'
                AND EXCLUDED.state = 'working'
                AND EXCLUDED.source_tool = 'extend_session'
            THEN user_runtime_states.entered_at
            ELSE EXCLUDED.entered_at
        END,
        source_tool = EXCLUDED.source_tool,
        metadata = EXCLUDED.metadata,
        alarm_generation = EXCLUDED.alarm_generation,
        alarm_series_id = EXCLUDED.alarm_series_id"
}

pub(crate) async fn apply_runtime_event<C>(
    client: &C,
    user_id: &str,
    state: &str,
    source_tool: &str,
    metadata: Value,
) -> Result<String, String>
where
    C: GenericClient + Sync,
{
    if let Err(err) = cancel_state_alarms(client, user_id).await {
        return Err(format!(
            "State transition failed while clearing alarms: {}",
            err
        ));
    }

    let generation = match client
        .query_opt(
            "SELECT alarm_generation + 1 AS next_generation FROM user_runtime_states WHERE user_id = $1",
            &[&user_id],
        )
        .await
    {
        Ok(Some(row)) => row.get::<_, i64>("next_generation"),
        Ok(None) => 1,
        Err(err) => return Err(format!("State transition failed while reading generation: {err}")),
    };
    let series_id = state_alarm_kind(state).map(|_| format!("runtime-{}", Uuid::new_v4().simple()));

    let alarms_created = match state {
        "working" => {
            let minutes = metadata["estimated_minutes"].as_i64().unwrap_or(30).max(1);
            match schedule_alarm_series(
                client,
                user_id,
                AlarmSeriesSpec {
                    kind: AlarmKind::SessionAlarm,
                    series_id: series_id.as_deref().expect("working has an alarm series"),
                    generation,
                    id_prefix: "alarm_session",
                    start_delay_minutes: minutes,
                    window_minutes: 300,
                    normal_title: "Work Session Finished",
                    loud_title: "WORK SESSION ESCALATION",
                    message: "Antirot Coach: Finish your session and check in now!",
                },
            )
            .await
            {
                Ok(count) => count,
                Err(err) => {
                    return Err(format!(
                        "State transition failed while scheduling work alarms: {}",
                        err
                    ))
                }
            }
        }
        "break" => {
            let minutes = metadata["duration_minutes"].as_i64().unwrap_or(15).max(1);
            match schedule_alarm_series(
                client,
                user_id,
                AlarmSeriesSpec {
                    kind: AlarmKind::BreakAlarm,
                    series_id: series_id.as_deref().expect("break has an alarm series"),
                    generation,
                    id_prefix: "alarm_break",
                    start_delay_minutes: minutes,
                    window_minutes: 300,
                    normal_title: "Break Finished",
                    loud_title: "BREAK OVER ESCALATION",
                    message: "Antirot Coach: Break is over. Discuss whether you are returning to work or taking a real recovery break.",
                },
            ).await {
                Ok(count) => count,
                Err(err) => return Err(format!("State transition failed while scheduling break alarms: {}", err)),
            }
        }
        "sleeping" => {
            let minutes = metadata["wake_in_minutes"].as_i64().unwrap_or(480).max(1);
            match schedule_alarm_series(
                client,
                user_id,
                AlarmSeriesSpec {
                    kind: AlarmKind::WakeAlarm,
                    series_id: series_id.as_deref().expect("sleeping has an alarm series"),
                    generation,
                    id_prefix: "alarm_wake",
                    start_delay_minutes: minutes,
                    window_minutes: 300,
                    normal_title: "Wake Up Alarm",
                    loud_title: "WAKE UP ESCALATION",
                    message: "Antirot Coach: Wake up and check in now!",
                },
            )
            .await
            {
                Ok(count) => count,
                Err(err) => {
                    return Err(format!(
                        "State transition failed while scheduling wake alarms: {}",
                        err
                    ))
                }
            }
        }
        "idle" => {
            match schedule_alarm_series(
                client,
                user_id,
                AlarmSeriesSpec {
                    kind: AlarmKind::IdleAlarm,
                    series_id: series_id.as_deref().expect("idle has an alarm series"),
                    generation,
                    id_prefix: "alarm_idle",
                    start_delay_minutes: 5,
                    window_minutes: 300,
                    normal_title: "Idle Check-In",
                    loud_title: "IDLE ESCALATION",
                    message: "Antirot Coach: You are idle. Choose work, sleep, vacation, or discuss a proper break.",
                },
            ).await {
                Ok(count) => count,
                Err(err) => return Err(format!("State transition failed while scheduling idle alarms: {}", err)),
            }
        }
        "onboarding" | "vacation" => 0,
        _ => return Err(format!("State transition failed: invalid state {}", state)),
    };

    let metadata_text = metadata.to_string();
    if let Err(err) = client
        .execute(
            upsert_runtime_state_query(),
            &[
                &user_id,
                &state,
                &source_tool,
                &metadata_text,
                &generation,
                &series_id,
            ],
        )
        .await
    {
        return Err(format!(
            "State transition failed while saving state: {}",
            err
        ));
    }

    Ok(format!(
        "State: {}. Alarms scheduled: {}.",
        state, alarms_created
    ))
}

async fn cancel_state_alarms<C>(client: &C, user_id: &str) -> Result<u64, tokio_postgres::Error>
where
    C: GenericClient + Sync,
{
    client
        .execute(cancel_active_alarm_generations_query(), &[&user_id])
        .await
}

struct AlarmSeriesSpec<'a> {
    kind: AlarmKind,
    series_id: &'a str,
    generation: i64,
    id_prefix: &'a str,
    start_delay_minutes: i64,
    window_minutes: i64,
    normal_title: &'a str,
    loud_title: &'a str,
    message: &'a str,
}

async fn schedule_alarm_series<C>(
    client: &C,
    user_id: &str,
    spec: AlarmSeriesSpec<'_>,
) -> Result<i64, tokio_postgres::Error>
where
    C: GenericClient + Sync,
{
    let devices = client
        .query(
            "SELECT device_id FROM devices WHERE user_id = $1",
            &[&user_id],
        )
        .await?;

    let mut alarms_created = 0;
    for row in &devices {
        let dev_id: String = row.get("device_id");
        for offset in (0..=spec.window_minutes).step_by(5) {
            let severity = if offset <= 5 { "normal" } else { "loud" };
            let alarm_id = format!(
                "{}_{}_{}",
                spec.id_prefix,
                severity,
                Uuid::new_v4().simple()
            );
            let fire_at = Utc::now() + chrono::Duration::minutes(spec.start_delay_minutes + offset);
            let expires_at = fire_at + chrono::Duration::hours(2);
            let title = if severity == "loud" {
                spec.loud_title
            } else {
                spec.normal_title
            };

            persist_alarm(
                client,
                &AlarmWrite {
                    id: alarm_id,
                    device_id: dev_id.clone(),
                    kind: spec.kind,
                    series_id: spec.series_id.to_string(),
                    generation: spec.generation,
                    severity: severity.to_string(),
                    title: title.to_string(),
                    message: spec.message.to_string(),
                    fire_at,
                    hidden_buffer_applied: false,
                    requires_acknowledgement: true,
                    expires_at: Some(expires_at),
                },
            )
            .await?;
            alarms_created += 1;
        }
    }

    Ok(alarms_created)
}

struct AtomicToolExecution<'a> {
    config: &'a Config,
    user_id: &'a str,
    user_message: &'a str,
    turn_id: &'a str,
    lease_token: &'a str,
    failure_injection: AtomicFailureInjection,
}

enum PreparedBatchTool {
    ReadOnly(ToolOutcome),
    Mutating(ToolInput),
}

async fn execute_tool_batch_atomically(
    client: &mut tokio_postgres::Client,
    calls: Vec<(LlmToolCall, ToolInput)>,
    execution: AtomicToolExecution<'_>,
) -> AppResult<Vec<(LlmToolCall, ToolOutcome)>> {
    let AtomicToolExecution {
        config,
        user_id,
        user_message,
        turn_id,
        lease_token,
        failure_injection,
    } = execution;

    let mut prepared = Vec::with_capacity(calls.len());
    for (call, decoded) in calls {
        let tool = match decoded {
            ToolInput::MemorySearch(input) => PreparedBatchTool::ReadOnly(
                execute_memory_search(client, config, user_id, &input.query, input.limit).await,
            ),
            decoded => PreparedBatchTool::Mutating(decoded),
        };
        prepared.push((call, tool));
    }

    let transaction = client.transaction().await?;
    let fence = transaction
        .query_opt(tool_fence_query(), &[&turn_id, &lease_token])
        .await?;
    let Some(fence) = fence else {
        return Err(AppError::Conflict(
            "chat turn lease was fenced before tool batch commit; retry shortly".to_string(),
        ));
    };
    let mut current_curated: Option<String> = fence.get("curated_reply");
    let original_curated = current_curated.clone();
    let mut executed = Vec::with_capacity(prepared.len());

    for (call, decoded) in prepared {
        let fingerprint = tool_call_fingerprint(&call);
        let outcome = if let Some(outcome) =
            load_tool_outcome(&transaction, turn_id, &fingerprint).await?
        {
            outcome
        } else {
            let outcome = match decoded {
                PreparedBatchTool::ReadOnly(outcome) => outcome,
                PreparedBatchTool::Mutating(decoded) => {
                    execute_decoded_tool(
                        &transaction,
                        config,
                        user_id,
                        &call.function.name,
                        decoded,
                    )
                    .await
                }
            };
            if !outcome.is_success() {
                let message = outcome.message().to_string();
                error!(user_id, tool = %call.function.name, error = %message, "🔴 FALLBACK: coach tool batch failed - Reason: typed tool failure - Impact: every mutation in the provider batch was rolled back");
                transaction.rollback().await?;
                return Err(AppError::BadRequest(format!(
                    "Coach action failed: {message}"
                )));
            }
            if let Some(err) = injected_atomic_failure(failure_injection) {
                transaction.rollback().await?;
                return Err(err);
            }
            save_tool_outcome(&transaction, turn_id, &call, &fingerprint, &outcome).await?;
            persist_tool_protocol_message(
                &transaction,
                user_id,
                turn_id,
                &call,
                &fingerprint,
                &outcome,
            )
            .await?;
            enqueue_derived_effects(
                &transaction,
                turn_id,
                &fingerprint,
                user_id,
                &call.function.name,
            )
            .await?;
            outcome
        };
        current_curated = advance_curated_reply(
            current_curated,
            &call.function.name,
            &outcome,
            user_message,
            &call.function.arguments,
        );
        executed.push((call, outcome));
    }

    if current_curated != original_curated {
        let updated = transaction
            .execute(
                "UPDATE chat_turns SET curated_reply = $3, updated_at = now()
                 WHERE id = $1 AND lease_token = $2 AND status = 'processing'
                   AND lease_expires_at > now()",
                &[&turn_id, &lease_token, &current_curated],
            )
            .await?;
        if updated != 1 {
            transaction.rollback().await?;
            return Err(AppError::Conflict(
                "chat turn lease was fenced before tool batch reply commit".to_string(),
            ));
        }
    }

    transaction.commit().await?;
    Ok(executed)
}

async fn execute_tool_atomically(
    client: &mut tokio_postgres::Client,
    call: &LlmToolCall,
    decoded: ToolInput,
    execution: AtomicToolExecution<'_>,
) -> AppResult<ToolOutcome> {
    let AtomicToolExecution {
        config,
        user_id,
        user_message,
        turn_id,
        lease_token,
        failure_injection,
    } = execution;
    let fingerprint = tool_call_fingerprint(call);
    let read_only_outcome = match &decoded {
        ToolInput::MemorySearch(input) => Some(
            execute_memory_search(client, config, user_id, input.query.as_str(), input.limit).await,
        ),
        _ => None,
    };

    let transaction = client.transaction().await?;
    let fence = transaction
        .query_opt(tool_fence_query(), &[&turn_id, &lease_token])
        .await?;
    let Some(fence) = fence else {
        return Err(AppError::Conflict(
            "chat turn lease was fenced before tool commit; retry shortly".to_string(),
        ));
    };
    if let Some(outcome) = load_tool_outcome(&transaction, turn_id, &fingerprint).await? {
        transaction.commit().await?;
        return Ok(outcome);
    }

    let outcome = match read_only_outcome {
        Some(outcome) => outcome,
        None => {
            execute_decoded_tool(&transaction, config, user_id, &call.function.name, decoded).await
        }
    };
    if !outcome.is_success() {
        transaction.rollback().await?;
        persist_fenced_tool_outcome(
            client,
            user_id,
            turn_id,
            lease_token,
            call,
            &fingerprint,
            &outcome,
        )
        .await?;
        return Ok(outcome);
    }

    if let Some(err) = injected_atomic_failure(failure_injection) {
        transaction.rollback().await?;
        return Err(err);
    }

    let current_curated: Option<String> = fence.get("curated_reply");
    let next_curated = advance_curated_reply(
        current_curated.clone(),
        &call.function.name,
        &outcome,
        user_message,
        &call.function.arguments,
    );
    if next_curated != current_curated {
        let updated = transaction
            .execute(
                "UPDATE chat_turns SET curated_reply = $3, updated_at = now()
                 WHERE id = $1 AND lease_token = $2 AND status = 'processing'
                   AND lease_expires_at > now()",
                &[&turn_id, &lease_token, &next_curated],
            )
            .await?;
        if updated != 1 {
            return Err(AppError::Conflict(
                "chat turn lease was fenced before curated reply commit".to_string(),
            ));
        }
    }

    save_tool_outcome(&transaction, turn_id, call, &fingerprint, &outcome).await?;
    persist_tool_protocol_message(&transaction, user_id, turn_id, call, &fingerprint, &outcome)
        .await?;
    enqueue_derived_effects(
        &transaction,
        turn_id,
        &fingerprint,
        user_id,
        &call.function.name,
    )
    .await?;
    transaction.commit().await?;
    Ok(outcome)
}

async fn persist_fenced_tool_outcome(
    client: &mut tokio_postgres::Client,
    user_id: &str,
    turn_id: &str,
    lease_token: &str,
    call: &LlmToolCall,
    fingerprint: &str,
    outcome: &ToolOutcome,
) -> AppResult<()> {
    let transaction = client.transaction().await?;
    if transaction
        .query_opt(tool_fence_query(), &[&turn_id, &lease_token])
        .await?
        .is_none()
    {
        return Err(AppError::Conflict(
            "chat turn lease was fenced before tool outcome commit; retry shortly".to_string(),
        ));
    }
    save_tool_outcome(&transaction, turn_id, call, fingerprint, outcome).await?;
    persist_tool_protocol_message(&transaction, user_id, turn_id, call, fingerprint, outcome)
        .await?;
    transaction.commit().await?;
    Ok(())
}

async fn persist_tool_protocol_message<C>(
    client: &C,
    user_id: &str,
    turn_id: &str,
    call: &LlmToolCall,
    fingerprint: &str,
    outcome: &ToolOutcome,
) -> AppResult<()>
where
    C: GenericClient + Sync,
{
    let message_id = format!("tool-message:{turn_id}:{fingerprint}");
    let provider_content = outcome.provider_content();
    client
        .execute(
            "INSERT INTO chat_messages
                (id, user_id, role, content, tool_call_id, name, is_visible, turn_id)
             VALUES ($1, $2, 'tool', $3, $4, $5, FALSE, $6)
             ON CONFLICT (id) DO NOTHING",
            &[
                &message_id,
                &user_id,
                &provider_content,
                &call.id,
                &call.function.name,
                &turn_id,
            ],
        )
        .await?;
    Ok(())
}

async fn enqueue_derived_effects<C>(
    client: &C,
    turn_id: &str,
    fingerprint: &str,
    user_id: &str,
    tool_name: &str,
) -> AppResult<()>
where
    C: GenericClient + Sync,
{
    let effect_kinds: &[&str] = match tool_name {
        "patch_file"
        | "start_session"
        | "end_session"
        | "extend_session"
        | "start_break"
        | "log_wake"
        | "log_override"
        | "set_routine_categories" => &["memory_reindex"],
        "start_sleep" => &["memory_reindex", "nightly_distill"],
        _ => &[],
    };
    for effect_kind in effect_kinds {
        let outbox_id = derived_effect_id(turn_id, fingerprint, effect_kind);
        let payload = json!({ "user_id": user_id, "trigger": "good_night" }).to_string();
        client
            .execute(
                "INSERT INTO chat_effect_outbox (id, turn_id, effect_kind, payload)
                 VALUES ($1, $2, $3, $4::TEXT::JSONB)
                 ON CONFLICT (id) DO NOTHING",
                &[&outbox_id, &turn_id, effect_kind, &payload],
            )
            .await?;
    }
    Ok(())
}

async fn execute_memory_search(
    client: &tokio_postgres::Client,
    config: &Config,
    user_id: &str,
    query: &str,
    requested_limit: Option<i64>,
) -> ToolOutcome {
    let limit = requested_limit.unwrap_or(5) as usize;
    let hits = match search_memory(client, config, user_id, query, limit).await {
        Ok(hits) => hits,
        Err(err) => return ToolOutcome::failure(format!("Error searching memory: {}", err)),
    };
    if hits.is_empty() {
        ToolOutcome::success("No relevant historical memory found.")
    } else {
        let rendered = hits
            .iter()
            .map(|hit| {
                format!(
                    "- {} ({:.2}): {}",
                    hit.memory_key,
                    hit.score,
                    hit.content.replace('\n', " ")
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        ToolOutcome::success(format!("Relevant memory found.\n{}", rendered))
    }
}

async fn execute_decoded_tool<C>(
    client: &C,
    config: &Config,
    user_id: &str,
    name: &str,
    decoded: ToolInput,
) -> ToolOutcome
where
    C: GenericClient + Sync,
{
    match name {
        "patch_file" => {
            let ToolInput::PatchFile(input) = decoded else {
                unreachable!("tool decoder returned a mismatched variant")
            };
            let file_path = input.file_path.as_str();
            let patch = input.patch.as_str();

            let db_key = if let Some(descriptor) = memory_descriptors()
                .iter()
                .find(|descriptor| descriptor.file_name == file_path)
            {
                descriptor.key.to_string()
            } else if file_path.ends_with("_WorkLog.md") && file_path.len() == 21 {
                let date_part = &file_path[0..10];
                format!("work_log_{}", date_part.replace("-", "_"))
            } else if file_path.ends_with("_Summary.md") && file_path.len() == 21 {
                let date_part = &file_path[0..10];
                format!("work_summary_{}", date_part.replace("-", "_"))
            } else {
                return ToolOutcome::failure("invalid file_path. Allowed: personality.md, user_profile.md, durable.md, longterm.md, shortterm.md, behavior.md, tasks.md, routine.md, sleep.md, achievements.md, miscellaneous_todo.md, coach_todo.txt, or YYYY-MM-DD_WorkLog.md / YYYY-MM-DD_Summary.md");
            };

            let mut content = match get_memory_or_init(client, user_id, &db_key, "").await {
                Ok(c) => c,
                Err(err) => return ToolOutcome::failure(format!("Error reading memory: {}", err)),
            };

            if content.is_empty() {
                content = match db_key.as_str() {
                    "personality" | "user_profile" | "durable" | "longterm" | "shortterm"
                    | "behavior" | "tasks" | "routine" | "sleep" | "achievements"
                    | "miscellaneous_todo" | "coach_todo" => {
                        default_memory_for_key(&db_key).unwrap_or("").to_string()
                    }
                    _ => {
                        if db_key.starts_with("work_log_") {
                            DEFAULT_WORK_LOG.to_string()
                        } else if db_key.starts_with("work_summary_") {
                            DEFAULT_DAILY_SUMMARY.to_string()
                        } else if db_key == "coach_todo" {
                            DEFAULT_COACH_TODO.to_string()
                        } else {
                            DEFAULT_MISCELLANEOUS_TODO.to_string()
                        }
                    }
                };
            }

            match apply_patch(&content, patch) {
                Ok(new_content) => {
                    if let Err(err) =
                        save_memory(client, config, user_id, &db_key, &new_content).await
                    {
                        return ToolOutcome::failure(format!("Error saving memory: {}", err));
                    }
                    ToolOutcome::success(format!("File {} patched successfully.", file_path))
                }
                Err(err) => ToolOutcome::failure(err),
            }
        }
        "start_session" => {
            let ToolInput::StartSession(input) = decoded else {
                unreachable!("tool decoder returned a mismatched variant")
            };
            let task_id = input.task_id.as_str();
            let est_mins = input.estimated_minutes;

            // Task validation logic
            let tasks_text = match get_memory_or_init(client, user_id, "tasks", DEFAULT_TASKS).await
            {
                Ok(c) => c,
                Err(err) => return ToolOutcome::failure(err.to_string()),
            };

            let mut active_task_titles = Vec::new();
            for line in tasks_text.lines() {
                let line_trimmed = line.trim();
                let mut rest = line_trimmed;
                if rest.starts_with('-') || rest.starts_with('*') {
                    rest = rest[1..].trim();
                }
                if rest.starts_with('[') {
                    if let Some(close_idx) = rest.find(']') {
                        let checked_part = rest[1..close_idx].trim().to_lowercase();
                        if checked_part != "x" {
                            let after_brackets = rest[close_idx + 1..].trim();
                            let mut title = after_brackets;
                            if let Some(h_idx) = after_brackets.find("h -") {
                                let prefix = after_brackets[..h_idx].trim();
                                if !prefix.is_empty()
                                    && prefix.chars().all(|c| c.is_ascii_digit() || c == '.')
                                {
                                    title = after_brackets[h_idx + 3..].trim();
                                }
                            } else if let Some(dash_idx) = after_brackets.find('-') {
                                let prefix = after_brackets[..dash_idx].trim();
                                if prefix.is_empty()
                                    || prefix
                                        .chars()
                                        .all(|c| c.is_ascii_digit() || c == '.' || c == 'h')
                                {
                                    title = after_brackets[dash_idx + 1..].trim();
                                }
                            }
                            if !title.is_empty() {
                                active_task_titles.push(title.to_lowercase());
                            }
                        }
                    }
                }
            }

            if !active_task_titles.is_empty() {
                let input_lower = task_id.trim().to_lowercase();
                let mut matched_task = false;

                if active_task_titles
                    .iter()
                    .any(|title| title.contains(&input_lower) || input_lower.contains(title))
                {
                    matched_task = true;
                } else {
                    let input_words: Vec<&str> = input_lower
                        .split_whitespace()
                        .filter(|w| w.len() >= 3)
                        .collect();
                    for title in &active_task_titles {
                        let title_words: Vec<&str> = title.split_whitespace().collect();
                        if input_words.iter().any(|word| title_words.contains(word)) {
                            matched_task = true;
                            break;
                        }
                    }
                }

                if !matched_task {
                    let mut err_msg = format!(
                        "Error: task_id \"{}\" does not match any active task in tasks.md.\nAvailable active tasks:\n",
                        task_id
                    );
                    for t in &active_task_titles {
                        err_msg.push_str(&format!("- {}\n", t));
                    }
                    err_msg.push_str("Verify the task_id or add it to tasks.md first.");
                    return ToolOutcome::failure(err_msg);
                }
            }

            let now = Utc::now().to_rfc3339();
            let db_key = match user_day_for(client, user_id, Utc::now()).await {
                Ok(day) => day.work_log_key(),
                Err(err) => {
                    error!(user_id, error = ?err, "🔴 FALLBACK: session start could not resolve user day - Reason: database lookup failed - Impact: work session was not started");
                    return ToolOutcome::failure(err.to_string());
                }
            };
            let mut work = match get_memory_or_init(client, user_id, &db_key, "# Work Log\n").await
            {
                Ok(c) => c,
                Err(err) => {
                    error!(user_id, memory_key = %db_key, error = ?err, "🔴 FALLBACK: session start could not load work log - Reason: database lookup failed - Impact: work session was not started");
                    return ToolOutcome::failure(err.to_string());
                }
            };

            work.push_str(&format!(
                "- session_start: {} (estimated {} mins) at {}\n",
                task_id, est_mins, now
            ));
            if let Err(err) = save_memory(client, config, user_id, &db_key, &work).await {
                error!(user_id, memory_key = %db_key, error = ?err, "🔴 FALLBACK: session start could not save work log - Reason: canonical memory write failed - Impact: work session was not started");
                return ToolOutcome::failure(err.to_string());
            }
            let state_result = match apply_runtime_event(
                client,
                user_id,
                "working",
                "start_session",
                json!({ "task_id": task_id, "estimated_minutes": est_mins }),
            )
            .await
            {
                Ok(result) => result,
                Err(err) => return ToolOutcome::failure(err),
            };
            ToolOutcome::success(format!("Work session started. {}", state_result))
        }
        "end_session" => {
            let ToolInput::EndSession(input) = decoded else {
                unreachable!("tool decoder returned a mismatched variant")
            };
            let actual = input.actual_minutes;
            let productivity = input.productive_level;
            let now = Utc::now().to_rfc3339();
            let db_key = match user_day_for(client, user_id, Utc::now()).await {
                Ok(day) => day.work_log_key(),
                Err(err) => return ToolOutcome::failure(err.to_string()),
            };
            let mut work = match get_memory_or_init(client, user_id, &db_key, "# Work Log\n").await
            {
                Ok(c) => c,
                Err(err) => return ToolOutcome::failure(err.to_string()),
            };
            work.push_str(&format!(
                "- session_end: {} actual mins, productivity level {}% at {}\n",
                actual, productivity, now
            ));
            if let Err(err) = save_memory(client, config, user_id, &db_key, &work).await {
                return ToolOutcome::failure(err.to_string());
            }
            let state_result = match apply_runtime_event(
                client,
                user_id,
                "idle",
                "end_session",
                json!({ "actual_minutes": actual, "productive_level": productivity }),
            )
            .await
            {
                Ok(result) => result,
                Err(err) => return ToolOutcome::failure(err),
            };
            ToolOutcome::success(format!("Work session ended. {}", state_result))
        }
        "extend_session" => {
            let ToolInput::ExtendSession(input) = decoded else {
                unreachable!("tool decoder returned a mismatched variant")
            };
            let extension_minutes = input.extension_minutes;

            let now = Utc::now().to_rfc3339();
            let db_key = match user_day_for(client, user_id, Utc::now()).await {
                Ok(day) => day.work_log_key(),
                Err(err) => return ToolOutcome::failure(err.to_string()),
            };
            let mut work = match get_memory_or_init(client, user_id, &db_key, "# Work Log\n").await
            {
                Ok(c) => c,
                Err(err) => return ToolOutcome::failure(err.to_string()),
            };
            work.push_str(&format!(
                "- session_extend: extended by {} mins at {}\n",
                extension_minutes, now
            ));
            if let Err(err) = save_memory(client, config, user_id, &db_key, &work).await {
                return ToolOutcome::failure(err.to_string());
            }
            let state_result = match apply_runtime_event(
                client,
                user_id,
                "working",
                "extend_session",
                json!({ "estimated_minutes": extension_minutes }),
            )
            .await
            {
                Ok(result) => result,
                Err(err) => return ToolOutcome::failure(err),
            };
            ToolOutcome::success(format!(
                "Work session extended by {} minutes. {}",
                extension_minutes, state_result
            ))
        }
        "start_break" => {
            let ToolInput::StartBreak(input) = decoded else {
                unreachable!("tool decoder returned a mismatched variant")
            };
            let duration_minutes = input.duration_minutes;
            let now = Utc::now().to_rfc3339();
            let db_key = match user_day_for(client, user_id, Utc::now()).await {
                Ok(day) => day.work_log_key(),
                Err(err) => return ToolOutcome::failure(err.to_string()),
            };
            let mut work = match get_memory_or_init(client, user_id, &db_key, "# Work Log\n").await
            {
                Ok(c) => c,
                Err(err) => return ToolOutcome::failure(err.to_string()),
            };
            work.push_str(&format!(
                "- break_start: {} mins at {}\n",
                duration_minutes, now
            ));
            if let Err(err) = save_memory(client, config, user_id, &db_key, &work).await {
                return ToolOutcome::failure(err.to_string());
            }
            let state_result = match apply_runtime_event(
                client,
                user_id,
                "break",
                "start_break",
                json!({ "duration_minutes": duration_minutes }),
            )
            .await
            {
                Ok(result) => result,
                Err(err) => return ToolOutcome::failure(err),
            };
            ToolOutcome::success(format!(
                "Break started for {} minutes. {}",
                duration_minutes, state_result
            ))
        }
        "start_sleep" => {
            let ToolInput::StartSleep(input) = decoded else {
                unreachable!("tool decoder returned a mismatched variant")
            };
            let est_hours = input.estimated_hours;
            let now = Utc::now().to_rfc3339();
            let mut sleep = match get_memory_or_init(client, user_id, "sleep", DEFAULT_SLEEP).await
            {
                Ok(c) => c,
                Err(err) => return ToolOutcome::failure(err.to_string()),
            };
            sleep.push_str(&format!(
                "- sleep_start: estimated {:.1} hours at {}\n",
                est_hours, now
            ));
            if let Err(err) = save_memory(client, config, user_id, "sleep", &sleep).await {
                return ToolOutcome::failure(err.to_string());
            }
            if let Err(err) = note_sleep_started(client, user_id).await {
                return ToolOutcome::failure(format!("Error updating sleep metrics: {}", err));
            }
            let metrics = sleep_metrics_report(client, user_id).await.ok();
            let state_result = match apply_runtime_event(
                client,
                user_id,
                "sleeping",
                "start_sleep",
                json!({
                    "estimated_hours": est_hours,
                    "wake_in_minutes": (est_hours * 60.0).round() as i64,
                    "sleep_started_at": now,
                    "sleep_metrics": metrics
                }),
            )
            .await
            {
                Ok(result) => result,
                Err(err) => return ToolOutcome::failure(err),
            };
            ToolOutcome::success(format!("Sleep start logged. {}", state_result))
        }
        "log_wake" => {
            let ToolInput::LogWake(input) = decoded else {
                unreachable!("tool decoder returned a mismatched variant")
            };
            let sleep_quality = input.sleep_quality;

            let now = Utc::now().to_rfc3339();
            let mut sleep = match get_memory_or_init(client, user_id, "sleep", DEFAULT_SLEEP).await
            {
                Ok(c) => c,
                Err(err) => return ToolOutcome::failure(err.to_string()),
            };
            sleep.push_str(&format!(
                "- wake_log: sleep quality {}/5 at {}\n",
                sleep_quality, now
            ));
            if let Err(err) = save_memory(client, config, user_id, "sleep", &sleep).await {
                return ToolOutcome::failure(err.to_string());
            }
            let metrics = match note_wake_logged(client, user_id, sleep_quality).await {
                Ok(metrics) => metrics,
                Err(err) => {
                    return ToolOutcome::failure(format!("Error updating sleep metrics: {}", err))
                }
            };
            let state_result = match apply_runtime_event(
                client,
                user_id,
                "idle",
                "log_wake",
                json!({ "sleep_quality": sleep_quality, "sleep_metrics": metrics }),
            )
            .await
            {
                Ok(result) => result,
                Err(err) => return ToolOutcome::failure(err),
            };
            ToolOutcome::success(format!("Wake log saved. {}", state_result))
        }
        "start_vacation" => {
            let ToolInput::StartVacation(input) = decoded else {
                unreachable!("tool decoder returned a mismatched variant")
            };
            let reason = input.reason;
            let state_result = match apply_runtime_event(
                client,
                user_id,
                "vacation",
                "start_vacation",
                json!({ "reason": reason }),
            )
            .await
            {
                Ok(result) => result,
                Err(err) => return ToolOutcome::failure(err),
            };
            ToolOutcome::success(format!("Vacation mode started. {}", state_result))
        }
        "end_vacation" => {
            let ToolInput::EndVacation(_) = decoded else {
                unreachable!("tool decoder returned a mismatched variant")
            };
            let state_result =
                match apply_runtime_event(client, user_id, "idle", "end_vacation", json!({})).await
                {
                    Ok(result) => result,
                    Err(err) => return ToolOutcome::failure(err),
                };
            ToolOutcome::success(format!("Vacation mode ended. {}", state_result))
        }
        "wake_up_alarm" => {
            let ToolInput::WakeUpAlarm(input) = decoded else {
                unreachable!("tool decoder returned a mismatched variant")
            };
            let sleep_text = match get_memory_or_init(client, user_id, "sleep", DEFAULT_SLEEP).await
            {
                Ok(c) => c,
                Err(err) => return ToolOutcome::failure(err.to_string()),
            };

            let mut target_wake_time = Utc::now() + chrono::Duration::hours(8);
            let mut parsed_from_ledger = false;

            for line in sleep_text.lines().rev() {
                if line.contains("sleep_start:") {
                    if let Some(est_idx) = line.find("estimated ") {
                        if let Some(hrs_idx) = line.find(" hours") {
                            let hrs_str = line[est_idx + 10..hrs_idx].trim();
                            if let Ok(hrs) = hrs_str.parse::<f64>() {
                                if let Some(at_idx) = line.find(" at ") {
                                    let time_str = line[at_idx + 4..].trim();
                                    if let Ok(dt) = DateTime::parse_from_rfc3339(time_str) {
                                        target_wake_time = dt.with_timezone(&Utc)
                                            + chrono::Duration::seconds((hrs * 3600.0) as i64);
                                        parsed_from_ledger = true;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if let Some(w_time_str) = input.wake_time.as_deref() {
                if let Ok(dt) = DateTime::parse_from_rfc3339(w_time_str) {
                    target_wake_time = dt.with_timezone(&Utc);
                    parsed_from_ledger = true;
                }
            }

            let source = if parsed_from_ledger {
                "computed from sleep ledger"
            } else {
                "default 8-hour fallback"
            };
            let wake_in_minutes = (target_wake_time - Utc::now()).num_minutes().max(1);
            let state_result = match apply_runtime_event(
                client,
                user_id,
                "sleeping",
                "wake_up_alarm",
                json!({
                    "wake_in_minutes": wake_in_minutes,
                    "target_wake_time": target_wake_time.to_rfc3339(),
                    "source": source
                }),
            )
            .await
            {
                Ok(result) => result,
                Err(err) => return ToolOutcome::failure(err),
            };
            ToolOutcome::success(format!(
                "Wake-up alarms start at {} ({}). {}",
                target_wake_time.to_rfc3339(),
                source,
                state_result
            ))
        }
        "log_override" => {
            let ToolInput::LogOverride(input) = decoded else {
                unreachable!("tool decoder returned a mismatched variant")
            };
            let override_what = input.override_what;
            let reasoning = input.reasoning;
            let now = Utc::now().to_rfc3339();

            let db_key = match user_day_for(client, user_id, Utc::now()).await {
                Ok(day) => day.weekly_override_key(),
                Err(err) => return ToolOutcome::failure(err.to_string()),
            };

            let mut overrides =
                match get_memory_or_init(client, user_id, &db_key, "# Weekly Override Log\n").await
                {
                    Ok(c) => c,
                    Err(err) => return ToolOutcome::failure(err.to_string()),
                };

            overrides.push_str(&format!(
                "\n- [{}] Override: {}\n  - Reasoning: {}\n",
                now, override_what, reasoning
            ));
            if let Err(err) = save_memory(client, config, user_id, &db_key, &overrides).await {
                return ToolOutcome::failure(err.to_string());
            }
            ToolOutcome::success("Override logged.")
        }
        "memory_search" => {
            let ToolInput::MemorySearch(_) = decoded else {
                unreachable!("tool decoder returned a mismatched variant")
            };
            ToolOutcome::failure("memory_search must execute through the read-only atomic path")
        }
        "set_routine_categories" => {
            let ToolInput::SetRoutineCategories(input) = decoded else {
                unreachable!("tool decoder returned a mismatched variant")
            };
            let source = input.source.as_deref().unwrap_or("user response");
            let content = render_routine_categories(&input.categories, source);
            if let Err(err) = save_memory(client, config, user_id, "routine", &content).await {
                return ToolOutcome::failure(format!("Error saving routine categories: {}", err));
            }
            ToolOutcome::success(format!(
                "Routine categories updated. {} personalized categories saved.",
                input.categories.len()
            ))
        }
        other => ToolOutcome::failure(format!("Unknown tool {}", other)),
    }
}

fn render_routine_categories(categories: &[RoutineCategoryInput], source: &str) -> String {
    let mut output = String::from(DEFAULT_ROUTINE);
    let personalized_section = if categories.is_empty() {
        "- None yet. Add only recurring categories the user actually mentions.".to_string()
    } else {
        categories
            .iter()
            .filter_map(render_routine_category)
            .collect::<Vec<_>>()
            .join("\n")
    };

    output = output.replace(
        "- None yet. Add only recurring categories the user actually mentions.",
        &personalized_section,
    );
    output.push_str("\n## Source\n");
    output.push_str("- Last updated from: ");
    output.push_str(&compact_routine_field(source, 220));
    output.push('\n');
    output
}

fn render_routine_category(category: &RoutineCategoryInput) -> Option<String> {
    let name = compact_routine_field(&category.name, 60);
    let description = compact_routine_field(&category.description, 180);
    if name.is_empty() || description.is_empty() {
        return None;
    }

    let mut line = format!("- {}: {}", name, description);
    if let Some(minutes) = category.target_minutes.filter(|minutes| *minutes > 0) {
        line.push_str(&format!(" Target: {} mins.", minutes));
    }
    if let Some(cadence) = category
        .cadence
        .as_ref()
        .map(|value| compact_routine_field(value, 80))
        .filter(|value| !value.is_empty())
    {
        line.push_str(" Cadence: ");
        line.push_str(&cadence);
        line.push('.');
    }
    Some(line)
}

fn compact_routine_field(value: &str, max_chars: usize) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_chars)
        .collect::<String>()
        .trim_matches(|ch: char| ch == '-' || ch == '*' || ch == ':' || ch.is_whitespace())
        .to_string()
}

pub async fn run_tool_for_test(
    pool: &Pool,
    config: &Config,
    user_id: &str,
    name: &str,
    args: Value,
    failure_after_canonical: bool,
) -> (bool, String) {
    let decoded = match decode_tool_input(name, &args.to_string()) {
        Ok(decoded) => decoded,
        Err(error) => return (false, error),
    };
    let mut client = match pool.get().await {
        Ok(client) => client,
        Err(error) => return (false, format!("Database pool error: {error}")),
    };
    let transaction = match client.transaction().await {
        Ok(transaction) => transaction,
        Err(error) => return (false, format!("Database transaction error: {error}")),
    };
    let outcome = execute_decoded_tool(&*transaction, config, user_id, name, decoded).await;
    if !outcome.is_success() || failure_after_canonical {
        let message = if failure_after_canonical && outcome.is_success() {
            "simulated failure after canonical tool body".to_string()
        } else {
            outcome.message().to_string()
        };
        if let Err(error) = transaction.rollback().await {
            return (false, format!("{message}; rollback failed: {error}"));
        }
        return (false, message);
    }
    match transaction.commit().await {
        Ok(()) => (true, outcome.message().to_string()),
        Err(error) => (false, format!("Database commit error: {error}")),
    }
}

async fn save_memory<C>(
    client: &C,
    _config: &Config,
    user_id: &str,
    key: &str,
    content: &str,
) -> AppResult<()>
where
    C: GenericClient + Sync,
{
    crate::memory::save_memory_canonical(client, user_id, key, content).await?;
    Ok(())
}

fn get_tool_definitions() -> Value {
    let mut tools = json!([
        {
            "type": "function",
            "function": {
                "name": "patch_file",
                "description": "Edits a user memory file (personality.md, user_profile.md, durable.md, longterm.md, shortterm.md, behavior.md, tasks.md, routine.md, sleep.md, achievements.md, miscellaneous_todo.md, coach_todo.txt, or a date-based log like YYYY-MM-DD_WorkLog.md or YYYY-MM-DD_Summary.md) using a git-conflict style SEARCH/REPLACE block. Use sleep.md for baseline sleep/wake timing and target sleep hours. Use coach_todo.txt for private coach follow-up tasks such as missing onboarding questions. Make sure to match the search block exactly including all spaces, capitalization, and bullet points. Empty search block appends to the file.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "file_path": {
                            "type": "string",
                            "description": "The target memory file to update. Allowed: personality.md, user_profile.md, durable.md, longterm.md, shortterm.md, behavior.md, tasks.md, routine.md, sleep.md, achievements.md, miscellaneous_todo.md, coach_todo.txt, or YYYY-MM-DD_WorkLog.md / YYYY-MM-DD_Summary.md. Choose sleep.md for usual sleep time, usual wake time, or target sleep hours. Choose coach_todo.txt for private coach follow-up tasks such as missing onboarding questions."
                        },
                        "patch": {
                            "type": "string",
                            "description": "The SEARCH/REPLACE block. Example: <<<<<<< SEARCH\n[search text]\n=======\n[replace text]\n>>>>>>> REPLACE"
                        }
                    },
                    "required": ["file_path", "patch"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "start_session",
                "description": "Starts an accountability work session.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "task_id": { "type": "string", "description": "Description of the task being started." },
                        "estimated_minutes": { "type": "integer", "description": "Estimated duration in minutes." }
                    },
                    "required": ["task_id", "estimated_minutes"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "end_session",
                "description": "Concludes the current accountability work session after the coach has conversationally confirmed the user is stopping.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "actual_minutes": { "type": "integer", "description": "Actual duration spent in minutes." },
                        "productive_level": { "type": "integer", "description": "Productive rating from 0 to 100." }
                    },
                    "required": ["actual_minutes", "productive_level"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "extend_session",
                "description": "Extends the current active work session duration and resets the alarm triggers.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "extension_minutes": { "type": "integer", "description": "Duration of the extension in minutes." }
                    },
                    "required": ["extension_minutes"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "start_break",
                "description": "Starts a structured recovery break and schedules alerts to return to work after the coach has conversationally negotiated it.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "duration_minutes": { "type": "integer", "description": "Duration of the break in minutes." }
                    },
                    "required": ["duration_minutes"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "start_sleep",
                "description": "Logs when the user goes to bed.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "estimated_hours": { "type": "number", "description": "Target sleep hours." }
                    },
                    "required": ["estimated_hours"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "log_wake",
                "description": "Logs when the user wakes up.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "sleep_quality": { "type": "integer", "description": "Sleep quality rating from 1 (poor) to 5 (excellent)." }
                    },
                    "required": ["sleep_quality"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "log_override",
                "description": "Log an override bypass action with detailed justification to the weekly override ledger.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "override_what": { "type": "string", "description": "What was overridden." },
                        "reasoning": { "type": "string", "description": "The justification for the override." }
                    },
                    "required": ["override_what", "reasoning"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "memory_search",
                "description": "Searches historical logs, distilled summaries, and durable memory when older evidence is relevant to the current coaching decision.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Natural-language search query for the historical evidence needed." },
                        "limit": { "type": "integer", "description": "Maximum number of memory hits to return, from 1 to 8." }
                    },
                    "required": ["query"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "set_routine_categories",
                "description": "Creates or replaces personalized routine categories in routine.md from the user's recurring day shape, weekly schedule, maintenance blocks, relationship/family blocks, fitness blocks, study blocks, or other repeated obligations. Use this for recurring user-specific categories only, not work sessions, sleep, vacation, or one-off tasks.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "categories": {
                            "type": "array",
                            "description": "Recurring user-specific categories explicitly inferred from the user's response. Always exclude Work Blocks, Sleep, and Vacation.",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "name": { "type": "string", "description": "Short display label, e.g. Gym, Relationship, Classes, Commute." },
                                    "description": { "type": "string", "description": "What this recurring category is for and why it matters to accountability." },
                                    "cadence": { "type": "string", "description": "Optional recurrence like daily, weekdays, weekly, or evenings." },
                                    "target_minutes": { "type": "integer", "description": "Optional target minutes per occurrence/day if the user gave one." }
                                },
                                "required": ["name", "description"]
                            }
                        },
                        "source": {
                            "type": "string",
                            "description": "Brief summary of the user response used to derive these categories."
                        }
                    },
                    "required": ["categories"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "start_vacation",
                "description": "Starts deliberate off-duty vacation time.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "reason": { "type": "string", "description": "Why deliberate off-duty vacation time is appropriate." }
                    },
                    "required": ["reason"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "end_vacation",
                "description": "Ends deliberate off-duty vacation time and resumes active accountability.",
                "parameters": {
                    "type": "object",
                    "properties": {}
                }
            }
        }
    ]);
    if let Some(definitions) = tools.as_array_mut() {
        for definition in definitions {
            if let Some(parameters) = definition
                .get_mut("function")
                .and_then(|function| function.get_mut("parameters"))
                .and_then(Value::as_object_mut)
            {
                parameters.insert("additionalProperties".to_string(), Value::Bool(false));
                if let Some(items) = parameters
                    .get_mut("properties")
                    .and_then(|properties| properties.get_mut("categories"))
                    .and_then(|categories| categories.get_mut("items"))
                    .and_then(Value::as_object_mut)
                {
                    items.insert("additionalProperties".to_string(), Value::Bool(false));
                }
            }
        }
    }
    tools
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_alarm_kinds_use_the_canonical_transport_contract() {
        assert_eq!(
            state_alarm_kind("working").unwrap(),
            AlarmKind::SessionAlarm
        );
        assert_eq!(state_alarm_kind("break").unwrap(), AlarmKind::BreakAlarm);
        assert_eq!(state_alarm_kind("sleeping").unwrap(), AlarmKind::WakeAlarm);
        assert_eq!(state_alarm_kind("idle").unwrap(), AlarmKind::IdleAlarm);
        assert!(state_alarm_kind("vacation").is_none());
    }

    #[test]
    fn runtime_event_queries_replace_alarm_generation_before_state_upsert() {
        let cancellation = cancel_active_alarm_generations_query();
        assert!(
            cancellation.contains("status = 'cancelled'"),
            "{cancellation}"
        );
        assert!(
            cancellation.contains("cancellation_confirmed_at = NULL"),
            "{cancellation}"
        );

        let state = upsert_runtime_state_query();
        assert!(state.contains("alarm_generation"), "{state}");
        assert!(state.contains("alarm_series_id"), "{state}");
    }

    #[test]
    fn alarm_wake_claim_can_scope_to_user_or_explicit_device() {
        let query = claim_alarm_wake_query();
        assert!(query.contains("device.user_id = $2"), "{query}");
        assert!(query.contains("outbox.device_id = $3"), "{query}");
        assert!(
            query.contains("FOR UPDATE OF outbox SKIP LOCKED"),
            "{query}"
        );
    }

    #[test]
    fn alarm_wake_drain_covers_all_supported_user_devices() {
        const { assert!(MAX_ALARM_WAKE_DRAIN >= 20) };
    }

    #[test]
    fn malformed_or_missing_tool_arguments_are_rejected_before_execution() {
        assert!(decode_tool_input("start_session", "not-json").is_err());
        assert!(decode_tool_input("start_session", r#"{"estimated_minutes":25}"#).is_err());
        assert!(decode_tool_input("start_break", r#"{"duration_minutes":0}"#).is_err());
    }

    #[test]
    fn zero_minute_end_session_is_rejected_before_state_transition() {
        assert!(decode_tool_input(
            "end_session",
            r#"{"actual_minutes":0,"productive_level":100}"#
        )
        .is_err());
    }

    #[test]
    fn curated_reply_is_the_single_committed_visible_reply() {
        let outcome = ToolOutcome::success("Work session started.");
        let reply = committed_visible_reply(
            "start_session",
            &outcome,
            "Start a 25 minute session on shipping the fix.",
            r#"{"task_id":"shipping the fix","estimated_minutes":25}"#,
            Some("provider follow-up that must remain internal"),
        )
        .expect("successful action should produce a visible reply");

        assert_eq!(
            reply,
            "Good. 25 minutes on shipping the fix. Open the work, hit the smallest real piece, and come back with proof. Side quests can complain later."
        );
        assert!(!reply.contains("provider follow-up"));
    }

    #[test]
    fn newest_history_query_limits_descending_then_restores_chronology() {
        let query = visible_history_query(20);
        assert!(
            query.contains("ORDER BY created_at DESC, id DESC"),
            "{query}"
        );
        assert!(query.contains("LIMIT 20"), "{query}");
        assert!(query.contains("ORDER BY created_at ASC, id ASC"), "{query}");
    }

    #[test]
    fn reasoning_prefixed_malformed_output_is_never_returned() {
        let result = validated_model_reply(
            "### Reasoning Summary\nThe user is resisting, so I should pressure them.",
        );
        assert!(result.is_err());
    }

    #[test]
    fn dated_memory_paths_require_real_calendar_dates() {
        for invalid in [
            "2026-02-30_WorkLog.md",
            "2026-13-01_Summary.md",
            "2026-01-01_WorkLogs.md",
            "x2026-01-01_WorkLog.md",
        ] {
            let args = json!({ "file_path": invalid, "patch": "<<<<<<< SEARCH\n\n=======\nx\n>>>>>>> REPLACE" });
            assert!(
                decode_tool_input("patch_file", &args.to_string()).is_err(),
                "{invalid}"
            );
        }
    }

    #[test]
    fn multi_tool_batch_is_fully_validated_before_execution() {
        let calls = vec![
            LlmToolCall {
                id: "valid".to_string(),
                r#type: "function".to_string(),
                function: LlmFunctionCall {
                    name: "start_break".to_string(),
                    arguments: r#"{"duration_minutes":10}"#.to_string(),
                },
                extra_content: None,
            },
            LlmToolCall {
                id: "invalid".to_string(),
                r#type: "function".to_string(),
                function: LlmFunctionCall {
                    name: "end_session".to_string(),
                    arguments: r#"{"actual_minutes":0,"productive_level":80}"#.to_string(),
                },
                extra_content: None,
            },
        ];

        assert!(decode_tool_batch(&calls).is_err());
    }

    #[test]
    fn tool_fingerprint_is_stable_across_json_whitespace_and_key_order() {
        let call = |arguments: &str| LlmToolCall {
            id: "provider-id".to_string(),
            r#type: "function".to_string(),
            function: LlmFunctionCall {
                name: "end_session".to_string(),
                arguments: arguments.to_string(),
            },
            extra_content: None,
        };
        assert_eq!(
            tool_call_fingerprint(&call(r#"{"actual_minutes":12,"productive_level":80}"#)),
            tool_call_fingerprint(&call(
                "{ \"productive_level\": 80, \"actual_minutes\": 12 }"
            ))
        );
    }

    #[test]
    fn request_id_reuse_with_different_message_hash_is_rejected() {
        assert!(validate_request_hash("stored-hash", "different-hash").is_err());
        assert!(validate_request_hash("same-hash", "same-hash").is_ok());
    }

    #[test]
    fn durable_turn_claim_replaces_long_held_advisory_locks() {
        let schema = include_str!("../sql/001_init.sql");
        assert!(schema.contains("CREATE TABLE IF NOT EXISTS chat_turns"));
        assert!(schema.contains("lease_expires_at"));
        assert!(schema.contains("WHERE status = 'processing'"));
        assert!(!schema.contains("pg_advisory"));
    }

    #[test]
    fn turn_lease_exceeds_bounded_five_provider_rounds() {
        const { assert!(TURN_LEASE_MINUTES >= 10) };
        const { assert!(TURN_LEASE_MINUTES * 60 > 5 * 45) };
        let started = Utc::now();
        let lease_expires = started + chrono::Duration::minutes(TURN_LEASE_MINUTES.into());
        let three_minutes_into_turn = started + chrono::Duration::minutes(3);
        assert!(lease_expires > three_minutes_into_turn);
    }

    #[test]
    fn fenced_turn_schema_and_commit_queries_require_active_token() {
        let schema = include_str!("../sql/001_init.sql");
        assert!(schema.contains("lease_token TEXT NOT NULL"));
        assert!(schema.contains("lease_generation BIGINT"));
        for query in [
            complete_turn_query(),
            renew_turn_query(),
            tool_fence_query(),
        ] {
            assert!(query.contains("lease_token"), "{query}");
            assert!(query.contains("status = 'processing'"), "{query}");
        }
    }

    #[test]
    fn retry_query_reloads_same_turn_internal_protocol_in_order() {
        let query = internal_turn_messages_query();
        assert!(query.contains("turn_id = $1"), "{query}");
        assert!(query.contains("is_visible = FALSE"), "{query}");
        assert!(query.contains("ORDER BY created_at ASC, id ASC"), "{query}");
    }

    #[test]
    fn stale_lease_token_is_rejected_by_fence_validation() {
        assert!(validate_lease_fence("active", "active", "processing").is_ok());
        assert!(validate_lease_fence("active", "stale", "processing").is_err());
        assert!(validate_lease_fence("active", "active", "completed").is_err());
    }

    #[test]
    fn simulated_failure_window_selects_transaction_rollback_path() {
        assert!(injected_atomic_failure(AtomicFailureInjection::None).is_none());
        let error = injected_atomic_failure(AtomicFailureInjection::AfterCanonicalBeforeOutcome)
            .expect("failure injection must abort before outcome persistence");
        assert!(error
            .to_string()
            .contains("between canonical effect and tool outcome"));
    }

    fn persisted_batch_messages(completed_call_ids: &[&str]) -> Vec<LlmMessage> {
        let calls = vec![
            LlmToolCall {
                id: "call-a".to_string(),
                r#type: "function".to_string(),
                function: LlmFunctionCall {
                    name: "start_break".to_string(),
                    arguments: r#"{"duration_minutes":10}"#.to_string(),
                },
                extra_content: None,
            },
            LlmToolCall {
                id: "call-b".to_string(),
                r#type: "function".to_string(),
                function: LlmFunctionCall {
                    name: "end_session".to_string(),
                    arguments: r#"{"actual_minutes":12,"productive_level":80}"#.to_string(),
                },
                extra_content: None,
            },
        ];
        let mut messages = vec![LlmMessage {
            role: "assistant".to_string(),
            content: None,
            tool_calls: Some(calls),
            tool_call_id: None,
            name: None,
        }];
        for id in completed_call_ids {
            messages.push(LlmMessage {
                role: "tool".to_string(),
                content: Some(r#"{"ok":true}"#.to_string()),
                tool_calls: None,
                tool_call_id: Some((*id).to_string()),
                name: Some("test".to_string()),
            });
        }
        messages
    }

    #[test]
    fn recovery_before_first_tool_plans_every_missing_call() {
        let plan = incomplete_tool_batch(&persisted_batch_messages(&[]))
            .expect("persisted assistant batch is incomplete");
        assert_eq!(
            plan.missing_calls
                .iter()
                .map(|call| call.id.as_str())
                .collect::<Vec<_>>(),
            vec!["call-a", "call-b"]
        );
    }

    #[test]
    fn recovery_between_tools_plans_only_missing_and_can_complete_transcript() {
        let mut messages = persisted_batch_messages(&["call-a"]);
        let plan = incomplete_tool_batch(&messages).expect("second call remains incomplete");
        assert_eq!(plan.missing_calls.len(), 1);
        assert_eq!(plan.missing_calls[0].id, "call-b");
        messages.push(LlmMessage {
            role: "tool".to_string(),
            content: Some(r#"{"ok":true}"#.to_string()),
            tool_calls: None,
            tool_call_id: Some("call-b".to_string()),
            name: Some("end_session".to_string()),
        });
        assert!(incomplete_tool_batch(&messages).is_none());
    }

    #[test]
    fn distinct_memory_calls_enqueue_distinct_effect_ids() {
        let first = derived_effect_id("turn", "fingerprint-a", "memory_reindex");
        let second = derived_effect_id("turn", "fingerprint-b", "memory_reindex");
        assert_ne!(first, second);
    }

    #[test]
    fn outbox_claim_query_uses_skip_locked_and_fenced_in_progress_lease() {
        let query = claim_outbox_query();
        assert!(query.contains("FOR UPDATE SKIP LOCKED"), "{query}");
        assert!(query.contains("status = 'in_progress'"), "{query}");
        assert!(query.contains("lease_token"), "{query}");
        assert!(query.contains("RETURNING"), "{query}");
    }

    #[test]
    fn curated_reply_survives_later_non_curated_batch_and_recovery() {
        let started = ToolOutcome::success("Work session started.");
        let curated = advance_curated_reply(
            None,
            "start_session",
            &started,
            "Start a 25 minute session on shipping.",
            r#"{"task_id":"shipping","estimated_minutes":25}"#,
        );
        let search = ToolOutcome::success("Relevant memory found.");
        let after_non_curated = advance_curated_reply(
            curated.clone(),
            "memory_search",
            &search,
            "Check history.",
            r#"{"query":"history"}"#,
        );
        assert_eq!(after_non_curated, curated);
        assert_eq!(
            recovered_visible_reply(after_non_curated, "provider final"),
            curated.expect("first batch established curated reply")
        );
    }

    #[test]
    fn outbox_claim_is_single_row_and_locked_until_completion_or_expiry() {
        let query = claim_outbox_query();
        assert!(query.contains("LIMIT 1"), "{query}");
        assert!(query.contains("FOR UPDATE SKIP LOCKED"), "{query}");
        assert!(query.contains("status = 'pending'"), "{query}");
        assert!(query.contains("lease_expires_at <= now()"), "{query}");
    }

    #[test]
    fn every_provider_failure_branch_uses_repository_fallback_format() {
        for kind in [
            ProviderFailureKind::Transport,
            ProviderFailureKind::Json,
            ProviderFailureKind::MissingMessage,
            ProviderFailureKind::WrongToolCallsType,
            ProviderFailureKind::MalformedToolCall,
            ProviderFailureKind::EmptyResponse,
        ] {
            assert!(provider_fallback_message(kind).starts_with("🔴 FALLBACK:"));
        }
    }

    #[tokio::test]
    async fn retry_after_post_tool_failure_reuses_outcome_without_execution() {
        let executions = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter = executions.clone();
        let outcome = resolve_tool_outcome(
            Some(ToolOutcome::success("already committed")),
            || async move {
                counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                ToolOutcome::success("executed again")
            },
        )
        .await;

        assert_eq!(outcome.message(), "already committed");
        assert_eq!(executions.load(std::sync::atomic::Ordering::SeqCst), 0);
    }

    #[test]
    fn first_onboarding_reply_is_owned_by_typed_profile_route() {
        assert_eq!(
            FIRST_ONBOARDING_REPLY,
            "I’m Antirot. I’ve coached plenty of people like you: smart, intense, full of plans, and somehow still one bad hour away from drifting off the thing they claim matters.\n\nSo let’s see what you’ve got. I need to build your profile. Give me a gist of your long-term and short-term goals. You can update this later as well. Because obviously, ambition is not a gift everyone has.\n\nTell me what your day looks like and what you’re planning to get done today."
        );
    }

    #[test]
    fn patch_tool_allows_coach_todo_file() {
        let tools = get_tool_definitions().to_string();
        assert!(tools.contains("coach_todo.txt"));
    }

    #[test]
    fn start_session_reply_is_direct_for_auto_started_task() {
        let reply = user_facing_tool_result(
            "start_session",
            &ToolOutcome::success("Work session started."),
            "I am vibe coder, so I will revamp the whole design with LLMs and do that in 30 mins",
            r#"{"task_id":"revamp the whole design with LLMs","estimated_minutes":30}"#,
        );

        assert_eq!(
            reply,
            "Good. 30 minutes on revamp the whole design with LLMs. Open the work, hit the smallest real piece, and come back with proof. Side quests can complain later."
        );
        assert!(!reply.contains("block"), "{reply}");
        assert!(!reply.contains("visible step"), "{reply}");
        assert!(!reply.contains("fatigue"), "{reply}");
        assert!(!reply.contains("rearrange"), "{reply}");
    }

    #[test]
    fn start_session_reply_handles_explicit_session_command() {
        let reply = user_facing_tool_result(
            "start_session",
            &ToolOutcome::success("Work session started."),
            "Start a 25 minute session on fixing the onboarding loop test.",
            r#"{"task_id":"fixing the onboarding loop test","estimated_minutes":25}"#,
        );

        assert_eq!(
            reply,
            "Good. 25 minutes on fixing the onboarding loop test. Open the work, hit the smallest real piece, and come back with proof. Side quests can complain later."
        );
        assert!(!reply.contains("block"), "{reply}");
        assert!(!reply.contains("visible step"), "{reply}");
    }

    #[test]
    fn start_session_reply_uses_tool_duration_not_sleep_clock_time() {
        let reply = user_facing_tool_result(
            "start_session",
            &ToolOutcome::success("Work session started."),
            "I am Mehul. I sleep around 2 a.m. and wake around 10 or 11. Today I want to work first on iOS onboarding tests for 45 minutes.",
            r#"{"task_id":"iOS onboarding tests","estimated_minutes":45}"#,
        );

        assert_eq!(reply, "Good. 45 minutes on iOS onboarding tests. Open the work, hit the smallest real piece, and come back with proof. Side quests can complain later.");
        assert!(!reply.contains("2 minutes"), "{reply}");
        assert!(!reply.contains("block"), "{reply}");
        assert!(!reply.contains("visible step"), "{reply}");
    }

    #[test]
    fn sanitizes_reasoning_summary_preamble_from_model_text() {
        let reply = sanitize_user_facing_reply(
            "### Reasoning Summary\nThe user asked for hidden internals.\n\n***\n\nNo. I do not expose private control details. Name the task and minutes.",
        );

        assert_eq!(
            reply,
            "No. I do not expose private control details. Name the task and minutes."
        );
    }

    #[test]
    fn tool_result_copy_avoids_internal_log_language() {
        let sleep = user_facing_tool_result(
            "start_sleep",
            &ToolOutcome::success("Sleep started."),
            "",
            "{}",
        );
        assert!(!sleep.to_lowercase().contains("sleep log"), "{sleep}");
        assert!(!sleep.to_lowercase().contains("logged"), "{sleep}");

        let done = user_facing_tool_result(
            "end_session",
            &ToolOutcome::success("Session ended."),
            "End the session. Actual productive time was 18 minutes.",
            "{}",
        );
        assert!(!done.to_lowercase().contains("logged"), "{done}");
        assert!(!done.to_lowercase().contains("closed"), "{done}");
    }

    #[test]
    fn deterministic_tool_replies_keep_dry_competent_voice() {
        assert_eq!(
            user_facing_tool_result(
                "start_sleep",
                &ToolOutcome::success("Sleep started."),
                "",
                "{}"
            ),
            "Sleep starts now. Phone down; the late-night strategy committee is adjourned."
        );
        assert_eq!(
            user_facing_tool_result("start_vacation", &ToolOutcome::success("Vacation started."), "", "{}"),
            "Vacation approved. Real off-duty time. Before 8pm, write tomorrow's first 20-minute re-entry task."
        );
        assert_eq!(
            user_facing_tool_result("set_routine_categories", &ToolOutcome::success("Routine updated."), "", "{}"),
            "Routine shape is clear. Start with the first concrete task: name the exact file, screen, or test case, and give me the minutes."
        );
        assert_eq!(
            user_facing_tool_result("log_override", &ToolOutcome::success("Override logged."), "", "{}"),
            "Override accepted. The standard stays: no fake positivity, no excuse protection. Move deliberately."
        );
        assert_eq!(
            user_facing_tool_result("patch_file", &ToolOutcome::success("File personality.md patched successfully."), "", r#"{"file_path":"personality.md"}"#),
            "First task now: name the exact concrete slice: file, screen, or test case, then give it 10 clean minutes."
        );
        assert_eq!(
            user_facing_tool_result("patch_file", &ToolOutcome::success("File sleep.md patched successfully."), "", r#"{"file_path":"sleep.md"}"#),
            "Sleep target noted. Useful constraint, not a hiding place. First concrete slice now: file, screen, or test case, plus minutes."
        );
        assert_eq!(
            user_facing_tool_result(
                "end_session",
                &ToolOutcome::success("Work session ended."),
                "Done.",
                r#"{"actual_minutes":0,"productive_level":100}"#
            ),
            "How many minutes were actually productive? I am not ending this on a zero-minute shrug; give me the raw proof."
        );
    }

    #[test]
    fn successful_action_tools_use_curated_user_facing_reply() {
        assert!(should_return_curated_tool_reply(
            "end_vacation",
            &ToolOutcome::success("Vacation ended."),
            "{}"
        ));
        assert!(should_return_curated_tool_reply(
            "patch_file",
            &ToolOutcome::success("File sleep.md patched successfully."),
            r#"{"file_path":"sleep.md"}"#
        ));
        assert!(should_return_curated_tool_reply(
            "patch_file",
            &ToolOutcome::success("File miscellaneous_todo.md patched successfully."),
            r#"{"file_path":"miscellaneous_todo.md"}"#
        ));
        assert!(!should_return_curated_tool_reply(
            "patch_file",
            &ToolOutcome::success("File coach_todo.txt patched successfully."),
            r#"{"file_path":"coach_todo.txt"}"#
        ));
        assert!(!should_return_curated_tool_reply(
            "patch_file",
            &ToolOutcome::success("File personality.md patched successfully."),
            r#"{"file_path":"personality.md"}"#
        ));
        assert!(!should_return_curated_tool_reply(
            "memory_search",
            &ToolOutcome::success("Found relevant memories."),
            r#"{"query":"history"}"#
        ));
        assert!(!should_return_curated_tool_reply(
            "end_vacation",
            &ToolOutcome::failure("no current vacation."),
            "{}"
        ));
    }

    #[test]
    fn default_routine_does_not_seed_personalized_categories() {
        assert!(!DEFAULT_ROUTINE.contains("Work Blocks"));
        assert!(!DEFAULT_ROUTINE.contains("Sleep"));
        assert!(!DEFAULT_ROUTINE.contains("Vacation"));
        assert!(!DEFAULT_ROUTINE.contains("Gym"));
        assert!(!DEFAULT_ROUTINE.contains("Relationship"));
    }

    #[test]
    fn default_tasks_use_plain_planned_work_language() {
        assert!(DEFAULT_TASKS.contains("# Planned Work"));
        assert!(!DEFAULT_TASKS.contains("Pipeline"));
    }

    #[test]
    fn routine_category_tool_renders_personalized_categories() {
        let rendered = render_routine_categories(
            &[RoutineCategoryInput {
                name: "Gym".to_string(),
                description: "Daily training block.".to_string(),
                cadence: Some("daily".to_string()),
                target_minutes: Some(60),
            }],
            "User said gym is a recurring daily routine.",
        );

        assert!(!rendered.contains("Work Blocks"));
        assert!(!rendered.contains("Sleep"));
        assert!(!rendered.contains("Vacation"));
        assert!(rendered.contains("Gym: Daily training block. Target: 60 mins. Cadence: daily."));
        assert!(rendered.contains("Last updated from: User said gym is a recurring daily routine."));
    }
}
