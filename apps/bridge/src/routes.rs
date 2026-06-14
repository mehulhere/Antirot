use axum::extract::{Multipart, Path, Query, State};
use axum::http::{HeaderMap, HeaderValue};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use chrono::{DateTime, Duration, Utc};
use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio_postgres::Row;
use tracing::{info, warn};
use uuid::Uuid;

use crate::auth::{
    get_user_id_from_auth, require_admin_auth, require_device_auth, require_device_auth_for,
    token_hash,
};
use crate::error::{AppError, AppResult};
use crate::llm::{
    build_context_report, build_context_report_for_test, chat_with_coach, run_tool_for_test,
};
use crate::memory::{save_memory_indexed, sleep_metrics_report};
use crate::models::{
    AlarmActionRequest, AlarmActionResponse, AlarmJob, ChatRequest, ChatResponse,
    CreateAlarmRequest, CreateAlarmResponse, DeliveryState, DeviceRegistrationRequest,
    DeviceRegistrationResponse, GoogleAuthRequest, GoogleAuthResponse, HealthResponse,
    MemoryResponse, PairingClaimRequest, PairingClaimResponse, SpeechSynthesisRequest,
    SpeechSynthesisResponse, SpeechTranscriptionResponse, SubscriptionResponse,
    SubscriptionUpdateRequest, UpdateMemoryRequest, WorkspaceDevice, WorkspaceDevicesResponse,
};
use crate::prompt::{allowed_memory_key, default_memory_for_key};
use crate::AppState;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PendingQuery {
    #[serde(alias = "device_id")]
    device_id: String,
    limit: Option<i64>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/auth/google", post(auth_google))
        .route("/pairing/claim", post(claim_pairing))
        .route("/devices/register", post(register_device))
        .route("/workspaces/{workspace_id}/devices", get(workspace_devices))
        .route("/alarms", post(create_alarm))
        .route("/alarms/pending", get(pending_alarms))
        .route("/alarms/cancel", post(cancel_alarms_by_kind))
        .route("/alarms/{alarm_id}/{action}", post(record_alarm_action))
        .route("/visits", get(get_and_increment_visits))
        .route(
            "/subscription",
            get(get_subscription).post(update_subscription),
        )
        .route("/memory/{key}", get(get_memory).put(update_memory))
        .route("/admin/context", get(admin_context))
        .route("/chat", post(chat_coach))
        .route("/speech/transcribe", post(transcribe_speech))
        .route("/speech/synthesize", post(synthesize_speech))
        .route("/test/reset", post(test_reset))
        .route("/test/tool", post(test_tool))
        .route("/test/state", get(test_state))
        .route("/test/context", get(test_context))
        .route("/v1/health", get(health))
        .route("/v1/auth/google", post(auth_google))
        .route("/v1/pairing/claim", post(claim_pairing))
        .route("/v1/devices/register", post(register_device))
        .route(
            "/v1/workspaces/{workspace_id}/devices",
            get(workspace_devices),
        )
        .route("/v1/alarms", post(create_alarm))
        .route("/v1/alarms/pending", get(pending_alarms))
        .route("/v1/alarms/cancel", post(cancel_alarms_by_kind))
        .route("/v1/alarms/{alarm_id}/{action}", post(record_alarm_action))
        .route("/v1/visits", get(get_and_increment_visits))
        .route(
            "/v1/subscription",
            get(get_subscription).post(update_subscription),
        )
        .route("/v1/memory/{key}", get(get_memory).put(update_memory))
        .route("/v1/admin/context", get(admin_context))
        .route("/v1/chat", post(chat_coach))
        .route("/v1/speech/transcribe", post(transcribe_speech))
        .route("/v1/speech/synthesize", post(synthesize_speech))
        .route("/v1/test/reset", post(test_reset))
        .route("/v1/test/tool", post(test_tool))
        .route("/v1/test/state", get(test_state))
        .route("/v1/test/context", get(test_context))
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        ok: true,
        service: "antirot-backend",
    })
}

