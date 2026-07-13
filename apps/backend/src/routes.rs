use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration as StdDuration;

use axum::extract::{DefaultBodyLimit, Multipart, Path, Query, Request, State};
use axum::http::header::SET_COOKIE;
use axum::http::{HeaderMap, HeaderValue};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use chrono::{DateTime, Datelike, Duration, FixedOffset, NaiveDate, SecondsFormat, Utc};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio_postgres::Row;
use tracing::{info, warn};
use uuid::Uuid;

use crate::alarm::{persist_alarm, AlarmWrite};
use crate::auth::{
    expired_session_cookie_header, get_user_id_from_auth, is_legacy_device_bootstrap,
    issue_session_jwt, require_admin_auth, require_device_auth_for, session_cookie_header,
    token_hash, validated_session_from_headers, SESSION_DAYS,
};
use crate::error::{AppError, AppResult};
use crate::llm::{
    build_context_report, build_context_report_for_test, chat_with_coach, run_tool_for_test,
    FIRST_ONBOARDING_REPLY,
};
use crate::memory::{
    create_memory_snapshot as create_memory_snapshot_record, list_memory_snapshots,
    restore_memory_snapshot as restore_memory_snapshot_record, MemorySnapshotSummary,
    MEMORY_SNAPSHOT_LIMIT,
};
use crate::memory::{
    distill_date_for_test, run_memory_activation_race_probe, run_memory_db_invariant_probe,
    save_memory_canonical, save_memory_indexed, sleep_metrics_report, user_day_for,
};
use crate::models::{
    AlarmActionRequest, AlarmActionResponse, AlarmCancellationTombstone, AlarmJob, AlarmKind,
    AlarmReconcileRequest, AlarmReconcileResponse, AuthMeResponse, ChatHistoryMessage,
    ChatHistoryResponse, ChatRequest, ChatResponse, CreateAlarmRequest, CreateAlarmResponse,
    CreateMemorySnapshotRequest, CreateMemorySnapshotResponse, CreateReportRequest,
    CreateReportResponse, DeliveryState, DeviceRegistrationRequest, DeviceRegistrationResponse,
    GoogleAuthRequest, GoogleAuthResponse, HealthResponse, ListMemorySnapshotsResponse,
    MemoryResponse, MemorySnapshotSummaryResponse, OnboardingProfileRequest,
    OnboardingProfileResponse, PairingClaimRequest, PairingClaimResponse, PendingAlarmsResponse,
    RestoreMemorySnapshotRequest, RestoreMemorySnapshotResponse, RuntimeStateResponse,
    RuntimeStateResponsePayload, SessionRequest, SessionResponse, SpeechSynthesisRequest,
    SpeechSynthesisResponse, SpeechTranscriptionResponse, StatsPeriodResponse, StatsResponse,
    SubscriptionResponse, SubscriptionUpdateRequest, UpdateMemoryRequest, WorkspaceDevice,
    WorkspaceDevicesResponse,
};
use crate::prompt::{allowed_memory_key, default_memory_for_key, normalize_memory_content};
use crate::secrets::encrypt_byok_key;
use crate::AppState;

const MAX_SYNTHESIZED_AUDIO_BYTES: usize = 10 * 1024 * 1024;
const MAX_INWORLD_STREAM_BUFFER_BYTES: usize = 15 * 1024 * 1024;
const MAX_CHAT_MESSAGE_CHARS: usize = 12_000;
const MAX_CHAT_REQUEST_ID_CHARS: usize = 200;
const MAX_AUDIO_UPLOAD_BYTES: usize = 25 * 1024 * 1024;
const MULTIPART_OVERHEAD_BYTES: usize = 1024 * 1024;
const MAX_ALARM_RECONCILE_ITEMS: usize = 200;
const MAX_ALARM_CANCELLATION_TOMBSTONES: i64 = 500;
static LEGACY_ALIAS_HITS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PendingQuery {
    #[serde(alias = "device_id")]
    device_id: String,
    limit: Option<i64>,
    reconcile: Option<bool>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(api_info))
        .route("/health", get(health))
        .route("/auth/session", post(auth_session))
        .route("/auth/me", get(auth_me))
        .route("/auth/logout", post(auth_logout))
        .route("/auth/google", post(auth_google))
        .route("/profile/onboarding", post(save_onboarding_profile))
        .route("/pairing/claim", post(claim_pairing))
        .route("/devices/register", post(register_device))
        .route("/workspaces/{workspace_id}/devices", get(workspace_devices))
        .route("/alarms", post(create_alarm))
        .route("/alarms/pending", get(pending_alarms))
        .route("/alarms/reconcile", post(reconcile_alarms))
        .route("/alarms/cancel", post(cancel_alarms_by_kind))
        .route("/alarms/{alarm_id}/{action}", post(record_alarm_action))
        .route("/visits", get(get_and_increment_visits))
        .route(
            "/subscription",
            get(get_subscription).post(update_subscription),
        )
        .route(
            "/memory/snapshots",
            get(get_memory_snapshots).post(create_memory_snapshot),
        )
        .route(
            "/memory/snapshots/{snapshot_id}/restore",
            post(restore_memory_snapshot),
        )
        .route("/memory/{key}", get(get_memory).put(update_memory))
        .route("/admin/context", get(admin_context))
        .route("/chat", post(chat_coach))
        .route("/chat/history", get(chat_history))
        .route("/state", get(get_runtime_state))
        .route("/stats", get(get_stats))
        .route("/reports", post(create_report))
        .route(
            "/speech/transcribe",
            post(transcribe_speech).layer(DefaultBodyLimit::max(
                MAX_AUDIO_UPLOAD_BYTES + MULTIPART_OVERHEAD_BYTES,
            )),
        )
        .route("/speech/synthesize", post(synthesize_speech))
        .route("/test/reset", post(test_reset))
        .route("/test/tool", post(test_tool))
        .route("/test/state", get(test_state))
        .route("/test/context", get(test_context))
        .route("/v1", get(api_info))
        .route("/v1/health", get(health))
        .route("/v1/auth/session", post(auth_session))
        .route("/v1/auth/me", get(auth_me))
        .route("/v1/auth/logout", post(auth_logout))
        .route("/v1/auth/google", post(auth_google))
        .route("/v1/profile/onboarding", post(save_onboarding_profile))
        .route("/v1/pairing/claim", post(claim_pairing))
        .route("/v1/devices/register", post(register_device))
        .route(
            "/v1/workspaces/{workspace_id}/devices",
            get(workspace_devices),
        )
        .route("/v1/alarms", post(create_alarm))
        .route("/v1/alarms/pending", get(pending_alarms))
        .route("/v1/alarms/reconcile", post(reconcile_alarms))
        .route("/v1/alarms/cancel", post(cancel_alarms_by_kind))
        .route("/v1/alarms/{alarm_id}/{action}", post(record_alarm_action))
        .route("/v1/visits", get(get_and_increment_visits))
        .route(
            "/v1/subscription",
            get(get_subscription).post(update_subscription),
        )
        .route(
            "/v1/memory/snapshots",
            get(get_memory_snapshots).post(create_memory_snapshot),
        )
        .route(
            "/v1/memory/snapshots/{snapshot_id}/restore",
            post(restore_memory_snapshot),
        )
        .route("/v1/memory/{key}", get(get_memory).put(update_memory))
        .route("/v1/admin/context", get(admin_context))
        .route("/v1/chat", post(chat_coach))
        .route("/v1/chat/history", get(chat_history))
        .route("/v1/state", get(get_runtime_state))
        .route("/v1/stats", get(get_stats))
        .route("/v1/reports", post(create_report))
        .route(
            "/v1/speech/transcribe",
            post(transcribe_speech).layer(DefaultBodyLimit::max(
                MAX_AUDIO_UPLOAD_BYTES + MULTIPART_OVERHEAD_BYTES,
            )),
        )
        .route("/v1/speech/synthesize", post(synthesize_speech))
        .route("/v1/test/reset", post(test_reset))
        .route("/v1/test/tool", post(test_tool))
        .route("/v1/test/state", get(test_state))
        .route("/v1/test/context", get(test_context))
        .route("/v1/test/memory/invariants", post(test_memory_invariants))
        .route(
            "/v1/test/memory/activation-race",
            post(test_memory_activation_race),
        )
        .route("/v1/test/memory/distill", post(test_memory_distill))
        .route("/v1/test/alarm-wake/seed", post(test_alarm_wake_seed))
        .layer(axum::middleware::from_fn(instrument_legacy_alias))
}

async fn instrument_legacy_alias(request: Request, next: Next) -> Response {
    let path = request.uri().path().to_string();
    let method = request.method().clone();
    let legacy = is_legacy_alias_path(&path);
    let mut response = next.run(request).await;
    if legacy {
        let hits = LEGACY_ALIAS_HITS.fetch_add(1, Ordering::Relaxed) + 1;
        warn!(%method, %path, hits, removal_date = "2026-10-31", "legacy unversioned API alias used");
        response
            .headers_mut()
            .insert("x-antirot-legacy-alias", HeaderValue::from_static("true"));
        if let Ok(value) = HeaderValue::from_str(&hits.to_string()) {
            response
                .headers_mut()
                .insert("x-antirot-legacy-hit-count", value);
        }
    }
    response
}

fn is_legacy_alias_path(path: &str) -> bool {
    const STATIC_PATHS: &[&str] = &[
        "/",
        "/health",
        "/auth/session",
        "/auth/me",
        "/auth/logout",
        "/auth/google",
        "/profile/onboarding",
        "/pairing/claim",
        "/devices/register",
        "/alarms",
        "/alarms/pending",
        "/alarms/reconcile",
        "/alarms/cancel",
        "/visits",
        "/subscription",
        "/memory/snapshots",
        "/admin/context",
        "/chat",
        "/chat/history",
        "/state",
        "/stats",
        "/reports",
        "/speech/transcribe",
        "/speech/synthesize",
        "/test/reset",
        "/test/tool",
        "/test/state",
        "/test/context",
    ];
    if STATIC_PATHS.contains(&path) {
        return true;
    }
    let Some(path) = path.strip_prefix('/') else {
        return false;
    };
    let segments = path.split('/').collect::<Vec<_>>();
    if segments.iter().any(|segment| segment.is_empty()) {
        return false;
    }
    matches!(
        segments.as_slice(),
        ["workspaces", _, "devices"]
            | ["alarms", _, _]
            | ["memory", _]
            | ["memory", "snapshots", _, "restore"]
    )
}

async fn api_info() -> Json<Value> {
    Json(json!({
        "ok": true,
        "service": "antirot-backend",
        "health": "/v1/health",
        "version": "v1"
    }))
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        ok: true,
        service: "antirot-backend",
        current_time_ist: current_time_ist(Utc::now()),
    })
}

