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
        "add_long_term_goal" => {
            let goal_text = args["goal_text"].as_str().unwrap_or("").trim();
            if goal_text.is_empty() {
                return "Error: goal_text parameter is required and cannot be empty".to_string();
            }
            let mut longterm = match get_memory_or_init(&client, user_id, "longterm", DEFAULT_LONGTERM).await {
                Ok(c) => c,
                Err(err) => return format!("Error reading longterm memory: {}", err),
            };
            // Append under Direction header
            let needle = "## Direction\n";
            if let Some(idx) = longterm.find(needle) {
                let insert_pos = idx + needle.len();
                longterm.insert_str(insert_pos, &format!("- {}\n", goal_text));
            } else {
                longterm.push_str(&format!("\n## Direction\n- {}\n", goal_text));
            }
            if let Err(err) = save_memory(&client, user_id, "longterm", &longterm).await {
                return format!("Error saving memory: {}", err);
            }
            "Success: Long term goal successfully added.".to_string()
        }
        "set_identity_framing" => {
            let direction = args["direction_text"].as_str().unwrap_or("").trim();
            let standards = args["standards_text"].as_str().unwrap_or("").trim();
            let content = format!(
                "# Long-Term Goals\n\n## Direction\n- {}\n\n## Standards\n- {}\n",
                direction, standards
            );
            if let Err(err) = save_memory(&client, user_id, "longterm", &content).await {
                return format!("Error saving memory: {}", err);
            }
            "Success: Identity framing updated.".to_string()
        }
        "set_short_term_priority" => {
            let priority = args["priority_text"].as_str().unwrap_or("").trim();
            let content = format!(
                "# Short-Term State\n\n## Current Priorities\n- {}\n\n## Constraints\n- Suppressed pressures go here.\n",
                priority
            );
            if let Err(err) = save_memory(&client, user_id, "shortterm", &content).await {
                return format!("Error saving memory: {}", err);
            }
            "Success: Short-term priority updated.".to_string()
        }
        "set_current_constraint" => {
            let constraint = args["constraint_text"].as_str().unwrap_or("").trim();
            let mut shortterm = match get_memory_or_init(&client, user_id, "shortterm", DEFAULT_SHORTTERM).await {
                Ok(c) => c,
                Err(err) => return format!("Error: {}", err),
            };
            let needle = "## Constraints\n";
            if let Some(idx) = shortterm.find(needle) {
                let insert_pos = idx + needle.len();
                shortterm.insert_str(insert_pos, &format!("- {}\n", constraint));
            } else {
                shortterm.push_str(&format!("\n## Constraints\n- {}\n", constraint));
            }
            if let Err(err) = save_memory(&client, user_id, "shortterm", &shortterm).await {
                return format!("Error: {}", err);
            }
            "Success: Constraint added to shortterm.md.".to_string()
        }
        "log_behavior_pattern" => {
            let ptype = args["pattern_type"].as_str().unwrap_or("pattern");
            let desc = args["description"].as_str().unwrap_or("");
            let now = Utc::now().to_rfc3339();
            let mut behavior = match get_memory_or_init(&client, user_id, "behavior", DEFAULT_BEHAVIOR).await {
                Ok(c) => c,
                Err(err) => return format!("Error: {}", err),
            };
            let needle = match ptype {
                "drift" => "## Drift Tendencies\n",
                "tactic" => "## Accountability Styles\n",
                _ => "## Recurring Patterns\n",
            };
            if let Some(idx) = behavior.find(needle) {
                let insert_pos = idx + needle.len();
                behavior.insert_str(insert_pos, &format!("- {} (logged at {})\n", desc, now));
            } else {
                behavior.push_str(&format!("\n{} - {} (logged at {})\n", needle, desc, now));
            }
            if let Err(err) = save_memory(&client, user_id, "behavior", &behavior).await {
                return format!("Error: {}", err);
            }
            "Success: Behavior pattern recorded.".to_string()
        }
        "add_pipeline_task" => {
            let title = args["title"].as_str().unwrap_or("");
            let hours = args["hours"].as_f64().unwrap_or(1.0);
            let mut tasks = match get_memory_or_init(&client, user_id, "tasks", "# Task Pipeline\n").await {
                Ok(c) => c,
                Err(err) => return format!("Error: {}", err),
            };
            tasks.push_str(&format!("[ ] {:.1}h - {}\n", hours, title));
            if let Err(err) = save_memory(&client, user_id, "tasks", &tasks).await {
                return format!("Error: {}", err);
            }
            "Success: Task added to tasks.md pipeline.".to_string()
        }
        "update_pipeline_task_status" => {
            let task_idx = args["task_index"].as_i64().unwrap_or(1) as usize;
            let status = args["status"].as_str().unwrap_or("completed");
            let tasks = match get_memory_or_init(&client, user_id, "tasks", "# Task Pipeline\n").await {
                Ok(c) => c,
                Err(err) => return format!("Error: {}", err),
            };
            
            let mut lines: Vec<String> = tasks.lines().map(String::from).collect();
            let mut task_lines_indices = Vec::new();
            for (idx, line) in lines.iter().enumerate() {
                if line.starts_with("[ ]") || line.starts_with("[x]") {
                    task_lines_indices.push(idx);
                }
            }

            if task_idx == 0 || task_idx > task_lines_indices.len() {
                return format!("Error: Task index {} is out of range. Total active/completed tasks: {}.", task_idx, task_lines_indices.len());
            }

            let target_line_idx = task_lines_indices[task_idx - 1];
            if status == "completed" {
                if let Some(remainder) = lines[target_line_idx].strip_prefix("[ ]") {
                    lines[target_line_idx] = format!("[x]{}", remainder);
                }
            } else if status == "deleted" {
                lines.remove(target_line_idx);
            }

            let new_content = lines.join("\n") + "\n";
            if let Err(err) = save_memory(&client, user_id, "tasks", &new_content).await {
                return format!("Error: {}", err);
            }
            format!("Success: Task index {} set to status {}.", task_idx, status)
        }
        "add_misc_todo" => {
            let todo_text = args["todo_text"].as_str().unwrap_or("");
            let mut todo = match get_memory_or_init(&client, user_id, "miscellaneous_todo", "# Miscellaneous Todo\n").await {
                Ok(c) => c,
                Err(err) => return format!("Error: {}", err),
            };
            todo.push_str(&format!("- {}\n", todo_text));
            if let Err(err) = save_memory(&client, user_id, "miscellaneous_todo", &todo).await {
                return format!("Error: {}", err);
            }
            "Success: Todo added to miscellaneous_todo.md list.".to_string()
        }
        "pop_misc_todo" => {
            let todo_idx = args["todo_index"].as_i64().unwrap_or(1) as usize;
            let todo = match get_memory_or_init(&client, user_id, "miscellaneous_todo", "# Miscellaneous Todo\n").await {
                Ok(c) => c,
                Err(err) => return format!("Error: {}", err),
            };
            let mut lines: Vec<String> = todo.lines().map(String::from).collect();
            let mut todo_indices = Vec::new();
            for (idx, line) in lines.iter().enumerate() {
                if line.trim().starts_with("- ") {
                    todo_indices.push(idx);
                }
            }
            if todo_idx == 0 || todo_idx > todo_indices.len() {
                return format!("Error: index {} is out of range.", todo_idx);
            }
            let target_line_idx = todo_indices[todo_idx - 1];
            lines.remove(target_line_idx);
            let new_content = lines.join("\n") + "\n";
            if let Err(err) = save_memory(&client, user_id, "miscellaneous_todo", &new_content).await {
                return format!("Error: {}", err);
            }
            "Success: Item removed from miscellaneous_todo.md list.".to_string()
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
                "name": "add_long_term_goal",
                "description": "Appends a new long-term goal to the user's longterm.md memory.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "goal_text": { "type": "string", "description": "The goal description." }
                    },
                    "required": ["goal_text"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "set_identity_framing",
                "description": "Updates direction and standards in the user's longterm.md memory.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "direction_text": { "type": "string", "description": "New long-term direction/vision." },
                        "standards_text": { "type": "string", "description": "Core standards and non-negotiables." }
                    },
                    "required": ["direction_text", "standards_text"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "set_short_term_priority",
                "description": "Sets current priorities in shortterm.md memory.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "priority_text": { "type": "string", "description": "Current active priorities." }
                    },
                    "required": ["priority_text"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "set_current_constraint",
                "description": "Sets health, sleep, or travel constraints in shortterm.md memory.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "constraint_text": { "type": "string", "description": "Current constraints or barriers." }
                    },
                    "required": ["constraint_text"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "log_behavior_pattern",
                "description": "Appends a behavioral observation (drift loop, pattern, or coaching style) to behavior.md.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "pattern_type": { "type": "string", "enum": ["pattern", "drift", "tactic"], "description": "Type of pattern." },
                        "description": { "type": "string", "description": "Details of the pattern/drift/tactic observed." }
                    },
                    "required": ["pattern_type", "description"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "add_pipeline_task",
                "description": "Appends a new task to the user's task pipeline (tasks.md).",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "title": { "type": "string", "description": "Task description." },
                        "hours": { "type": "number", "description": "Estimated time in hours." }
                    },
                    "required": ["title", "hours"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "update_pipeline_task_status",
                "description": "Marks a task in tasks.md as completed or deleted.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "task_index": { "type": "integer", "description": "The 1-based index of the task in tasks.md." },
                        "status": { "type": "string", "enum": ["completed", "deleted"], "description": "Target status." }
                    },
                    "required": ["task_index", "status"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "add_misc_todo",
                "description": "Appends an intrusive thought or small admin task to miscellaneous_todo.md.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "todo_text": { "type": "string", "description": "The administrative task or thought." }
                    },
                    "required": ["todo_text"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "pop_misc_todo",
                "description": "Removes a todo task from the miscellaneous_todo.md list.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "todo_index": { "type": "integer", "description": "The 1-based index of the item." }
                    },
                    "required": ["todo_index"]
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