async fn auth_google(
    State(state): State<AppState>,
    Json(request): Json<GoogleAuthRequest>,
) -> AppResult<Json<GoogleAuthResponse>> {
    validate_non_empty("idToken", &request.id_token)?;
    validate_non_empty("deviceId", &request.device_id)?;
    validate_non_empty("platform", &request.platform)?;

    if state.config.google_allowed_client_ids.is_empty() {
        return Err(AppError::BadRequest(
            "Google OAuth is not configured on this backend".to_string(),
        ));
    }

    let profile =
        verify_google_id_token(&request.id_token, &state.config.google_allowed_client_ids).await?;
    let client = state.pool.get().await?;
    let fallback_user_id = Uuid::new_v4().to_string();
    let existing_user = client
        .query_opt(
            "
            SELECT user_id
            FROM auth_identities
            WHERE provider = 'google' AND provider_subject = $1
            ",
            &[&profile.sub],
        )
        .await?;
    let preferred_user_id = existing_user
        .map(|row| row.get::<_, String>("user_id"))
        .unwrap_or(fallback_user_id);

    let row = client
        .query_one(
            "
            INSERT INTO users (id, email, display_name, avatar_url)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (email) DO UPDATE SET
                display_name = EXCLUDED.display_name,
                avatar_url = EXCLUDED.avatar_url,
                updated_at = now()
            RETURNING id
            ",
            &[
                &preferred_user_id,
                &profile.email,
                &profile.name,
                &profile.picture,
            ],
        )
        .await?;
    let user_id: String = row.get("id");

    client
        .execute(
            "
            INSERT INTO auth_identities (provider, provider_subject, user_id, email)
            VALUES ('google', $1, $2, $3)
            ON CONFLICT (provider, provider_subject) DO UPDATE SET
                user_id = EXCLUDED.user_id,
                email = EXCLUDED.email,
                updated_at = now()
            ",
            &[&profile.sub, &user_id, &profile.email],
        )
        .await?;

    let device_token = format!("antirot_{}", Uuid::new_v4().simple());
    let api_token_hash = token_hash(&device_token);
    let app_version = request.app_version.unwrap_or_else(|| "unknown".to_string());
    let notification_capability = request
        .notification_capability
        .unwrap_or_else(|| "unknown".to_string());
    let usage_capability = request
        .usage_capability
        .unwrap_or_else(|| "unknown".to_string());

    client
        .execute(
            "
            INSERT INTO devices (
                device_id,
                user_id,
                api_token_hash,
                platform,
                app_version,
                notification_capability,
                usage_capability,
                push_provider,
                push_token
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (device_id) DO UPDATE SET
                user_id = EXCLUDED.user_id,
                api_token_hash = EXCLUDED.api_token_hash,
                platform = EXCLUDED.platform,
                app_version = EXCLUDED.app_version,
                notification_capability = EXCLUDED.notification_capability,
                usage_capability = EXCLUDED.usage_capability,
                push_provider = COALESCE(EXCLUDED.push_provider, devices.push_provider),
                push_token = COALESCE(EXCLUDED.push_token, devices.push_token),
                updated_at = now()
            ",
            &[
                &request.device_id,
                &user_id,
                &api_token_hash,
                &request.platform,
                &app_version,
                &notification_capability,
                &usage_capability,
                &request.push_provider,
                &request.push_token,
            ],
        )
        .await?;

    info!(
        device_id = %request.device_id,
        email = %profile.email,
        "registered Google-authenticated device"
    );

    Ok(Json(GoogleAuthResponse {
        ok: true,
        device_id: request.device_id,
        device_token,
        email: profile.email,
        name: profile.name,
        message: "Signed in with Google".to_string(),
    }))
}

async fn register_device(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<DeviceRegistrationRequest>,
) -> AppResult<Json<DeviceRegistrationResponse>> {
    require_device_auth(&headers, &state.config, &state.pool).await?;
    validate_non_empty("deviceId", &request.device_id)?;
    validate_non_empty("platform", &request.platform)?;

    let client = state.pool.get().await?;
    client
        .execute(
            "
            INSERT INTO devices (
                device_id,
                platform,
                app_version,
                notification_capability,
                usage_capability,
                push_provider,
                push_token
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (device_id) DO UPDATE SET
                platform = EXCLUDED.platform,
                app_version = EXCLUDED.app_version,
                notification_capability = EXCLUDED.notification_capability,
                usage_capability = EXCLUDED.usage_capability,
                push_provider = EXCLUDED.push_provider,
                push_token = EXCLUDED.push_token,
                updated_at = now()
            ",
            &[
                &request.device_id,
                &request.platform,
                &request.app_version,
                &request.notification_capability,
                &request.usage_capability,
                &request.push_provider,
                &request.push_token,
            ],
        )
        .await?;

    info!(device_id = %request.device_id, platform = %request.platform, "registered device");
    Ok(Json(DeviceRegistrationResponse {
        ok: true,
        device_id: request.device_id,
        message: Some("Registered device".to_string()),
    }))
}

async fn workspace_devices(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(workspace_id): Path<String>,
) -> AppResult<Json<WorkspaceDevicesResponse>> {
    require_admin_auth(&headers, &state.config)?;
    validate_non_empty("workspaceId", &workspace_id)?;

    let client = state.pool.get().await?;
    let rows = client
        .query(
            "
            SELECT device_id, device_name, platform, notification_capability, paired_at
            FROM devices
            WHERE workspace_id = $1
            ORDER BY paired_at DESC NULLS LAST, updated_at DESC
            LIMIT 20
            ",
            &[&workspace_id],
        )
        .await?;

    Ok(Json(WorkspaceDevicesResponse {
        ok: true,
        workspace_id,
        devices: rows
            .iter()
            .map(|row| WorkspaceDevice {
                device_id: row.get("device_id"),
                device_name: row.get("device_name"),
                platform: row.get("platform"),
                notification_capability: row.get("notification_capability"),
                paired_at: row.get("paired_at"),
            })
            .collect(),
    }))
}

async fn claim_pairing(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<PairingClaimRequest>,
) -> AppResult<Json<PairingClaimResponse>> {
    validate_pairing_code(&request.code)?;
    validate_non_empty("deviceId", &request.device_id)?;
    require_device_auth_for(&headers, &state.config, &state.pool, &request.device_id).await?;

    let code_hash = token_hash(&normalize_pairing_code(&request.code));
    let mut client = state.pool.get().await?;
    let transaction = client.transaction().await?;
    let session = transaction
        .query_opt(
            "
            SELECT id, workspace_id
            FROM pairing_sessions
            WHERE code_hash = $1
              AND used_at IS NULL
              AND expires_at > now()
              AND attempt_count < 5
            FOR UPDATE
            ",
            &[&code_hash],
        )
        .await?;

    let Some(session) = session else {
        return Err(AppError::BadRequest(
            "Pairing code is invalid or expired".to_string(),
        ));
    };

    let session_id: String = session.get("id");
    let workspace_id: String = session.get("workspace_id");
    let user_row = transaction
        .query_opt(
            "SELECT user_id FROM devices WHERE device_id = $1",
            &[&request.device_id],
        )
        .await?;
    let Some(user_row) = user_row else {
        return Err(AppError::Unauthorized);
    };
    let user_id: Option<String> = user_row.get("user_id");
    let Some(user_id) = user_id else {
        return Err(AppError::Unauthorized);
    };
    let device_name = request
        .device_name
        .clone()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| default_device_name(request.platform.as_deref()));

    transaction
        .execute(
            "
            UPDATE devices
            SET workspace_id = $1,
                device_name = $2,
                paired_at = now(),
                updated_at = now()
            WHERE device_id = $3
            ",
            &[&workspace_id, &device_name, &request.device_id],
        )
        .await?;

    transaction
        .execute(
            "
            UPDATE pairing_sessions
            SET used_at = now(),
                claimed_device_id = $1,
                claimed_user_id = $2,
                device_name = $3
            WHERE id = $4
            ",
            &[&request.device_id, &user_id, &device_name, &session_id],
        )
        .await?;

    transaction.commit().await?;
    info!(
        device_id = %request.device_id,
        workspace_id = %workspace_id,
        "paired device with workspace"
    );
    Ok(Json(PairingClaimResponse {
        ok: true,
        workspace_id,
        device_id: request.device_id,
        message: "Device paired".to_string(),
    }))
}

async fn create_alarm(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateAlarmRequest>,
) -> AppResult<Json<CreateAlarmResponse>> {
    require_admin_auth(&headers, &state.config)?;
    validate_non_empty("deviceId", &request.device_id)?;
    validate_non_empty("title", &request.title)?;

    let id = request.normalized_id();
    let kind = request.normalized_kind();
    let severity = request.normalized_severity();
    let hidden_buffer_applied = request.hidden_buffer_applied.unwrap_or(false);
    let requires_acknowledgement = request.requires_acknowledgement.unwrap_or(true);
    let client = state.pool.get().await?;

    let device_exists = client
        .query_opt(
            "SELECT 1 FROM devices WHERE device_id = $1",
            &[&request.device_id],
        )
        .await?
        .is_some();
    if !device_exists {
        return Err(AppError::BadRequest(format!(
            "device {} is not registered",
            request.device_id
        )));
    }

    let row = client
        .query_one(
            "
            INSERT INTO alarms (
                id,
                device_id,
                kind,
                severity,
                title,
                message,
                fire_at,
                hidden_buffer_applied,
                requires_acknowledgement,
                expires_at,
                status
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'pending')
            ON CONFLICT (id) DO UPDATE SET
                device_id = EXCLUDED.device_id,
                kind = EXCLUDED.kind,
                severity = EXCLUDED.severity,
                title = EXCLUDED.title,
                message = EXCLUDED.message,
                fire_at = EXCLUDED.fire_at,
                hidden_buffer_applied = EXCLUDED.hidden_buffer_applied,
                requires_acknowledgement = EXCLUDED.requires_acknowledgement,
                expires_at = EXCLUDED.expires_at,
                status = 'pending',
                delivery_attempts = 0,
                last_delivered_at = NULL,
                updated_at = now()
            RETURNING id, kind, severity, title, message, fire_at,
                hidden_buffer_applied, requires_acknowledgement, expires_at
            ",
            &[
                &id,
                &request.device_id,
                &kind,
                &severity,
                &request.title,
                &request.message,
                &request.fire_at,
                &hidden_buffer_applied,
                &requires_acknowledgement,
                &request.expires_at,
            ],
        )
        .await?;

    maybe_send_apns_wake(&state, &request.device_id, &id).await;

    info!(alarm_id = %id, device_id = %request.device_id, "queued alarm");
    Ok(Json(CreateAlarmResponse {
        ok: true,
        alarm: alarm_from_row(&row),
        delivery: DeliveryState {
            mode: "pending_fetch".to_string(),
            status: "queued".to_string(),
        },
    }))
}

async fn maybe_send_apns_wake(state: &AppState, device_id: &str, alarm_id: &str) {
    let client = match state.pool.get().await {
        Ok(client) => client,
        Err(error) => {
            warn!(
                alarm_id,
                device_id,
                error = %error,
                "🔴 FALLBACK: APNs wake skipped - Reason: database pool unavailable after queuing alarm - Impact: iOS app must poll/open before scheduling"
            );
            return;
        }
    };

    let row = match client
        .query_opt(
            "
            SELECT push_provider, push_token
            FROM devices
            WHERE device_id = $1
            ",
            &[&device_id],
        )
        .await
    {
        Ok(row) => row,
        Err(error) => {
            warn!(
                alarm_id,
                device_id,
                error = %error,
                "🔴 FALLBACK: APNs wake skipped - Reason: device push lookup failed - Impact: iOS app must poll/open before scheduling"
            );
            return;
        }
    };

    let Some(row) = row else {
        return;
    };
    let push_provider: Option<String> = row.get("push_provider");
    let push_token: Option<String> = row.get("push_token");
    if push_provider.as_deref() != Some("apns") {
        warn!(
            alarm_id,
            device_id,
            "🔴 FALLBACK: APNs wake skipped - Reason: device has no APNs provider - Impact: iOS app must poll/open before scheduling"
        );
        return;
    }
    let Some(push_token) = push_token.filter(|value| !value.trim().is_empty()) else {
        warn!(
            alarm_id,
            device_id,
            "🔴 FALLBACK: APNs wake skipped - Reason: device has no APNs token - Impact: iOS app must poll/open before scheduling"
        );
        return;
    };

    if let Err(error) = crate::apns::send_alarm_wake(&state.config, &push_token, alarm_id).await {
        warn!(
            alarm_id,
            device_id,
            error = %error,
            "🔴 FALLBACK: APNs wake failed - Reason: backend could not complete APNs request - Impact: iOS app must poll/open before scheduling"
        );
    }
}

async fn pending_alarms(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<PendingQuery>,
) -> AppResult<Json<Vec<AlarmJob>>> {
    require_device_auth(&headers, &state.config, &state.pool).await?;
    validate_non_empty("deviceId", &query.device_id)?;
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let mut client = state.pool.get().await?;
    let transaction = client.transaction().await?;

    let rows = transaction
        .query(
            "
            SELECT id, kind, severity, title, message, fire_at,
                hidden_buffer_applied, requires_acknowledgement, expires_at
            FROM alarms
            WHERE device_id = $1
              AND status = 'pending'
              AND (expires_at IS NULL OR expires_at > now())
            ORDER BY fire_at ASC
            LIMIT $2
            FOR UPDATE SKIP LOCKED
            ",
            &[&query.device_id, &limit],
        )
        .await?;

    let alarm_ids: Vec<String> = rows.iter().map(|row| row.get("id")).collect();
    if !alarm_ids.is_empty() {
        transaction
            .execute(
                "
                UPDATE alarms
                SET status = 'delivered',
                    delivery_attempts = delivery_attempts + 1,
                    last_delivered_at = now(),
                    updated_at = now()
                WHERE id = ANY($1)
                ",
                &[&alarm_ids],
            )
            .await?;
    }
    transaction.commit().await?;

    info!(
        device_id = %query.device_id,
        count = alarm_ids.len(),
        "returned pending alarms"
    );
    Ok(Json(rows.iter().map(alarm_from_row).collect()))
}

async fn record_alarm_action(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((alarm_id, path_action)): Path<(String, String)>,
    Json(request): Json<AlarmActionRequest>,
) -> AppResult<Json<AlarmActionResponse>> {
    require_device_auth(&headers, &state.config, &state.pool).await?;
    validate_non_empty("deviceId", &request.device_id)?;
    validate_non_empty("alarmId", &alarm_id)?;

    let action = normalize_action(&path_action, &request.action)?;
    let mut client = state.pool.get().await?;
    let transaction = client.transaction().await?;

    let exists = transaction
        .query_opt(
            "SELECT 1 FROM alarms WHERE id = $1 AND device_id = $2",
            &[&alarm_id, &request.device_id],
        )
        .await?
        .is_some();
    if !exists {
        return Err(AppError::NotFound);
    }

    transaction
        .execute(
            "
            INSERT INTO alarm_events (alarm_id, device_id, action, minutes, occurred_at)
            VALUES ($1, $2, $3, $4, $5)
            ",
            &[
                &alarm_id,
                &request.device_id,
                &action,
                &request.minutes,
                &request.at,
            ],
        )
        .await?;

    let status = match action.as_str() {
        "snooze" => {
            let minutes = request.minutes.unwrap_or(9).clamp(1, 180);
            let next_fire_at = Utc::now() + Duration::minutes(i64::from(minutes));
            transaction
                .execute(
                    "
                    UPDATE alarms
                    SET status = 'pending',
                        fire_at = $1,
                        delivery_attempts = 0,
                        last_delivered_at = NULL,
                        updated_at = now()
                    WHERE id = $2
                    ",
                    &[&next_fire_at, &alarm_id],
                )
                .await?;
            "pending".to_string()
        }
        "scheduled" => {
            transaction
                .execute(
                    "UPDATE alarms SET status = 'scheduled', updated_at = now() WHERE id = $1",
                    &[&alarm_id],
                )
                .await?;
            "scheduled".to_string()
        }
        "ack" | "stop" | "dismiss" => {
            transaction
                .execute(
                    "UPDATE alarms SET status = 'acknowledged', updated_at = now() WHERE id = $1",
                    &[&alarm_id],
                )
                .await?;

            let alarm_info = transaction
                .query_opt(
                    "SELECT kind, device_id FROM alarms WHERE id = $1",
                    &[&alarm_id],
                )
                .await?;

            if let Some(row) = alarm_info {
                let kind: String = row.get("kind");
                let device_id: String = row.get("device_id");
                if matches!(
                    kind.as_str(),
                    "session_alarm" | "break_alarm" | "wake_alarm" | "idle_alarm"
                ) {
                    transaction
                        .execute(
                            "DELETE FROM alarms WHERE device_id = $1 AND kind = $2 AND status = 'pending'",
                            &[&device_id, &kind],
                        )
                        .await?;
                }
            }

            "acknowledged".to_string()
        }
        other => {
            warn!(alarm_id = %alarm_id, action = %other, "recorded unknown alarm action");
            transaction
                .execute(
                    "UPDATE alarms SET status = 'action_recorded', updated_at = now() WHERE id = $1",
                    &[&alarm_id],
                )
                .await?;
            "action_recorded".to_string()
        }
    };

    transaction.commit().await?;
    info!(alarm_id = %alarm_id, action = %action, status = %status, "recorded alarm action");
    Ok(Json(AlarmActionResponse {
        ok: true,
        alarm_id,
        status,
    }))
}

fn alarm_from_row(row: &Row) -> AlarmJob {
    AlarmJob {
        id: row.get("id"),
        kind: row.get("kind"),
        severity: row.get("severity"),
        title: row.get("title"),
        message: row.get("message"),
        fire_at: row.get::<_, DateTime<Utc>>("fire_at"),
        hidden_buffer_applied: row.get("hidden_buffer_applied"),
        requires_acknowledgement: row.get("requires_acknowledgement"),
        expires_at: row.get::<_, Option<DateTime<Utc>>>("expires_at"),
    }
}

fn validate_non_empty(name: &str, value: &str) -> AppResult<()> {
    if value.trim().is_empty() {
        Err(AppError::BadRequest(format!("{name} is required")))
    } else {
        Ok(())
    }
}

fn normalize_pairing_code(code: &str) -> String {
    code.chars()
        .filter(|character| character.is_ascii_digit())
        .collect()
}

fn validate_pairing_code(code: &str) -> AppResult<()> {
    let normalized = normalize_pairing_code(code);
    if normalized.len() == 6 {
        Ok(())
    } else {
        Err(AppError::BadRequest(
            "pairing code must be 6 digits".to_string(),
        ))
    }
}

fn default_device_name(platform: Option<&str>) -> String {
    match platform {
        Some("ios") => "iPhone".to_string(),
        Some("android") => "Android phone".to_string(),
        _ => "Phone".to_string(),
    }
}

fn normalize_action(path_action: &str, body_action: &str) -> AppResult<String> {
    let path_action = path_action.trim().to_ascii_lowercase();
    let body_action = body_action.trim().to_ascii_lowercase();
    if body_action.is_empty() || body_action == path_action {
        Ok(path_action)
    } else {
        Err(AppError::BadRequest(format!(
            "path action {path_action} does not match body action {body_action}"
        )))
    }
}

#[derive(Debug, Deserialize)]
struct GoogleTokenInfo {
    sub: String,
    aud: String,
    email: Option<String>,
    email_verified: Option<serde_json::Value>,
    name: Option<String>,
    picture: Option<String>,
}

#[derive(Debug)]
struct GoogleProfile {
    sub: String,
    email: String,
    name: Option<String>,
    picture: Option<String>,
}

async fn verify_google_id_token(
    id_token: &str,
    allowed_client_ids: &[String],
) -> AppResult<GoogleProfile> {
    let response = reqwest::Client::new()
        .get("https://oauth2.googleapis.com/tokeninfo")
        .query(&[("id_token", id_token)])
        .send()
        .await
        .map_err(|error| {
            AppError::BadRequest(format!("Google token verification failed: {error}"))
        })?;

    if !response.status().is_success() {
        return Err(AppError::Unauthorized);
    }

    let token_info = response.json::<GoogleTokenInfo>().await.map_err(|error| {
        AppError::BadRequest(format!("Google token response was invalid: {error}"))
    })?;

    if !allowed_client_ids
        .iter()
        .any(|client_id| constant_time_string_eq(client_id, &token_info.aud))
    {
        return Err(AppError::Unauthorized);
    }

    if !google_email_verified(token_info.email_verified.as_ref()) {
        return Err(AppError::Unauthorized);
    }

    let email = token_info.email.ok_or_else(|| {
        AppError::BadRequest("Google account did not include an email address".to_string())
    })?;

    Ok(GoogleProfile {
        sub: token_info.sub,
        email,
        name: token_info.name,
        picture: token_info.picture,
    })
}

fn google_email_verified(value: Option<&serde_json::Value>) -> bool {
    match value {
        Some(serde_json::Value::Bool(verified)) => *verified,
        Some(serde_json::Value::String(verified)) => verified == "true",
        _ => false,
    }
}

fn constant_time_string_eq(left: &str, right: &str) -> bool {
    let left = left.as_bytes();
    let right = right.as_bytes();
    let max_len = left.len().max(right.len());
    let mut diff = left.len() ^ right.len();
    for index in 0..max_len {
        let a = *left.get(index).unwrap_or(&0);
        let b = *right.get(index).unwrap_or(&0);
        diff |= (a ^ b) as usize;
    }
    diff == 0
}

#[derive(Debug, Deserialize)]
struct VisitsQuery {
    increment: Option<bool>,
}

async fn get_and_increment_visits(
    State(state): State<AppState>,
    Query(query): Query<VisitsQuery>,
) -> AppResult<impl IntoResponse> {
    let client = state.pool.get().await?;
    let should_increment = query.increment.unwrap_or(true);

    let count: i64 = if should_increment {
        let row = client
            .query_one(
                "
                UPDATE page_views
                SET count = count + 1
                WHERE id = 'homepage'
                RETURNING count
                ",
                &[],
            )
            .await?;
        row.get("count")
    } else {
        let row = client
            .query_one(
                "
                SELECT count FROM page_views WHERE id = 'homepage'
                ",
                &[],
            )
            .await?;
        row.get("count")
    };

    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_static("*"),
    );

    #[derive(serde::Serialize)]
    struct VisitsResponse {
        count: i64,
    }

    Ok((headers, Json(VisitsResponse { count })))
}