async fn save_onboarding_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<OnboardingProfileRequest>,
) -> AppResult<Json<OnboardingProfileResponse>> {
    let name = request.name.trim().to_string();
    let timezone = request.timezone.trim().to_string();
    validate_onboarding_profile(&name, &timezone)?;
    let user_id = get_user_id_from_auth(&headers, &state.config, &state.pool).await?;
    let mut client = state.pool.get().await?;
    let transaction = client.transaction().await?;
    let updated = transaction
        .execute(
            "UPDATE users SET display_name=$2, timezone=$3, updated_at=now() WHERE id=$1",
            &[&user_id, &name, &timezone],
        )
        .await?;
    if updated != 1 {
        return Err(AppError::NotFound);
    }
    let profile = format!(
        "# User Profile\n\n- Name: {name}\n- Preferred address: {name}\n- Timezone: {timezone}\n\n## Notes\n- Learn the user over time without building a creepy dossier.\n"
    );
    save_memory_canonical(&*transaction, &user_id, "user_profile", &profile).await?;
    transaction.commit().await?;
    Ok(Json(OnboardingProfileResponse {
        ok: true,
        name,
        timezone,
        reply: FIRST_ONBOARDING_REPLY.to_string(),
    }))
}

fn current_time_ist(now: DateTime<Utc>) -> String {
    let ist =
        FixedOffset::east_opt(5 * 60 * 60 + 30 * 60).expect("IST fixed offset should be valid");
    now.with_timezone(&ist)
        .to_rfc3339_opts(SecondsFormat::Secs, true)
}

async fn auth_google(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<GoogleAuthRequest>,
) -> AppResult<impl IntoResponse> {
    enforce_rate_limit(&state, &headers, "google_auth", 30)?;
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
    let user_id = if let Some(existing_user) = existing_user {
        let user_id: String = existing_user.get("user_id");
        client
            .execute(
                "UPDATE users SET email=$2, display_name=$3, avatar_url=$4, updated_at=now()
                 WHERE id=$1",
                &[&user_id, &profile.email, &profile.name, &profile.picture],
            )
            .await?;
        user_id
    } else {
        if client
            .query_opt("SELECT 1 FROM users WHERE email=$1", &[&profile.email])
            .await?
            .is_some()
        {
            return Err(AppError::Conflict(
                "email is already registered; use an authenticated explicit account-link flow"
                    .to_string(),
            ));
        }
        client
            .execute(
                "INSERT INTO users (id,email,display_name,avatar_url) VALUES ($1,$2,$3,$4)",
                &[
                    &fallback_user_id,
                    &profile.email,
                    &profile.name,
                    &profile.picture,
                ],
            )
            .await?;
        fallback_user_id
    };

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

    let device_row = client
        .query_opt(
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
            WHERE devices.user_id = EXCLUDED.user_id
            RETURNING device_id, session_version
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
    let Some(device_row) = device_row else {
        warn!(
            device_id = %request.device_id,
            user_id = %user_id,
            "rejected Google sign-in for device owned by another user"
        );
        return Err(AppError::Unauthorized);
    };
    let session_version: i64 = device_row.get("session_version");

    info!(
        device_id = %request.device_id,
        user_id = %user_id,
        email = %profile.email,
        "registered Google-authenticated device"
    );

    let session_jwt =
        issue_session_jwt(&state.config, &user_id, &request.device_id, session_version)?;
    let mut headers = HeaderMap::new();
    headers.insert(
        SET_COOKIE,
        HeaderValue::from_str(&session_cookie_header(
            &session_jwt,
            SESSION_DAYS * 24 * 60 * 60,
        ))?,
    );

    Ok((
        headers,
        Json(GoogleAuthResponse {
            ok: true,
            user_id,
            device_id: request.device_id,
            device_token,
            email: profile.email,
            name: profile.name,
            message: "Signed in with Google".to_string(),
        }),
    ))
}

async fn auth_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<SessionRequest>,
) -> AppResult<impl IntoResponse> {
    if let Some(claims) = validated_session_from_headers(&headers, &state.config, &state.pool).await
    {
        let session_jwt =
            issue_session_jwt(&state.config, &claims.sub, &claims.device_id, claims.ver)?;
        let mut response_headers = HeaderMap::new();
        response_headers.insert(
            SET_COOKIE,
            HeaderValue::from_str(&session_cookie_header(
                &session_jwt,
                SESSION_DAYS * 24 * 60 * 60,
            ))?,
        );
        return Ok((
            response_headers,
            Json(SessionResponse {
                ok: true,
                user_id: claims.sub,
                device_id: claims.device_id,
                device_token: None,
                expires_in_days: SESSION_DAYS,
            }),
        ));
    }

    enforce_rate_limit(&state, &headers, "anonymous_session", 10)?;
    if !state.config.allow_anonymous_sessions {
        return Err(AppError::Unauthorized);
    }

    let mut client = state.pool.get().await?;
    let transaction = client.transaction().await?;
    let user_id = Uuid::new_v4().to_string();
    let device_id = request
        .device_id
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| format!("web-{}", Uuid::new_v4().simple()));
    let platform = request
        .platform
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "web".to_string());
    let device_token = format!("antirot_{}", Uuid::new_v4().simple());
    let api_token_hash = token_hash(&device_token);
    let email = format!("{user_id}@anon.antirot.local");

    transaction
        .execute(
            "
            INSERT INTO users (id, email, display_name)
            VALUES ($1, $2, $3)
            ON CONFLICT (id) DO NOTHING
            ",
            &[&user_id, &email, &"Anonymous User"],
        )
        .await?;

    let inserted_device = transaction
        .query_opt(
            "
            INSERT INTO devices (
                device_id,
                user_id,
                api_token_hash,
                platform,
                app_version,
                notification_capability,
                usage_capability
            )
            VALUES ($1, $2, $3, $4, 'web', 'browser', 'unknown')
            ON CONFLICT (device_id) DO NOTHING
            RETURNING device_id
            ",
            &[&device_id, &user_id, &api_token_hash, &platform],
        )
        .await?;
    if inserted_device.is_none() {
        warn!(device_id = %device_id, "rejected anonymous session device ID collision");
        return Err(AppError::BadRequest(
            "deviceId is already registered; generate a new device ID".to_string(),
        ));
    }

    transaction
        .execute(
            "
            INSERT INTO user_runtime_states (user_id, state, source_tool)
            VALUES ($1, 'onboarding', 'auth_session')
            ON CONFLICT (user_id) DO NOTHING
            ",
            &[&user_id],
        )
        .await?;
    transaction.commit().await?;

    let session_jwt = issue_session_jwt(&state.config, &user_id, &device_id, 1)?;
    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        SET_COOKIE,
        HeaderValue::from_str(&session_cookie_header(
            &session_jwt,
            SESSION_DAYS * 24 * 60 * 60,
        ))?,
    );

    info!(
        user_id = %user_id,
        device_id = %device_id,
        "created browser session"
    );

    Ok((
        response_headers,
        Json(SessionResponse {
            ok: true,
            user_id,
            device_id,
            device_token: Some(device_token),
            expires_in_days: SESSION_DAYS,
        }),
    ))
}

async fn auth_me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<AuthMeResponse>> {
    let claims = validated_session_from_headers(&headers, &state.config, &state.pool)
        .await
        .ok_or(AppError::Unauthorized)?;
    Ok(Json(AuthMeResponse {
        ok: true,
        user_id: claims.sub,
        device_id: Some(claims.device_id),
    }))
}

async fn auth_logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<impl IntoResponse> {
    if let Some(claims) = validated_session_from_headers(&headers, &state.config, &state.pool).await
    {
        let client = state.pool.get().await?;
        client
            .execute(
                "UPDATE devices SET session_version = session_version + 1, updated_at=now()
                 WHERE device_id=$1 AND user_id=$2 AND session_version=$3",
                &[&claims.device_id, &claims.sub, &claims.ver],
            )
            .await?;
    }
    let mut headers = HeaderMap::new();
    headers.insert(
        SET_COOKIE,
        HeaderValue::from_str(&expired_session_cookie_header())?,
    );
    Ok((headers, Json(json!({ "ok": true }))))
}

