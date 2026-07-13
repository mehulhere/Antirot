use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlarmKind {
    NormalWake,
    LoudWake,
    RoutineOverdue,
    SessionOverdue,
    NonResponse,
    SessionAlarm,
    BreakAlarm,
    WakeAlarm,
    IdleAlarm,
    Test,
}

impl AlarmKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NormalWake => "normal_wake",
            Self::LoudWake => "loud_wake",
            Self::RoutineOverdue => "routine_overdue",
            Self::SessionOverdue => "session_overdue",
            Self::NonResponse => "non_response",
            Self::SessionAlarm => "session_alarm",
            Self::BreakAlarm => "break_alarm",
            Self::WakeAlarm => "wake_alarm",
            Self::IdleAlarm => "idle_alarm",
            Self::Test => "test",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "normal_wake" => Some(Self::NormalWake),
            "loud_wake" => Some(Self::LoudWake),
            "routine_overdue" => Some(Self::RoutineOverdue),
            "session_overdue" => Some(Self::SessionOverdue),
            "non_response" => Some(Self::NonResponse),
            "session_alarm" => Some(Self::SessionAlarm),
            "break_alarm" => Some(Self::BreakAlarm),
            "wake_alarm" => Some(Self::WakeAlarm),
            "idle_alarm" => Some(Self::IdleAlarm),
            "test" => Some(Self::Test),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceRegistrationRequest {
    pub device_id: String,
    pub platform: String,
    pub app_version: String,
    pub notification_capability: String,
    pub usage_capability: String,
    pub push_provider: Option<String>,
    pub push_token: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceRegistrationResponse {
    pub ok: bool,
    pub device_id: String,
    pub device_token: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleAuthRequest {
    pub id_token: String,
    pub device_id: String,
    pub platform: String,
    pub app_version: Option<String>,
    pub notification_capability: Option<String>,
    pub usage_capability: Option<String>,
    pub push_provider: Option<String>,
    pub push_token: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleAuthResponse {
    pub ok: bool,
    pub user_id: String,
    pub device_id: String,
    pub device_token: String,
    pub email: String,
    pub name: Option<String>,
    pub message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionRequest {
    pub device_id: Option<String>,
    pub platform: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionResponse {
    pub ok: bool,
    pub user_id: String,
    pub device_id: String,
    pub device_token: Option<String>,
    pub expires_in_days: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthMeResponse {
    pub ok: bool,
    pub user_id: String,
    pub device_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingProfileRequest {
    pub name: String,
    pub timezone: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingProfileResponse {
    pub ok: bool,
    pub name: String,
    pub timezone: String,
    pub reply: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingClaimRequest {
    pub code: String,
    pub device_id: String,
    pub device_name: Option<String>,
    pub platform: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingClaimResponse {
    pub ok: bool,
    pub workspace_id: String,
    pub device_id: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceDevice {
    pub device_id: String,
    pub device_name: Option<String>,
    pub platform: String,
    pub notification_capability: String,
    pub paired_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceDevicesResponse {
    pub ok: bool,
    pub workspace_id: String,
    pub devices: Vec<WorkspaceDevice>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAlarmRequest {
    pub id: Option<String>,
    pub device_id: String,
    pub kind: Option<AlarmKind>,
    pub severity: Option<String>,
    pub title: String,
    pub message: String,
    pub fire_at: DateTime<Utc>,
    pub hidden_buffer_applied: Option<bool>,
    pub requires_acknowledgement: Option<bool>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlarmJob {
    pub id: String,
    pub kind: AlarmKind,
    pub series_id: String,
    pub generation: i64,
    pub delivery_token: Option<String>,
    pub severity: String,
    pub title: String,
    pub message: String,
    pub fire_at: DateTime<Utc>,
    pub hidden_buffer_applied: bool,
    pub requires_acknowledgement: bool,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlarmActionRequest {
    pub device_id: String,
    pub action: String,
    pub at: DateTime<Utc>,
    pub minutes: Option<i32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlarmActionResponse {
    pub ok: bool,
    pub alarm_id: String,
    pub status: String,
    pub cancelled_series_ids: Vec<String>,
    pub replacement_alarm: Option<AlarmJob>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlarmCancellationTombstone {
    pub series_id: String,
    pub local_alarm_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingAlarmsResponse {
    pub alarms: Vec<AlarmJob>,
    pub cancelled_series_ids: Vec<String>,
    pub cancelled_alarm_ids: Vec<String>,
    pub cancellations: Vec<AlarmCancellationTombstone>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledAlarmConfirmation {
    pub alarm_id: String,
    pub delivery_token: String,
    pub local_alarm_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlarmReconcileRequest {
    pub device_id: String,
    pub scheduled: Vec<ScheduledAlarmConfirmation>,
    pub cancelled_series_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlarmReconcileResponse {
    pub ok: bool,
    pub scheduled_count: i64,
    pub cancellation_count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAlarmResponse {
    pub ok: bool,
    pub alarm: AlarmJob,
    pub delivery: DeliveryState,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryState {
    pub mode: String,
    pub status: String,
}

#[cfg(test)]
mod alarm_contract_tests {
    use super::*;

    #[test]
    fn canonical_alarm_kinds_serialize_to_mobile_contract_values() {
        let cases = [
            (AlarmKind::NormalWake, "normal_wake"),
            (AlarmKind::LoudWake, "loud_wake"),
            (AlarmKind::RoutineOverdue, "routine_overdue"),
            (AlarmKind::SessionOverdue, "session_overdue"),
            (AlarmKind::NonResponse, "non_response"),
            (AlarmKind::SessionAlarm, "session_alarm"),
            (AlarmKind::BreakAlarm, "break_alarm"),
            (AlarmKind::WakeAlarm, "wake_alarm"),
            (AlarmKind::IdleAlarm, "idle_alarm"),
            (AlarmKind::Test, "test"),
        ];

        for (kind, expected) in cases {
            assert_eq!(
                serde_json::to_string(&kind).unwrap(),
                format!("\"{expected}\"")
            );
            assert_eq!(kind.as_str(), expected);
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    pub ok: bool,
    pub service: &'static str,
    pub current_time_ist: String,
}

impl CreateAlarmRequest {
    pub fn normalized_id(&self) -> String {
        self.id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string())
    }

    pub fn normalized_kind(&self) -> AlarmKind {
        self.kind.unwrap_or(AlarmKind::Test)
    }

    pub fn normalized_severity(&self) -> String {
        self.severity
            .clone()
            .unwrap_or_else(|| "normal".to_string())
    }
}

// Antirot Standalone Chat, Memory, and Subscription Models
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionUpdateRequest {
    pub user_id: Option<String>,
    pub tier: String,
    pub status: Option<String>,
    pub byok_api_key: Option<String>,
    pub byok_provider: Option<String>,
    pub active_until_days: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionResponse {
    pub ok: bool,
    pub tier: String,
    pub status: String,
    pub byok_provider: Option<String>,
    pub active_until: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMemoryRequest {
    pub content: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryResponse {
    pub ok: bool,
    pub key: String,
    pub content: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMemorySnapshotRequest {
    pub device_id: Option<String>,
    pub title: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemorySnapshotSummaryResponse {
    pub id: String,
    pub device_id: Option<String>,
    pub title: String,
    pub reason: String,
    pub memory_keys: Vec<String>,
    pub runtime_state: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMemorySnapshotResponse {
    pub ok: bool,
    pub snapshot: MemorySnapshotSummaryResponse,
    pub retained_count: i64,
    pub retention_limit: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListMemorySnapshotsResponse {
    pub ok: bool,
    pub snapshots: Vec<MemorySnapshotSummaryResponse>,
    pub retention_limit: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreMemorySnapshotRequest {
    pub restore_runtime_state: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreMemorySnapshotResponse {
    pub ok: bool,
    pub snapshot: MemorySnapshotSummaryResponse,
    pub restored_memory_keys: Vec<String>,
    pub restored_runtime_state: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatRequest {
    pub message: String,
    pub request_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatResponse {
    pub ok: bool,
    pub reply: String,
    pub runtime_state: Option<RuntimeStateResponsePayload>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStateResponsePayload {
    pub state: String,
    pub source_tool: Option<String>,
    pub metadata: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStateResponse {
    pub ok: bool,
    pub runtime_state: Option<RuntimeStateResponsePayload>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsPeriodResponse {
    pub label: String,
    pub work_minutes: i64,
    pub idle_minutes: i64,
    pub unproductive_desk_minutes: i64,
    pub sessions_completed: i64,
    pub tasks_done: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsResponse {
    pub ok: bool,
    pub generated_at: DateTime<Utc>,
    pub today: StatsPeriodResponse,
    pub week: StatsPeriodResponse,
    pub month: StatsPeriodResponse,
    pub checked_tasks_total: i64,
    pub note: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatHistoryMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatHistoryResponse {
    pub ok: bool,
    pub messages: Vec<ChatHistoryMessage>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportEvent {
    pub at: DateTime<Utc>,
    pub kind: String,
    pub summary: String,
    pub detail: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateReportRequest {
    pub device_id: Option<String>,
    pub title: String,
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
    pub report_markdown: String,
    pub events: Vec<ReportEvent>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateReportResponse {
    pub ok: bool,
    pub report_id: String,
    pub saved_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeechTranscriptionResponse {
    pub ok: bool,
    pub text: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeechSynthesisRequest {
    pub text: String,
    pub voice_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeechSynthesisResponse {
    pub ok: bool,
    pub audio_base64: String,
    pub content_type: String,
}