// Antirot Standalone Endpoint Handlers
async fn get_subscription(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<SubscriptionResponse>> {
    let user_id = get_user_id_from_auth(&headers, &state.config, &state.pool).await?;
    let client = state.pool.get().await?;
    let row = client
        .query_opt(
            "
            SELECT subscription_tier, subscription_status, byok_provider, subscription_active_until
            FROM users
            WHERE id = $1
            ",
            &[&user_id],
        )
        .await?;

    let Some(row) = row else {
        return Err(AppError::NotFound);
    };

    Ok(Json(SubscriptionResponse {
        ok: true,
        tier: row.get("subscription_tier"),
        status: row.get("subscription_status"),
        byok_provider: row.get("byok_provider"),
        active_until: row.get("subscription_active_until"),
    }))
}

async fn update_subscription(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<SubscriptionUpdateRequest>,
) -> AppResult<Json<SubscriptionResponse>> {
    let user_id = get_user_id_from_auth(&headers, &state.config, &state.pool).await?;
    let client = state.pool.get().await?;

    let active_until = if let Some(days) = req.active_until_days {
        Some(Utc::now() + Duration::days(days))
    } else {
        let row = client
            .query_one(
                "SELECT subscription_active_until FROM users WHERE id = $1",
                &[&user_id],
            )
            .await?;
        row.get("subscription_active_until")
    };

    let status = req.status.unwrap_or_else(|| "active".to_string());

    client
        .execute(
            "
            UPDATE users
            SET subscription_tier = $1,
                subscription_status = $2,
                byok_api_key = COALESCE($3, byok_api_key),
                byok_provider = COALESCE($4, byok_provider),
                subscription_active_until = $5,
                updated_at = now()
            WHERE id = $6
            ",
            &[
                &req.tier,
                &status,
                &req.byok_api_key,
                &req.byok_provider,
                &active_until,
                &user_id,
            ],
        )
        .await?;

    Ok(Json(SubscriptionResponse {
        ok: true,
        tier: req.tier,
        status,
        byok_provider: req.byok_provider,
        active_until,
    }))
}

async fn get_memory(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(key): Path<String>,
) -> AppResult<Json<MemoryResponse>> {
    let user_id = get_user_id_from_auth(&headers, &state.config, &state.pool).await?;
    let client = state.pool.get().await?;

    if !allowed_memory_key(&key) {
        return Err(AppError::BadRequest("Invalid memory key".to_string()));
    }

    let row = client
        .query_opt(
            "SELECT content, updated_at FROM user_memories WHERE user_id = $1 AND memory_key = $2",
            &[&user_id, &key],
        )
        .await?;

    if let Some(row) = row {
        Ok(Json(MemoryResponse {
            ok: true,
            key,
            content: row.get("content"),
            updated_at: row.get("updated_at"),
        }))
    } else {
        let default_content = default_memory_for_key(&key).unwrap_or("# Miscellaneous Todo\n");
        Ok(Json(MemoryResponse {
            ok: true,
            key,
            content: default_content.to_string(),
            updated_at: Utc::now(),
        }))
    }
}

async fn update_memory(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(key): Path<String>,
    Json(req): Json<UpdateMemoryRequest>,
) -> AppResult<Json<MemoryResponse>> {
    let user_id = get_user_id_from_auth(&headers, &state.config, &state.pool).await?;
    let client = state.pool.get().await?;

    if !allowed_memory_key(&key) {
        return Err(AppError::BadRequest("Invalid memory key".to_string()));
    }

    save_memory_indexed(&client, &state.config, &user_id, &key, &req.content).await?;

    Ok(Json(MemoryResponse {
        ok: true,
        key,
        content: req.content,
        updated_at: Utc::now(),
    }))
}

async fn chat_coach(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ChatRequest>,
) -> AppResult<Json<ChatResponse>> {
    let user_id = get_user_id_from_auth(&headers, &state.config, &state.pool).await?;
    let reply = chat_with_coach(&state.pool, &state.config, &user_id, &req.message).await?;
    Ok(Json(ChatResponse { ok: true, reply }))
}

async fn transcribe_speech(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> AppResult<Json<SpeechTranscriptionResponse>> {
    let _user_id = get_user_id_from_auth(&headers, &state.config, &state.pool).await?;
    let api_key = state
        .config
        .speech
        .fireworks_api_key
        .as_deref()
        .ok_or_else(|| {
            AppError::BadRequest("Fireworks speech-to-text is not configured".to_string())
        })?;

    let mut uploaded_file = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|err| AppError::BadRequest(format!("invalid audio upload: {err}")))?
    {
        if field.name() != Some("file") {
            continue;
        }

        let file_name = field.file_name().unwrap_or("antirot-voice.m4a").to_string();
        let content_type = field.content_type().unwrap_or("audio/mp4").to_string();
        let bytes = field
            .bytes()
            .await
            .map_err(|err| AppError::BadRequest(format!("invalid audio upload: {err}")))?;

        if bytes.len() > 25 * 1024 * 1024 {
            return Err(AppError::BadRequest(
                "audio upload is too large; keep voice notes under 25MB".to_string(),
            ));
        }

        uploaded_file = Some((file_name, content_type, bytes.to_vec()));
        break;
    }

    let (file_name, content_type, bytes) = uploaded_file
        .ok_or_else(|| AppError::BadRequest("multipart field `file` is required".to_string()))?;
    let part = Part::bytes(bytes)
        .file_name(file_name)
        .mime_str(&content_type)?;
    let form = Form::new()
        .part("file", part)
        .text("model", state.config.speech.fireworks_stt_model.clone())
        .text("response_format", "json");
    let url = format!(
        "{}/audio/transcriptions",
        state
            .config
            .speech
            .fireworks_audio_base_url
            .trim_end_matches('/')
    );

    let response = reqwest::Client::new()
        .post(&url)
        .header("Authorization", api_key)
        .multipart(form)
        .send()
        .await?;
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        warn!(
            status = status.as_u16(),
            "🔴 FALLBACK: Fireworks speech transcription failed - Reason: provider returned non-success - Impact: user must type or retry voice input"
        );
        return Err(AppError::BadRequest(format!(
            "Fireworks speech transcription failed with HTTP {}: {}",
            status.as_u16(),
            body.chars().take(300).collect::<String>()
        )));
    }

    let value: Value = serde_json::from_str(&body).map_err(|_| {
        AppError::BadRequest("Fireworks returned invalid transcription JSON".to_string())
    })?;
    let text = value
        .get("text")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    if text.is_empty() {
        return Err(AppError::BadRequest(
            "Fireworks returned an empty transcription".to_string(),
        ));
    }

    Ok(Json(SpeechTranscriptionResponse { ok: true, text }))
}

