use std::fs;

use anyhow::{Context, Result};
use chrono::Utc;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{info, warn};

use crate::config::{ApnsConfig, Config};

#[derive(Debug, Serialize, Deserialize)]
struct ApnsClaims {
    iss: String,
    iat: i64,
}

pub async fn send_alarm_wake(config: &Config, push_token: &str, alarm_id: &str) -> Result<()> {
    let Some(apns) = config.apns.as_ref() else {
        warn!(
            alarm_id,
            "🔴 FALLBACK: APNs wake skipped - Reason: APNs env is not configured - Impact: iOS app must poll/open before the alarm is scheduled"
        );
        return Ok(());
    };

    let jwt = build_provider_token(apns)?;
    let url = format!("{}/3/device/{}", apns.endpoint, push_token);
    let payload = json!({
        "aps": {
            "content-available": 1
        },
        "reason": "alarm_queued",
        "alarmId": alarm_id
    });

    let response = reqwest::Client::new()
        .post(url)
        .bearer_auth(jwt)
        .header("apns-topic", &apns.topic)
        .header("apns-push-type", "background")
        .header("apns-priority", "5")
        .json(&payload)
        .send()
        .await
        .context("failed to send APNs wake request")?;

    let status = response.status();
    if status.is_success() {
        info!(alarm_id, "sent APNs background wake");
        return Ok(());
    }

    let body = response
        .text()
        .await
        .unwrap_or_else(|_| "<unreadable>".to_string());
    warn!(
        alarm_id,
        status = %status,
        body = %body,
        "🔴 FALLBACK: APNs wake failed - Reason: Apple rejected or failed the push - Impact: iOS app must poll/open before the alarm is scheduled"
    );
    Ok(())
}

fn build_provider_token(config: &ApnsConfig) -> Result<String> {
    let mut header = Header::new(Algorithm::ES256);
    header.kid = Some(config.key_id.clone());

    let claims = ApnsClaims {
        iss: config.team_id.clone(),
        iat: Utc::now().timestamp(),
    };

    let pem = match (&config.private_key_pem, &config.private_key_path) {
        (Some(value), _) => value.replace("\\n", "\n"),
        (None, Some(path)) => fs::read_to_string(path)
            .with_context(|| format!("failed to read APNs private key at {path}"))?,
        (None, None) => anyhow::bail!("APNs private key is missing"),
    };

    let key = EncodingKey::from_ec_pem(pem.as_bytes()).context("invalid APNs EC private key")?;
    encode(&header, &claims, &key).context("failed to sign APNs provider token")
}
