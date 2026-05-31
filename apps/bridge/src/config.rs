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

        Ok(Self {
            bind,
            database_url,
            admin_token,
            device_token,
            google_allowed_client_ids,
        })
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
