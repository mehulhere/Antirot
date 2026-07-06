use chrono::{DateTime, Datelike, FixedOffset, SecondsFormat, Utc};
use deadpool_postgres::Pool;
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::config::Config;
use crate::error::{AppError, AppResult};
use crate::memory::{
    distill_idle_if_due, distill_today, note_sleep_started, note_wake_logged, save_memory_indexed,
    search_memory, sleep_metrics_report,
};
use crate::prompt::{
    build_coach_system_prompt, default_memory_for_key, BuiltPrompt, MemorySection,
    PromptBuildReport, PromptContext, DEFAULT_COACH_TODO, DEFAULT_DAILY_SUMMARY,
    DEFAULT_MISCELLANEOUS_TODO, DEFAULT_ROUTINE, DEFAULT_SLEEP, DEFAULT_TASKS, DEFAULT_WORK_LOG,
};

const EARLY_SESSION_MINIMUM_MINUTES: i64 = 5;

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

#[derive(Debug, Deserialize)]
struct RoutineCategoryInput {
    name: String,
    description: String,
    #[serde(default)]
    cadence: Option<String>,
    #[serde(default)]
    target_minutes: Option<i64>,
}

async fn get_vertex_access_token() -> Result<(String, String), AppError> {
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

    let client = reqwest::Client::new();
    let res = client
        .post(&creds.token_uri)
        .form(&[
            ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
            ("assertion", &jwt),
        ])
        .send()
        .await
        .map_err(|e| AppError::BadRequest(format!("Token request failed: {}", e)))?;

    if !res.status().is_success() {
        let err_body = res.text().await.unwrap_or_default();
        return Err(AppError::BadRequest(format!(
            "GCP Token server returned error: {}",
            err_body
        )));
    }

    let token_resp: GcpTokenResponse = res
        .json()
        .await
        .map_err(|e| AppError::BadRequest(format!("Failed to parse GCP token response: {}", e)))?;

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

pub async fn chat_with_coach(
    pool: &Pool,
    config: &Config,
    user_id: &str,
    user_message: &str,
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
    let byok_api_key: Option<String> = user_row.get("byok_api_key");
    let byok_provider: Option<String> = user_row.get("byok_provider");
    let active_until: Option<DateTime<Utc>> = user_row.get("subscription_active_until");

    // Check if subscription is active
    let is_active = status == "active"
        || active_until.map(|dt| dt > Utc::now()).unwrap_or(false)
        || user_id == "admin";

    if !is_active {
        return Ok("🔴 Antirot Coach: Your subscription is inactive. Please activate your subscription ($1/mo BYOK or $5/mo FocusEngine tailored LLM) in Settings to resume coaching.".to_string());
    }

    if is_first_onboarding_request(user_message) {
        return Ok(FIRST_ONBOARDING_REPLY.to_string());
    }

    if let Some(outcome) = distill_idle_if_due(pool, config, user_id).await? {
        if outcome.distilled {
            info!(user_id, date = %outcome.date, "idle-triggered nightly memory distillation completed before chat");
        }
    }

    // Resolve LLM key, provider, and model based on subscription tier
    let (mut api_key, provider, model) = if tier == "byok" && user_id != "admin" {
        let key = byok_api_key.unwrap_or_default();
        let prov = byok_provider.unwrap_or_else(|| "openai".to_string());
        let default_model = match prov.as_str() {
            "gemini" => "gemini-3.5-flash",
            "vertex" => "google/gemini-3.5-flash",
            "openrouter" => "meta-llama/llama-3-70b-instruct",
            _ => "gpt-4o-mini",
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
        .query(
            "
            SELECT role, content, tool_calls::TEXT as tool_calls, tool_call_id, name
            FROM chat_messages
            WHERE user_id = $1
              AND role IN ('user', 'assistant')
              AND tool_calls IS NULL
              AND tool_call_id IS NULL
              AND content IS NOT NULL
            ORDER BY created_at ASC
            LIMIT 20
            ",
            &[&user_id],
        )
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

    // Save user message to database
    let user_msg_id = Uuid::new_v4().to_string();
    client
        .execute(
            "
            INSERT INTO chat_messages (id, user_id, role, content)
            VALUES ($1, $2, 'user', $3)
            ",
            &[&user_msg_id, &user_id, &user_message],
        )
        .await?;

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
        _ => "https://api.openai.com/v1/chat/completions".to_string(),
    };

    let mut loop_count = 0;
    let max_loops = 5;
    let mut final_text = String::new();
    let mut start_session_reply_override: Option<String> = None;

    while loop_count < max_loops {
        loop_count += 1;
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
        if provider == "vertex" {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        } else if provider == "gemini" {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        } else if provider == "openrouter" {
            request = request
                .header("Authorization", format!("Bearer {}", api_key))
                .header("HTTP-Referer", "https://antirot.org")
                .header("X-Title", "Antirot Coaching Platform");
        } else {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request
            .send()
            .await
            .map_err(|err| AppError::BadRequest(format!("LLM API request failed: {:?}", err)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            error!(status = %status, body = %body, "LLM provider returned error");
            return Err(AppError::BadRequest(format!(
                "LLM provider error (status {}): {}",
                status, body
            )));
        }

        let response_json: Value = response.json().await.map_err(|err| {
            AppError::BadRequest(format!("Failed to parse LLM JSON response: {}", err))
        })?;

        let choice = &response_json["choices"][0];
        let message_val = &choice["message"];
        let content: Option<String> = message_val["content"]
            .as_str()
            .map(sanitize_user_facing_reply);

        let tool_calls: Option<Vec<LlmToolCall>> =
            message_val["tool_calls"].as_array().map(|arr| {
                arr.iter()
                    .filter_map(|item| serde_json::from_value(item.clone()).ok())
                    .collect()
            });

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
                INSERT INTO chat_messages (id, user_id, role, content, tool_calls)
                VALUES ($1, $2, 'assistant', $3, $4::TEXT::JSONB)
                ",
                &[&assistant_msg_id, &user_id, &content, &tool_calls_json],
            )
            .await?;

        if let Some(calls) = tool_calls {
            if calls.is_empty() {
                if let Some(text) = content {
                    final_text = text;
                }
                break;
            }

            let mut tool_results = Vec::new();
            for call in calls {
                info!(tool = %call.function.name, "LLM requested tool execution");
                let result_text = execute_tool_locally(
                    pool,
                    config,
                    user_id,
                    &call.function.name,
                    &call.function.arguments,
                )
                .await;
                let user_facing_reply = user_facing_tool_result(
                    &call.function.name,
                    &result_text,
                    user_message,
                    &call.function.arguments,
                );
                if call.function.name == "start_session" && result_text.starts_with("Success:") {
                    start_session_reply_override = Some(user_facing_reply.clone());
                }
                tool_results.push(format!("{}: {}", call.function.name, result_text));

                let tool_msg = LlmMessage {
                    role: "tool".to_string(),
                    content: Some(result_text.clone()),
                    tool_calls: None,
                    tool_call_id: Some(call.id.clone()),
                    name: Some(call.function.name.clone()),
                };
                request_messages.push(tool_msg.clone());
                messages.push(tool_msg);

                // Save tool result to DB
                let tool_msg_id = Uuid::new_v4().to_string();
                client
                    .execute(
                        "
                        INSERT INTO chat_messages (id, user_id, role, content, tool_call_id, name)
                        VALUES ($1, $2, 'tool', $3, $4, $5)
                        ",
                        &[
                            &tool_msg_id,
                            &user_id,
                            &Some(result_text),
                            &Some(call.id),
                            &Some(call.function.name),
                        ],
                    )
                    .await?;
            }
        } else {
            if let Some(text) = content {
                final_text = text;
            }
            break;
        }
    }

    if let Some(reply) = start_session_reply_override {
        final_text = reply;
    }

    Ok(sanitize_user_facing_reply(&final_text))
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

fn user_facing_tool_result(
    tool_name: &str,
    result_text: &str,
    user_message: &str,
    tool_arguments: &str,
) -> String {
    if !result_text.starts_with("Success:") {
        return format!("I hit a backend problem: {}", result_text);
    }

    match tool_name {
        "patch_file" if patched_file_from_result(result_text) == Some("sleep.md") => "Sleep baseline noted. Now pick today's first real task; bedtime trivia does not ship the app.".to_string(),
        "patch_file" if patched_file_from_result(result_text) == Some("miscellaneous_todo.md") => "Parked for later. Do not chase the shiny side quest; finish what is already open.".to_string(),
        "patch_file" if patched_file_from_result(result_text) == Some("coach_todo.txt") => "I will carry that forward. Stay with the question in front of you.".to_string(),
        "patch_file" => "New standard is in. Back to the arena: name the current top task and give it 10 clean minutes.".to_string(),
        "start_session" => start_session_reply(tool_arguments, user_message),
        "extend_session" => "Extra time granted. Use it like borrowed money: intentionally, with receipts at check-in.".to_string(),
        "end_session" => "Round finished. Choose the next move now: another focused run, a real break, sleep, or a plan update.".to_string(),
        "start_break" => {
            let duration_minutes = extract_break_duration_minutes(result_text).unwrap_or(15);
            format!(
                "Break approved: {} minutes. Make it an actual reset, not a tiny vacation with denial.",
                duration_minutes
            )
        }
        "start_sleep" => "Sleep starts now. Phone down. No last-minute life redesign in the blue-light courtroom.".to_string(),
        "wake_up_alarm" => "Wake plan set. When it fires, check in before the bargaining committee wakes up.".to_string(),
        "log_wake" => "You're awake. Pick one concrete task and run 20 minutes before your brain starts negotiating.".to_string(),
        "start_vacation" => "Vacation approved. Real vacation, not guilt cosplay. Before 8pm, write tomorrow's first 20-minute re-entry task.".to_string(),
        "end_vacation" => "Vacation is over. Gentle ramp, not heroic montage: choose one 20-minute task and begin.".to_string(),
        "log_override" => "Override accepted. Own the tradeoff, skip the self-fanfic, and move deliberately.".to_string(),
        "memory_search" => "I checked the relevant history. Use the evidence; no mythology required. Choose the next move.".to_string(),
        "set_routine_categories" => "Routine shape is clear. Now protect the next task from schedule soup.".to_string(),
        _ => "Handled. Next move.".to_string(),
    }
}

fn patched_file_from_result(result_text: &str) -> Option<&str> {
    result_text
        .strip_prefix("Success: File ")
        .and_then(|rest| rest.strip_suffix(" patched successfully."))
}

fn extract_break_duration_minutes(result_text: &str) -> Option<i64> {
    let marker = "Break started for ";
    let start = result_text.find(marker)? + marker.len();
    let digits: String = result_text[start..]
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect();
    digits.parse().ok()
}

fn start_session_reply(tool_arguments: &str, user_message: &str) -> String {
    let parsed_args: Value = serde_json::from_str(tool_arguments).unwrap_or(Value::Null);
    let task_from_args = parsed_args["task_id"].as_str().map(str::trim).filter(|task| !task.is_empty());
    let minutes_from_args = parsed_args["estimated_minutes"].as_i64().filter(|minutes| *minutes > 0);

    let lower = user_message.to_ascii_lowercase();
    let minutes = minutes_from_args.or_else(|| extract_first_integer(&lower)).unwrap_or(0);
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
            "Good. {} minutes on {}. Get it in front of you, attack the smallest real piece, and report back before your brain opens twelve tabs.",
            minutes, task
        )
    } else {
        format!(
            "Good. {} is the target. Get it in front of you, attack the smallest real piece, and report back before your brain opens twelve tabs.",
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
    let today_key = now.format("%Y_%m_%d").to_string();
    let today_log_key = format!("work_log_{}", today_key);
    let today_log = get_memory_or_init(client, user_id, &today_log_key, DEFAULT_WORK_LOG).await?;

    let mut combined_summaries = String::new();
    for i in 0..3 {
        let day = now - chrono::Duration::days(i);
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
        memory_section(client, user_id, "tasks", "Task Pipeline (tasks.md)").await?,
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

async fn get_memory_or_init(
    client: &tokio_postgres::Client,
    user_id: &str,
    key: &str,
    default: &str,
) -> AppResult<String> {
    let row = client
        .query_opt(
            "SELECT content FROM user_memories WHERE user_id = $1 AND memory_key = $2",
            &[&user_id, &key],
        )
        .await?;

    match row {
        Some(row) => Ok(row.get("content")),
        None => {
            client
                .execute(
                    "
                    INSERT INTO user_memories (user_id, memory_key, content)
                    VALUES ($1, $2, $3)
                    ON CONFLICT DO NOTHING
                    ",
                    &[&user_id, &key, &default],
                )
                .await?;
            Ok(default.to_string())
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

async fn transition_user_state(
    client: &tokio_postgres::Client,
    user_id: &str,
    state: &str,
    source_tool: &str,
    metadata: Value,
) -> String {
    if let Err(err) = cancel_state_alarms(client, user_id).await {
        return format!("State transition failed while clearing alarms: {}", err);
    }

    let alarms_created = match state {
        "working" => {
            let minutes = metadata["estimated_minutes"].as_i64().unwrap_or(30).max(1);
            match schedule_alarm_series(
                client,
                user_id,
                "session_alarm",
                "alarm_session",
                minutes,
                300,
                "Work Session Finished",
                "WORK SESSION ESCALATION",
                "Antirot Coach: Finish your session and check in now!",
            )
            .await
            {
                Ok(count) => count,
                Err(err) => {
                    return format!(
                        "State transition failed while scheduling work alarms: {}",
                        err
                    )
                }
            }
        }
        "break" => {
            let minutes = metadata["duration_minutes"].as_i64().unwrap_or(15).max(1);
            match schedule_alarm_series(
                client,
                user_id,
                "break_alarm",
                "alarm_break",
                minutes,
                300,
                "Break Finished",
                "BREAK OVER ESCALATION",
                "Antirot Coach: Break is over. Discuss whether you are returning to work or taking a real recovery break.",
            ).await {
                Ok(count) => count,
                Err(err) => return format!("State transition failed while scheduling break alarms: {}", err),
            }
        }
        "sleeping" => {
            let minutes = metadata["wake_in_minutes"].as_i64().unwrap_or(480).max(1);
            match schedule_alarm_series(
                client,
                user_id,
                "wake_alarm",
                "alarm_wake",
                minutes,
                300,
                "Wake Up Alarm",
                "WAKE UP ESCALATION",
                "Antirot Coach: Wake up and check in now!",
            )
            .await
            {
                Ok(count) => count,
                Err(err) => {
                    return format!(
                        "State transition failed while scheduling wake alarms: {}",
                        err
                    )
                }
            }
        }
        "idle" => {
            match schedule_alarm_series(
                client,
                user_id,
                "idle_alarm",
                "alarm_idle",
                5,
                300,
                "Idle Check-In",
                "IDLE ESCALATION",
                "Antirot Coach: You are idle. Choose work, sleep, vacation, or discuss a proper break.",
            ).await {
                Ok(count) => count,
                Err(err) => return format!("State transition failed while scheduling idle alarms: {}", err),
            }
        }
        "onboarding" | "vacation" => 0,
        _ => return format!("State transition failed: invalid state {}", state),
    };

    let metadata_text = metadata.to_string();
    if let Err(err) = client
        .execute(
            "
            INSERT INTO user_runtime_states (user_id, state, entered_at, source_tool, metadata)
            VALUES ($1, $2, now(), $3, $4::TEXT::JSONB)
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
                metadata = EXCLUDED.metadata
            ",
            &[&user_id, &state, &source_tool, &metadata_text],
        )
        .await
    {
        return format!("State transition failed while saving state: {}", err);
    }

    format!("State: {}. Alarms scheduled: {}.", state, alarms_created)
}

async fn cancel_state_alarms(
    client: &tokio_postgres::Client,
    user_id: &str,
) -> Result<u64, tokio_postgres::Error> {
    client
        .execute(
            "
            DELETE FROM alarms
            WHERE device_id IN (SELECT device_id FROM devices WHERE user_id = $1)
              AND kind IN ('session_alarm', 'break_alarm', 'wake_alarm', 'idle_alarm')
              AND status = 'pending'
            ",
            &[&user_id],
        )
        .await
}

async fn schedule_alarm_series(
    client: &tokio_postgres::Client,
    user_id: &str,
    kind: &str,
    id_prefix: &str,
    start_delay_minutes: i64,
    window_minutes: i64,
    normal_title: &str,
    loud_title: &str,
    message: &str,
) -> Result<i64, tokio_postgres::Error> {
    let devices = client
        .query(
            "SELECT device_id FROM devices WHERE user_id = $1",
            &[&user_id],
        )
        .await?;

    let mut alarms_created = 0;
    for row in &devices {
        let dev_id: String = row.get("device_id");
        for offset in (0..=window_minutes).step_by(5) {
            let severity = if offset <= 5 { "normal" } else { "loud" };
            let alarm_id = format!("{}_{}_{}", id_prefix, severity, Uuid::new_v4().simple());
            let fire_at = Utc::now() + chrono::Duration::minutes(start_delay_minutes + offset);
            let expires_at = fire_at + chrono::Duration::hours(2);
            let title = if severity == "loud" {
                loud_title
            } else {
                normal_title
            };

            let inserted = client
                .execute(
                    "
                    INSERT INTO alarms (id, device_id, kind, severity, title, message, fire_at, expires_at, status)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'pending')
                    ",
                    &[
                        &alarm_id,
                        &dev_id,
                        &kind,
                        &severity,
                        &title,
                        &message,
                        &fire_at,
                        &Some(expires_at),
                    ],
                )
                .await?;
            alarms_created += inserted as i64;
        }
    }

    Ok(alarms_created)
}

async fn execute_tool_locally(
    pool: &Pool,
    config: &Config,
    user_id: &str,
    name: &str,
    args_str: &str,
) -> String {
    let args: Value = serde_json::from_str(args_str).unwrap_or(Value::Null);
    let client = match pool.get().await {
        Ok(c) => c,
        Err(err) => return format!("Database pool error: {}", err),
    };

    match name {
        "patch_file" => {
            let file_path = args["file_path"].as_str().unwrap_or("");
            let patch = args["patch"].as_str().unwrap_or("");

            let db_key = if file_path == "longterm.md" {
                "longterm".to_string()
            } else if file_path == "personality.md" {
                "personality".to_string()
            } else if file_path == "user_profile.md" {
                "user_profile".to_string()
            } else if file_path == "durable.md" {
                "durable".to_string()
            } else if file_path == "shortterm.md" {
                "shortterm".to_string()
            } else if file_path == "behavior.md" {
                "behavior".to_string()
            } else if file_path == "tasks.md" {
                "tasks".to_string()
            } else if file_path == "routine.md" {
                "routine".to_string()
            } else if file_path == "sleep.md" {
                "sleep".to_string()
            } else if file_path == "achievements.md" {
                "achievements".to_string()
            } else if file_path == "miscellaneous_todo.md" {
                "miscellaneous_todo".to_string()
            } else if file_path == "coach_todo.txt" {
                "coach_todo".to_string()
            } else if file_path.ends_with("_WorkLog.md") && file_path.len() == 21 {
                let date_part = &file_path[0..10];
                format!("work_log_{}", date_part.replace("-", "_"))
            } else if file_path.ends_with("_Summary.md") && file_path.len() == 21 {
                let date_part = &file_path[0..10];
                format!("work_summary_{}", date_part.replace("-", "_"))
            } else {
                return "Error: invalid file_path. Allowed: personality.md, user_profile.md, durable.md, longterm.md, shortterm.md, behavior.md, tasks.md, routine.md, sleep.md, achievements.md, miscellaneous_todo.md, coach_todo.txt, or YYYY-MM-DD_WorkLog.md / YYYY-MM-DD_Summary.md".to_string();
            };

            let mut content = match get_memory_or_init(&client, user_id, &db_key, "").await {
                Ok(c) => c,
                Err(err) => return format!("Error reading memory: {}", err),
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
                        save_memory(&client, config, user_id, &db_key, &new_content).await
                    {
                        return format!("Error saving memory: {}", err);
                    }
                    format!("Success: File {} patched successfully.", file_path)
                }
                Err(err) => err,
            }
        }
        "start_session" => {
            let task_id = args["task_id"].as_str().unwrap_or("Unknown Task");
            let est_mins = args["estimated_minutes"].as_i64().unwrap_or(30);

            // Task validation logic
            let tasks_text =
                match get_memory_or_init(&client, user_id, "tasks", DEFAULT_TASKS).await {
                    Ok(c) => c,
                    Err(err) => return format!("Error: {}", err),
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
                    return err_msg;
                }
            }

            let now = Utc::now().to_rfc3339();
            let today = Utc::now().format("%Y_%m_%d").to_string();
            let db_key = format!("work_log_{}", today);
            let mut work = match get_memory_or_init(&client, user_id, &db_key, "# Work Log\n").await
            {
                Ok(c) => c,
                Err(err) => return format!("Error: {}", err),
            };

            work.push_str(&format!(
                "- session_start: {} (estimated {} mins) at {}\n",
                task_id, est_mins, now
            ));
            if let Err(err) = save_memory(&client, config, user_id, &db_key, &work).await {
                return format!("Error: {}", err);
            }
            let state_result = transition_user_state(
                &client,
                user_id,
                "working",
                "start_session",
                json!({ "task_id": task_id, "estimated_minutes": est_mins }),
            )
            .await;
            format!("Success: Work session started. {}", state_result)
        }
        "end_session" => {
            let actual = args["actual_minutes"].as_i64().unwrap_or(0);
            let productivity = args["productive_level"].as_i64().unwrap_or(100);
            let now = Utc::now().to_rfc3339();
            let today = Utc::now().format("%Y_%m_%d").to_string();
            let db_key = format!("work_log_{}", today);
            let mut work = match get_memory_or_init(&client, user_id, &db_key, "# Work Log\n").await
            {
                Ok(c) => c,
                Err(err) => return format!("Error: {}", err),
            };
            work.push_str(&format!(
                "- session_end: {} actual mins, productivity level {}% at {}\n",
                actual, productivity, now
            ));
            if let Err(err) = save_memory(&client, config, user_id, &db_key, &work).await {
                return format!("Error: {}", err);
            }
            let state_result = transition_user_state(
                &client,
                user_id,
                "idle",
                "end_session",
                json!({ "actual_minutes": actual, "productive_level": productivity }),
            )
            .await;
            format!("Success: Work session ended. {}", state_result)
        }
        "extend_session" => {
            let extension_minutes = args["extension_minutes"].as_i64().unwrap_or(15);

            let now = Utc::now().to_rfc3339();
            let today = Utc::now().format("%Y_%m_%d").to_string();
            let db_key = format!("work_log_{}", today);
            let mut work = match get_memory_or_init(&client, user_id, &db_key, "# Work Log\n").await
            {
                Ok(c) => c,
                Err(err) => return format!("Error: {}", err),
            };
            work.push_str(&format!(
                "- session_extend: extended by {} mins at {}\n",
                extension_minutes, now
            ));
            if let Err(err) = save_memory(&client, config, user_id, &db_key, &work).await {
                return format!("Error: {}", err);
            }
            let state_result = transition_user_state(
                &client,
                user_id,
                "working",
                "extend_session",
                json!({ "estimated_minutes": extension_minutes }),
            )
            .await;
            format!(
                "Success: Work session extended by {} minutes. {}",
                extension_minutes, state_result
            )
        }
        "start_break" => {
            let duration_minutes = args["duration_minutes"].as_i64().unwrap_or(15);
            let now = Utc::now().to_rfc3339();
            let today = Utc::now().format("%Y_%m_%d").to_string();
            let db_key = format!("work_log_{}", today);
            let mut work = match get_memory_or_init(&client, user_id, &db_key, "# Work Log\n").await
            {
                Ok(c) => c,
                Err(err) => return format!("Error: {}", err),
            };
            work.push_str(&format!(
                "- break_start: {} mins at {}\n",
                duration_minutes, now
            ));
            if let Err(err) = save_memory(&client, config, user_id, &db_key, &work).await {
                return format!("Error: {}", err);
            }
            let state_result = transition_user_state(
                &client,
                user_id,
                "break",
                "start_break",
                json!({ "duration_minutes": duration_minutes }),
            )
            .await;
            format!(
                "Success: Break started for {} minutes. {}",
                duration_minutes, state_result
            )
        }
        "start_sleep" => {
            let est_hours = args["estimated_hours"].as_f64().unwrap_or(8.0);
            let now = Utc::now().to_rfc3339();
            let mut sleep = match get_memory_or_init(&client, user_id, "sleep", DEFAULT_SLEEP).await
            {
                Ok(c) => c,
                Err(err) => return format!("Error: {}", err),
            };
            sleep.push_str(&format!(
                "- sleep_start: estimated {:.1} hours at {}\n",
                est_hours, now
            ));
            if let Err(err) = save_memory(&client, config, user_id, "sleep", &sleep).await {
                return format!("Error: {}", err);
            }
            if let Err(err) = note_sleep_started(&client, user_id).await {
                return format!("Error updating sleep metrics: {}", err);
            }
            if let Err(err) = distill_today(&client, config, user_id, "good_night").await {
                return format!("Error distilling nightly memory: {}", err);
            }
            let metrics = sleep_metrics_report(&client, user_id).await.ok();
            let state_result = transition_user_state(
                &client,
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
            .await;
            format!("Success: Sleep start logged. {}", state_result)
        }
        "log_wake" => {
            let sleep_quality = args["sleep_quality"].as_i64().unwrap_or(3);

            let now = Utc::now().to_rfc3339();
            let mut sleep = match get_memory_or_init(&client, user_id, "sleep", DEFAULT_SLEEP).await
            {
                Ok(c) => c,
                Err(err) => return format!("Error: {}", err),
            };
            sleep.push_str(&format!(
                "- wake_log: sleep quality {}/5 at {}\n",
                sleep_quality, now
            ));
            if let Err(err) = save_memory(&client, config, user_id, "sleep", &sleep).await {
                return format!("Error: {}", err);
            }
            let metrics = match note_wake_logged(&client, user_id, sleep_quality).await {
                Ok(metrics) => metrics,
                Err(err) => return format!("Error updating sleep metrics: {}", err),
            };
            let state_result = transition_user_state(
                &client,
                user_id,
                "idle",
                "log_wake",
                json!({ "sleep_quality": sleep_quality, "sleep_metrics": metrics }),
            )
            .await;
            format!("Success: Wake log saved. {}", state_result)
        }
        "start_vacation" => {
            let reason = args["reason"].as_str().unwrap_or("Vacation mode");
            let state_result = transition_user_state(
                &client,
                user_id,
                "vacation",
                "start_vacation",
                json!({ "reason": reason }),
            )
            .await;
            format!("Success: Vacation mode started. {}", state_result)
        }
        "end_vacation" => {
            let state_result =
                transition_user_state(&client, user_id, "idle", "end_vacation", json!({})).await;
            format!("Success: Vacation mode ended. {}", state_result)
        }
        "wake_up_alarm" => {
            let sleep_text =
                match get_memory_or_init(&client, user_id, "sleep", DEFAULT_SLEEP).await {
                    Ok(c) => c,
                    Err(err) => return format!("Error: {}", err),
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

            if let Some(w_time_str) = args["wake_time"].as_str() {
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
            let state_result = transition_user_state(
                &client,
                user_id,
                "sleeping",
                "wake_up_alarm",
                json!({
                    "wake_in_minutes": wake_in_minutes,
                    "target_wake_time": target_wake_time.to_rfc3339(),
                    "source": source
                }),
            )
            .await;
            format!(
                "Success: Wake-up alarms start at {} ({}). {}",
                target_wake_time.to_rfc3339(),
                source,
                state_result
            )
        }
        "log_override" => {
            let override_what = args["override_what"].as_str().unwrap_or("");
            let reasoning = args["reasoning"].as_str().unwrap_or("");
            let now = Utc::now().to_rfc3339();

            let iso_week = Utc::now().iso_week();
            let db_key = format!("override_{}_W{:02}", iso_week.year(), iso_week.week());

            let mut overrides = match get_memory_or_init(
                &client,
                user_id,
                &db_key,
                "# Weekly Override Log\n",
            )
            .await
            {
                Ok(c) => c,
                Err(err) => return format!("Error: {}", err),
            };

            overrides.push_str(&format!(
                "\n- [{}] Override: {}\n  - Reasoning: {}\n",
                now, override_what, reasoning
            ));
            if let Err(err) = save_memory(&client, config, user_id, &db_key, &overrides).await {
                return format!("Error: {}", err);
            }
            "Success: Override logged.".to_string()
        }
        "memory_search" => {
            let query = args["query"].as_str().unwrap_or("");
            let limit = args["limit"].as_i64().unwrap_or(5).clamp(1, 8) as usize;
            let hits = match search_memory(&client, config, user_id, query, limit).await {
                Ok(hits) => hits,
                Err(err) => return format!("Error searching memory: {}", err),
            };
            if hits.is_empty() {
                "Success: No relevant historical memory found.".to_string()
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
                format!("Success: Relevant memory found.\n{}", rendered)
            }
        }
        "set_routine_categories" => {
            let categories_value = args
                .get("categories")
                .cloned()
                .unwrap_or_else(|| Value::Array(Vec::new()));
            let categories: Vec<RoutineCategoryInput> =
                match serde_json::from_value(categories_value) {
                    Ok(value) => value,
                    Err(err) => return format!("Error parsing routine categories: {}", err),
                };
            let source = args["source"].as_str().unwrap_or("user response");
            let content = render_routine_categories(&categories, source);
            if let Err(err) = save_memory(&client, config, user_id, "routine", &content).await {
                return format!("Error saving routine categories: {}", err);
            }
            format!(
                "Success: Routine categories updated. {} personalized categories saved.",
                categories.len()
            )
        }
        other => format!("Error: Unknown tool {}", other),
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
) -> String {
    execute_tool_locally(pool, config, user_id, name, &args.to_string()).await
}

async fn save_memory(
    client: &tokio_postgres::Client,
    config: &Config,
    user_id: &str,
    key: &str,
    content: &str,
) -> AppResult<()> {
    save_memory_indexed(client, config, user_id, key, content).await
}

fn get_tool_definitions() -> Value {
    json!([
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
                "description": "Starts vacation mode when the user is deliberately off duty.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "reason": { "type": "string", "description": "Why vacation mode is appropriate." }
                    },
                    "required": ["reason"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "end_vacation",
                "description": "Ends vacation mode and resumes active accountability.",
                "parameters": {
                    "type": "object",
                    "properties": {}
                }
            }
        }
    ])
}

fn is_first_onboarding_request(user_message: &str) -> bool {
    user_message.contains("The user just shared their name during onboarding")
        && user_message.contains("Antirot first onboarding message")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_onboarding_request_returns_exact_deterministic_reply() {
        assert!(is_first_onboarding_request(
            "The user just shared their name during onboarding. Return the deterministic Antirot first onboarding message exactly.\nName: Mehul"
        ));

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
            "Success: Work session started.",
            "I am vibe coder, so I will revamp the whole design with LLMs and do that in 30 mins",
            r#"{"task_id":"revamp the whole design with LLMs","estimated_minutes":30}"#,
        );

        assert_eq!(
            reply,
            "Good. 30 minutes on revamp the whole design with LLMs. Get it in front of you, attack the smallest real piece, and report back before your brain opens twelve tabs."
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
            "Success: Work session started.",
            "Start a 25 minute session on fixing the onboarding loop test.",
            r#"{"task_id":"fixing the onboarding loop test","estimated_minutes":25}"#,
        );

        assert_eq!(
            reply,
            "Good. 25 minutes on fixing the onboarding loop test. Get it in front of you, attack the smallest real piece, and report back before your brain opens twelve tabs."
        );
        assert!(!reply.contains("block"), "{reply}");
        assert!(!reply.contains("visible step"), "{reply}");
    }

    #[test]
    fn start_session_reply_uses_tool_duration_not_sleep_clock_time() {
        let reply = user_facing_tool_result(
            "start_session",
            "Success: Work session started.",
            "I am Mehul. I sleep around 2 a.m. and wake around 10 or 11. Today I want to work first on iOS onboarding tests for 45 minutes.",
            r#"{"task_id":"iOS onboarding tests","estimated_minutes":45}"#,
        );

        assert_eq!(reply, "Good. 45 minutes on iOS onboarding tests. Get it in front of you, attack the smallest real piece, and report back before your brain opens twelve tabs.");
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
        let sleep = user_facing_tool_result("start_sleep", "Success: Sleep started.", "", "{}");
        assert!(!sleep.to_lowercase().contains("sleep log"), "{sleep}");
        assert!(!sleep.to_lowercase().contains("logged"), "{sleep}");

        let done = user_facing_tool_result(
            "end_session",
            "Success: Session ended.",
            "End the session. Actual productive time was 18 minutes.",
            "{}",
        );
        assert!(!done.to_lowercase().contains("logged"), "{done}");
        assert!(!done.to_lowercase().contains("closed"), "{done}");
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
