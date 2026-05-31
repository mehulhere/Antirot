use axum::http::HeaderMap;
use deadpool_postgres::Pool;
use sha2::{Digest, Sha256};

use crate::config::Config;
use crate::error::{AppError, AppResult};

pub fn require_admin_auth(headers: &HeaderMap, config: &Config) -> AppResult<()> {
    let token = bearer_token(headers).ok_or(AppError::Unauthorized)?;
    if constant_time_eq(token, &config.admin_token) {
        Ok(())
    } else {
        Err(AppError::Unauthorized)
    }
}

pub async fn require_device_auth(
    headers: &HeaderMap,
    config: &Config,
    pool: &Pool,
) -> AppResult<()> {
    let token = bearer_token(headers).ok_or(AppError::Unauthorized)?;
    if constant_time_eq(token, &config.device_token) || constant_time_eq(token, &config.admin_token)
    {
        return Ok(());
    }

    let client = pool.get().await?;
    let token_hash = token_hash(token);
    let exists = client
        .query_opt(
            "SELECT 1 FROM devices WHERE api_token_hash = $1",
            &[&token_hash],
        )
        .await?
        .is_some();
    if exists {
        Ok(())
    } else {
        Err(AppError::Unauthorized)
    }
}

pub async fn require_device_auth_for(
    headers: &HeaderMap,
    config: &Config,
    pool: &Pool,
    device_id: &str,
) -> AppResult<()> {
    let token = bearer_token(headers).ok_or(AppError::Unauthorized)?;
    if constant_time_eq(token, &config.device_token) || constant_time_eq(token, &config.admin_token)
    {
        return Ok(());
    }

    let client = pool.get().await?;
    let token_hash = token_hash(token);
    let matches = client
        .query_opt(
            "SELECT 1 FROM devices WHERE api_token_hash = $1 AND device_id = $2",
            &[&token_hash, &device_id],
        )
        .await?
        .is_some();
    if matches {
        Ok(())
    } else {
        Err(AppError::Unauthorized)
    }
}

pub fn token_hash(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    let value = headers
        .get(axum::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?;
    value.strip_prefix("Bearer ").map(str::trim)
}

fn constant_time_eq(left: &str, right: &str) -> bool {
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
