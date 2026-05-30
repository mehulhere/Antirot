use std::env;
use std::net::SocketAddr;

use anyhow::{Context, Result};

#[derive(Clone, Debug)]
pub struct Config {
    pub bind: SocketAddr,
    pub database_url: String,
    pub admin_token: String,
    pub device_token: String,
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

        Ok(Self {
            bind,
            database_url,
            admin_token,
            device_token,
        })
    }
}
