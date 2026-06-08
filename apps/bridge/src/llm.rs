use std::time::Duration;
use chrono::{DateTime, Utc};
use deadpool_postgres::Pool;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;
use tracing::{info, error};

use crate::config::Config;
use crate::error::{AppError, AppResult};

const DEFAULT_LONGTERM: &str = "# Long-Term Goals\n\n## Direction\n- Distilled long-term goals go here.\n\n## Standards\n- High standards, honest recovery, no fake praise.\n";
const DEFAULT_SHORTTERM: &str = "# Short-Term State\n\n## Current Priorities\n- Near-term priorities go here.\n\n## Constraints\n- Sleep, health, vacation mode go here.\n";
const DEFAULT_BEHAVIOR: &str = "# Behavior Memory\n\n## Recurring Patterns\n- Stable patterns go here.\n\n## Drift Tendencies\n- Known drift loops go here.\n\n## Accountability Styles\n- Tactics that work/fail go here.\n";

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

    // Resolve LLM key, provider, and model based on subscription tier
    let (api_key, provider, model) = if tier == "byok" {
        let key = byok_api_key.ok_or_else(|| {
            AppError::BadRequest("BYOK API key is missing. Please configure it in Settings.".to_string())
        })?;
        let prov = byok_provider.unwrap_or_else(|| "openai".to_string());
        let default_model = match prov.as_str() {
            "gemini" => "gemini-1.5-flash",
            "openrouter" => "meta-llama/llama-3-70b-instruct",
            _ => "gpt-4o-mini",
        };
        (key, prov, default_model.to_string())
    } else {
        // Tailored LLM configuration loaded from environment
        let key = std::env::var("ANTIROT_TAILORED_LLM_KEY").unwrap_or_default();
        let prov = std::env::var("ANTIROT_TAILORED_LLM_PROVIDER").unwrap_or_else(|_| "gemini".to_string());
        let mdl = std::env::var("ANTIROT_TAILORED_LLM_MODEL").unwrap_or_else(|_| "gemini-1.5-flash".to_string());
        if key.is_empty() {
            return Err(AppError::BadRequest("Tailored LLM Key is not configured on this bridge backend".to_string()));
        }
        (key, prov, mdl)
    };

    // 2. Fetch or initialize memories
    let longterm = get_memory_or_init(&client, user_id, "longterm", DEFAULT_LONGTERM).await?;
    let shortterm = get_memory_or_init(&client, user_id, "shortterm", DEFAULT_SHORTTERM).await?;
    let behavior = get_memory_or_init(&client, user_id, "behavior", DEFAULT_BEHAVIOR).await?;
    let tasks = get_memory_or_init(&client, user_id, "tasks", "# Task Pipeline\n").await?;
    let sleep = get_memory_or_init(&client, user_id, "sleep", "# Sleep Ledger\n").await?;
    let work = get_memory_or_init(&client, user_id, "work", "# Work Ledger\n").await?;
    let miscellaneous_todo = get_memory_or_init(&client, user_id, "miscellaneous_todo", "# Miscellaneous Todo\n").await?;

    // 3. Load chat history
    let history_rows = client
        .query(
            "
            SELECT role, content, tool_calls::TEXT as tool_calls, tool_call_id, name
            FROM chat_messages
            WHERE user_id = $1
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

    // 4. Assemble system prompt with current memory context
    let system_context = format!(
        "You are Antirot, a strict but intelligent sports coach for users with ADHD-like attention drift. You motivate through identity reinforcement, capability framing, standards, and memory of past work. You are emotionally restrained, skeptical of excuses, and rarely use praise. Your primary tool is natural chat, but you must invoke specialized tools to update the user's memory files. Never make generic file changes.

--- CURRENT USER MEMORY ---

### Long-term Goals (longterm.md):
{longterm}

### Short-term State & Constraints (shortterm.md):
{shortterm}

### Behavior Patterns & Tactics (behavior.md):
{behavior}

### Task Pipeline (tasks.md):
{tasks}

### Miscellaneous Todo List (miscellaneous_todo.md):
{miscellaneous_todo}

### Sleep Log (sleep.md):
{sleep}

### Work Log (work.md):
{work}
"
    );

    // Filter messages to prepend system context
    let mut request_messages = vec![LlmMessage {
        role: "system".to_string(),
        content: Some(system_context),
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
        "gemini" => "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions",
        "openrouter" => "https://openrouter.ai/api/v1/chat/completions",
        _ => "https://api.openai.com/v1/chat/completions",
    };

    let tools = get_tool_definitions();
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

        let mut request = http_client.post(url).json(&request_payload);
        if provider == "gemini" {
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
            AppError::BadRequest(format!("LLM API request failed: {}", err))
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
                VALUES ($1, $2, 'assistant', $3, $4::JSONB)
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

            for call in calls {
                info!(tool = %call.function.name, "LLM requested tool execution");
                let result_text = execute_tool_locally(pool, config, user_id, &call.function.name, &call.function.arguments).await;

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

    Ok(final_text)
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

            match file_path {
                "longterm.md" | "shortterm.md" | "behavior.md" | "tasks.md" | "sleep.md" | "work.md" | "miscellaneous_todo.md" => {}
                _ => return "Error: invalid file_path. Allowed: longterm.md, shortterm.md, behavior.md, tasks.md, sleep.md, work.md, miscellaneous_todo.md".to_string(),
            }

            let db_key = file_path.strip_suffix(".md").unwrap_or(file_path);
            let mut content = match get_memory_or_init(&client, user_id, db_key, "").await {
                Ok(c) => c,
                Err(err) => return format!("Error reading memory: {}", err),
            };

            if content.is_empty() {
                content = match db_key {
                    "longterm" => DEFAULT_LONGTERM.to_string(),
                    "shortterm" => DEFAULT_SHORTTERM.to_string(),
                    "behavior" => DEFAULT_BEHAVIOR.to_string(),
                    "tasks" => "# Task Pipeline\n".to_string(),
                    "sleep" => "# Sleep Ledger\n".to_string(),
                    "work" => "# Work Ledger\n".to_string(),
                    _ => "# Miscellaneous Todo\n".to_string(),
                };
            }

            match apply_patch(&content, patch) {
                Ok(new_content) => {
                    if let Err(err) = save_memory(&client, user_id, db_key, &new_content).await {
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
            let now = Utc::now().to_rfc3339();
            let mut work = match get_memory_or_init(&client, user_id, "work", "# Work Ledger\n").await {
                Ok(c) => c,
                Err(err) => return format!("Error: {}", err),
            };
            work.push_str(&format!("- session_start: {} (estimated {} mins) at {}\n", task_id, est_mins, now));
            if let Err(err) = save_memory(&client, user_id, "work", &work).await {
                return format!("Error: {}", err);
            }
            "Success: Work session started.".to_string()
        }
        "end_session" => {
            let actual = args["actual_minutes"].as_i64().unwrap_or(0);
            let productivity = args["productive_level"].as_i64().unwrap_or(100);
            let now = Utc::now().to_rfc3339();
            let mut work = match get_memory_or_init(&client, user_id, "work", "# Work Ledger\n").await {
                Ok(c) => c,
                Err(err) => return format!("Error: {}", err),
            };
            work.push_str(&format!("- session_end: {} actual mins, productivity level {}% at {}\n", actual, productivity, now));
            if let Err(err) = save_memory(&client, user_id, "work", &work).await {
                return format!("Error: {}", err);
            }
            "Success: Work session ended.".to_string()
        }
        "start_sleep" => {
            let est_hours = args["estimated_hours"].as_f64().unwrap_or(8.0);
            let now = Utc::now().to_rfc3339();
            let mut sleep = match get_memory_or_init(&client, user_id, "sleep", "# Sleep Ledger\n").await {
                Ok(c) => c,
                Err(err) => return format!("Error: {}", err),
            };
            sleep.push_str(&format!("- sleep_start: estimated {:.1} hours at {}\n", est_hours, now));
            if let Err(err) = save_memory(&client, user_id, "sleep", &sleep).await {
                return format!("Error: {}", err);
            }
            "Success: Sleep start logged.".to_string()
        }
        "log_wake" => {
            let tired = args["tiredness_level"].as_i64().unwrap_or(5);
            let now = Utc::now().to_rfc3339();
            let mut sleep = match get_memory_or_init(&client, user_id, "sleep", "# Sleep Ledger\n").await {
                Ok(c) => c,
                Err(err) => return format!("Error: {}", err),
            };
            sleep.push_str(&format!("- wake_log: tiredness level {}/10 at {}\n", tired, now));
            if let Err(err) = save_memory(&client, user_id, "sleep", &sleep).await {
                return format!("Error: {}", err);
            }
            "Success: Wake log saved.".to_string()
        }
        "trigger_normal_alarm" | "trigger_loud_alarm" => {
            // Find user's registered devices and create a pending alarm
            let severity = if name == "trigger_loud_alarm" { "loud" } else { "normal" };
            let title = if name == "trigger_loud_alarm" { "LOUD ESCALATION" } else { "Wake Alarm Escalation" };
            let message = "Antirot Coach: Wake up and respond now!";
            
            let devices = match client
                .query(
                    "SELECT device_id FROM devices WHERE user_id = $1",
                    &[&user_id],
                )
                .await
            {
                Ok(rows) => rows,
                Err(err) => return format!("Error querying user devices: {}", err),
            };

            if devices.is_empty() {
                return "Fallback: No paired devices found for this user. Escalation logged, but cannot trigger phone alarm.".to_string();
            }

            let mut success_count = 0;
            for row in devices {
                let dev_id: String = row.get("device_id");
                let alarm_id = format!("alarm_{}_{}", severity, Uuid::new_v4().simple());
                let fire_at = Utc::now();
                let expires_at = fire_at + chrono::Duration::hours(2);

                let insert_result = client
                    .execute(
                        "
                        INSERT INTO alarms (id, device_id, kind, severity, title, message, fire_at, expires_at, status)
                        VALUES ($1, $2, 'coaching_escalation', $3, $4, $5, $6, $7, 'pending')
                        ",
                        &[
                            &alarm_id,
                            &dev_id,
                            &severity,
                            &title,
                            &message,
                            &fire_at,
                            &Some(expires_at),
                        ],
                    )
                    .await;

                if insert_result.is_ok() {
                    success_count += 1;
                    // Trigger push notification if available
                    trigger_push_wake_background(pool, config, &dev_id, &alarm_id).await;
                }
            }
            format!("Success: Queued coaching escalation alarm for {} devices.", success_count)
        }
        other => format!("Error: Unknown tool {}", other),
    }
}

async fn trigger_push_wake_background(pool: &Pool, config: &Config, device_id: &str, alarm_id: &str) {
    let client = match pool.get().await {
        Ok(c) => c,
        Err(_) => return,
    };
    let row = match client.query_opt("SELECT push_token, push_provider FROM devices WHERE device_id = $1", &[&device_id]).await {
        Ok(Some(r)) => r,
        _ => return,
    };
    let push_provider: Option<String> = row.get("push_provider");
    let push_token: Option<String> = row.get("push_token");
    if push_provider.as_deref() == Some("apns") {
        if let Some(token) = push_token.filter(|t| !t.trim().is_empty()) {
            let _ = crate::apns::send_alarm_wake(config, &token, alarm_id).await;
        }
    }
}


async fn save_memory(
    client: &tokio_postgres::Client,
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
    Ok(())
}

fn get_tool_definitions() -> Value {
    json!([
        {
            "type": "function",
            "function": {
                "name": "patch_file",
                "description": "Edits a user memory file (longterm.md, shortterm.md, behavior.md, tasks.md, sleep.md, work.md, miscellaneous_todo.md) using a git-conflict style SEARCH/REPLACE block. Make sure to match the search block exactly including all spaces, capitalization, and bullet points. Empty search block appends to the file.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "file_path": {
                            "type": "string",
                            "enum": ["longterm.md", "shortterm.md", "behavior.md", "tasks.md", "sleep.md", "work.md", "miscellaneous_todo.md"],
                            "description": "The target memory file to update."
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
                        "tiredness_level": { "type": "integer", "description": "Tiredness level from 1 (refreshed) to 10 (exhausted)." }
                    },
                    "required": ["tiredness_level"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "trigger_normal_alarm",
                "description": "Triggers a regular warning alarm callback.",
                "parameters": { "type": "object", "properties": {} }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "trigger_loud_alarm",
                "description": "Triggers a loud alarm immediately.",
                "parameters": { "type": "object", "properties": {} }
            }
        }
    ])
}