async fn register_device(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<DeviceRegistrationRequest>,
) -> AppResult<Json<DeviceRegistrationResponse>> {
    validate_non_empty("deviceId", &request.device_id)?;
    validate_non_empty("platform", &request.platform)?;
    let client = state.pool.get().await?;
    let issued_device_token = if is_legacy_device_bootstrap(&headers, &state.config) {
        let device_token = format!("antirot_{}", Uuid::new_v4().simple());
        let api_token_hash = token_hash(&device_token);
        let registered = client
            .query_opt(
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
                VALUES ($1, 'admin', $2, $3, $4, $5, $6, $7, $8)
                ON CONFLICT (device_id) DO UPDATE SET
                    user_id = COALESCE(devices.user_id, 'admin'),
                    api_token_hash = EXCLUDED.api_token_hash,
                    platform = EXCLUDED.platform,
                    app_version = EXCLUDED.app_version,
                    notification_capability = EXCLUDED.notification_capability,
                    usage_capability = EXCLUDED.usage_capability,
                    push_provider = EXCLUDED.push_provider,
                    push_token = EXCLUDED.push_token,
                    updated_at = now()
                WHERE devices.api_token_hash IS NULL
                RETURNING device_id
                ",
                &[
                    &request.device_id,
                    &api_token_hash,
                    &request.platform,
                    &request.app_version,
                    &request.notification_capability,
                    &request.usage_capability,
                    &request.push_provider,
                    &request.push_token,
                ],
            )
            .await?;
        if registered.is_none() {
            return Err(AppError::Unauthorized);
        }
        Some(device_token)
    } else {
        require_device_auth_for(&headers, &state.config, &state.pool, &request.device_id).await?;
        let updated = client
            .execute(
                "
            UPDATE devices
            SET platform = $1,
                app_version = $2,
                notification_capability = $3,
                usage_capability = $4,
                push_provider = $5,
                push_token = $6,
                updated_at = now()
            WHERE device_id = $7
            ",
                &[
                    &request.platform,
                    &request.app_version,
                    &request.notification_capability,
                    &request.usage_capability,
                    &request.push_provider,
                    &request.push_token,
                    &request.device_id,
                ],
            )
            .await?;
        if updated == 0 {
            return Err(AppError::NotFound);
        }
        None
    };

    info!(device_id = %request.device_id, platform = %request.platform, "registered device");
    Ok(Json(DeviceRegistrationResponse {
        ok: true,
        device_id: request.device_id,
        device_token: issued_device_token,
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
    enforce_rate_limit(&state, &headers, "pairing_claim", 10)?;
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
                attempt_count = attempt_count + 1,
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
    let series_id = format!("explicit-{id}");
    let generation = 1_i64;
    let severity = request.normalized_severity();
    let hidden_buffer_applied = request.hidden_buffer_applied.unwrap_or(false);
    let requires_acknowledgement = request.requires_acknowledgement.unwrap_or(true);
    let mut client = state.pool.get().await?;
    let transaction = client.transaction().await?;

    let device = transaction
        .query_opt(
            "SELECT user_id FROM devices WHERE device_id = $1",
            &[&request.device_id],
        )
        .await?;
    let Some(device) = device else {
        return Err(AppError::BadRequest(format!(
            "device {} is not registered",
            request.device_id
        )));
    };
    let device_user_id: Option<String> = device.get("user_id");

    let row = persist_alarm(
        &*transaction,
        &AlarmWrite {
            id: id.clone(),
            device_id: request.device_id.clone(),
            kind,
            series_id: series_id.clone(),
            generation,
            severity,
            title: request.title.clone(),
            message: request.message.clone(),
            fire_at: request.fire_at,
            hidden_buffer_applied,
            requires_acknowledgement,
            expires_at: request.expires_at,
        },
    )
    .await?;
    transaction.commit().await?;
    let _ = device_user_id;

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

fn lease_pending_alarms_query() -> &'static str {
    "WITH candidates AS (
         SELECT id FROM alarms
         WHERE device_id = $1
           AND (status = 'pending'
             OR (status = 'leased' AND delivery_lease_expires_at <= now()))
           AND (expires_at IS NULL OR expires_at > now())
         ORDER BY fire_at ASC
         LIMIT $2
         FOR UPDATE SKIP LOCKED
     )
     UPDATE alarms alarm
     SET status = 'leased', delivery_token = $3,
         delivery_lease_expires_at = now() + interval '10 minutes',
         delivery_attempts = delivery_attempts + 1,
         last_delivered_at = now(), updated_at = now()
     FROM candidates
     WHERE alarm.id = candidates.id
     RETURNING alarm.id, alarm.kind, alarm.series_id, alarm.generation,
         alarm.delivery_token, alarm.severity, alarm.title, alarm.message,
         alarm.fire_at, alarm.hidden_buffer_applied,
         alarm.requires_acknowledgement, alarm.expires_at"
}

fn confirm_scheduled_alarm_query() -> &'static str {
    "UPDATE alarms SET status = 'scheduled', scheduled_local_id = $4,
         scheduled_at = now(), delivery_lease_expires_at = NULL, updated_at = now()
     WHERE id = $1 AND device_id = $2 AND delivery_token = $3
       AND (status = 'leased'
         OR (status = 'scheduled' AND scheduled_local_id = $4))"
}

fn cancel_alarm_generations_by_kind_query() -> &'static str {
    "UPDATE alarms SET status = 'cancelled', cancellation_confirmed_at = NULL,
         delivery_token = NULL, delivery_lease_expires_at = NULL, updated_at = now()
     WHERE (series_id, generation) IN (
         SELECT series_id, generation FROM alarms
         WHERE device_id = $1 AND kind = $2
           AND status IN ('pending', 'leased', 'scheduled')
     ) AND status IN ('pending', 'leased', 'scheduled')"
}

fn acknowledge_alarm_generation_query() -> &'static str {
    "UPDATE alarms SET status = 'cancelled', cancellation_confirmed_at = NULL,
         delivery_token = NULL, delivery_lease_expires_at = NULL, updated_at = now()
     WHERE series_id = $1 AND generation = $2
       AND status IN ('pending', 'leased', 'scheduled')"
}

fn snooze_action_replay_query() -> &'static str {
    "SELECT parameters_hash, duration_minutes, request_device_id,
         replacement_alarm_id, replacement_series_id, replacement_generation
     FROM alarm_action_replays
     WHERE original_alarm_id = $1 AND action = 'snooze'
     FOR UPDATE"
}

fn persist_snooze_action_replay_query() -> &'static str {
    "INSERT INTO alarm_action_replays
        (original_alarm_id, action, parameters_hash, duration_minutes, request_device_id,
         replacement_alarm_id, replacement_series_id, replacement_generation)
     VALUES ($1, 'snooze', $2, $3, $4, $5, $6, $7)
     ON CONFLICT (original_alarm_id, action) DO NOTHING"
}

async fn pending_alarms(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<PendingQuery>,
) -> AppResult<Json<Value>> {
    validate_non_empty("deviceId", &query.device_id)?;
    require_device_auth_for(&headers, &state.config, &state.pool, &query.device_id).await?;
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let mut client = state.pool.get().await?;
    let transaction = client.transaction().await?;
    let delivery_token = Uuid::new_v4().to_string();

    let rows = transaction
        .query(
            lease_pending_alarms_query(),
            &[&query.device_id, &limit, &delivery_token],
        )
        .await?;
    transaction
        .execute(
            "UPDATE alarms SET cancellation_confirmed_at=now(), updated_at=now()
             WHERE device_id=$1 AND status='cancelled'
               AND cancellation_confirmed_at IS NULL
               AND updated_at < now()-interval '90 days'",
            &[&query.device_id],
        )
        .await?;
    let cancellations = transaction
        .query(
            "SELECT series_id, scheduled_local_id AS local_alarm_id FROM alarms
             WHERE device_id = $1 AND status = 'cancelled'
               AND cancellation_confirmed_at IS NULL
             ORDER BY updated_at ASC
             LIMIT $2",
            &[&query.device_id, &MAX_ALARM_CANCELLATION_TOMBSTONES],
        )
        .await?;
    transaction.commit().await?;

    info!(
        device_id = %query.device_id,
        count = rows.len(),
        "leased pending alarms"
    );
    let mut cancellation_groups = std::collections::BTreeMap::<String, Vec<String>>::new();
    for row in &cancellations {
        cancellation_groups.entry(row.get("series_id")).or_default();
        if let Some(local_alarm_id) = row.get::<_, Option<String>>("local_alarm_id") {
            cancellation_groups
                .get_mut(&row.get::<_, String>("series_id"))
                .expect("cancellation group was inserted")
                .push(local_alarm_id);
        }
    }
    let response = PendingAlarmsResponse {
        alarms: rows.iter().map(alarm_from_row).collect(),
        cancelled_series_ids: cancellations
            .iter()
            .map(|row| row.get("series_id"))
            .collect::<std::collections::HashSet<String>>()
            .into_iter()
            .collect(),
        cancelled_alarm_ids: cancellations
            .iter()
            .filter_map(|row| row.get::<_, Option<String>>("local_alarm_id"))
            .collect(),
        cancellations: cancellation_groups
            .into_iter()
            .map(|(series_id, local_alarm_ids)| AlarmCancellationTombstone {
                series_id,
                local_alarm_ids,
            })
            .collect(),
    };
    if query.reconcile.unwrap_or(false) {
        Ok(Json(serde_json::to_value(response).map_err(|error| {
            AppError::BadRequest(format!("failed to serialize alarm reconciliation: {error}"))
        })?))
    } else {
        Ok(Json(serde_json::to_value(response.alarms).map_err(
            |error| AppError::BadRequest(format!("failed to serialize pending alarms: {error}")),
        )?))
    }
}

async fn reconcile_alarms(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<AlarmReconcileRequest>,
) -> AppResult<Json<AlarmReconcileResponse>> {
    validate_non_empty("deviceId", &request.device_id)?;
    if request.scheduled.len() > MAX_ALARM_RECONCILE_ITEMS
        || request.cancelled_series_ids.len() > MAX_ALARM_RECONCILE_ITEMS
    {
        return Err(AppError::BadRequest(format!(
            "alarm reconciliation arrays must contain at most {MAX_ALARM_RECONCILE_ITEMS} items"
        )));
    }
    require_device_auth_for(&headers, &state.config, &state.pool, &request.device_id).await?;
    let mut client = state.pool.get().await?;
    let transaction = client.transaction().await?;
    let mut scheduled_count = 0_i64;
    for confirmation in &request.scheduled {
        validate_non_empty("alarmId", &confirmation.alarm_id)?;
        validate_non_empty("deliveryToken", &confirmation.delivery_token)?;
        validate_non_empty("localAlarmId", &confirmation.local_alarm_id)?;
        scheduled_count += transaction
            .execute(
                confirm_scheduled_alarm_query(),
                &[
                    &confirmation.alarm_id,
                    &request.device_id,
                    &confirmation.delivery_token,
                    &confirmation.local_alarm_id,
                ],
            )
            .await? as i64;
    }
    if scheduled_count != request.scheduled.len() as i64 {
        return Err(AppError::Conflict(
            "one or more alarm delivery leases expired; fetch and schedule again".to_string(),
        ));
    }
    let cancellation_count = if request.cancelled_series_ids.is_empty() {
        0
    } else {
        transaction
            .execute(
                "UPDATE alarms SET cancellation_confirmed_at = now(), updated_at = now()
                 WHERE device_id = $1 AND series_id = ANY($2) AND status = 'cancelled'",
                &[&request.device_id, &request.cancelled_series_ids],
            )
            .await? as i64
    };
    transaction.commit().await?;
    Ok(Json(AlarmReconcileResponse {
        ok: true,
        scheduled_count,
        cancellation_count,
    }))
}