async fn synthesize_speech(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<SpeechSynthesisRequest>,
) -> AppResult<Json<SpeechSynthesisResponse>> {
    let _user_id = get_user_id_from_auth(&headers, &state.config, &state.pool).await?;
    validate_non_empty("text", &req.text)?;

    if req.text.chars().count() > 1_200 {
        return Err(AppError::BadRequest(
            "speech synthesis text must be 1200 characters or less".to_string(),
        ));
    }

    let api_key = state
        .config
        .speech
        .async_api_key
        .as_deref()
        .ok_or_else(|| {
            AppError::BadRequest("Async text-to-speech is not configured".to_string())
        })?;
    let voice_id = req
        .voice_id
        .or_else(|| state.config.speech.async_tts_voice_id.clone())
        .ok_or_else(|| {
            AppError::BadRequest("ASYNC_TTS_VOICE_ID is required for speech synthesis".to_string())
        })?;
    let url = format!(
        "{}/text_to_speech/streaming",
        state.config.speech.async_base_url.trim_end_matches('/')
    );
    let payload = json!({
        "model_id": state.config.speech.async_tts_model.clone(),
        "transcript": req.text,
        "voice": {
            "mode": "id",
            "id": voice_id
        }
    });

    let response = reqwest::Client::new()
        .post(&url)
        .header("x-api-key", api_key)
        .header("version", "v1")
        .json(&payload)
        .send()
        .await?;
    let status = response.status();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();
    let bytes = response.bytes().await?;
    if !status.is_success() {
        let body = String::from_utf8_lossy(&bytes);
        warn!(
            status = status.as_u16(),
            "🔴 FALLBACK: Async speech synthesis failed - Reason: provider returned non-success - Impact: coach reply remains readable text-only"
        );
        return Err(AppError::BadRequest(format!(
            "Async speech synthesis failed with HTTP {}: {}",
            status.as_u16(),
            body.chars().take(300).collect::<String>()
        )));
    }

    Ok(Json(SpeechSynthesisResponse {
        ok: true,
        audio_base64: BASE64_STANDARD.encode(bytes),
        content_type,
    }))
}

