use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use tokio_postgres::Row;
use tracing::{info, warn};

use crate::auth::{require_auth, AuthScope};
use crate::error::{AppError, AppResult};
use crate::models::{
    AlarmActionRequest, AlarmActionResponse, AlarmJob, CreateAlarmRequest, CreateAlarmResponse,
    DeliveryState, DeviceRegistrationRequest, DeviceRegistrationResponse, HealthResponse,
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
        .route("/devices/register", post(register_device))
        .route("/alarms", post(create_alarm))
        .route("/alarms/pending", get(pending_alarms))
        .route("/alarms/{alarm_id}/{action}", post(record_alarm_action))
        .route("/visits", get(get_and_increment_visits))
        .route("/v1/health", get(health))
        .route("/v1/devices/register", post(register_device))
        .route("/v1/alarms", post(create_alarm))
        .route("/v1/alarms/pending", get(pending_alarms))
        .route("/v1/alarms/{alarm_id}/{action}", post(record_alarm_action))
        .route("/v1/visits", get(get_and_increment_visits))
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        ok: true,
        service: "antirot-bridge",
    })
}

async fn register_device(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<DeviceRegistrationRequest>,
) -> AppResult<Json<DeviceRegistrationResponse>> {
    require_auth(&headers, &state.config, AuthScope::Device)?;
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

async fn create_alarm(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateAlarmRequest>,
) -> AppResult<Json<CreateAlarmResponse>> {
    require_auth(&headers, &state.config, AuthScope::Admin)?;
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

async fn pending_alarms(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<PendingQuery>,
) -> AppResult<Json<Vec<AlarmJob>>> {
    require_auth(&headers, &state.config, AuthScope::Device)?;
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
    require_auth(&headers, &state.config, AuthScope::Device)?;
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

async fn get_and_increment_visits(
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let client = state.pool.get().await?;
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
    let count: i64 = row.get("count");

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