async fn record_alarm_action(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((alarm_id, path_action)): Path<(String, String)>,
    Json(request): Json<AlarmActionRequest>,
) -> AppResult<Json<AlarmActionResponse>> {
    validate_non_empty("deviceId", &request.device_id)?;
    validate_non_empty("alarmId", &alarm_id)?;
    require_device_auth_for(&headers, &state.config, &state.pool, &request.device_id).await?;

    let action = normalize_action(&path_action, &request.action)?;
    let mut client = state.pool.get().await?;
    let transaction = client.transaction().await?;

    let alarm_row = transaction
        .query_opt(
            "SELECT series_id, generation, kind, severity, title, message,
                hidden_buffer_applied, requires_acknowledgement
             FROM alarms WHERE id = $1 AND device_id = $2 FOR UPDATE",
            &[&alarm_id, &request.device_id],
        )
        .await?
        .ok_or(AppError::NotFound)?;
    let series_id: String = alarm_row.get("series_id");
    let generation: i64 = alarm_row.get("generation");

    if action != "snooze" {
        transaction
            .execute(
                "INSERT INTO alarm_events (alarm_id, device_id, action, minutes, occurred_at)
                 VALUES ($1, $2, $3, $4, $5)",
                &[
                    &alarm_id,
                    &request.device_id,
                    &action,
                    &request.minutes,
                    &request.at,
                ],
            )
            .await?;
    }

    let mut replacement_alarm = None;
    let status = match action.as_str() {
        "snooze" => {
            let minutes = request.minutes.unwrap_or(9).clamp(1, 180);
            let parameters_hash = format!("minutes:{minutes}");
            if let Some(replay) = transaction
                .query_opt(snooze_action_replay_query(), &[&alarm_id])
                .await?
            {
                let stored_hash: String = replay.get("parameters_hash");
                let stored_minutes: i32 = replay.get("duration_minutes");
                let stored_device_id: String = replay.get("request_device_id");
                if stored_hash != parameters_hash
                    || stored_minutes != minutes
                    || stored_device_id != request.device_id
                {
                    return Err(AppError::Conflict(
                        "snooze was already applied with different parameters".to_string(),
                    ));
                }
                let replacement_alarm_id: String = replay.get("replacement_alarm_id");
                let replacement_row = transaction
                    .query_one(
                        "SELECT id, kind, series_id, generation, delivery_token, severity,
                             title, message, fire_at, hidden_buffer_applied,
                             requires_acknowledgement, expires_at
                         FROM alarms WHERE id = $1 AND device_id = $2",
                        &[&replacement_alarm_id, &request.device_id],
                    )
                    .await?;
                replacement_alarm = Some(alarm_from_row(&replacement_row));
                "pending".to_string()
            } else {
                transaction
                    .execute(
                        "INSERT INTO alarm_events (alarm_id, device_id, action, minutes, occurred_at)
                         VALUES ($1, $2, 'snooze', $3, $4)",
                        &[&alarm_id, &request.device_id, &minutes, &request.at],
                    )
                    .await?;
                let next_fire_at = Utc::now() + Duration::minutes(i64::from(minutes));
                transaction
                    .execute(
                        acknowledge_alarm_generation_query(),
                        &[&series_id, &generation],
                    )
                    .await?;
                transaction
                    .execute(
                        "UPDATE alarms SET cancellation_confirmed_at = now(), updated_at = now()
                     WHERE id = $1",
                        &[&alarm_id],
                    )
                    .await?;
                let next_series_id = format!("snooze-{}", Uuid::new_v4().simple());
                let next_generation = transaction
                    .query_opt(
                        "UPDATE user_runtime_states
                     SET alarm_generation = GREATEST(alarm_generation, $2) + 1
                     WHERE user_id = (SELECT user_id FROM devices WHERE device_id = $1)
                     RETURNING alarm_generation",
                        &[&request.device_id, &generation],
                    )
                    .await?
                    .map(|row| row.get::<_, i64>("alarm_generation"))
                    .unwrap_or(generation + 1);
                let kind_value: String = alarm_row.get("kind");
                let kind = AlarmKind::parse(&kind_value).unwrap_or(AlarmKind::Test);
                let severity: String = alarm_row.get("severity");
                let title: String = alarm_row.get("title");
                let message: String = alarm_row.get("message");
                let hidden_buffer_applied: bool = alarm_row.get("hidden_buffer_applied");
                let requires_acknowledgement: bool = alarm_row.get("requires_acknowledgement");
                let devices = transaction
                    .query(
                        "SELECT device_id FROM devices
                     WHERE user_id = (SELECT user_id FROM devices WHERE device_id = $1)",
                        &[&request.device_id],
                    )
                    .await?;
                for device in devices {
                    let device_id: String = device.get("device_id");
                    let replacement_row = persist_alarm(
                        &*transaction,
                        &AlarmWrite {
                            id: format!("alarm_snooze_{}", Uuid::new_v4().simple()),
                            device_id: device_id.clone(),
                            kind,
                            series_id: next_series_id.clone(),
                            generation: next_generation,
                            severity: severity.clone(),
                            title: title.clone(),
                            message: message.clone(),
                            fire_at: next_fire_at,
                            hidden_buffer_applied,
                            requires_acknowledgement,
                            expires_at: Some(next_fire_at + Duration::hours(2)),
                        },
                    )
                    .await?;
                    if device_id == request.device_id {
                        replacement_alarm = Some(alarm_from_row(&replacement_row));
                    }
                }
                if replacement_alarm.is_none() {
                    return Err(AppError::BadRequest(
                        "snooze replacement device is no longer registered".to_string(),
                    ));
                }
                let replacement = replacement_alarm
                    .as_ref()
                    .expect("requesting device replacement was checked");
                let inserted = transaction
                    .execute(
                        persist_snooze_action_replay_query(),
                        &[
                            &alarm_id,
                            &parameters_hash,
                            &minutes,
                            &request.device_id,
                            &replacement.id,
                            &replacement.series_id,
                            &replacement.generation,
                        ],
                    )
                    .await?;
                if inserted != 1 {
                    return Err(AppError::Conflict(
                        "snooze replay identity was claimed concurrently; retry".to_string(),
                    ));
                }
                "pending".to_string()
            }
        }
        "scheduled" => {
            return Err(AppError::BadRequest(
                "use /alarms/reconcile with the delivery token to confirm scheduling".to_string(),
            ));
        }
        "ack" | "stop" | "dismiss" | "clear" => {
            transaction
                .execute(
                    acknowledge_alarm_generation_query(),
                    &[&series_id, &generation],
                )
                .await?;
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
        cancelled_series_ids: if action == "snooze"
            || matches!(action.as_str(), "ack" | "stop" | "dismiss" | "clear")
        {
            vec![series_id]
        } else {
            Vec::new()
        },
        replacement_alarm,
    }))
}