fn require_test_endpoints_enabled() -> AppResult<()> {
    if std::env::var("ANTIROT_ENABLE_TEST_ENDPOINTS")
        .ok()
        .as_deref()
        == Some("1")
    {
        Ok(())
    } else {
        Err(AppError::NotFound)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TestResetRequest {
    user_id: Option<String>,
    device_id: Option<String>,
    device_token: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TestToolRequest {
    user_id: Option<String>,
    name: String,
    args: Option<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TestStateQuery {
    user_id: Option<String>,
    device_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TestContextQuery {
    user_id: Option<String>,
    provider: Option<String>,
    model: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TestStateRow {
    state: String,
    source_tool: Option<String>,
    metadata: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TestAlarmCount {
    kind: String,
    severity: String,
    count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TestStateResponse {
    ok: bool,
    user_id: String,
    device_id: String,
    runtime_state: Option<TestStateRow>,
    alarm_counts: Vec<TestAlarmCount>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TestToolResponse {
    ok: bool,
    result: String,
    snapshot: TestStateResponse,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TestContextResponse {
    ok: bool,
    user_id: String,
    report: crate::prompt::PromptBuildReport,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AdminContextResponse {
    ok: bool,
    user_id: String,
    report: crate::prompt::PromptBuildReport,
    runtime_state: Option<TestStateRow>,
    sleep_metrics: crate::memory::SleepMetricsReport,
}

async fn test_reset(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<TestResetRequest>,
) -> AppResult<Json<TestStateResponse>> {
    require_test_endpoints_enabled()?;
    require_admin_auth(&headers, &state.config)?;

    let user_id = req.user_id.unwrap_or_else(|| "admin".to_string());
    let device_id = req.device_id.unwrap_or_else(|| "test-device".to_string());
    let api_token_hash = req.device_token.as_ref().map(|token| token_hash(token));
    let client = state.pool.get().await?;

    client
        .execute(
            "
            INSERT INTO users (id, email, display_name, subscription_tier, subscription_status)
            VALUES ($1, $2, 'Test User', 'tailored', 'active')
            ON CONFLICT (id) DO UPDATE SET
                subscription_tier = 'tailored',
                subscription_status = 'active',
                updated_at = now()
            ",
            &[&user_id, &format!("{}@test.antirot.local", user_id)],
        )
        .await?;

    client
        .execute("DELETE FROM alarms WHERE device_id = $1", &[&device_id])
        .await?;
    client
        .execute("DELETE FROM chat_messages WHERE user_id = $1", &[&user_id])
        .await?;
    client
        .execute("DELETE FROM user_memories WHERE user_id = $1", &[&user_id])
        .await?;
    client
        .execute(
            "DELETE FROM user_runtime_states WHERE user_id = $1",
            &[&user_id],
        )
        .await?;

    client
        .execute(
            "
            INSERT INTO devices (
                device_id,
                user_id,
                workspace_id,
                device_name,
                api_token_hash,
                platform,
                app_version,
                notification_capability,
                usage_capability,
                paired_at
            )
            VALUES ($1, $2, 'main', 'Backend Userflow Test Device', $3, 'ios', 'test', 'remote_notification', 'unknown', now())
            ON CONFLICT (device_id) DO UPDATE SET
                user_id = EXCLUDED.user_id,
                workspace_id = EXCLUDED.workspace_id,
                device_name = EXCLUDED.device_name,
                api_token_hash = EXCLUDED.api_token_hash,
                platform = EXCLUDED.platform,
                app_version = EXCLUDED.app_version,
                notification_capability = EXCLUDED.notification_capability,
                usage_capability = EXCLUDED.usage_capability,
                paired_at = EXCLUDED.paired_at,
                updated_at = now()
            ",
            &[&device_id, &user_id, &api_token_hash],
        )
        .await?;

    client
        .execute(
            "
            INSERT INTO user_runtime_states (user_id, state, source_tool, metadata)
            VALUES ($1, 'onboarding', 'test_reset', '{}'::JSONB)
            ON CONFLICT (user_id) DO UPDATE SET
                state = 'onboarding',
                entered_at = now(),
                source_tool = 'test_reset',
                metadata = '{}'::JSONB
            ",
            &[&user_id],
        )
        .await?;

    Ok(Json(test_snapshot(&client, &user_id, &device_id).await?))
}

async fn test_context(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<TestContextQuery>,
) -> AppResult<Json<TestContextResponse>> {
    require_test_endpoints_enabled()?;
    require_admin_auth(&headers, &state.config)?;

    let user_id = query.user_id.unwrap_or_else(|| "admin".to_string());
    let provider = query.provider.unwrap_or_else(|| "gemini".to_string());
    let model = query
        .model
        .unwrap_or_else(|| "gemini-3.5-flash".to_string());
    let report =
        build_context_report_for_test(&state.pool, &state.config, &user_id, &provider, &model)
            .await?;

    Ok(Json(TestContextResponse {
        ok: true,
        user_id,
        report,
    }))
}

async fn admin_context(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<TestContextQuery>,
) -> AppResult<Json<AdminContextResponse>> {
    require_admin_auth(&headers, &state.config)?;

    let user_id = query.user_id.unwrap_or_else(|| "admin".to_string());
    let provider = query.provider.unwrap_or_else(|| "gemini".to_string());
    let model = query
        .model
        .unwrap_or_else(|| "gemini-3.5-flash".to_string());
    let report =
        build_context_report(&state.pool, &state.config, &user_id, &provider, &model).await?;
    let client = state.pool.get().await?;
    let runtime_state = runtime_state_row(&client, &user_id).await?;
    let sleep_metrics = sleep_metrics_report(&client, &user_id).await?;

    Ok(Json(AdminContextResponse {
        ok: true,
        user_id,
        report,
        runtime_state,
        sleep_metrics,
    }))
}

async fn test_tool(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<TestToolRequest>,
) -> AppResult<Json<TestToolResponse>> {
    require_test_endpoints_enabled()?;
    require_admin_auth(&headers, &state.config)?;
    validate_non_empty("name", &req.name)?;

    let user_id = req.user_id.unwrap_or_else(|| "admin".to_string());
    let result = run_tool_for_test(
        &state.pool,
        &state.config,
        &user_id,
        &req.name,
        req.args.unwrap_or(Value::Null),
    )
    .await;

    let client = state.pool.get().await?;
    let device_id = client
        .query_opt(
            "SELECT device_id FROM devices WHERE user_id = $1 ORDER BY updated_at DESC LIMIT 1",
            &[&user_id],
        )
        .await?
        .map(|row| row.get("device_id"))
        .unwrap_or_else(|| "test-device".to_string());

    Ok(Json(TestToolResponse {
        ok: result.starts_with("Success:"),
        result,
        snapshot: test_snapshot(&client, &user_id, &device_id).await?,
    }))
}

async fn test_state(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<TestStateQuery>,
) -> AppResult<Json<TestStateResponse>> {
    require_test_endpoints_enabled()?;
    require_admin_auth(&headers, &state.config)?;

    let user_id = query.user_id.unwrap_or_else(|| "admin".to_string());
    let device_id = query.device_id.unwrap_or_else(|| "test-device".to_string());
    let client = state.pool.get().await?;
    Ok(Json(test_snapshot(&client, &user_id, &device_id).await?))
}

async fn test_snapshot(
    client: &tokio_postgres::Client,
    user_id: &str,
    device_id: &str,
) -> AppResult<TestStateResponse> {
    let runtime_state = runtime_state_row(client, user_id).await?;

    let alarm_rows = client
        .query(
            "
            SELECT kind, severity, COUNT(*)::BIGINT AS count
            FROM alarms
            WHERE device_id = $1 AND status = 'pending'
            GROUP BY kind, severity
            ORDER BY kind, severity
            ",
            &[&device_id],
        )
        .await?;

    Ok(TestStateResponse {
        ok: true,
        user_id: user_id.to_string(),
        device_id: device_id.to_string(),
        runtime_state,
        alarm_counts: alarm_rows
            .iter()
            .map(|row| TestAlarmCount {
                kind: row.get("kind"),
                severity: row.get("severity"),
                count: row.get("count"),
            })
            .collect(),
    })
}

async fn runtime_state_row(
    client: &tokio_postgres::Client,
    user_id: &str,
) -> AppResult<Option<TestStateRow>> {
    Ok(client
        .query_opt(
            "
            SELECT state, source_tool, metadata::TEXT AS metadata
            FROM user_runtime_states
            WHERE user_id = $1
            ",
            &[&user_id],
        )
        .await?
        .map(|row| TestStateRow {
            state: row.get("state"),
            source_tool: row.get("source_tool"),
            metadata: row.get("metadata"),
        }))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelAlarmsRequest {
    pub device_id: String,
    pub kind: String,
}

#[derive(Debug, Serialize)]
pub struct CancelAlarmsResponse {
    pub ok: bool,
    pub count: i64,
}

async fn cancel_alarms_by_kind(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CancelAlarmsRequest>,
) -> AppResult<Json<CancelAlarmsResponse>> {
    require_admin_auth(&headers, &state.config)?;
    validate_non_empty("deviceId", &request.device_id)?;
    validate_non_empty("kind", &request.kind)?;

    let client = state.pool.get().await?;
    let count = client
        .execute(
            "DELETE FROM alarms WHERE device_id = $1 AND kind = $2 AND status = 'pending'",
            &[&request.device_id, &request.kind],
        )
        .await? as i64;

    Ok(Json(CancelAlarmsResponse { ok: true, count }))
}
