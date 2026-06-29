use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
    pub kind: Option<String>,
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
    pub kind: String,
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

    pub fn normalized_kind(&self) -> String {
        self.kind.clone().unwrap_or_else(|| "test".to_string())
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
pub struct ChatRequest {
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatResponse {
    pub ok: bool,
    pub reply: String,
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
