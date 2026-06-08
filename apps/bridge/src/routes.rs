use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;
use tracing::{info, warn};
use uuid::Uuid;

use crate::auth::{
    get_user_id_from_auth, require_admin_auth, require_device_auth, require_device_auth_for,
    token_hash,
};
use crate::error::{AppError, AppResult};
use crate::llm::chat_with_coach;
use crate::models::{
    AlarmActionRequest, AlarmActionResponse, AlarmJob, ChatRequest, ChatResponse,
    CreateAlarmRequest, CreateAlarmResponse, DeliveryState, DeviceRegistrationRequest,
    DeviceRegistrationResponse, GoogleAuthRequest, GoogleAuthResponse, HealthResponse,
    MemoryResponse, PairingClaimRequest, PairingClaimResponse, SubscriptionResponse,
    SubscriptionUpdateRequest, UpdateMemoryRequest, WorkspaceDevice, WorkspaceDevicesResponse,
};
use crate::AppState;


#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PendingQuery {
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
        .route("/subscription", get(get_subscription).post(update_subscription))
        .route("/memory/{key}", get(get_memory).put(update_memory))
        .route("/chat", post(chat_coach))
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
        .route("/v1/subscription", get(get_subscription).post(update_subscription))
        .route("/v1/memory/{key}", get(get_memory).put(update_memory))
        .route("/v1/chat", post(chat_coach))
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        ok: true,
        service: "antirot-bridge",
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
            "Google OAuth is not configured on this bridge".to_string(),
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
            "🔴 FALLBACK: APNs wake failed - Reason: bridge could not complete APNs request - Impact: iOS app must poll/open before scheduling"
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
                if kind == "session_alarm" || kind == "wake_alarm" {
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
            .query_one("SELECT subscription_active_until FROM users WHERE id = $1", &[&user_id])
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

    match key.as_str() {
        "longterm" | "shortterm" | "behavior" | "tasks" | "sleep" | "work" | "miscellaneous_todo" => {}
        _ => return Err(AppError::BadRequest("Invalid memory key".to_string())),
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
        let default_content = match key.as_str() {
            "longterm" => "# Long-Term Goals\n\n## Direction\n- Distilled long-term goals go here.\n\n## Standards\n- High standards, honest recovery, no fake praise.\n",
            "shortterm" => "# Short-Term State\n\n## Current Priorities\n- Near-term priorities go here.\n\n## Constraints\n- Sleep, health, vacation mode go here.\n",
            "behavior" => "# Behavior Memory\n\n## Recurring Patterns\n- Stable patterns go here.\n\n## Drift Tendencies\n- Known drift loops go here.\n\n## Accountability Styles\n- Tactics that work/fail go here.\n",
            "tasks" => "# Task Pipeline\n",
            "sleep" => "# Sleep Ledger\n",
            "work" => "# Work Ledger\n",
            _ => "# Miscellaneous Todo\n",
        };
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

    match key.as_str() {
        "longterm" | "shortterm" | "behavior" | "tasks" | "sleep" | "work" | "miscellaneous_todo" => {}
        _ => return Err(AppError::BadRequest("Invalid memory key".to_string())),
    }

    client
        .execute(
            "
            INSERT INTO user_memories (user_id, memory_key, content, updated_at)
            VALUES ($1, $2, $3, now())
            ON CONFLICT (user_id, memory_key) DO UPDATE SET
                content = EXCLUDED.content,
                updated_at = now()
            ",
            &[&user_id, &key, &req.content],
        )
        .await?;

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

