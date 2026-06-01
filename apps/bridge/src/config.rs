use std::env;
use std::net::SocketAddr;

use anyhow::{Context, Result};

#[derive(Clone, Debug)]
pub struct Config {
    pub bind: SocketAddr,
    pub database_url: String,
    pub admin_token: String,
    pub device_token: String,
    pub google_allowed_client_ids: Vec<String>,
    pub apns: Option<ApnsConfig>,
}

#[derive(Clone, Debug)]
pub struct ApnsConfig {
    pub team_id: String,
    pub key_id: String,
    pub private_key_path: Option<String>,
    pub private_key_pem: Option<String>,
    pub topic: String,
    pub endpoint: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let bind = env::var("ANTIROT_BRIDGE_BIND")
            .unwrap_or_else(|_| "127.0.0.1:8787".to_string())
            .parse()
            .context("ANTIROT_BRIDGE_BIND must be a socket address like 127.0.0.1:8787")?;
        let database_url = env::var("DATABASE_URL")
            .context("DATABASE_URL is required, for example postgres://antirot:secret@localhost/antirot_bridge")?;
        let admin_token =
            env::var("ANTIROT_ADMIN_TOKEN").context("ANTIROT_ADMIN_TOKEN is required")?;
        let device_token =
            env::var("ANTIROT_DEVICE_TOKEN").context("ANTIROT_DEVICE_TOKEN is required")?;
        let google_allowed_client_ids = google_allowed_client_ids();
        let apns = apns_config();

        Ok(Self {
            bind,
            database_url,
            admin_token,
            device_token,
            google_allowed_client_ids,
            apns,
        })
    }
}

fn apns_config() -> Option<ApnsConfig> {
    let team_id = env::var("ANTIROT_APNS_TEAM_ID").ok()?;
    let key_id = env::var("ANTIROT_APNS_KEY_ID").ok()?;
    let private_key_path = env::var("ANTIROT_APNS_PRIVATE_KEY_PATH")
        .ok()
        .filter(|value| !value.trim().is_empty());
    let private_key_pem = env::var("ANTIROT_APNS_PRIVATE_KEY_PEM")
        .ok()
        .filter(|value| !value.trim().is_empty());
    let topic =
        env::var("ANTIROT_APNS_TOPIC").unwrap_or_else(|_| "com.mehulhere.Antirot".to_string());
    let environment = env::var("ANTIROT_APNS_ENV")
        .unwrap_or_else(|_| "sandbox".to_string())
        .to_ascii_lowercase();
    let endpoint = apns_endpoint_for_environment(&environment).to_string();

    Some(ApnsConfig {
        team_id,
        key_id,
        private_key_path,
        private_key_pem,
        topic,
        endpoint,
    })
}

fn apns_endpoint_for_environment(environment: &str) -> &'static str {
    match environment {
        "production" | "prod" => "https://api.push.apple.com",
        _ => "https://api.sandbox.push.apple.com",
    }
}

fn google_allowed_client_ids() -> Vec<String> {
    let mut values = Vec::new();
    for key in [
        "GOOGLE_ALLOWED_CLIENT_IDS",
        "GOOGLE_IOS_CLIENT_ID",
        "GOOGLE_ANDROID_CLIENT_ID",
        "GOOGLE_WEB_CLIENT_ID",
    ] {
        if let Ok(raw) = env::var(key) {
            values.extend(
                raw.split(',')
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToOwned::to_owned),
            );
        }
    }
    values.sort();
    values.dedup();
    values
}

#[cfg(test)]
mod tests {
    use super::apns_endpoint_for_environment;

    #[test]
    fn apns_endpoint_uses_sandbox_by_default_for_unknown_values() {
        assert_eq!(
            apns_endpoint_for_environment("sandbox"),
            "https://api.sandbox.push.apple.com"
        );
        assert_eq!(
            apns_endpoint_for_environment("development"),
            "https://api.sandbox.push.apple.com"
        );
        assert_eq!(
            apns_endpoint_for_environment(""),
            "https://api.sandbox.push.apple.com"
        );
    }

    #[test]
    fn apns_endpoint_accepts_production_aliases() {
        assert_eq!(
            apns_endpoint_for_environment("production"),
            "https://api.push.apple.com"
        );
        assert_eq!(
            apns_endpoint_for_environment("prod"),
            "https://api.push.apple.com"
        );
    }
}