fn alarm_from_row(row: &Row) -> AlarmJob {
    let kind_value: String = row.get("kind");
    AlarmJob {
        id: row.get("id"),
        kind: AlarmKind::parse(&kind_value).unwrap_or(AlarmKind::Test),
        series_id: row.get("series_id"),
        generation: row.get("generation"),
        delivery_token: row.get("delivery_token"),
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

fn validate_onboarding_profile(name: &str, timezone: &str) -> AppResult<()> {
    validate_non_empty("name", name)?;
    if name.chars().count() > 120 {
        return Err(AppError::BadRequest(
            "name must be at most 120 characters".to_string(),
        ));
    }
    timezone.parse::<chrono_tz::Tz>().map_err(|_| {
        AppError::BadRequest("timezone must be a valid IANA identifier".to_string())
    })?;
    Ok(())
}

fn enforce_rate_limit(
    state: &AppState,
    headers: &HeaderMap,
    bucket: &str,
    requests_per_minute: u32,
) -> AppResult<()> {
    if state.rate_limiter.check(
        headers,
        bucket,
        requests_per_minute,
        StdDuration::from_secs(60),
    ) {
        return Ok(());
    }

    warn!(bucket, requests_per_minute, "request rate limit exceeded");
    Err(AppError::TooManyRequests)
}

fn normalize_pairing_code(code: &str) -> String {
    code.chars()
        .filter(|character| character.is_ascii_hexdigit())
        .flat_map(char::to_lowercase)
        .collect()
}

fn validate_pairing_code(code: &str) -> AppResult<()> {
    let normalized = normalize_pairing_code(code);
    if normalized.len() == 32 {
        Ok(())
    } else {
        Err(AppError::BadRequest(
            "pairing code must be a 32-character secure code".to_string(),
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
    let response = reqwest::Client::builder()
        .timeout(StdDuration::from_secs(15))
        .build()
        .map_err(|_| AppError::Upstream("Google token verification is unavailable".to_string()))?
        .get("https://oauth2.googleapis.com/tokeninfo")
        .query(&[("id_token", id_token)])
        .send()
        .await
        .map_err(|error| {
            warn!(error = %error, "🔴 FALLBACK: Google token verification failed - Reason: OAuth verification request could not complete - Impact: sign-in can be retried");
            AppError::Upstream("Google token verification failed".to_string())
        })?;

    if !response.status().is_success() {
        return Err(AppError::Unauthorized);
    }

    let token_info = response
        .json::<GoogleTokenInfo>()
        .await
        .map_err(|_| AppError::Upstream("Google token response was invalid".to_string()))?;

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

    #[derive(serde::Serialize)]
    struct VisitsResponse {
        count: i64,
    }

    Ok(Json(VisitsResponse { count }))
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
    require_admin_auth(&headers, &state.config)?;
    validate_byok_provider(req.byok_provider.as_deref())?;
    let target_user_id = req.user_id.unwrap_or_else(|| "admin".to_string());
    let client = state.pool.get().await?;

    if req
        .active_until_days
        .is_some_and(|days| !(-36_500..=36_500).contains(&days))
    {
        return Err(AppError::BadRequest(
            "activeUntilDays must be between -36500 and 36500".to_string(),
        ));
    }
    let requested_active_until = req
        .active_until_days
        .map(|days| Utc::now() + Duration::days(days));
    let status = req.status.unwrap_or_else(|| "active".to_string());
    let encrypted_byok_key = req
        .byok_api_key
        .as_deref()
        .map(|value| encrypt_byok_key(&state.config, value))
        .transpose()?;
    let row = client
        .query_opt(
            "
            UPDATE users
            SET subscription_tier = $1,
                subscription_status = $2,
                byok_api_key = COALESCE($3, byok_api_key),
                byok_provider = COALESCE($4, byok_provider),
                subscription_active_until = COALESCE($5, subscription_active_until),
                updated_at = now()
            WHERE id = $6
            RETURNING subscription_tier, subscription_status, byok_provider, subscription_active_until
            ",
            &[
                &req.tier,
                &status,
                &encrypted_byok_key,
                &req.byok_provider,
                &requested_active_until,
                &target_user_id,
            ],
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

fn validate_byok_provider(provider: Option<&str>) -> AppResult<()> {
    if provider.is_some_and(|value| !matches!(value, "openai" | "gemini" | "openrouter")) {
        return Err(AppError::BadRequest(
            "byokProvider must be one of openai, gemini, or openrouter".to_string(),
        ));
    }
    Ok(())
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
        let content: String = row.get("content");
        let content = normalize_memory_content(&key, &content);
        Ok(Json(MemoryResponse {
            ok: true,
            key,
            content,
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

async fn create_memory_snapshot(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateMemorySnapshotRequest>,
) -> AppResult<Json<CreateMemorySnapshotResponse>> {
    let user_id = get_user_id_from_auth(&headers, &state.config, &state.pool).await?;
    let title = req
        .title
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "Manual memory snapshot".to_string());
    let reason = req
        .reason
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "manual".to_string());
    let client = state.pool.get().await?;
    let outcome =
        create_memory_snapshot_record(&client, &user_id, req.device_id.as_deref(), &title, &reason)
            .await?;

    Ok(Json(CreateMemorySnapshotResponse {
        ok: true,
        snapshot: snapshot_summary_response(outcome.snapshot),
        retained_count: outcome.retained_count,
        retention_limit: MEMORY_SNAPSHOT_LIMIT,
    }))
}

async fn get_memory_snapshots(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<ListMemorySnapshotsResponse>> {
    let user_id = get_user_id_from_auth(&headers, &state.config, &state.pool).await?;
    let client = state.pool.get().await?;
    let snapshots = list_memory_snapshots(&client, &user_id)
        .await?
        .into_iter()
        .map(snapshot_summary_response)
        .collect();

    Ok(Json(ListMemorySnapshotsResponse {
        ok: true,
        snapshots,
        retention_limit: MEMORY_SNAPSHOT_LIMIT,
    }))
}

async fn restore_memory_snapshot(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(snapshot_id): Path<String>,
    Json(req): Json<RestoreMemorySnapshotRequest>,
) -> AppResult<Json<RestoreMemorySnapshotResponse>> {
    let user_id = get_user_id_from_auth(&headers, &state.config, &state.pool).await?;
    let mut client = state.pool.get().await?;
    let outcome = restore_memory_snapshot_record(
        &mut client,
        &state.config,
        &user_id,
        &snapshot_id,
        req.restore_runtime_state.unwrap_or(true),
    )
    .await?;
    Ok(Json(RestoreMemorySnapshotResponse {
        ok: true,
        snapshot: snapshot_summary_response(outcome.snapshot),
        restored_memory_keys: outcome.restored_memory_keys,
        restored_runtime_state: outcome.restored_runtime_state,
    }))
}

fn snapshot_summary_response(snapshot: MemorySnapshotSummary) -> MemorySnapshotSummaryResponse {
    MemorySnapshotSummaryResponse {
        id: snapshot.id,
        device_id: snapshot.device_id,
        title: snapshot.title,
        reason: snapshot.reason,
        memory_keys: snapshot.memory_keys,
        runtime_state: snapshot.runtime_state,
        created_at: snapshot.created_at,
    }
}

async fn chat_coach(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ChatRequest>,
) -> AppResult<Json<ChatResponse>> {
    validate_chat_request(&req)?;
    enforce_rate_limit(&state, &headers, "coach_chat_ip", 60)?;
    let user_id = get_user_id_from_auth(&headers, &state.config, &state.pool).await?;
    if !state
        .rate_limiter
        .check_key("coach_chat_user", &user_id, 30, StdDuration::from_secs(60))
    {
        warn!(user_id, "authenticated chat rate limit exceeded");
        return Err(AppError::TooManyRequests);
    }
    let _chat_permit = state
        .chat_concurrency
        .clone()
        .try_acquire_owned()
        .map_err(|_| {
            warn!(user_id, "global chat provider concurrency limit exceeded");
            AppError::TooManyRequests
        })?;
    let reply = chat_with_coach(
        &state.pool,
        &state.config,
        &user_id,
        &req.message,
        &req.request_id,
    )
    .await?;
    let client = state.pool.get().await?;
    let runtime_state = runtime_state_response_payload(&client, &user_id).await?;
    Ok(Json(ChatResponse {
        ok: true,
        reply,
        runtime_state,
    }))
}

async fn chat_history(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<ChatHistoryResponse>> {
    let user_id = get_user_id_from_auth(&headers, &state.config, &state.pool).await?;
    let client = state.pool.get().await?;
    let rows = client
        .query(
            "
            SELECT id, role, content, created_at
            FROM (
                SELECT id, role, content, created_at
                FROM chat_messages
                WHERE user_id = $1
                    AND is_visible = TRUE
                    AND role IN ('user', 'assistant')
                    AND content IS NOT NULL
                    AND length(trim(content)) > 0
                ORDER BY created_at DESC, id DESC
                LIMIT 100
            ) newest
            ORDER BY created_at ASC, id ASC
            ",
            &[&user_id],
        )
        .await?;
    let messages = rows
        .into_iter()
        .map(|row| ChatHistoryMessage {
            id: row.get("id"),
            role: row.get("role"),
            content: row.get("content"),
            created_at: row.get("created_at"),
        })
        .collect();
    Ok(Json(ChatHistoryResponse { ok: true, messages }))
}

fn validate_chat_request(req: &ChatRequest) -> AppResult<()> {
    validate_non_empty("message", &req.message)?;
    if req.message.chars().count() > MAX_CHAT_MESSAGE_CHARS {
        return Err(AppError::BadRequest(format!(
            "message must be at most {MAX_CHAT_MESSAGE_CHARS} characters"
        )));
    }
    validate_non_empty("requestId", &req.request_id)?;
    if req.request_id.chars().count() > MAX_CHAT_REQUEST_ID_CHARS {
        return Err(AppError::BadRequest(format!(
            "requestId must be at most {MAX_CHAT_REQUEST_ID_CHARS} characters"
        )));
    }
    if Uuid::parse_str(&req.request_id).is_err() {
        return Err(AppError::BadRequest("requestId must be a UUID".to_string()));
    }
    Ok(())
}

async fn get_runtime_state(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<RuntimeStateResponse>> {
    let user_id = get_user_id_from_auth(&headers, &state.config, &state.pool).await?;
    let client = state.pool.get().await?;
    Ok(Json(RuntimeStateResponse {
        ok: true,
        runtime_state: runtime_state_response_payload(&client, &user_id).await?,
    }))
}

async fn get_stats(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<StatsResponse>> {
    let user_id = get_user_id_from_auth(&headers, &state.config, &state.pool).await?;
    let client = state.pool.get().await?;
    let rows = client
        .query(
            "
            SELECT memory_key, content
            FROM user_memories
            WHERE user_id = $1
                AND (memory_key LIKE 'work_log_%' OR memory_key = 'tasks')
            ORDER BY memory_key ASC
            ",
            &[&user_id],
        )
        .await?;

    let now = Utc::now();
    let today = user_day_for(&**client, &user_id, now).await?.current_date();
    let week_start = today - Duration::days(today.weekday().num_days_from_monday() as i64);
    let month_start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1)
        .expect("valid first day of current month");

    let runtime_row = client
        .query_opt(
            "
            SELECT state, entered_at
            FROM user_runtime_states
            WHERE user_id = $1
            ",
            &[&user_id],
        )
        .await?;
    let runtime_state: Option<String> = runtime_row.as_ref().map(|row| row.get("state"));
    let runtime_entered_at: Option<DateTime<Utc>> =
        runtime_row.as_ref().map(|row| row.get("entered_at"));

    let mut day_logs = Vec::new();
    let mut checked_tasks_total = 0;
    for row in rows {
        let key: String = row.get("memory_key");
        let content: String = row.get("content");
        if key == "tasks" {
            checked_tasks_total = count_checked_tasks(&content);
        } else if let Some(date) = work_log_date(&key) {
            day_logs.push(parse_day_log(date, &content));
        }
    }

    if runtime_state.as_deref() == Some("idle") {
        if let Some(entered_at) = runtime_entered_at {
            for log in &mut day_logs {
                if log.date == today {
                    log.idle_minutes += (now - entered_at).num_minutes().max(0);
                }
            }
        }
    }

    let today_stats = aggregate_stats("Today", &day_logs, today, today);
    let week_stats = aggregate_stats("This week", &day_logs, week_start, today);
    let month_stats = aggregate_stats("This month", &day_logs, month_start, today);

    Ok(Json(StatsResponse {
        ok: true,
        generated_at: now,
        today: today_stats,
        week: week_stats,
        month: month_stats,
        checked_tasks_total,
        note: "Stats are derived from work logs. Idle time is estimated from gaps between logged sessions and the current idle runtime state.".to_string(),
    }))
}

async fn create_report(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateReportRequest>,
) -> AppResult<Json<CreateReportResponse>> {
    let user_id = get_user_id_from_auth(&headers, &state.config, &state.pool).await?;
    validate_non_empty("title", &req.title)?;
    validate_non_empty("reportMarkdown", &req.report_markdown)?;
    if req.report_markdown.chars().count() > 250_000 {
        return Err(AppError::BadRequest(
            "Report is too large. Trim the captured window and try again.".to_string(),
        ));
    }
    if req.events.len() > 1_000 {
        return Err(AppError::BadRequest(
            "Report has too many events. Trim the captured window and try again.".to_string(),
        ));
    }

    let report_id = Uuid::new_v4().to_string();
    let events = serde_json::to_string(&req.events).map_err(|err| {
        AppError::BadRequest(format!("Failed to serialize report events: {}", err))
    })?;
    let client = state.pool.get().await?;
    client
        .execute(
            "
            INSERT INTO user_reports (
                id,
                user_id,
                device_id,
                title,
                window_start,
                window_end,
                report_markdown,
                events
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8::TEXT::JSONB)
            ",
            &[
                &report_id,
                &user_id,
                &req.device_id,
                &req.title,
                &req.window_start,
                &req.window_end,
                &req.report_markdown,
                &events,
            ],
        )
        .await?;

    info!(
        report_id = %report_id,
        user_id = %user_id,
        event_count = req.events.len(),
        "saved user report"
    );

    Ok(Json(CreateReportResponse {
        ok: true,
        report_id,
        saved_at: Utc::now(),
    }))
}

async fn transcribe_speech(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> AppResult<Json<SpeechTranscriptionResponse>> {
    let user_id = get_user_id_from_auth(&headers, &state.config, &state.pool).await?;
    ensure_paid_provider_entitlement(&state.pool, &user_id).await?;
    enforce_rate_limit(&state, &headers, "speech_stt_ip", 30)?;
    let api_key = state
        .config
        .speech
        .smallest_api_key
        .as_deref()
        .ok_or_else(|| {
            AppError::BadRequest("Smallest speech-to-text is not configured".to_string())
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

        if bytes.len() > MAX_AUDIO_UPLOAD_BYTES {
            return Err(AppError::BadRequest(
                "audio upload is too large; keep voice notes under 25MB".to_string(),
            ));
        }

        uploaded_file = Some((file_name, content_type, bytes.to_vec()));
        break;
    }

    let (file_name, content_type, bytes) = uploaded_file
        .ok_or_else(|| AppError::BadRequest("multipart field `file` is required".to_string()))?;
    if !state
        .rate_limiter
        .check_key("speech_stt_user", &user_id, 20, StdDuration::from_secs(60))
    {
        return Err(AppError::TooManyRequests);
    }
    reserve_daily_provider_usage(
        &state.pool,
        &user_id,
        "speech_stt_bytes",
        bytes.len() as i64,
        state.config.speech_daily_stt_bytes,
    )
    .await?;
    let _speech_permit = state
        .speech_concurrency
        .clone()
        .try_acquire_owned()
        .map_err(|_| AppError::TooManyRequests)?;
    info!(
        file_name,
        content_type,
        byte_count = bytes.len(),
        "transcribing VAD speech segment with Smallest Pulse HTTP STT"
    );
    let text = transcribe_smallest_prerecorded(
        &state.config.speech.smallest_stt_url,
        api_key,
        content_type,
        bytes,
    )
    .await?;
    if text.is_empty() {
        warn!(
            "🔴 FALLBACK: Smallest transcription empty - Reason: provider returned no usable text - Impact: user must type or retry voice input"
        );
        return Err(AppError::BadRequest(
            "Smallest returned an empty transcription".to_string(),
        ));
    }

    Ok(Json(SpeechTranscriptionResponse { ok: true, text }))
}

async fn synthesize_speech(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<SpeechSynthesisRequest>,
) -> AppResult<Json<SpeechSynthesisResponse>> {
    let user_id = get_user_id_from_auth(&headers, &state.config, &state.pool).await?;
    ensure_paid_provider_entitlement(&state.pool, &user_id).await?;
    enforce_rate_limit(&state, &headers, "speech_tts_ip", 60)?;
    validate_non_empty("text", &req.text)?;

    if req.text.chars().count() > 1_200 {
        return Err(AppError::BadRequest(
            "speech synthesis text must be 1200 characters or less".to_string(),
        ));
    }
    if !state
        .rate_limiter
        .check_key("speech_tts_user", &user_id, 40, StdDuration::from_secs(60))
    {
        return Err(AppError::TooManyRequests);
    }
    reserve_daily_provider_usage(
        &state.pool,
        &user_id,
        "speech_tts_chars",
        req.text.chars().count() as i64,
        state.config.speech_daily_tts_chars,
    )
    .await?;
    let _speech_permit = state
        .speech_concurrency
        .clone()
        .try_acquire_owned()
        .map_err(|_| AppError::TooManyRequests)?;

    let api_key = state
        .config
        .speech
        .inworld_api_key
        .as_deref()
        .ok_or_else(|| {
            AppError::BadRequest("Inworld text-to-speech is not configured".to_string())
        })?;
    let voice_id = req
        .voice_id
        .or_else(|| state.config.speech.inworld_tts_voice_id.clone())
        .ok_or_else(|| {
            AppError::BadRequest(
                "INWORLD_TTS_VOICE_ID is required for speech synthesis".to_string(),
            )
        })?;
    let url = format!(
        "{}/tts/v1/voice:stream",
        state.config.speech.inworld_base_url.trim_end_matches('/')
    );
    let payload = json!({
        "text": req.text,
        "voiceId": voice_id,
        "modelId": state.config.speech.inworld_tts_model.clone(),
        "audioConfig": {
            "audioEncoding": "MP3",
            "sampleRateHertz": 44100
        }
    });

    let http_client = reqwest::Client::builder()
        .timeout(StdDuration::from_secs(45))
        .build()
        .map_err(|err| {
            warn!(error = %err, "🔴 FALLBACK: Inworld transport failed - Reason: HTTP client could not be built - Impact: coach reply remains readable text-only and speech can be retried");
            AppError::BadRequest("Inworld speech synthesis transport is unavailable".to_string())
        })?;
    let response = http_client
        .post(&url)
        .header("Authorization", format!("Basic {api_key}"))
        .json(&payload)
        .send()
        .await
        .map_err(|err| {
            warn!(error = %err, "🔴 FALLBACK: Inworld transport failed - Reason: provider request could not be sent - Impact: coach reply remains readable text-only and speech can be retried");
            AppError::BadRequest("Inworld speech synthesis request failed".to_string())
        })?;
    let status = response.status();
    if !status.is_success() {
        let bytes = response.bytes().await.map_err(|err| {
            warn!(status = status.as_u16(), error = %err, "🔴 FALLBACK: Inworld response body read failed - Reason: provider error response could not be read - Impact: coach reply remains readable text-only and speech can be retried");
            AppError::BadRequest(format!(
                "Inworld speech synthesis failed with HTTP {} and an unreadable response",
                status.as_u16()
            ))
        })?;
        let _body_was_read_and_bounded = bytes.len();
        warn!(
            status = status.as_u16(),
            "🔴 FALLBACK: Inworld provider returned non-success - Reason: provider rejected the synthesis request - Impact: coach reply remains readable text-only and speech can be retried"
        );
        return Err(AppError::Upstream(format!(
            "speech synthesis provider returned HTTP {}",
            status.as_u16()
        )));
    }
    let bytes = collect_inworld_streamed_audio(response).await?;
    if bytes.is_empty() {
        warn!(
            "🔴 FALLBACK: Inworld speech output empty - Reason: provider stream contained no usable audio - Impact: coach reply remains readable text-only and speech can be retried"
        );
        return Err(AppError::BadRequest(
            "Inworld returned an empty speech synthesis stream".to_string(),
        ));
    }

    Ok(Json(SpeechSynthesisResponse {
        ok: true,
        audio_base64: BASE64_STANDARD.encode(bytes),
        content_type: "audio/mpeg".to_string(),
    }))
}

async fn ensure_paid_provider_entitlement(
    pool: &deadpool_postgres::Pool,
    user_id: &str,
) -> AppResult<()> {
    if user_id == "admin" {
        return Ok(());
    }
    let client = pool.get().await?;
    let active = client
        .query_opt(
            "SELECT subscription_status='active'
                    OR subscription_active_until > now() AS active
             FROM users WHERE id=$1",
            &[&user_id],
        )
        .await?
        .is_some_and(|row| row.get::<_, bool>("active"));
    if !active {
        return Err(AppError::Unauthorized);
    }
    Ok(())
}

async fn reserve_daily_provider_usage(
    pool: &deadpool_postgres::Pool,
    user_id: &str,
    usage_kind: &str,
    units: i64,
    limit: i64,
) -> AppResult<()> {
    let client = pool.get().await?;
    let reserved = client
        .query_opt(
            "INSERT INTO provider_usage_daily (user_id,usage_date,usage_kind,units)
             VALUES ($1,CURRENT_DATE,$2,$3)
             ON CONFLICT (user_id,usage_date,usage_kind) DO UPDATE
             SET units=provider_usage_daily.units + EXCLUDED.units, updated_at=now()
             WHERE provider_usage_daily.units + EXCLUDED.units <= $4
             RETURNING units",
            &[&user_id, &usage_kind, &units, &limit],
        )
        .await?;
    if reserved.is_none() {
        warn!(
            user_id,
            usage_kind, units, limit, "paid provider daily quota exceeded"
        );
        return Err(AppError::TooManyRequests);
    }
    Ok(())
}

async fn transcribe_smallest_prerecorded(
    base_url: &str,
    api_key: &str,
    content_type: String,
    bytes: Vec<u8>,
) -> AppResult<String> {
    let url = smallest_prerecorded_stt_url(base_url);
    let http_client = reqwest::Client::builder()
        .timeout(StdDuration::from_secs(45))
        .build()
        .map_err(|err| {
            warn!(error = %err, "🔴 FALLBACK: Smallest transport failed - Reason: HTTP client could not be built - Impact: user must type or retry voice input");
            AppError::BadRequest("Smallest speech transcription transport is unavailable".to_string())
        })?;
    let response = http_client
        .post(&url)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", content_type)
        .body(bytes)
        .send()
        .await
        .map_err(|err| {
            warn!(error = %err, "🔴 FALLBACK: Smallest transport failed - Reason: provider request could not be sent - Impact: user must type or retry voice input");
            AppError::BadRequest("Smallest speech transcription request failed".to_string())
        })?;
    let status = response.status();
    let body = response.text().await.map_err(|err| {
        warn!(status = status.as_u16(), error = %err, "🔴 FALLBACK: Smallest response body read failed - Reason: provider response could not be read - Impact: user must type or retry voice input");
        AppError::BadRequest("Smallest transcription response could not be read".to_string())
    })?;
    if !status.is_success() {
        warn!(
            status = status.as_u16(),
            "🔴 FALLBACK: Smallest provider returned non-success - Reason: provider rejected the transcription request - Impact: user must type or retry voice input"
        );
        return Err(AppError::Upstream(format!(
            "speech transcription provider returned HTTP {}",
            status.as_u16()
        )));
    }

    let value: Value = serde_json::from_str(&body).map_err(|err| {
        warn!(error = %err, "🔴 FALLBACK: Smallest response JSON invalid - Reason: provider returned malformed transcription JSON - Impact: user must type or retry voice input");
        AppError::BadRequest("Smallest returned invalid transcription JSON".to_string())
    })?;
    let text = value
        .get("transcription")
        .or_else(|| value.get("text"))
        .or_else(|| value.get("full_transcript"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    Ok(text)
}

fn smallest_prerecorded_stt_url(base_url: &str) -> String {
    let separator = if base_url.contains('?') { '&' } else { '?' };
    format!("{base_url}{separator}model=pulse&language=en")
}

async fn collect_inworld_streamed_audio(response: reqwest::Response) -> AppResult<Vec<u8>> {
    let mut stream = response.bytes_stream();
    let mut pending = String::new();
    let mut audio = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|err| {
            warn!(error = %err, "🔴 FALLBACK: Inworld stream read failed - Reason: provider audio stream could not be read - Impact: coach reply remains readable text-only and speech can be retried");
            AppError::BadRequest("Inworld speech synthesis stream could not be read".to_string())
        })?;
        if chunk.len() > MAX_INWORLD_STREAM_BUFFER_BYTES.saturating_sub(pending.len()) {
            warn!(
                "🔴 FALLBACK: Inworld stream frame too large - Reason: provider stream exceeded the bounded parser buffer - Impact: coach reply remains readable text-only and speech can be retried"
            );
            return Err(AppError::BadRequest(
                "Inworld speech synthesis stream frame is too large".to_string(),
            ));
        }
        pending.push_str(&String::from_utf8_lossy(&chunk));
        drain_inworld_stream_values(&mut pending, &mut audio)?;
    }
    drain_inworld_stream_values(&mut pending, &mut audio)?;
    Ok(audio)
}

fn drain_inworld_stream_values(pending: &mut String, audio: &mut Vec<u8>) -> AppResult<()> {
    if pending.len() > MAX_INWORLD_STREAM_BUFFER_BYTES {
        warn!(
            "🔴 FALLBACK: Inworld stream frame too large - Reason: provider stream exceeded the bounded parser buffer - Impact: coach reply remains readable text-only and speech can be retried"
        );
        return Err(AppError::BadRequest(
            "Inworld speech synthesis stream frame is too large".to_string(),
        ));
    }
    loop {
        let trimmed = pending.trim_start();
        let removed = pending.len() - trimmed.len();
        if removed > 0 {
            pending.drain(..removed);
        }
        if pending.is_empty() {
            return Ok(());
        }

        let mut stream = serde_json::Deserializer::from_str(pending).into_iter::<Value>();
        match stream.next() {
            Some(Ok(value)) => {
                let offset = stream.byte_offset();
                handle_inworld_stream_value(value, audio)?;
                pending.drain(..offset);
            }
            Some(Err(err)) if err.is_eof() => return Ok(()),
            Some(Err(err)) => {
                warn!(error = %err, "🔴 FALLBACK: Inworld stream JSON invalid - Reason: provider returned malformed streamed JSON - Impact: coach reply remains readable text-only and speech can be retried");
                return Err(AppError::BadRequest(format!(
                    "Inworld returned invalid speech synthesis stream JSON: {err}"
                )));
            }
            None => return Ok(()),
        }
    }
}

fn handle_inworld_stream_value(value: Value, audio: &mut Vec<u8>) -> AppResult<()> {
    if let Some(error) = value.get("error") {
        warn!(
            "🔴 FALLBACK: Inworld provider stream error - Reason: provider returned an explicit streamed error - Impact: coach reply remains readable text-only and speech can be retried"
        );
        return Err(AppError::BadRequest(format!(
            "Inworld speech synthesis stream error: {}",
            error.to_string().chars().take(300).collect::<String>()
        )));
    }

    let audio_content = value
        .get("result")
        .and_then(|result| result.get("audioContent"))
        .or_else(|| value.get("audioContent"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !audio_content.is_empty() {
        let bytes = BASE64_STANDARD.decode(audio_content).map_err(|err| {
            warn!(error = %err, "🔴 FALLBACK: Inworld stream audio invalid - Reason: provider returned invalid base64 audio - Impact: coach reply remains readable text-only and speech can be retried");
            AppError::BadRequest(format!("Inworld returned invalid audioContent: {err}"))
        })?;
        if bytes.len() > MAX_SYNTHESIZED_AUDIO_BYTES.saturating_sub(audio.len()) {
            warn!(
                "🔴 FALLBACK: Inworld speech output too large - Reason: provider audio exceeded the bounded response limit - Impact: coach reply remains readable text-only and speech can be retried"
            );
            return Err(AppError::BadRequest(
                "Inworld speech synthesis output is too large".to_string(),
            ));
        }
        audio.extend(bytes);
    }
    Ok(())
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
    runtime_state: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TestToolRequest {
    user_id: Option<String>,
    name: String,
    args: Option<Value>,
    failure_after_canonical: Option<bool>,
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TestMemoryRequest {
    user_id: String,
    date: Option<NaiveDate>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TestAlarmWakeSeedRequest {
    user_id: String,
    device_id: String,
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
    alarm_wake_outbox_count: i64,
    alarm_wake_pending_count: i64,
    alarm_wake_in_progress_count: i64,
    alarm_wake_completed_count: i64,
    alarm_wake_attempt_count: i64,
    memory_index_pending_count: i64,
    memory_index_completed_count: i64,
    sleep_sample_count: i32,
    distilled_dates: Vec<String>,
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
    let runtime_state = match req.runtime_state.as_deref().unwrap_or("onboarding") {
        "onboarding" => "onboarding",
        "idle" => "idle",
        "working" => "working",
        "break" => "break",
        "sleeping" => "sleeping",
        "vacation" => "vacation",
        _ => {
            return Err(AppError::BadRequest(
                "invalid test runtime state".to_string(),
            ))
        }
    };
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
        .execute(
            "DELETE FROM memory_index_jobs WHERE user_id = $1",
            &[&user_id],
        )
        .await?;
    client
        .execute(
            "DELETE FROM memory_index_states WHERE user_id = $1",
            &[&user_id],
        )
        .await?;
    client
        .execute("DELETE FROM memory_chunks WHERE user_id = $1", &[&user_id])
        .await?;
    client
        .execute(
            "DELETE FROM memory_distillations WHERE user_id = $1",
            &[&user_id],
        )
        .await?;
    client
        .execute(
            "DELETE FROM user_state_metrics WHERE user_id = $1",
            &[&user_id],
        )
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
            VALUES ($1, $2, 'test_reset', '{}'::JSONB)
            ON CONFLICT (user_id) DO UPDATE SET
                state = EXCLUDED.state,
                entered_at = now(),
                source_tool = 'test_reset',
                metadata = '{}'::JSONB
            ",
            &[&user_id, &runtime_state],
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

async fn test_memory_invariants(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<TestMemoryRequest>,
) -> AppResult<Json<Value>> {
    require_test_endpoints_enabled()?;
    require_admin_auth(&headers, &state.config)?;
    let mut client = state.pool.get().await?;
    Ok(Json(
        run_memory_db_invariant_probe(&mut client, &request.user_id).await?,
    ))
}

async fn test_memory_activation_race(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<TestMemoryRequest>,
) -> AppResult<Json<Value>> {
    require_test_endpoints_enabled()?;
    require_admin_auth(&headers, &state.config)?;
    Ok(Json(
        run_memory_activation_race_probe(&state.pool, &state.config, &request.user_id).await?,
    ))
}

async fn test_memory_distill(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<TestMemoryRequest>,
) -> AppResult<Json<crate::memory::DistillationOutcome>> {
    require_test_endpoints_enabled()?;
    require_admin_auth(&headers, &state.config)?;
    let date = request
        .date
        .ok_or_else(|| AppError::BadRequest("date is required".to_string()))?;
    let client = state.pool.get().await?;
    Ok(Json(
        distill_date_for_test(&client, &state.config, &request.user_id, date).await?,
    ))
}

async fn test_alarm_wake_seed(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<TestAlarmWakeSeedRequest>,
) -> AppResult<Json<TestStateResponse>> {
    require_test_endpoints_enabled()?;
    require_admin_auth(&headers, &state.config)?;
    let mut client = state.pool.get().await?;
    let transaction = client.transaction().await?;
    transaction
        .execute(
            "UPDATE devices SET push_provider='apns', push_token='test-apns-token', updated_at=now()
             WHERE device_id=$1 AND user_id=$2",
            &[&request.device_id, &request.user_id],
        )
        .await?;
    for suffix in ["pending", "expired"] {
        let alarm_id = format!("test-worker-alarm-{suffix}-{}", Uuid::new_v4());
        let series_id = format!("test-worker-series-{suffix}-{}", Uuid::new_v4());
        persist_alarm(
            &*transaction,
            &AlarmWrite {
                id: alarm_id,
                device_id: request.device_id.clone(),
                kind: AlarmKind::Test,
                series_id,
                generation: 1,
                severity: "normal".to_string(),
                title: "Background wake worker probe".to_string(),
                message: "Test-only autonomous outbox processing probe".to_string(),
                fire_at: Utc::now() + Duration::minutes(5),
                hidden_buffer_applied: false,
                requires_acknowledgement: true,
                expires_at: None,
            },
        )
        .await?;
    }
    transaction
        .execute(
            "UPDATE alarm_wake_outbox
             SET status='in_progress', lease_token='expired-test-lease',
                 lease_expires_at=now() - interval '1 minute'
             WHERE device_id=$1 AND id LIKE 'wake:test-worker-series-expired-%'",
            &[&request.device_id],
        )
        .await?;
    transaction.commit().await?;
    let client = state.pool.get().await?;
    Ok(Json(
        test_snapshot(&client, &request.user_id, &request.device_id).await?,
    ))
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
    let sleep_metrics = sleep_metrics_report(&**client, &user_id).await?;

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
    let (ok, result) = run_tool_for_test(
        &state.pool,
        &state.config,
        &user_id,
        &req.name,
        req.args.unwrap_or(Value::Null),
        req.failure_after_canonical.unwrap_or(false),
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
        ok,
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
    let alarm_wake_counts = client
        .query_one(
            "SELECT
                 COUNT(*)::BIGINT AS total,
                 COUNT(*) FILTER (WHERE status='pending')::BIGINT AS pending,
                 COUNT(*) FILTER (WHERE status='in_progress')::BIGINT AS in_progress,
                 COUNT(*) FILTER (WHERE status='completed')::BIGINT AS completed,
                 COALESCE(SUM(attempts),0)::BIGINT AS attempts
             FROM alarm_wake_outbox WHERE device_id = $1",
            &[&device_id],
        )
        .await?;
    let index_counts = client
        .query_one(
            "SELECT
               COUNT(*) FILTER (WHERE status IN ('pending','in_progress'))::BIGINT AS pending,
               COUNT(*) FILTER (WHERE status='completed')::BIGINT AS completed
             FROM memory_index_jobs WHERE user_id=$1",
            &[&user_id],
        )
        .await?;
    let sleep_sample_count = client
        .query_opt(
            "SELECT sleep_sample_count FROM user_state_metrics WHERE user_id=$1",
            &[&user_id],
        )
        .await?
        .map(|row| row.get("sleep_sample_count"))
        .unwrap_or(0);
    let distilled_dates = client
        .query(
            "SELECT distilled_date::TEXT AS date FROM memory_distillations WHERE user_id=$1 ORDER BY distilled_date",
            &[&user_id],
        )
        .await?
        .into_iter()
        .map(|row| row.get("date"))
        .collect();

    Ok(TestStateResponse {
        ok: true,
        user_id: user_id.to_string(),
        device_id: device_id.to_string(),
        runtime_state,
        alarm_wake_outbox_count: alarm_wake_counts.get("total"),
        alarm_wake_pending_count: alarm_wake_counts.get("pending"),
        alarm_wake_in_progress_count: alarm_wake_counts.get("in_progress"),
        alarm_wake_completed_count: alarm_wake_counts.get("completed"),
        alarm_wake_attempt_count: alarm_wake_counts.get("attempts"),
        memory_index_pending_count: index_counts.get("pending"),
        memory_index_completed_count: index_counts.get("completed"),
        sleep_sample_count,
        distilled_dates,
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

async fn runtime_state_response_payload(
    client: &tokio_postgres::Client,
    user_id: &str,
) -> AppResult<Option<RuntimeStateResponsePayload>> {
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
        .map(|row| RuntimeStateResponsePayload {
            state: row.get("state"),
            source_tool: row.get("source_tool"),
            metadata: row.get("metadata"),
        }))
}

#[derive(Debug, Clone)]
struct DayStats {
    date: NaiveDate,
    work_minutes: i64,
    idle_minutes: i64,
    unproductive_desk_minutes: i64,
    sessions_completed: i64,
    session_starts: Vec<DateTime<Utc>>,
    session_ends: Vec<DateTime<Utc>>,
}

fn work_log_date(key: &str) -> Option<NaiveDate> {
    let suffix = key.strip_prefix("work_log_")?;
    NaiveDate::parse_from_str(suffix, "%Y_%m_%d").ok()
}

fn parse_day_log(date: NaiveDate, content: &str) -> DayStats {
    let mut stats = DayStats {
        date,
        work_minutes: 0,
        idle_minutes: 0,
        unproductive_desk_minutes: 0,
        sessions_completed: 0,
        session_starts: Vec::new(),
        session_ends: Vec::new(),
    };

    for line in content.lines() {
        if line.contains("session_start:") {
            if let Some(at) = parse_timestamp_after_at(line) {
                stats.session_starts.push(at);
            }
        } else if line.contains("session_end:") {
            let actual = parse_number_before(line, " actual mins").unwrap_or(0);
            let productivity = parse_number_after(line, "productivity level ")
                .unwrap_or(100)
                .clamp(0, 100);
            stats.work_minutes += actual;
            stats.unproductive_desk_minutes += actual * (100 - productivity) / 100;
            stats.sessions_completed += 1;
            if let Some(at) = parse_timestamp_after_at(line) {
                stats.session_ends.push(at);
            }
        }
    }

    stats.session_starts.sort();
    stats.session_ends.sort();
    stats.idle_minutes += idle_gaps_between_sessions(&stats.session_starts, &stats.session_ends);
    stats
}

fn aggregate_stats(
    label: &str,
    logs: &[DayStats],
    start: NaiveDate,
    end: NaiveDate,
) -> StatsPeriodResponse {
    let mut work_minutes = 0;
    let mut idle_minutes = 0;
    let mut unproductive_desk_minutes = 0;
    let mut sessions_completed = 0;
    for log in logs {
        if log.date < start || log.date > end {
            continue;
        }
        work_minutes += log.work_minutes;
        idle_minutes += log.idle_minutes;
        unproductive_desk_minutes += log.unproductive_desk_minutes;
        sessions_completed += log.sessions_completed;
    }

    StatsPeriodResponse {
        label: label.to_string(),
        work_minutes,
        idle_minutes,
        unproductive_desk_minutes,
        sessions_completed,
        tasks_done: sessions_completed,
    }
}

fn idle_gaps_between_sessions(starts: &[DateTime<Utc>], ends: &[DateTime<Utc>]) -> i64 {
    let mut idle = 0;
    for end in ends {
        if let Some(next_start) = starts.iter().find(|start| *start > end) {
            idle += (*next_start - *end).num_minutes().clamp(0, 24 * 60);
        }
    }
    idle
}

fn count_checked_tasks(content: &str) -> i64 {
    content
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            trimmed.starts_with("- [x]") || trimmed.starts_with("- [X]")
        })
        .count() as i64
}

fn parse_timestamp_after_at(line: &str) -> Option<DateTime<Utc>> {
    let marker = " at ";
    let value = line.rsplit_once(marker)?.1.trim();
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|date| date.with_timezone(&Utc))
}

fn parse_number_before(line: &str, marker: &str) -> Option<i64> {
    let before = line.split_once(marker)?.0;
    before
        .split_whitespace()
        .rev()
        .find_map(|part| part.parse::<i64>().ok())
}

fn parse_number_after(line: &str, marker: &str) -> Option<i64> {
    let after = line.split_once(marker)?.1;
    let digits = after
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    digits.parse::<i64>().ok()
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
            cancel_alarm_generations_by_kind_query(),
            &[&request.device_id, &request.kind],
        )
        .await? as i64;

    Ok(Json(CancelAlarmsResponse { ok: true, count }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn onboarding_profile_requires_name_and_valid_iana_timezone() {
        assert!(validate_onboarding_profile("Mehul", "Asia/Kolkata").is_ok());
        assert!(validate_onboarding_profile("", "Asia/Kolkata").is_err());
        assert!(validate_onboarding_profile("Mehul", "IST").is_err());
    }

    #[test]
    fn pending_alarm_delivery_uses_retryable_leases() {
        let query = lease_pending_alarms_query();
        assert!(
            query.contains("delivery_lease_expires_at <= now()"),
            "{query}"
        );
        assert!(query.contains("delivery_token"), "{query}");
        assert!(query.contains("FOR UPDATE SKIP LOCKED"), "{query}");
        assert!(!query.contains("status = 'delivered'"), "{query}");
    }

    #[test]
    fn scheduling_confirmation_is_fenced_by_delivery_token() {
        let query = confirm_scheduled_alarm_query();
        assert!(query.contains("delivery_token = $3"), "{query}");
        assert!(query.contains("status = 'scheduled'"), "{query}");
        assert!(query.contains("scheduled_local_id"), "{query}");
        assert!(query.contains("scheduled_local_id = $4"), "{query}");
        assert!(query.contains("status = 'leased'"), "{query}");
    }

    #[test]
    fn admin_cancellation_creates_tombstones_for_every_active_delivery_state() {
        let query = cancel_alarm_generations_by_kind_query();
        assert!(query.contains("status = 'cancelled'"), "{query}");
        assert!(
            query.contains("'pending', 'leased', 'scheduled'"),
            "{query}"
        );
        assert!(
            query.contains("cancellation_confirmed_at = NULL"),
            "{query}"
        );
        assert!(!query.contains("DELETE FROM alarms"), "{query}");
    }

    #[test]
    fn snooze_replay_is_durable_and_parameter_fenced() {
        let load = snooze_action_replay_query();
        assert!(load.contains("alarm_action_replays"), "{load}");
        assert!(load.contains("FOR UPDATE"), "{load}");
        assert!(load.contains("duration_minutes"), "{load}");

        let persist = persist_snooze_action_replay_query();
        assert!(persist.contains("replacement_alarm_id"), "{persist}");
        assert!(persist.contains("replacement_series_id"), "{persist}");
        assert!(persist.contains("replacement_generation"), "{persist}");
        assert!(persist.contains("ON CONFLICT"), "{persist}");
    }

    #[test]
    fn acknowledgement_cancels_the_whole_alarm_generation() {
        let query = acknowledge_alarm_generation_query();
        assert!(query.contains("series_id"), "{query}");
        assert!(query.contains("generation"), "{query}");
        assert!(query.contains("status = 'cancelled'"), "{query}");
        assert!(!query.contains("kind ="), "{query}");
    }

    #[test]
    fn chat_request_rejects_empty_and_oversized_messages() {
        let empty = ChatRequest {
            message: "   ".to_string(),
            request_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        };
        assert!(matches!(
            validate_chat_request(&empty),
            Err(AppError::BadRequest(_))
        ));

        let oversized = ChatRequest {
            message: "x".repeat(MAX_CHAT_MESSAGE_CHARS + 1),
            request_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        };
        assert!(matches!(
            validate_chat_request(&oversized),
            Err(AppError::BadRequest(_))
        ));
    }

    #[test]
    fn chat_request_accepts_bounded_idempotency_key() {
        let request = ChatRequest {
            message: "Start the focused task.".to_string(),
            request_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        };
        assert!(validate_chat_request(&request).is_ok());
    }

    #[test]
    fn chat_request_rejects_non_uuid_idempotency_key() {
        let request = ChatRequest {
            message: "Start the focused task.".to_string(),
            request_id: "mobile-turn-123".to_string(),
        };
        let error = validate_chat_request(&request).expect_err("requestId must be a UUID");
        assert!(error.to_string().contains("requestId must be a UUID"));
    }

    #[test]
    fn current_time_ist_formats_utc_time_with_ist_offset() {
        let now = DateTime::parse_from_rfc3339("2026-06-29T19:40:05Z")
            .unwrap()
            .with_timezone(&Utc);

        assert_eq!(current_time_ist(now), "2026-06-30T01:10:05+05:30");
    }

    #[test]
    fn legacy_alias_telemetry_matches_only_real_unversioned_routes() {
        for path in [
            "/",
            "/health",
            "/auth/session",
            "/auth/me",
            "/auth/logout",
            "/auth/google",
            "/profile/onboarding",
            "/pairing/claim",
            "/devices/register",
            "/workspaces/main/devices",
            "/alarms",
            "/alarms/pending",
            "/alarms/reconcile",
            "/alarms/cancel",
            "/alarms/example/ack",
            "/visits",
            "/subscription",
            "/memory/snapshots",
            "/memory/snapshots/snapshot-id/restore",
            "/memory/tasks",
            "/admin/context",
            "/chat",
            "/chat/history",
            "/state",
            "/stats",
            "/reports",
            "/speech/transcribe",
            "/speech/synthesize",
            "/test/reset",
            "/test/tool",
            "/test/state",
            "/test/context",
        ] {
            assert!(is_legacy_alias_path(path), "expected legacy route: {path}");
        }
        for path in [
            "/v1/health",
            "/healthz",
            "/health/foo",
            "/chatty",
            "/chat/arbitrary",
            "/auth",
            "/auth/session/nested",
            "/workspaces/main",
            "/workspaces/main/devices/nested",
            "/alarms/example",
            "/alarms/example/ack/nested",
            "/memory",
            "/memory/snapshots/id",
            "/memory/snapshots/id/restore/nested",
            "/speech",
            "/speech/transcribe/nested",
            "/test/arbitrary",
            "/state/nested",
        ] {
            assert!(!is_legacy_alias_path(path), "expected non-route: {path}");
        }
    }

    #[test]
    fn parse_day_log_sums_work_idle_and_unproductive_minutes() {
        let date = NaiveDate::from_ymd_opt(2026, 7, 5).unwrap();
        let log = "\
# Work Log
- session_start: Build stats page (estimated 30 mins) at 2026-07-05T08:00:00Z
- session_end: 20 actual mins, productivity level 75% at 2026-07-05T08:20:00Z
- session_start: Fix tabs (estimated 15 mins) at 2026-07-05T08:50:00Z
- session_end: 10 actual mins, productivity level 100% at 2026-07-05T09:00:00Z
";

        let parsed = parse_day_log(date, log);

        assert_eq!(parsed.work_minutes, 30);
        assert_eq!(parsed.unproductive_desk_minutes, 5);
        assert_eq!(parsed.idle_minutes, 30);
        assert_eq!(parsed.sessions_completed, 2);
    }

    #[test]
    fn inworld_audio_stream_rejects_output_over_limit() {
        let mut audio = vec![0; MAX_SYNTHESIZED_AUDIO_BYTES];
        let value = json!({ "audioContent": BASE64_STANDARD.encode([1_u8]) });

        let result = handle_inworld_stream_value(value, &mut audio);

        assert!(
            matches!(result, Err(AppError::BadRequest(message)) if message.contains("too large"))
        );
    }

    #[test]
    fn inworld_stream_rejects_unbounded_incomplete_json() {
        let mut pending = "x".repeat(MAX_INWORLD_STREAM_BUFFER_BYTES + 1);
        let mut audio = Vec::new();

        let result = drain_inworld_stream_values(&mut pending, &mut audio);

        assert!(
            matches!(result, Err(AppError::BadRequest(message)) if message.contains("too large"))
        );
    }

    #[test]
    fn speech_failure_logs_cover_all_provider_branches() {
        let source = include_str!("routes.rs");
        for reason in [
            "Smallest transport failed",
            "Smallest response body read failed",
            "Smallest provider returned non-success",
            "Smallest response JSON invalid",
            "Smallest transcription empty",
            "Inworld transport failed",
            "Inworld response body read failed",
            "Inworld provider returned non-success",
            "Inworld stream read failed",
            "Inworld stream JSON invalid",
            "Inworld speech output empty",
        ] {
            assert!(
                source.contains(&format!("🔴 FALLBACK: {reason} - Reason:")),
                "missing fallback log for {reason}"
            );
        }
    }
}
