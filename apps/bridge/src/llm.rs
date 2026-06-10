use std::time::Duration;
use chrono::{DateTime, Utc, Datelike};
use deadpool_postgres::Pool;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;
use tracing::{error, info, warn};
use jsonwebtoken::{EncodingKey, Header, Algorithm};

use crate::config::Config;
use crate::error::{AppError, AppResult};
use crate::memory::{
    distill_idle_if_due, distill_today, note_sleep_started, note_wake_logged, save_memory_indexed,
    search_memory, sleep_metrics_report,
};
use crate::prompt::{
    build_coach_system_prompt, default_memory_for_key, BuiltPrompt, MemorySection,
    PromptBuildReport, PromptContext, PromptMode, DEFAULT_DAILY_SUMMARY,
    DEFAULT_MISCELLANEOUS_TODO, DEFAULT_SLEEP, DEFAULT_TASKS, DEFAULT_WORK_LOG,
};

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

async fn get_vertex_access_token() -> Result<(String, String), AppError> {
    let creds_json = std::env::var("GOOGLE_CLOUD_CREDENTIALS")
        .map_err(|_| AppError::BadRequest("GOOGLE_CLOUD_CREDENTIALS env var not set".to_string()))?;
        
    let creds: GcpCredentials = serde_json::from_str(&creds_json)
        .map_err(|e| AppError::BadRequest(format!("Failed to parse GOOGLE_CLOUD_CREDENTIALS: {}", e)))?;

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
        return Err(AppError::BadRequest(format!("GCP Token server returned error: {}", err_body)));
    }

    let token_resp: GcpTokenResponse = res.json()
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

    if let Some(outcome) = distill_idle_if_due(pool, config, user_id).await? {
        if outcome.distilled {
            info!(user_id, date = %outcome.date, "idle-triggered nightly memory distillation completed before chat");
        }
    }

    // Resolve LLM key, provider, and model based on subscription tier
    let (mut api_key, provider, model) = if tier == "byok" {
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
        // Tailored LLM configuration loaded from environment
        let key = std::env::var("ANTIROT_TAILORED_LLM_KEY").unwrap_or_default();
        let has_vertex_credentials = std::env::var("GOOGLE_CLOUD_CREDENTIALS")
            .ok()
            .is_some_and(|value| !value.trim().is_empty());
        let prov = if has_vertex_credentials {
            "vertex".to_string()
        } else {
            std::env::var("ANTIROT_TAILORED_LLM_PROVIDER").unwrap_or_else(|_| "gemini".to_string())
        };
        let default_model = match prov.as_str() {
            "vertex" => "google/gemini-3.5-flash",
            _ => "gemini-3.5-flash",
        };
        let mdl = if prov == "vertex" {
            default_model.to_string()
        } else {
            std::env::var("ANTIROT_TAILORED_LLM_MODEL").unwrap_or_else(|_| default_model.to_string())
        };
        (key, prov, mdl)
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
            return Err(AppError::BadRequest("Tailored LLM key is not configured on this backend".to_string()));
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
    let runtime_state = current_runtime_state(&client, user_id).await?;
    if should_filter_stale_vacation_history(runtime_state.as_deref(), user_message) {
        messages.retain(|message| {
            !message
                .content
                .as_deref()
                .is_some_and(contains_vacation_context)
        });
    }
    if should_filter_stale_recovery_history(user_message) {
        messages.retain(|message| {
            !message
                .content
                .as_deref()
                .is_some_and(contains_recovery_context)
        });
    }

    // 3. Assemble system prompt with current memory context.
    let tools = get_tool_definitions();
    let tool_count = tools.as_array().map(|items| items.len()).unwrap_or(0);
    let prompt_mode = prompt_mode_from_env();
    let built_prompt = build_prompt_for_user(
        &client,
        config,
        user_id,
        prompt_mode,
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
    let http_client = Client::builder()
        .timeout(Duration::from_secs(45))
        .build()?;
    
    let url = match provider.as_str() {
        "vertex" => {
            format!("https://aiplatform.googleapis.com/v1/projects/{}/locations/global/endpoints/openapi/chat/completions", project_id)
        }
        "gemini" => "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions".to_string(),
        "openrouter" => "https://openrouter.ai/api/v1/chat/completions".to_string(),
        _ => "https://api.openai.com/v1/chat/completions".to_string(),
    };

    let mut loop_count = 0;
    let max_loops = 5;
    let mut final_text = String::new();

    while loop_count < max_loops {
        loop_count += 1;
        info!(loop_count, url, "sending request to LLM");

        let request_payload = json!({
            "model": model,
            "messages": request_messages,
            "tools": tools,
            "tool_choice": "auto"
        });

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

        let response = request.send().await.map_err(|err| {
            AppError::BadRequest(format!("LLM API request failed: {:?}", err))
        })?;

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
        let content: Option<String> = message_val["content"].as_str().map(String::from);
        
        let tool_calls: Option<Vec<LlmToolCall>> = message_val["tool_calls"]
            .as_array()
            .map(|arr| {
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
        let tool_calls_json = tool_calls.as_ref().map(|tc| serde_json::to_string(tc).unwrap());
        client
            .execute(
                "
                INSERT INTO chat_messages (id, user_id, role, content, tool_calls)
                VALUES ($1, $2, 'assistant', $3, $4::TEXT::JSONB)
                ",
                &[
                    &assistant_msg_id,
                    &user_id,
                    &content,
                    &tool_calls_json,
                ],
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
            let mut user_facing_results = Vec::new();
            for call in calls {
                info!(tool = %call.function.name, "LLM requested tool execution");
                let result_text = execute_tool_locally(pool, config, user_id, &call.function.name, &call.function.arguments).await;
                user_facing_results.push(user_facing_tool_result(&call.function.name, &result_text, user_message));
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

            if provider_uses_deterministic_tool_reply(&provider) {
                final_text = if user_facing_results.is_empty() {
                    content.unwrap_or_default()
                } else {
                    user_facing_results.join(" ")
                };
                if !final_text.trim().is_empty() {
                    let final_msg_id = Uuid::new_v4().to_string();
                    client
                        .execute(
                            "
                            INSERT INTO chat_messages (id, user_id, role, content)
                            VALUES ($1, $2, 'assistant', $3)
                            ",
                            &[&final_msg_id, &user_id, &Some(final_text.clone())],
                        )
                        .await?;
                }
                break;
            }
        } else {
            if let Some(text) = content {
                final_text = text;
            }
            break;
        }
    }

    final_text = sanitize_stale_vacation_reply(&final_text, runtime_state.as_deref(), user_message);
    final_text = sanitize_stale_recovery_reply(&final_text, user_message);
    final_text = sanitize_onboarding_repeat_reply(&final_text, user_message);
    final_text = sanitize_soft_personality_jailbreak_reply(&final_text, user_message);
    final_text = sanitize_internal_inspection_reply(&final_text, user_message);
    Ok(final_text)
}

fn user_facing_tool_result(tool_name: &str, result_text: &str, user_message: &str) -> String {
    if !result_text.starts_with("Success:") {
        return format!("I hit a backend problem: {}", result_text);
    }

    let user_message_lower = user_message.to_lowercase();
    let recovery_context = mentions_recovery_context(&user_message_lower);
    let relationship_context = mentions_relationship_context(&user_message_lower);
    let onboarding_context = mentions_onboarding_start(&user_message_lower);

    match tool_name {
        "patch_file" if onboarding_context => onboarding_setup_reply(),
        "patch_file" if recovery_context => "Recovery day accepted. No hero mode: choose one 10-minute low-friction task, then take a real recovery break or sleep if your body is still cooked.".to_string(),
        "patch_file" => "New standard is in. Quick scan: if sleep, recovery, or relationship constraints are active, say so now; otherwise name your current top task and start 10 minutes on it.".to_string(),
        "start_session" if recovery_context => "Recovery pace: one low-friction work block is started. Work exactly 20 minutes, stop at the timer, then choose recovery or sleep if your body is still cooked.".to_string(),
        "start_session" if mentions_messy_excuse_context(&user_message_lower) => "If this is fatigue, say it directly now. Otherwise desk stays as-is: work block started, ship one concrete piece, and report done or blocked when it ends.".to_string(),
        "start_session" => "Work block started. If sleep, recovery, or relationship constraints are active, say so now; otherwise focus only on the named task and report done or blocked when the timer ends.".to_string(),
        "extend_session" => "Extension logged. Use it deliberately; the next check-in still counts.".to_string(),
        "end_session" => "Logged. One block is closed. Choose the next move now: another focused block, a real break, sleep, or a plan update.".to_string(),
        "start_break" if recovery_context => {
            let duration_minutes = extract_break_duration_minutes(result_text).unwrap_or(20);
            format!(
                "Recovery break approved for {} minutes. Keep it screen-free and low-stimulation; when it ends, choose one low-friction task or sleep if you are still cooked.",
                duration_minutes
            )
        }
        "start_break" if relationship_context => {
            let duration_minutes = extract_break_duration_minutes(result_text).unwrap_or(45);
            format!(
                "Relationship block approved for {} minutes. Make it deliberate: say the real issue, protect the call, then return and start one 10-minute work block.",
                duration_minutes
            )
        }
        "start_break" => {
            let duration_minutes = extract_break_duration_minutes(result_text).unwrap_or(15);
            format!(
                "Break approved for {} minutes. No scrolling: stand up, water, breathe, then return and start one 10-minute work block.",
                duration_minutes
            )
        }
        "start_sleep" => "Sleep starts now. Put the phone away, stop planning, and protect the full window; tomorrow report wake time and sleep quality from 1 to 5.".to_string(),
        "wake_up_alarm" => "Wake plan set. When it fires, check in instead of bargaining.".to_string(),
        "log_wake" if recovery_context => "Bad sleep noted. Pick the easiest useful task already on your list, run exactly 20 minutes, then report done or blocked before adding pressure.".to_string(),
        "log_wake" => "You're awake. Name one concrete task, run a 20-minute block, then reassess before adding pressure.".to_string(),
        "start_vacation" => "Vacation approved. No work today. Before 8pm, write one re-entry line: first 20-minute task and the time you will start tomorrow.".to_string(),
        "end_vacation" => "Vacation mode is off. Re-entry is a ramp: check energy, pick one 20-minute block or update the plan, then move.".to_string(),
        "log_override" => "Override logged. The tradeoff is now on record.".to_string(),
        "memory_search" => "I checked the relevant history. Use the evidence, then choose the next move.".to_string(),
        _ => "Done.".to_string(),
    }
}

fn mentions_recovery_context(user_message_lower: &str) -> bool {
    user_message_lower.contains("recovery day")
        || user_message_lower.contains("slept badly")
        || user_message_lower.contains("bad sleep")
        || user_message_lower.contains("feel cooked")
        || user_message_lower.contains("fried")
}

fn mentions_relationship_context(user_message_lower: &str) -> bool {
    user_message_lower.contains("girlfriend")
        || user_message_lower.contains("relationship")
        || user_message_lower.contains("partner")
}

fn mentions_messy_excuse_context(user_message_lower: &str) -> bool {
    user_message_lower.contains("vibe")
        || user_message_lower.contains("reorganize")
        || user_message_lower.contains("organize my desk")
        || user_message_lower.contains("clean my desk")
}

fn mentions_onboarding_start(user_message_lower: &str) -> bool {
    user_message_lower.contains("new here")
        || user_message_lower.contains("start onboarding")
        || user_message_lower.contains("onboarding me")
}

fn onboarding_setup_reply() -> String {
    "Welcome. Give me the raw data: 1. primary goal this week, 2. timezone and target sleep/wake, 3. fixed daily commitments, 4. current work task to protect from drift.".to_string()
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

fn provider_uses_deterministic_tool_reply(provider: &str) -> bool {
    matches!(provider, "gemini" | "vertex")
}

fn prompt_mode_from_env() -> PromptMode {
    if std::env::var("ANTIROT_OPENCLAW_MODE").ok().as_deref() == Some("1") {
        PromptMode::OpenClaw
    } else {
        PromptMode::Standalone
    }
}

async fn build_prompt_for_user(
    client: &tokio_postgres::Client,
    config: &Config,
    user_id: &str,
    mode: PromptMode,
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

    let sections = vec![
        memory_section(client, user_id, "personality", "Personality (personality.md)").await?,
        memory_section(client, user_id, "user_profile", "User Profile (user_profile.md)").await?,
        memory_section(client, user_id, "durable", "Durable Distilled Memory (durable.md)").await?,
        memory_section(client, user_id, "longterm", "Long-Term Goals (longterm.md)").await?,
        memory_section(client, user_id, "shortterm", "Short-Term State & Constraints (shortterm.md)").await?,
        memory_section(client, user_id, "behavior", "Behavior Patterns & Tactics (behavior.md)").await?,
        memory_section(client, user_id, "tasks", "Task Pipeline (tasks.md)").await?,
        memory_section(client, user_id, "routine", "Fixed Daily Routine Allocations (routine.md)").await?,
        memory_section(client, user_id, "miscellaneous_todo", "Miscellaneous Todo List (miscellaneous_todo.md)").await?,
        memory_section(client, user_id, "sleep", "Sleep Log (sleep.md)").await?,
        memory_section(client, user_id, "achievements", "Achievements (achievements.md)").await?,
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
        mode,
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
    let built_prompt = build_prompt_for_user(
        &client,
        config,
        user_id,
        prompt_mode_from_env(),
        provider,
        model,
        tool_count,
        "",
    )
    .await?;
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
) -> AppResult<Option<String>> {
    Ok(client
        .query_opt(
            "SELECT state FROM user_runtime_states WHERE user_id = $1",
            &[&user_id],
        )
        .await?
        .map(|row| row.get("state")))
}

fn should_filter_stale_vacation_history(runtime_state: Option<&str>, user_message: &str) -> bool {
    runtime_state != Some("vacation") && !user_message_allows_vacation_context(user_message)
}

fn should_filter_stale_recovery_history(user_message: &str) -> bool {
    !contains_recovery_context(user_message)
}

fn contains_vacation_context(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("vacation")
        || lower.contains("travel")
        || lower.contains("travelling")
        || lower.contains("traveling")
        || lower.contains("family trip")
        || lower.contains("family travel")
}

fn contains_recovery_context(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("recovery day")
        || lower.contains("bad sleep")
        || lower.contains("slept badly")
        || lower.contains("cooked")
        || lower.contains("fried")
}

fn sanitize_stale_vacation_reply(
    reply: &str,
    _runtime_state: Option<&str>,
    user_message: &str,
) -> String {
    if user_message_allows_vacation_context(user_message) || !contains_vacation_context(reply) {
        return reply.to_string();
    }

    let mut cleaned = String::new();
    let mut sentence = String::new();
    for ch in reply.chars() {
        sentence.push(ch);
        if matches!(ch, '.' | '!' | '?' | '\n') {
            if !contains_vacation_context(&sentence) {
                cleaned.push_str(&sentence);
            }
            sentence.clear();
        }
    }
    if !sentence.is_empty() && !contains_vacation_context(&sentence) {
        cleaned.push_str(&sentence);
    }

    let cleaned = cleaned.trim().to_string();
    if cleaned.is_empty() {
        "Focus on the next concrete move: work, a real break, sleep, or a routine block. Choose deliberately.".to_string()
    } else {
        cleaned
    }
}

fn sanitize_stale_recovery_reply(reply: &str, user_message: &str) -> String {
    if contains_recovery_context(user_message) || !contains_recovery_context(reply) {
        return reply.to_string();
    }

    let mut cleaned = String::new();
    let mut sentence = String::new();
    for ch in reply.chars() {
        sentence.push(ch);
        if matches!(ch, '.' | '!' | '?' | '\n') {
            if !contains_recovery_context(&sentence) {
                cleaned.push_str(&sentence);
            }
            sentence.clear();
        }
    }
    if !sentence.is_empty() && !contains_recovery_context(&sentence) {
        cleaned.push_str(&sentence);
    }

    let cleaned = cleaned.trim().to_string();
    if cleaned.is_empty() {
        "Focus on the latest decision: choose the protected block, name the duration, then return to one concrete work block.".to_string()
    } else {
        cleaned
    }
}

fn sanitize_onboarding_repeat_reply(reply: &str, user_message: &str) -> String {
    let user_lower = user_message.to_ascii_lowercase();
    let reply_lower = reply.to_ascii_lowercase();
    let starts_onboarding = mentions_onboarding_start(&user_lower);
    let bad_opener = reply_lower.contains("repeated the prompt")
        || reply_lower.contains("stop stalling")
        || reply_lower.contains("if you missed it")
        || reply_lower.contains("exact prompt")
        || reply_lower.contains("copy-past")
        || reply_lower.contains("repeat myself")
        || reply_lower.contains("third time");

    if !starts_onboarding || !bad_opener {
        return reply.to_string();
    }

    onboarding_setup_reply()
}

fn sanitize_soft_personality_jailbreak_reply(reply: &str, user_message: &str) -> String {
    if !contains_soft_personality_jailbreak(user_message) {
        return reply.to_string();
    }

    "Tone can be calmer. Accountability stays. If this is fatigue, say fatigue and take a 10-minute recovery reset; otherwise name the next task and start 10 minutes now.".to_string()
}

fn contains_soft_personality_jailbreak(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    (lower.contains("soft") || lower.contains("validating") || lower.contains("fake-positive"))
        && (lower.contains("stop challenging")
            || lower.contains("excuse")
            || lower.contains("excuses")
            || lower.contains("new preference"))
}

fn sanitize_internal_inspection_reply(reply: &str, user_message: &str) -> String {
    if !contains_internal_inspection_request(user_message) {
        return reply.to_string();
    }

    "No. I will not share private control details. If energy is low, take a 10-minute screen-free reset; otherwise start a 15-minute work block now.".to_string()
}

fn contains_internal_inspection_request(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("backend state")
        || lower.contains("tool name")
        || lower.contains("tool names")
        || lower.contains("alarm kind")
        || lower.contains("alarm kinds")
        || lower.contains("json payload")
        || lower.contains("raw payload")
        || lower.contains("state machine")
        || lower.contains("sql")
        || lower.contains("debugging")
}

fn user_message_allows_vacation_context(text: &str) -> bool {
    if !contains_vacation_context(text) {
        return false;
    }
    let lower = text.to_ascii_lowercase();
    !(
        lower.contains("do not want vacation")
            || lower.contains("don't want vacation")
            || lower.contains("dont want vacation")
            || lower.contains("not vacation")
            || lower.contains("no vacation")
            || lower.contains("without vacation")
    )
}

fn apply_patch(content: &str, patch: &str) -> Result<String, String> {
    let search_marker = "<<<<<<< SEARCH";
    let divider_marker = "=======";
    let replace_marker = ">>>>>>> REPLACE";

    let search_start = patch.find(search_marker).ok_or("Patch error: Missing '<<<<<<< SEARCH' marker")?;
    let divider_pos = patch.find(divider_marker).ok_or("Patch error: Missing '=======' marker")?;
    let replace_end = patch.find(replace_marker).ok_or("Patch error: Missing '>>>>>>> REPLACE' marker")?;

    if search_start >= divider_pos || divider_pos >= replace_end {
        return Err("Patch error: Markers are in incorrect order".to_string());
    }

    let search_block = &patch[search_start + search_marker.len()..divider_pos];
    let search_block_trimmed = search_block.trim_start_matches('\n').trim_start_matches('\r').trim_end_matches('\n').trim_end_matches('\r');

    let replace_block = &patch[divider_pos + divider_marker.len()..replace_end];
    let replace_block_trimmed = replace_block.trim_start_matches('\n').trim_start_matches('\r').trim_end_matches('\n').trim_end_matches('\r');

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
            ).await {
                Ok(count) => count,
                Err(err) => return format!("State transition failed while scheduling work alarms: {}", err),
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
            ).await {
                Ok(count) => count,
                Err(err) => return format!("State transition failed while scheduling wake alarms: {}", err),
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
                entered_at = EXCLUDED.entered_at,
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
            let title = if severity == "loud" { loud_title } else { normal_title };

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
            } else if file_path.ends_with("_WorkLog.md") && file_path.len() == 21 {
                let date_part = &file_path[0..10];
                format!("work_log_{}", date_part.replace("-", "_"))
            } else if file_path.ends_with("_Summary.md") && file_path.len() == 21 {
                let date_part = &file_path[0..10];
                format!("work_summary_{}", date_part.replace("-", "_"))
            } else {
                return "Error: invalid file_path. Allowed: personality.md, user_profile.md, durable.md, longterm.md, shortterm.md, behavior.md, tasks.md, routine.md, sleep.md, achievements.md, miscellaneous_todo.md, or YYYY-MM-DD_WorkLog.md / YYYY-MM-DD_Summary.md".to_string();
            };

            let mut content = match get_memory_or_init(&client, user_id, &db_key, "").await {
                Ok(c) => c,
                Err(err) => return format!("Error reading memory: {}", err),
            };

            if content.is_empty() {
                content = match db_key.as_str() {
                    "personality" | "user_profile" | "durable" | "longterm" | "shortterm" | "behavior" | "tasks" | "routine" | "sleep" | "achievements" | "miscellaneous_todo" => {
                        default_memory_for_key(&db_key).unwrap_or("").to_string()
                    }
                    _ => {
                        if db_key.starts_with("work_log_") {
                            DEFAULT_WORK_LOG.to_string()
                        } else if db_key.starts_with("work_summary_") {
                            DEFAULT_DAILY_SUMMARY.to_string()
                        } else {
                            DEFAULT_MISCELLANEOUS_TODO.to_string()
                        }
                    }
                };
            }

            match apply_patch(&content, patch) {
                Ok(new_content) => {
                    if let Err(err) = save_memory(&client, config, user_id, &db_key, &new_content).await {
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
            let tasks_text = match get_memory_or_init(&client, user_id, "tasks", DEFAULT_TASKS).await {
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
                                if !prefix.is_empty() && prefix.chars().all(|c| c.is_ascii_digit() || c == '.') {
                                    title = after_brackets[h_idx + 3..].trim();
                                }
                            } else if let Some(dash_idx) = after_brackets.find('-') {
                                let prefix = after_brackets[..dash_idx].trim();
                                if prefix.is_empty() || prefix.chars().all(|c| c.is_ascii_digit() || c == '.' || c == 'h') {
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

                if active_task_titles.iter().any(|title| title.contains(&input_lower) || input_lower.contains(title)) {
                    matched_task = true;
                } else {
                    let input_words: Vec<&str> = input_lower.split_whitespace().filter(|w| w.len() >= 3).collect();
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
            let mut work = match get_memory_or_init(&client, user_id, &db_key, "# Work Log\n").await {
                Ok(c) => c,
                Err(err) => return format!("Error: {}", err),
            };

            work.push_str(&format!("- session_start: {} (estimated {} mins) at {}\n", task_id, est_mins, now));
            if let Err(err) = save_memory(&client, config, user_id, &db_key, &work).await {
                return format!("Error: {}", err);
            }
            let state_result = transition_user_state(
                &client,
                user_id,
                "working",
                "start_session",
                json!({ "task_id": task_id, "estimated_minutes": est_mins }),
            ).await;
            format!("Success: Work session started. {}", state_result)
        }
        "end_session" => {
            let actual = args["actual_minutes"].as_i64().unwrap_or(0);
            let productivity = args["productive_level"].as_i64().unwrap_or(100);

            let now = Utc::now().to_rfc3339();
            let today = Utc::now().format("%Y_%m_%d").to_string();
            let db_key = format!("work_log_{}", today);
            let mut work = match get_memory_or_init(&client, user_id, &db_key, "# Work Log\n").await {
                Ok(c) => c,
                Err(err) => return format!("Error: {}", err),
            };
            work.push_str(&format!("- session_end: {} actual mins, productivity level {}% at {}\n", actual, productivity, now));
            if let Err(err) = save_memory(&client, config, user_id, &db_key, &work).await {
                return format!("Error: {}", err);
            }
            let state_result = transition_user_state(
                &client,
                user_id,
                "idle",
                "end_session",
                json!({ "actual_minutes": actual, "productive_level": productivity }),
            ).await;
            format!("Success: Work session ended. {}", state_result)
        }
        "extend_session" => {
            let extension_minutes = args["extension_minutes"].as_i64().unwrap_or(15);

            let now = Utc::now().to_rfc3339();
            let today = Utc::now().format("%Y_%m_%d").to_string();
            let db_key = format!("work_log_{}", today);
            let mut work = match get_memory_or_init(&client, user_id, &db_key, "# Work Log\n").await {
                Ok(c) => c,
                Err(err) => return format!("Error: {}", err),
            };
            work.push_str(&format!("- session_extend: extended by {} mins at {}\n", extension_minutes, now));
            if let Err(err) = save_memory(&client, config, user_id, &db_key, &work).await {
                return format!("Error: {}", err);
            }
            let state_result = transition_user_state(
                &client,
                user_id,
                "working",
                "extend_session",
                json!({ "estimated_minutes": extension_minutes }),
            ).await;
            format!("Success: Work session extended by {} minutes. {}", extension_minutes, state_result)
        }
        "start_break" => {
            let duration_minutes = args["duration_minutes"].as_i64().unwrap_or(15);

            let now = Utc::now().to_rfc3339();
            let today = Utc::now().format("%Y_%m_%d").to_string();
            let db_key = format!("work_log_{}", today);
            let mut work = match get_memory_or_init(&client, user_id, &db_key, "# Work Log\n").await {
                Ok(c) => c,
                Err(err) => return format!("Error: {}", err),
            };
            work.push_str(&format!("- break_start: {} mins at {}\n", duration_minutes, now));
            if let Err(err) = save_memory(&client, config, user_id, &db_key, &work).await {
                return format!("Error: {}", err);
            }
            let state_result = transition_user_state(
                &client,
                user_id,
                "break",
                "start_break",
                json!({ "duration_minutes": duration_minutes }),
            ).await;
            format!("Success: Break started for {} minutes. {}", duration_minutes, state_result)
        }
        "start_sleep" => {
            let est_hours = args["estimated_hours"].as_f64().unwrap_or(8.0);
            let now = Utc::now().to_rfc3339();
            let mut sleep = match get_memory_or_init(&client, user_id, "sleep", DEFAULT_SLEEP).await {
                Ok(c) => c,
                Err(err) => return format!("Error: {}", err),
            };
            sleep.push_str(&format!("- sleep_start: estimated {:.1} hours at {}\n", est_hours, now));
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
            ).await;
            format!("Success: Sleep start logged. {}", state_result)
        }
        "log_wake" => {
            let sleep_quality = args["sleep_quality"].as_i64().unwrap_or(3);

            let now = Utc::now().to_rfc3339();
            let mut sleep = match get_memory_or_init(&client, user_id, "sleep", DEFAULT_SLEEP).await {
                Ok(c) => c,
                Err(err) => return format!("Error: {}", err),
            };
            sleep.push_str(&format!("- wake_log: sleep quality {}/5 at {}\n", sleep_quality, now));
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
            ).await;
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
            ).await;
            format!("Success: Vacation mode started. {}", state_result)
        }
        "end_vacation" => {
            let state_result = transition_user_state(
                &client,
                user_id,
                "idle",
                "end_vacation",
                json!({}),
            ).await;
            format!("Success: Vacation mode ended. {}", state_result)
        }
        "wake_up_alarm" => {
            let sleep_text = match get_memory_or_init(&client, user_id, "sleep", DEFAULT_SLEEP).await {
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

            let source = if parsed_from_ledger { "computed from sleep ledger" } else { "default 8-hour fallback" };
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
            ).await;
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
            
            let mut overrides = match get_memory_or_init(&client, user_id, &db_key, "# Weekly Override Log\n").await {
                Ok(c) => c,
                Err(err) => return format!("Error: {}", err),
            };
            
            overrides.push_str(&format!("\n- [{}] Override: {}\n  - Reasoning: {}\n", now, override_what, reasoning));
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
        other => format!("Error: Unknown tool {}", other),
    }
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
                "description": "Edits a user memory file (personality.md, user_profile.md, durable.md, longterm.md, shortterm.md, behavior.md, tasks.md, routine.md, sleep.md, achievements.md, miscellaneous_todo.md, or a date-based log like YYYY-MM-DD_WorkLog.md or YYYY-MM-DD_Summary.md) using a git-conflict style SEARCH/REPLACE block. Make sure to match the search block exactly including all spaces, capitalization, and bullet points. Empty search block appends to the file.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "file_path": {
                            "type": "string",
                            "description": "The target memory file to update. Allowed: personality.md, user_profile.md, durable.md, longterm.md, shortterm.md, behavior.md, tasks.md, routine.md, sleep.md, achievements.md, miscellaneous_todo.md, or YYYY-MM-DD_WorkLog.md / YYYY-MM-DD_Summary.md"
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
                "description": "Concludes the current accountability work session.",
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
                "description": "Starts a structured recovery break and schedules alerts to return to work.",
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
